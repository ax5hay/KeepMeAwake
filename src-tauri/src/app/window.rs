//! Main window lifecycle.
//!
//! The window is created on demand rather than at startup. When the user
//! closes it we let Tauri destroy it (default) so the WebView2 processes go
//! away and the app idles at a few megabytes; the tray keeps the app alive.
//! With `free_memory_when_hidden` off, the window is hidden instead so it
//! reopens instantly.

use tauri::{AppHandle, Manager, WebviewWindowBuilder, WindowEvent};

use super::AppState;

/// Label of the one and only window, matching `tauri.conf.json`.
pub const MAIN: &str = "main";

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

    let window = WebviewWindowBuilder::from_config(app, &config)?.build()?;

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

    window.show()?;
    window.set_focus()?;
    Ok(())
}
