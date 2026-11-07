//! IPC surface exposed to the UI. Each command is a thin adapter over the engine;
//! the return value is always the fresh snapshot so the UI never guesses.

// Tauri's command macro requires owned `State` and `AppHandle` arguments.
#![allow(clippy::needless_pass_by_value)]

use tauri::{AppHandle, State};

use super::AppState;
use crate::core::{PlanRequest, Settings, Snapshot};
use crate::persistence;

/// Current state. Also nudges the engine to re-read the battery, since this is
/// called whenever the window opens.
#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.engine.refresh();
    state.engine.snapshot()
}

/// Change the plan (off, indefinite, for N seconds, until a time).
#[tauri::command]
pub fn set_plan(state: State<'_, AppState>, request: PlanRequest) -> Snapshot {
    state.engine.set_plan(request)
}

/// Flip between off and the default plan.
#[tauri::command]
pub fn toggle_session(state: State<'_, AppState>) -> Snapshot {
    state.engine.toggle()
}

/// Rest for the remainder of the open schedule window.
#[tauri::command]
pub fn skip_schedule(state: State<'_, AppState>) -> Snapshot {
    state.engine.skip_today()
}

/// Undo a skip: the open schedule window holds the laptop awake again.
#[tauri::command]
pub fn resume_schedule(state: State<'_, AppState>) -> Snapshot {
    state.engine.unskip()
}

/// Persist and apply new settings. Fails only if the file cannot be written;
/// the engine still receives the new values in that case.
#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> Result<Snapshot, String> {
    let settings = settings.sanitized();
    let snapshot = state.engine.update_settings(settings.clone());
    persistence::save(&state.settings_path, &settings).map_err(|err| {
        log::error!("could not save settings: {err}");
        format!("Settings applied but could not be saved: {err}")
    })?;
    Ok(snapshot)
}

/// Exit the application completely (not just hide to tray).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
