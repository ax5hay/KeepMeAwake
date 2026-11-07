//! Desktop toasts for the moments that matter while the window is closed.
//!
//! Toasts are local (Windows' own notification centre, no network) and best
//! effort: a toast that fails to show is logged and forgotten, and can never
//! affect the engine.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Shows one toast: the app icon, a title line and a body line.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        log::warn!("could not show a notification: {err}");
    }
}
