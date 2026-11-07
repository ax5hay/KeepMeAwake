//! The Tauri shell: wires the engine to commands, the tray icon and the window.
//! Kept thin on purpose; nothing here is worth unit testing beyond the pure
//! helpers in [`tray`].

pub mod commands;
pub mod tray;
pub mod window;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{Target, TargetKind};

use crate::core::settings::StartupPlan;
use crate::core::{PlanRequest, Settings};
use crate::engine::{Engine, EngineEvent, Platform, default_plan};
use crate::persistence;
use crate::platform::SystemClock;

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
            commands::update_settings,
            commands::quit_app,
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
    let config_dir = app.path().app_config_dir()?;
    let settings_path = persistence::settings_path(&config_dir);
    let settings = persistence::load(&settings_path);
    log::info!("settings loaded from {}", settings_path.display());

    let engine = Arc::new(Engine::start(
        make_platform(),
        settings.clone(),
        app.package_info().version.to_string(),
    ));
    apply_startup_plan(&engine, &settings);

    app.manage(AppState {
        engine: Arc::clone(&engine),
        settings_path,
    });
    tray::create(app)?;

    // Push every engine change to the tray and to the UI (if it is open).
    let handle = app.clone();
    engine.subscribe(Box::new(move |event| match event {
        EngineEvent::Snapshot(snapshot) => {
            tray::refresh(&handle, snapshot);
            if let Err(err) = handle.emit(SNAPSHOT_EVENT, snapshot) {
                log::debug!("no UI to receive snapshot: {err}");
            }
        }
        EngineEvent::TimerEnded => log::info!("timed session ended"),
        EngineEvent::Paused(reason) => log::info!("session paused: {reason:?}"),
        EngineEvent::Resumed => log::info!("session resumed"),
        EngineEvent::ScheduleStarted { .. } => log::info!("schedule window opened"),
        EngineEvent::ScheduleEnded { .. } => log::info!("schedule window closed"),
    }));

    let launched_hidden = std::env::args().any(|a| a == HIDDEN_FLAG);
    if !(launched_hidden || settings.start_hidden) {
        window::open_at_startup(app)?;
    }
    Ok(())
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

#[cfg(not(windows))]
fn make_platform() -> Platform {
    use crate::platform::{AlwaysOnMains, NullPowerGuard};
    Platform {
        guard: Box::new(NullPowerGuard),
        battery: Box::new(AlwaysOnMains),
        clock: Arc::new(SystemClock),
    }
}
