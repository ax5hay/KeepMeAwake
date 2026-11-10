//! IPC surface exposed to the UI. Each command is a thin adapter over the engine;
//! the return value is always the fresh snapshot so the UI never guesses.

// Tauri's command macro requires owned `State` and `AppHandle` arguments.
#![allow(clippy::needless_pass_by_value)]

use tauri::{AppHandle, Manager, State};

use super::ai::{AiStatus, Feature};
use super::update::UpdateInfo;
use super::{AppState, hotkey};
use crate::core::ai::{self, Understood};
use crate::core::almanac::{Almanac, Mark, Span};
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
    let current = state.engine.settings();
    // The app writes this; the window's copy may be a check behind.
    settings.last_update_check_unix_ms = current.last_update_check_unix_ms;
    let previous = current.hotkey;
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
    state.almanac.set_on(settings.almanac.enabled);
    state
        .almanac
        .set_break_after(settings.almanac.break_after_mins);
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

/// The almanac: presence and marked days, after one fresh look so today is
/// current to the moment.
#[tauri::command]
pub fn get_almanac(state: State<'_, AppState>) -> Almanac {
    state.almanac.read()
}

/// Marks a day (`2026-11-10`) as leave or as not a leave, or takes its mark
/// away with `null`. Fails for a day that is not a date.
#[tauri::command]
pub fn mark_day(
    state: State<'_, AppState>,
    day: String,
    mark: Option<Mark>,
) -> Result<Almanac, String> {
    state
        .almanac
        .mark(&day, mark)
        .ok_or_else(|| format!("{day} is not a day"))
}

/// Corrects a day by hand to one stretch of presence, in place of whatever
/// was read for it, or goes back to what was read with `null`.
#[tauri::command]
pub fn fix_day(
    state: State<'_, AppState>,
    day: String,
    stretch: Option<Span>,
) -> Result<Almanac, String> {
    state
        .almanac
        .fix(&day, stretch)
        .ok_or_else(|| format!("{day} cannot be corrected to that"))
}

/// Empties the almanac, in memory and on disk. Whoever is here now starts
/// it again.
#[tauri::command]
pub fn clear_almanac(state: State<'_, AppState>) -> Result<Almanac, String> {
    state.almanac.clear().map_err(|err| {
        log::warn!("could not clear the almanac: {err}");
        format!("The almanac was cleared but could not be saved: {err}")
    })
}

/// Whether an API key for Claude is stored. The key itself never comes back.
#[tauri::command]
pub fn ai_status(state: State<'_, AppState>) -> AiStatus {
    state.ai.status()
}

/// Stores the API key in the OS credential store, or removes it with
/// `null`. Says only whether there is one now.
#[tauri::command]
pub fn set_ai_key(state: State<'_, AppState>, key: Option<String>) -> Result<AiStatus, String> {
    state.ai.set_key(key.as_deref()).map_err(|err| {
        log::warn!("could not store the API key: {err}");
        format!("The key could not be stored: {err}")
    })
}

/// Asks Claude to put a week of the almanac into words. `digest` is the
/// week as the window laid it out, and is all that is sent.
#[tauri::command]
pub async fn ai_read_week(state: State<'_, AppState>, digest: String) -> Result<String, String> {
    let settings = state.engine.settings().ai;
    let read = async {
        let key = state.ai.unlock(&settings, Feature::WeekInWords)?;
        let body = ai::week_request(&digest)?;
        ai::text_of(&state.ai.ask(&key, &body).await?)
    };
    read.await.map_err(|err| err.sentence())
}

/// Asks Claude what plan a sentence means. Nothing is started here: the
/// window shows what was understood and asks first. `now` is the local
/// date and time as the window has it.
#[tauri::command]
pub async fn ai_plan(
    state: State<'_, AppState>,
    sentence: String,
    now: String,
) -> Result<Understood, String> {
    let settings = state.engine.settings().ai;
    let read = async {
        let key = state.ai.unlock(&settings, Feature::TellTheSun)?;
        let body = ai::plan_request(&sentence, &now)?;
        ai::understood(&state.ai.ask(&key, &body).await?)
    };
    read.await.map_err(|err| err.sentence())
}

/// The newer release the daily check found, if any.
#[tauri::command]
pub fn get_update(state: State<'_, AppState>) -> Option<UpdateInfo> {
    state.update.lock().clone()
}

/// Asks GitHub now, because someone asked: the same check the daily one
/// makes, with the switch on or off. Returns the newer release, if any.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Option<UpdateInfo> {
    let handle = app.clone();
    // The check blocks on the network; keep it off the window's thread.
    let done = tauri::async_runtime::spawn_blocking(move || {
        super::update::check_and_record(&handle);
    })
    .await;
    if let Err(err) = done {
        log::debug!("the update check did not finish: {err}");
    }
    app.try_state::<AppState>()
        .and_then(|state| state.update.lock().clone())
}

/// Download, verify and install the newer release, then relaunch. Resolves
/// only on failure, with a sentence for the window; on success the app is
/// gone before it could.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    super::update::install(app).await
}

/// Exit the application completely (not just hide to tray).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
