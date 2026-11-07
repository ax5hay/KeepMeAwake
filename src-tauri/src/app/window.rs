//! Main window lifecycle.
//!
//! The window is created on demand rather than at startup. When the user
//! closes it we let Tauri destroy it (default) so the WebView2 processes go
//! away and the app idles at a few megabytes; the tray keeps the app alive.
//! With `free_memory_when_hidden` off, the window is hidden instead so it
//! reopens instantly.

use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindowBuilder, WindowEvent};

use super::AppState;

/// Label of the one and only window, matching `tauri.conf.json`.
pub const MAIN: &str = "main";

/// How long to wait for the UI to call `show()` before forcing it.
const SHOW_FALLBACK: Duration = Duration::from_millis(2500);

/// Creates the window during `setup`, where we are already on the main thread
/// and the event loop has not started, so the deferred path in [`show_main`]
/// would never run.
pub fn open_at_startup(app: &AppHandle) -> tauri::Result<()> {
    create(app)
}

/// Shows the main window, creating it first if needed. Safe to call from any
/// thread, including tray and plugin callbacks.
pub fn show_main(app: AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // Window creation from a synchronous event handler can deadlock on
    // Windows; hop to a fresh thread and let Tauri marshal to the main thread.
    std::thread::spawn(move || {
        if let Err(err) = create(&app) {
            log::error!("could not create main window: {err}");
        }
    });
}

fn create(app: &AppHandle) -> tauri::Result<()> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN)
        .cloned()
        .ok_or_else(|| tauri::Error::WindowNotFound)?;

    // The window starts hidden; the UI shows it once it has rendered its first
    // state, so there is never a blank flash. If the UI fails to do so, show
    // it anyway after a grace period so the user is never locked out.
    let window = WebviewWindowBuilder::from_config(app, &config)?.build()?;
    {
        let window = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(SHOW_FALLBACK);
            if !window.is_visible().unwrap_or(true) {
                log::warn!("UI did not report ready; showing window anyway");
                let _ = window.show();
            }
        });
    }

    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let settings = handle.state::<AppState>().engine.settings();
            if !settings.close_to_tray {
                let handle = handle.clone();
                std::thread::spawn(move || handle.exit(0));
            } else if !settings.free_memory_when_hidden {
                api.prevent_close();
                if let Some(window) = handle.get_webview_window(MAIN) {
                    let _ = window.hide();
                }
            }
            // Otherwise: let the close go through, destroying the webview.
        }
    });

    Ok(())
}
