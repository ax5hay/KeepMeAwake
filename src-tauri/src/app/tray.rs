//! Tray icon: the primary surface of the app once the window is closed.
//!
//! Left click opens the window. Right click shows a menu that mirrors the
//! window's main actions so most days the window never needs to open.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use super::{AppState, window};
use crate::core::{Mode, PlanRequest, Snapshot};

/// Identifier used to look the tray up for updates.
pub const TRAY_ID: &str = "main";

const ICON_ON: &[u8] = include_bytes!("../../icons/tray/on.png");
const ICON_OFF: &[u8] = include_bytes!("../../icons/tray/off.png");
const ICON_PAUSED: &[u8] = include_bytes!("../../icons/tray/paused.png");

/// Durations offered in the "Keep awake for" submenu, in seconds.
pub const DURATIONS: &[(u64, &str)] = &[
    (30 * 60, "30 minutes"),
    (60 * 60, "1 hour"),
    (2 * 60 * 60, "2 hours"),
    (4 * 60 * 60, "4 hours"),
    (8 * 60 * 60, "8 hours"),
];

const ID_STATUS: &str = "status";
const ID_TOGGLE: &str = "toggle";
const ID_INDEFINITE: &str = "indefinite";
const ID_OPEN: &str = "open";
const ID_QUIT: &str = "quit";
const ID_DURATION_PREFIX: &str = "for:";

/// Creates the tray icon. Called once during setup.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let snapshot = app.state::<AppState>().engine.snapshot();
    let menu = build_menu(app, &snapshot)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(&snapshot)?)
        .icon_as_template(false)
        .tooltip(tooltip(&snapshot))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_main(tray.app_handle().clone());
            }
        })
        .build(app)?;
    Ok(())
}

/// Updates icon, tooltip and menu to reflect `snapshot`.
pub fn refresh(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(icon) = icon_for(snapshot) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(tooltip(snapshot)));
    match build_menu(app, snapshot) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(err) => log::warn!("could not rebuild tray menu: {err}"),
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    let engine = &app.state::<AppState>().engine;
    match id {
        ID_TOGGLE => {
            engine.toggle();
        }
        ID_INDEFINITE => {
            engine.set_plan(PlanRequest::Indefinite);
        }
        ID_OPEN => window::show_main(app.clone()),
        ID_QUIT => app.exit(0),
        other => {
            if let Some(seconds) = parse_duration_id(other) {
                engine.set_plan(PlanRequest::For { seconds });
            }
        }
    }
}

fn build_menu<R: Runtime>(app: &AppHandle<R>, snapshot: &Snapshot) -> tauri::Result<Menu<R>> {
    let status = MenuItem::with_id(app, ID_STATUS, status_line(snapshot), false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, ID_TOGGLE, toggle_label(snapshot), true, None::<&str>)?;

    let mut duration_items: Vec<MenuItem<R>> = Vec::with_capacity(DURATIONS.len());
    for (seconds, label) in DURATIONS {
        duration_items.push(MenuItem::with_id(
            app,
            duration_id(*seconds),
            *label,
            true,
            None::<&str>,
        )?);
    }
    let indefinite = MenuItem::with_id(app, ID_INDEFINITE, "Indefinitely", true, None::<&str>)?;
    let mut submenu_items: Vec<&dyn tauri::menu::IsMenuItem<R>> = Vec::new();
    for item in &duration_items {
        submenu_items.push(item);
    }
    let sep = PredefinedMenuItem::separator(app)?;
    submenu_items.push(&sep);
    submenu_items.push(&indefinite);
    let durations = Submenu::with_items(app, "Keep awake for", true, &submenu_items)?;

    let open = MenuItem::with_id(app, ID_OPEN, "Open Keep me awake", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &toggle,
            &durations,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn icon_for(snapshot: &Snapshot) -> tauri::Result<Image<'static>> {
    let bytes = match (snapshot.mode, snapshot.paused) {
        (Mode::Off, _) => ICON_OFF,
        (_, Some(_)) => ICON_PAUSED,
        (_, None) => ICON_ON,
    };
    Image::from_bytes(bytes)
}

fn duration_id(seconds: u64) -> String {
    format!("{ID_DURATION_PREFIX}{seconds}")
}

fn parse_duration_id(id: &str) -> Option<u64> {
    id.strip_prefix(ID_DURATION_PREFIX)?.parse().ok()
}

/// Tooltip shown when hovering the tray icon.
pub fn tooltip(snapshot: &Snapshot) -> String {
    format!("Keep me awake \u{2014} {}", status_line(snapshot))
}

/// One line describing the current state, shared by tooltip and menu.
pub fn status_line(snapshot: &Snapshot) -> String {
    match (snapshot.mode, snapshot.paused) {
        (Mode::Off, _) => "Sleeping normally".to_owned(),
        (_, Some(reason)) => format!("Paused: {}", pause_text(reason)),
        (Mode::Indefinite, None) => "Awake indefinitely".to_owned(),
        (Mode::Timed, None) => match snapshot.remaining_secs {
            Some(secs) => format!("Awake, {} left", format_duration(secs)),
            None => "Awake".to_owned(),
        },
    }
}

fn toggle_label(snapshot: &Snapshot) -> &'static str {
    if snapshot.mode == Mode::Off {
        "Keep awake"
    } else {
        "Let it sleep"
    }
}

fn pause_text(reason: crate::core::PauseReason) -> &'static str {
    use crate::core::PauseReason::{LowBattery, OnBattery, PowerRequestFailed};
    match reason {
        LowBattery => "battery is low",
        OnBattery => "running on battery",
        PowerRequestFailed => "Windows refused the request",
    }
}

/// Compact human duration: `2h 05m`, `12m`, `45s`.
pub fn format_duration(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    match (hours, minutes) {
        (0, 0) => format!("{seconds}s"),
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m:02}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BatteryState, GuardState, PauseReason, Settings};

    fn snap(mode: Mode, remaining: Option<u64>, paused: Option<PauseReason>) -> Snapshot {
        Snapshot {
            mode,
            started_at_unix_ms: None,
            ends_at_unix_ms: None,
            remaining_secs: remaining,
            plan_secs: None,
            guards: GuardState::default(),
            paused,
            battery: BatteryState::default(),
            settings: Settings::default(),
            app_version: String::new(),
        }
    }

    #[test]
    fn formats_durations_compactly() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(12 * 60 + 30), "12m");
        assert_eq!(format_duration(3600), "1h 00m");
        assert_eq!(format_duration(2 * 3600 + 5 * 60), "2h 05m");
    }

    #[test]
    fn status_lines_cover_every_state() {
        assert_eq!(
            status_line(&snap(Mode::Off, None, None)),
            "Sleeping normally"
        );
        assert_eq!(
            status_line(&snap(Mode::Indefinite, None, None)),
            "Awake indefinitely"
        );
        assert_eq!(
            status_line(&snap(Mode::Timed, Some(90 * 60), None)),
            "Awake, 1h 30m left"
        );
        assert_eq!(
            status_line(&snap(Mode::Indefinite, None, Some(PauseReason::LowBattery))),
            "Paused: battery is low"
        );
    }

    #[test]
    fn tooltip_is_prefixed_with_app_name() {
        assert!(tooltip(&snap(Mode::Off, None, None)).starts_with("Keep me awake"));
    }

    #[test]
    fn toggle_label_flips() {
        assert_eq!(toggle_label(&snap(Mode::Off, None, None)), "Keep awake");
        assert_eq!(
            toggle_label(&snap(Mode::Timed, Some(1), None)),
            "Let it sleep"
        );
    }

    #[test]
    fn duration_ids_round_trip() {
        for (secs, _) in DURATIONS {
            assert_eq!(parse_duration_id(&duration_id(*secs)), Some(*secs));
        }
        assert_eq!(parse_duration_id("quit"), None);
        assert_eq!(parse_duration_id("for:abc"), None);
    }

    #[test]
    fn icon_bytes_decode() {
        assert!(icon_for(&snap(Mode::Off, None, None)).is_ok());
        assert!(icon_for(&snap(Mode::Indefinite, None, None)).is_ok());
        assert!(icon_for(&snap(Mode::Indefinite, None, Some(PauseReason::OnBattery))).is_ok());
    }
}
