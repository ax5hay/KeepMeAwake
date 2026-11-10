//! Updates: one signed check a day against GitHub Releases, and an install
//! on request.
//!
//! The check is the app's only outbound request. It is off with the switch
//! (Settings → About → Check for updates daily), and a failure is logged at
//! debug and forgotten: an offline laptop is not an error. Whatever is
//! downloaded is verified against the public key in `tauri.conf.json`
//! before it is run.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use super::{AppState, tray};
use crate::persistence;

/// How often to ask.
pub const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// The longest one sleep lasts. A sleeping laptop stops the clock a thread
/// sleeps on, so a single 24 h sleep could run a day late; waking every few
/// hours to compare wall-clock times bounds that.
pub const RECHECK: Duration = Duration::from_secs(6 * 60 * 60);

/// How long after launch the first check waits, so it never competes with
/// the window opening or a network that is still coming up.
pub const AFTER_LAUNCH: Duration = Duration::from_secs(30);

/// Event the window hears when a newer version is found.
pub const UPDATE_EVENT: &str = "update";

/// Event the window hears while an update downloads.
pub const PROGRESS_EVENT: &str = "update-progress";

/// A newer release, as the window and the tray show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateInfo {
    /// Its version, `0.3.1`.
    pub version: String,
    /// Its release notes, if it has any.
    pub notes: Option<String>,
}

/// How far a download has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Progress {
    /// Bytes so far.
    pub downloaded: u64,
    /// Bytes in all, when the server said.
    pub total: Option<u64>,
}

/// How long until the next check is due: zero if it already is. A last
/// check stamped in the future (the clock moved back) counts as due.
pub fn next_check_due(last_unix_ms: u64, now_unix_ms: u64) -> Duration {
    if last_unix_ms > now_unix_ms {
        return Duration::ZERO;
    }
    let due = last_unix_ms.saturating_add(millis(EVERY));
    Duration::from_millis(due.saturating_sub(now_unix_ms))
}

/// When the window is opened, a check is made if the last one is at least
/// this old. There is no way for a release to announce itself to the app
/// (that would take a server pushing to it), so this is what makes a new
/// release show up the next time someone looks, not a day later.
pub const ON_OPEN_AFTER: Duration = Duration::from_secs(60 * 60);

/// Whether opening the window should check now: the last check is at least
/// [`ON_OPEN_AFTER`] old, or stamped in the future (the clock moved back).
pub fn due_on_open(last_unix_ms: u64, now_unix_ms: u64) -> bool {
    last_unix_ms > now_unix_ms || now_unix_ms - last_unix_ms >= millis(ON_OPEN_AFTER)
}

/// A check is running. Opening the window twice, or asking by hand while
/// the daily check is out, must not make a second request.
static CHECKING: AtomicBool = AtomicBool::new(false);

/// Checks as the window opens, off its thread, if the switch is on, no
/// newer release is already known and the last check is old enough.
pub fn check_on_open(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let settings = state.engine.settings();
    let known = state.update.lock().is_some();
    if !settings.check_for_updates
        || known
        || !due_on_open(settings.last_update_check_unix_ms, tray::now_unix_ms())
    {
        return;
    }
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("keepmeawake-update-open".into())
        .spawn(move || check_and_record(&app));
    if let Err(err) = spawned {
        log::debug!("could not check for an update on open: {err}");
    }
}

/// Whole percent of a download, when its size is known.
pub fn percent(progress: Progress) -> Option<u8> {
    let total = progress.total.filter(|&t| t > 0)?;
    let share = progress.downloaded.min(total).saturating_mul(100) / total;
    u8::try_from(share).ok()
}

/// Starts the thread that checks once a day while the switch is on: a
/// little after launch, then 24 hours after the last check. It sleeps in
/// between and makes no request with the switch off.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("keepmeawake-update".into())
        .spawn(move || {
            thread::sleep(AFTER_LAUNCH);
            let mut first = true;
            loop {
                let Some(state) = app.try_state::<AppState>() else {
                    return;
                };
                let settings = state.engine.settings();
                let wait = match (settings.check_for_updates, first) {
                    (false, _) => RECHECK,
                    (true, true) => Duration::ZERO,
                    (true, false) => {
                        next_check_due(settings.last_update_check_unix_ms, tray::now_unix_ms())
                    }
                };
                if wait.is_zero() {
                    first = false;
                    check_and_record(&app);
                    continue;
                }
                first = false;
                thread::sleep(wait.min(RECHECK));
            }
        });
    if let Err(err) = spawned {
        log::warn!("could not start the update check: {err}");
    }
}

/// One check: remembers what it found (or that nothing is newer), tells the
/// window and the tray, and stamps the time so the next is a day away even
/// if this one failed.
pub(crate) fn check_and_record(app: &AppHandle) {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    let found = on_this_thread(check_now(app));
    CHECKING.store(false, Ordering::SeqCst);
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mut settings = state.engine.settings();
    settings.last_update_check_unix_ms = tray::now_unix_ms();
    let snapshot = state.engine.update_settings(settings.clone());
    if let Err(err) = persistence::save(&state.settings_path, &settings) {
        log::warn!("could not record the update check: {err}");
    }
    (*state.update.lock()).clone_from(&found);
    if let Some(info) = &found {
        log::info!("update available: {}", info.version);
        if let Err(err) = app.emit(UPDATE_EVENT, info) {
            log::debug!("no window to tell about the update: {err}");
        }
    }
    tray::refresh(app, &snapshot);
}

/// Runs `work` to completion on the calling thread. Tauri's shared runtime
/// would start a worker per CPU core the first time it is used, for one
/// request a day; a current-thread runtime does the same work with none,
/// and is dropped right after.
fn on_this_thread<T>(work: impl Future<Output = Option<T>>) -> Option<T> {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(work),
        Err(err) => {
            log::debug!("no runtime for the update check: {err}");
            None
        }
    }
}

/// Asks GitHub once. `None` for "nothing newer" and for any failure, which
/// is logged at debug: offline is normal for a laptop.
pub async fn check_now(app: &AppHandle) -> Option<UpdateInfo> {
    let updater = app
        .updater()
        .inspect_err(|err| log::debug!("updater unavailable: {err}"))
        .ok()?;
    match updater.check().await {
        Ok(Some(update)) => Some(UpdateInfo {
            version: update.version,
            notes: update.body,
        }),
        Ok(None) => {
            log::debug!("no update: this is the newest release");
            None
        }
        Err(err) => {
            log::debug!("update check failed: {err}");
            None
        }
    }
}

/// Downloads the newer release, has the plugin verify its signature,
/// installs it and relaunches. The window hears `update-progress` each
/// time the whole percentage moves. On Windows the installer takes over
/// and the plugin exits the app; the engine is shut down first, so the
/// power request is released and the stretch ends as `you`, not `sleep`.
pub async fn install(app: AppHandle) -> Result<(), String> {
    let engine = Arc::clone(&app.state::<AppState>().engine);
    let almanac = Arc::clone(&app.state::<AppState>().almanac);
    let updater = app
        .updater_builder()
        .on_before_exit(move || {
            almanac.flush();
            engine.shutdown();
        })
        .build()
        .map_err(|err| format!("The updater is unavailable: {err}"))?;
    let update = updater
        .check()
        .await
        .map_err(|err| format!("Could not reach GitHub: {err}"))?
        .ok_or_else(|| "Already up to date.".to_owned())?;
    log::info!("installing {}", update.version);
    let handle = app.clone();
    let mut downloaded: u64 = 0;
    let mut shown: Option<u8> = None;
    update
        .download_and_install(
            |chunk, total| {
                downloaded = downloaded.saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
                let progress = Progress { downloaded, total };
                let now = percent(progress);
                // Thousands of chunks; the window only needs each percent.
                if now != shown || now.is_none() {
                    shown = now;
                    let _ = handle.emit(PROGRESS_EVENT, progress);
                }
            },
            || log::info!("update downloaded and verified"),
        )
        .await
        .map_err(|err| {
            log::warn!("update not installed: {err}");
            format!("The update could not be installed: {err}")
        })?;
    app.restart()
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_800_000_000_000;
    const HOUR_MS: u64 = 3_600_000;

    #[test]
    fn opening_the_window_checks_when_the_last_check_is_an_hour_old() {
        assert!(due_on_open(0, T0), "never checked");
        assert!(!due_on_open(T0, T0 + HOUR_MS - 1));
        assert!(due_on_open(T0, T0 + HOUR_MS));
        assert!(due_on_open(T0 + HOUR_MS, T0), "a check from the future");
    }

    #[test]
    fn a_check_is_due_a_day_after_the_last() {
        assert_eq!(next_check_due(0, T0), Duration::ZERO, "never checked");
        assert_eq!(next_check_due(T0, T0), EVERY);
        assert_eq!(
            next_check_due(T0, T0 + 23 * HOUR_MS),
            Duration::from_secs(3600)
        );
        assert_eq!(next_check_due(T0, T0 + 24 * HOUR_MS), Duration::ZERO);
        assert_eq!(next_check_due(T0, T0 + 50 * HOUR_MS), Duration::ZERO);
    }

    #[test]
    fn a_clock_moved_back_counts_as_due() {
        assert_eq!(next_check_due(T0 + HOUR_MS, T0), Duration::ZERO);
    }

    #[test]
    fn percent_is_whole_and_only_with_a_size() {
        let at = |downloaded, total| percent(Progress { downloaded, total });
        assert_eq!(at(42, Some(100)), Some(42));
        assert_eq!(at(1, Some(3)), Some(33));
        assert_eq!(at(5_000, Some(5_000)), Some(100));
        assert_eq!(at(9_000, Some(5_000)), Some(100), "never past 100");
        assert_eq!(at(10, None), None);
        assert_eq!(at(10, Some(0)), None);
    }

    #[test]
    fn update_info_serialises_snake_case() {
        let info = UpdateInfo {
            version: "0.3.1".into(),
            notes: None,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["version"], "0.3.1");
        assert!(json["notes"].is_null());
    }
}
