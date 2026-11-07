//! IPC surface exposed to the UI. Each command is a thin adapter over the engine;
//! the return value is always the fresh snapshot so the UI never guesses.

// Tauri's command macro requires owned `State` and `AppHandle` arguments.
#![allow(clippy::needless_pass_by_value)]

use tauri::{AppHandle, State};

use super::{AppState, hotkey};
use crate::core::history::History;
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

/// Give a timed session another 30 minutes. Does nothing otherwise.
#[tauri::command]
pub fn extend_session(state: State<'_, AppState>) -> Snapshot {
    state.engine.extend(crate::engine::EXTEND_BY)
}

/// Answer the overnight guard: someone is here, so it leaves this Always
/// session alone until it ends.
#[tauri::command]
pub fn keep_it_up(state: State<'_, AppState>) -> Snapshot {
    state.engine.keep_it_up()
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

/// Persist and apply new settings. Fails if the file cannot be written (the
/// engine still receives the new values) or if a new hotkey cannot be
/// registered (everything else is applied and saved; the hotkey keeps its
/// previous value).
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Snapshot, String> {
    let mut settings = settings.sanitized();
    let previous = state.engine.settings().hotkey;
    let mut hotkey_error = None;
    if settings.hotkey != previous
        && let Err(err) = hotkey::apply(&app, settings.hotkey.as_deref())
    {
        if let Err(again) = hotkey::apply(&app, previous.as_deref()) {
            log::warn!("could not restore the previous hotkey: {again}");
        }
        settings.hotkey = previous;
        hotkey_error = Some(err);
    }
    let snapshot = state.engine.update_settings(settings.clone());
    persistence::save(&state.settings_path, &settings).map_err(|err| {
        log::error!("could not save settings: {err}");
        format!("Settings applied but could not be saved: {err}")
    })?;
    hotkey_error.map_or(Ok(snapshot), Err)
}

/// The session log, oldest entry first.
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> History {
    state.history.lock().clone()
}

/// Empties the session log, in memory and on disk. A stretch running now is
/// dropped with it and not logged when it ends.
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<History, String> {
    let mut history = state.history.lock();
    *history = History::default();
    persistence::save_history(&state.history_path, &history).map_err(|err| {
        log::warn!("could not clear history: {err}");
        format!("History cleared but could not be saved: {err}")
    })?;
    Ok(history.clone())
}

/// Exit the application completely (not just hide to tray).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
