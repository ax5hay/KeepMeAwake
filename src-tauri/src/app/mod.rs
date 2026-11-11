//! The Tauri shell: wires the engine to commands, the tray icon and the window.
//! Kept thin on purpose; nothing here is worth unit testing beyond the pure
//! helpers in [`tray`].

pub mod after_end;
pub mod ai;
pub mod almanac;
pub mod cli;
pub mod commands;
pub mod history;
pub mod hotkey;
pub mod notify;
pub mod tray;
pub mod update;
pub mod window;

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{Target, TargetKind};

use crate::core::history::{Ended, History};
use crate::core::settings::StartupPlan;
use crate::core::{PlanRequest, Settings};
use crate::engine::{Engine, EngineEvent, Listener, Platform, default_plan};
use crate::persistence;
use crate::platform::{IdleProbe, SystemClock, TaskbarTheme};

/// Command line flag the autostart entry passes so we boot straight to the tray.
pub const HIDDEN_FLAG: &str = "--hidden";

/// Event name the UI listens to for state updates.
pub const SNAPSHOT_EVENT: &str = "snapshot";

/// Shared state available to every command.
pub struct AppState {
    /// The engine, shared with the tray and event listeners.
    pub engine: Arc<Engine>,
    /// Where settings are persisted.
    pub settings_path: PathBuf,
    /// The session log, shared with the engine listener that writes it.
    pub history: Arc<Mutex<History>>,
    /// Where the session log is persisted.
    pub history_path: PathBuf,
    /// The almanac and the thread that keeps it.
    pub almanac: Arc<almanac::Keeper>,
    /// AI: the keys and the client. Asks nothing unless asked.
    pub ai: ai::Assistant,
    /// Light or dark taskbar, for the tray icon set.
    pub taskbar: Box<dyn TaskbarTheme>,
    /// A newer release, as last found by the daily check.
    pub update: Mutex<Option<update::UpdateInfo>>,
}

/// Builds and runs the application. Blocks until exit.
pub fn run() {
    // `--status`, `--help`, `--run` and mistakes are answered here.
    if let Some(code) = cli::early() {
        std::process::exit(code);
    }
    let app = tauri::Builder::default()
        // Must be first so a second launch is intercepted before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A command from a terminal is carried out, not a window opened.
            match (cli::command(&argv), app.try_state::<AppState>()) {
                (Some(command), Some(state)) => cli::apply(&state.engine, &command),
                _ => window::show_main(app.clone()),
            }
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_FLAG])))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(hotkey::plugin())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info })
                // The updater logs an offline check as an error; `app::update`
                // logs what matters itself, at debug for a check.
                .level_for("tauri_plugin_updater", log::LevelFilter::Off)
                .targets([
                    Target::new(TargetKind::LogDir { file_name: Some("keepmeawake".into()) }),
                    Target::new(TargetKind::Stdout),
                ])
                .max_file_size(256 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::set_plan,
            commands::toggle_session,
            commands::extend_session,
            commands::keep_it_up,
            commands::skip_schedule,
            commands::resume_schedule,
            commands::update_settings,
            commands::quit_app,
            commands::set_mini,
            commands::set_wide,
            commands::print_window,
            commands::get_history,
            commands::clear_history,
            commands::get_almanac,
            commands::mark_day,
            commands::fix_day,
            commands::clear_almanac,
            commands::ai_status,
            commands::set_ai_key,
            commands::ai_read_week,
            commands::ai_read_day,
            commands::ai_ask,
            commands::ai_plan,
            commands::get_update,
            commands::check_for_update,
            commands::install_update,
            commands::running_apps,
        ])
        .setup(|app| setup(app.handle()).map_err(Into::into))
        .build(tauri::generate_context!())
        .unwrap_or_else(|err| {
            log::error!("failed to build the application: {err}");
            std::process::exit(1)
        });

    app.run(|app, event| match event {
        // `code` is `None` when the last window closed; keep living in the tray.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Exit => {
            cli::forget();
            if let Some(state) = app.try_state::<AppState>() {
                state.almanac.flush();
                state.engine.shutdown();
            }
        }
        _ => {}
    });
}

fn setup(app: &AppHandle) -> tauri::Result<()> {
    // A menu bar app: no Dock icon, no Cmd-Tab entry. Set before any window
    // exists so the Dock never flashes one.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;

    let config_dir = app.path().app_config_dir()?;
    let settings_path = persistence::settings_path(&config_dir);
    let settings = persistence::load(&settings_path);
    log::info!("settings loaded from {}", settings_path.display());

    let history_path = persistence::history_path(&config_dir);
    let mut history = persistence::load_history(&history_path);
    // An entry still open means the last run never saw its stretch end: the
    // machine slept or shut down, or the app was killed. Either way it did
    // not hold, so it is logged as sleep without telling the cases apart.
    if history.mark_unfinished_as(Ended::Sleep) {
        save_history(&history_path, &history);
    }
    let history = Arc::new(Mutex::new(history));

    // Every engine change goes to the history log, the tray and the UI (if
    // it is open). Attached before the engine's first step so the log hears
    // a startup plan or a schedule window already open at launch.
    let listener: Listener = {
        let handle = app.clone();
        let history = Arc::clone(&history);
        let history_path = history_path.clone();
        Box::new(move |event| {
            {
                let mut log = history.lock();
                if history::record(&mut log, event, tray::now_unix_ms()) {
                    save_history(&history_path, &log);
                }
            }
            on_engine_event(&handle, event);
        })
    };
    let engine = Arc::new(Engine::start_with(
        make_platform(),
        settings.clone(),
        app.package_info().version.to_string(),
        vec![listener],
    ));
    apply_startup_plan(&engine, &settings);
    watch_sleep(&engine);
    // Launched by a command from a terminal: carry it out, quietly.
    let command = cli::command(&std::env::args().collect::<Vec<_>>());
    if let Some(command) = &command {
        cli::apply(&engine, command);
    }

    // The almanac is kept beside the engine, not by it: who is at the
    // machine is noted with the sun up or down alike.
    let almanac_path = persistence::almanac_path(&config_dir);
    let keeper = Arc::new(almanac::Keeper::new(
        persistence::load_almanac(&almanac_path),
        almanac_path,
        make_idle(),
        Arc::new(SystemClock),
        settings.almanac.enabled,
    ));
    keeper.set_break_after(settings.almanac.break_after_mins);
    {
        let handle = app.clone();
        keeper.start(Box::new(move |minutes| notify::break_due(&handle, minutes)));
    }

    app.manage(AppState {
        engine: Arc::clone(&engine),
        settings_path,
        history,
        history_path,
        almanac: keeper,
        ai: ai::Assistant::new(Box::new(ai::OsKeyStore)),
        taskbar: make_taskbar(),
        update: Mutex::new(None),
    });
    tray::create(app)?;
    watch_taskbar_theme(app, &engine);
    update::start(app);
    if let Err(err) = hotkey::apply(app, settings.hotkey.as_deref()) {
        log::warn!("hotkey not registered at launch: {err}");
    }

    let launched_hidden = std::env::args().any(|a| a == HIDDEN_FLAG) || command.is_some();
    if !(launched_hidden || settings.start_hidden) {
        window::open_at_startup(app)?;
    }
    Ok(())
}

/// Pushes an engine change to the tray, the UI and the notification centre.
fn on_engine_event(handle: &AppHandle, event: &EngineEvent) {
    match event {
        EngineEvent::Snapshot(snapshot) => {
            tray::refresh(handle, snapshot);
            cli::note(snapshot);
            if let Err(err) = handle.emit(SNAPSHOT_EVENT, snapshot) {
                log::debug!("no UI to receive snapshot: {err}");
            }
        }
        other => {
            match other {
                EngineEvent::TimerEnded { plan_secs } => {
                    log::info!("timed session of {plan_secs}s ended");
                }
                EngineEvent::Paused(reason, battery) => {
                    log::info!("session paused: {reason:?} at {battery:?}");
                }
                EngineEvent::Resumed => log::info!("session resumed"),
                EngineEvent::ScheduleStarted { .. } => log::info!("schedule window opened"),
                EngineEvent::ScheduleEnded { .. } => log::info!("schedule window closed"),
                EngineEvent::SessionStarted { mode, .. } => log::info!("sun up: {mode:?}"),
                EngineEvent::SessionEnded { ended, .. } => {
                    log::info!("sun down: {ended:?}");
                    after_end::on_ended(handle, *ended);
                }
                EngineEvent::SunsetSoon { secs_left, .. } => {
                    log::info!("the sun sets in {secs_left}s");
                }
                EngineEvent::OvernightAsk { idle_secs, .. } => {
                    log::info!("no input for {idle_secs}s; asking before resting");
                }
                EngineEvent::Snapshot(_) => {}
            }
            notify::on_event(handle, other);
        }
    }
}

/// Writes the log, logging rather than failing: history is never worth
/// interrupting anything for.
fn save_history(path: &std::path::Path, history: &History) {
    if let Err(err) = persistence::save_history(path, history) {
        log::warn!("could not save history: {err}");
    }
}

fn apply_startup_plan(engine: &Engine, settings: &Settings) {
    let request = match settings.startup_plan {
        StartupPlan::Off => return,
        StartupPlan::Indefinite => PlanRequest::Indefinite,
        StartupPlan::DefaultDuration => default_plan(settings),
    };
    engine.set_plan(request);
}

/// Tells the engine when the machine sleeps and wakes, so a stretch is
/// split at a sleep and a timer that ran out asleep is noticed at once.
fn watch_sleep(engine: &Arc<Engine>) {
    let engine = Arc::clone(engine);
    let on: Box<dyn Fn(bool) + Send + Sync> = Box::new(move |asleep| {
        let now = tray::now_unix_ms();
        if asleep {
            engine.slept(now);
        } else {
            engine.woke(now);
        }
    });
    #[cfg(windows)]
    let watched = crate::platform::windows::watch_sleep(on);
    #[cfg(target_os = "macos")]
    let watched = crate::platform::macos::watch_sleep(on);
    #[cfg(not(any(windows, target_os = "macos")))]
    let watched: Result<(), crate::platform::PlatformError> = {
        drop(on);
        Err(crate::platform::PlatformError::Unsupported)
    };
    if let Err(err) = watched {
        log::warn!("sleep and wake will not be noticed: {err}");
    }
}

#[cfg(windows)]
fn make_taskbar() -> Box<dyn TaskbarTheme> {
    Box::new(crate::platform::windows::WindowsTaskbarTheme)
}

#[cfg(not(windows))]
fn make_taskbar() -> Box<dyn TaskbarTheme> {
    Box::new(crate::platform::DarkTaskbar)
}

/// Swaps the tray icon set when Windows switches between a light and a dark
/// taskbar. The thread spends its life blocked in the kernel on a registry
/// notification; it wakes only when the Personalize key changes.
#[cfg(windows)]
fn watch_taskbar_theme(app: &AppHandle, engine: &Arc<Engine>) {
    use crate::platform::windows::wait_for_theme_change;

    let app = app.clone();
    let engine = Arc::clone(engine);
    let spawned = std::thread::Builder::new()
        .name("keepmeawake-theme".into())
        .spawn(move || {
            loop {
                if let Err(err) = wait_for_theme_change() {
                    log::warn!("stopped following the taskbar theme: {err}");
                    return;
                }
                log::debug!(
                    "personalisation changed; light taskbar: {}",
                    crate::platform::windows::taskbar_is_light()
                );
                tray::refresh(&app, &engine.snapshot());
            }
        });
    if let Err(err) = spawned {
        log::warn!("could not watch the taskbar theme: {err}");
    }
}

#[cfg(not(windows))]
fn watch_taskbar_theme(_app: &AppHandle, _engine: &Arc<Engine>) {}

/// The idle reading for the almanac: the same read-only call the engine's
/// overnight guard makes, through a probe of its own.
#[cfg(windows)]
fn make_idle() -> Box<dyn IdleProbe> {
    Box::new(crate::platform::windows::WindowsIdle)
}

#[cfg(target_os = "macos")]
fn make_idle() -> Box<dyn IdleProbe> {
    Box::new(crate::platform::macos::MacIdle)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn make_idle() -> Box<dyn IdleProbe> {
    Box::new(crate::platform::NoIdle)
}

#[cfg(windows)]
fn make_platform() -> Platform {
    use crate::platform::NullPowerGuard;
    use crate::platform::windows::{
        WindowsActivity, WindowsBattery, WindowsIdle, WindowsPowerGuard,
    };

    let guard: Box<dyn crate::platform::PowerGuard> = match WindowsPowerGuard::new() {
        Ok(guard) => Box::new(guard),
        Err(err) => {
            log::error!("power requests unavailable, the app will report failures: {err}");
            Box::new(NullPowerGuard)
        }
    };
    Platform {
        guard,
        battery: Box::new(WindowsBattery),
        idle: Box::new(WindowsIdle),
        activity: Box::new(WindowsActivity),
        clock: Arc::new(SystemClock),
    }
}

#[cfg(target_os = "macos")]
fn make_platform() -> Platform {
    use crate::platform::NullPowerGuard;
    use crate::platform::macos::{MacActivity, MacBattery, MacIdle, MacPowerGuard};

    let guard: Box<dyn crate::platform::PowerGuard> = match MacPowerGuard::new() {
        Ok(guard) => Box::new(guard),
        Err(err) => {
            log::error!("power assertions unavailable, the app will report failures: {err}");
            Box::new(NullPowerGuard)
        }
    };
    Platform {
        guard,
        battery: Box::new(MacBattery),
        idle: Box::new(MacIdle),
        activity: Box::new(MacActivity::new()),
        clock: Arc::new(SystemClock),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn make_platform() -> Platform {
    use crate::platform::{AlwaysOnMains, NoActivity, NoIdle, NullPowerGuard};
    Platform {
        guard: Box::new(NullPowerGuard),
        battery: Box::new(AlwaysOnMains),
        idle: Box::new(NoIdle),
        activity: Box::new(NoActivity),
        clock: Arc::new(SystemClock),
    }
}
