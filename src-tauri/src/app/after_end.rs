//! When it ends: lock, turn the screen off or sleep after a stretch ends on
//! its own, if nobody is at the machine.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use super::AppState;
use crate::core::AfterEnd;
use crate::core::history::Ended;

/// The pause before acting, so the power request is gone and the toast
/// has shown before the screen goes dark.
const SETTLE: Duration = Duration::from_secs(3);

/// Called for every `SessionEnded`. Acts on a short-lived thread when the
/// setting asks for it and nobody has touched the machine for a minute.
pub fn on_ended(app: &AppHandle, ended: Ended) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let action = state.engine.settings().after_end;
    if !action.applies(ended) {
        return;
    }
    let idle = super::make_idle().idle();
    if !action.acts(ended, idle) {
        log::info!("not acting on {action:?} after {ended:?}: someone is here ({idle:?})");
        return;
    }
    let spawned = thread::Builder::new()
        .name("keepmeawake-after-end".into())
        .spawn(move || {
            thread::sleep(SETTLE);
            log::info!("{action:?} after {ended:?}");
            if let Err(err) = act(action) {
                log::warn!("could not {action:?}: {err}");
            }
        });
    if let Err(err) = spawned {
        log::warn!("could not start the after-end thread: {err}");
    }
}

#[cfg(windows)]
fn act(action: AfterEnd) -> Result<(), crate::platform::PlatformError> {
    crate::platform::windows::after_end(action)
}

#[cfg(target_os = "macos")]
fn act(action: AfterEnd) -> Result<(), crate::platform::PlatformError> {
    crate::platform::macos::after_end(action)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn act(_action: AfterEnd) -> Result<(), crate::platform::PlatformError> {
    Err(crate::platform::PlatformError::Unsupported)
}
