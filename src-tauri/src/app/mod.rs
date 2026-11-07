//! The Tauri shell: wires the engine to commands, the tray icon and the window.
//! Kept thin on purpose; nothing here is worth unit testing beyond the pure
//! helpers in [`tray`].

pub mod commands;
pub mod history;
pub mod hotkey;
pub mod notify;
pub mod tray;
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
use crate::platform::{SystemClock, TaskbarTheme};

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
    /// Light or dark taskbar, for the tray icon set.
    pub taskbar: Box<dyn TaskbarTheme>,
}

/// Builds and runs the application. Blocks until exit.
pub fn run() {
    let app = tauri::Builder::default()
        // Must be first so a second launch is intercepted before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window::show_main(app.clone());
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_FLAG])))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(hotkey::plugin())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info })
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
            commands::skip_schedule,
            commands::resume_schedule,
            commands::update_settings,
            commands::quit_app,
            commands::get_history,
            commands::clear_history,
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
            if let Some(state) = app.try_state::<AppState>() {
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

    app.manage(AppState {
        engine: Arc::clone(&engine),
        settings_path,
        history,
        history_path,
        taskbar: make_taskbar(),
    });
    tray::create(app)?;
    watch_taskbar_theme(app, &engine);
    if let Err(err) = hotkey::apply(app, settings.hotkey.as_deref()) {
        log::warn!("hotkey not registered at launch: {err}");
    }

    let launched_hidden = std::env::args().any(|a| a == HIDDEN_FLAG);
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
                EngineEvent::SessionEnded { ended, .. } => log::info!("sun down: {ended:?}"),
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

#[cfg(windows)]
fn make_platform() -> Platform {
    use crate::platform::NullPowerGuard;
    use crate::platform::windows::{WindowsBattery, WindowsPowerGuard};

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
        clock: Arc::new(SystemClock),
    }
}

#[cfg(target_os = "macos")]
fn make_platform() -> Platform {
    use crate::platform::NullPowerGuard;
    use crate::platform::macos::{MacBattery, MacPowerGuard};

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
        clock: Arc::new(SystemClock),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn make_platform() -> Platform {
    use crate::platform::{AlwaysOnMains, NullPowerGuard};
    Platform {
        guard: Box::new(NullPowerGuard),
        battery: Box::new(AlwaysOnMains),
        clock: Arc::new(SystemClock),
    }
}
