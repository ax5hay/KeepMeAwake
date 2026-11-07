//! Main window lifecycle.
//!
//! The window is created on demand rather than at startup. When the user
//! closes it we let Tauri destroy it (default) so the webview (WebView2 on
//! Windows, WKWebView on macOS) goes away and the app idles at a few
//! megabytes; the tray or menu bar icon keeps the app alive.
//! With `free_memory_when_hidden` off, the window is hidden instead so it
//! reopens instantly.

use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use super::AppState;

/// Label of the one and only window, matching `tauri.conf.json`.
pub const MAIN: &str = "main";

/// How long to wait for the UI to call `show()` before forcing it.
const SHOW_FALLBACK: Duration = Duration::from_millis(2500);

/// Creates the window during `setup`, where we are already on the main thread
/// and the event loop has not started, so the deferred path in [`show_main`]
/// would never run.
pub fn open_at_startup(app: &AppHandle) -> tauri::Result<()> {
    create(app).map(|_| ())
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
    #[cfg(not(target_os = "macos"))]
    std::thread::spawn(move || {
        if let Err(err) = create(&app) {
            log::error!("could not create main window: {err}");
        }
    });
    // AppKit wants windows made on the main thread itself. Tray and plugin
    // callbacks already run there, in which case this runs at once; from
    // any other thread it is sent over.
    #[cfg(target_os = "macos")]
    {
        let main = app.clone();
        let queued = main.run_on_main_thread(move || {
            if let Err(err) = create(&app) {
                log::error!("could not create main window: {err}");
            }
        });
        if let Err(err) = queued {
            log::error!("could not reach the main thread to open the window: {err}");
        }
    }
}

/// Shows the main window dropped down beneath the menu bar icon at `icon`,
/// creating it first if needed. Every click places it afresh, so it reopens
/// under the icon after a close.
#[cfg(target_os = "macos")]
pub fn show_under_icon(app: AppHandle, icon: tauri::Rect) {
    if let Some(window) = app.get_webview_window(MAIN) {
        place_under(&window, icon);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let main = app.clone();
    let queued = main.run_on_main_thread(move || match create(&app) {
        // Still hidden: the UI shows it once it has rendered, already in place.
        Ok(window) => place_under(&window, icon),
        Err(err) => log::error!("could not create main window: {err}"),
    });
    if let Err(err) = queued {
        log::error!("could not reach the main thread to open the window: {err}");
    }
}

/// Moves `window` so it hangs centred below `icon`, inside the work area of
/// the screen the icon is on. Placement is worked out in points: macOS gives
/// the icon in pixels at its own screen's scale, and a window on a screen of
/// another scale would be misplaced by a pixel position.
#[cfg(target_os = "macos")]
fn place_under(window: &WebviewWindow, icon: tauri::Rect) {
    use crate::core::placement::{Rect, Screen, Size, drop_down};

    let window_scale = window.scale_factor().unwrap_or(1.0);
    let at = icon.position.to_physical::<f64>(window_scale);
    let size = icon.size.to_physical::<f64>(window_scale);
    let icon = Rect {
        x: pixel(at.x),
        y: pixel(at.y),
        width: u32::try_from(pixel(size.width)).unwrap_or(0),
        height: u32::try_from(pixel(size.height)).unwrap_or(0),
    };
    let screens: Vec<Screen> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let work = monitor.work_area();
            Screen {
                bounds: Rect {
                    x: monitor.position().x,
                    y: monitor.position().y,
                    width: monitor.size().width,
                    height: monitor.size().height,
                },
                work_area: Rect {
                    x: work.position.x,
                    y: work.position.y,
                    width: work.size.width,
                    height: work.size.height,
                },
                scale: monitor.scale_factor(),
            }
        })
        .collect();
    let Ok(outer) = window.outer_size() else {
        return;
    };
    let outer = outer.to_logical::<f64>(window_scale);
    let points = Size {
        width: u32::try_from(pixel(outer.width)).unwrap_or(0),
        height: u32::try_from(pixel(outer.height)).unwrap_or(0),
    };
    let Some(point) = drop_down(icon, points, &screens) else {
        log::debug!(
            "no screen under the menu bar icon at {icon:?}; leaving the window where it is"
        );
        return;
    };
    log::debug!("window under the menu bar icon at {point:?} pt");
    if let Err(err) = window.set_position(tauri::LogicalPosition::new(point.x, point.y)) {
        log::warn!("could not move the window under the menu bar icon: {err}");
    }
}

/// A screen coordinate in whole pixels.
#[cfg(target_os = "macos")]
#[allow(clippy::cast_possible_truncation)] // screen coordinates are far inside i32
fn pixel(value: f64) -> i32 {
    value.round() as i32
}

fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
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

    Ok(window)
}
