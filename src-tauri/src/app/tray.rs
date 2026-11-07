//! Tray icon: the primary surface of the app once the window is closed.
//!
//! Left click opens the window. Right click shows a menu that mirrors the
//! window's main actions so most days the window never needs to open.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, TimeZone};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use super::{AppState, window};
use crate::core::{Mode, PlanRequest, Snapshot};

/// Identifier used to look the tray up for updates.
pub const TRAY_ID: &str = "main";

// Each binary carries only its own platform's set.
#[cfg(not(target_os = "macos"))]
const ICON_ON: &[u8] = include_bytes!("../../icons/tray/on.png");
#[cfg(not(target_os = "macos"))]
const ICON_OFF: &[u8] = include_bytes!("../../icons/tray/off.png");
#[cfg(not(target_os = "macos"))]
const ICON_PAUSED: &[u8] = include_bytes!("../../icons/tray/paused.png");
#[cfg(not(target_os = "macos"))]
const ICON_ON_INK: &[u8] = include_bytes!("../../icons/tray/on-ink.png");
#[cfg(not(target_os = "macos"))]
const ICON_OFF_INK: &[u8] = include_bytes!("../../icons/tray/off-ink.png");
#[cfg(not(target_os = "macos"))]
const ICON_PAUSED_INK: &[u8] = include_bytes!("../../icons/tray/paused-ink.png");
#[cfg(target_os = "macos")]
const ICON_ON_TEMPLATE: &[u8] = include_bytes!("../../icons/tray/on-template.png");
#[cfg(target_os = "macos")]
const ICON_OFF_TEMPLATE: &[u8] = include_bytes!("../../icons/tray/off-template.png");
#[cfg(target_os = "macos")]
const ICON_PAUSED_TEMPLATE: &[u8] = include_bytes!("../../icons/tray/paused-template.png");

/// Menu bar icons are template images on macOS: black with alpha, tinted by
/// the system for a light or dark menu bar.
const TEMPLATE: bool = cfg!(target_os = "macos");

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
const ID_SKIP: &str = "skip";
const ID_EXTEND: &str = "extend";

/// The tray item that pushes a timed session's end later.
pub const EXTEND_LABEL: &str = "Give it another 30m";
const ID_INDEFINITE: &str = "indefinite";
const ID_OPEN: &str = "open";
const ID_QUIT: &str = "quit";
const ID_DURATION_PREFIX: &str = "for:";

/// Creates the tray icon. Called once during setup.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let snapshot = state.engine.snapshot();
    let menu = build_menu(app, &snapshot)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(&snapshot, state.taskbar.is_light())?)
        .icon_as_template(TEMPLATE)
        .tooltip(tooltip(&snapshot))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                // A Mac drops the window beneath the icon, like a menu; Windows
                // reopens it wherever it last was.
                #[cfg(target_os = "macos")]
                window::show_under_icon(tray.app_handle().clone(), rect);
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = rect;
                    window::show_main(tray.app_handle().clone());
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Updates icon, tooltip and menu to reflect `snapshot`. The icon set
/// follows the taskbar theme (one registry read, microseconds).
pub fn refresh(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let light = app
        .try_state::<AppState>()
        .is_some_and(|state| state.taskbar.is_light());
    if let Ok(icon) = icon_for(snapshot, light) {
        // Plain `set_icon` drops the template flag on macOS; this keeps it.
        let _ = tray.set_icon_with_as_template(Some(icon), TEMPLATE);
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
        ID_EXTEND => {
            engine.extend(crate::engine::EXTEND_BY);
        }
        ID_SKIP => {
            if engine.snapshot().schedule.skipped {
                engine.unskip();
            } else {
                engine.skip_today();
            }
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
    let skip = skip_label(snapshot)
        .map(|label| MenuItem::with_id(app, ID_SKIP, label, true, None::<&str>))
        .transpose()?;
    let extend = extend_label(snapshot)
        .map(|label| MenuItem::with_id(app, ID_EXTEND, label, true, None::<&str>))
        .transpose()?;

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

    let separators = [
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
    ];
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![&status, &separators[0], &toggle];
    if let Some(extend) = &extend {
        items.push(extend);
    }
    if let Some(skip) = &skip {
        items.push(skip);
    }
    items.extend([
        &durations as &dyn tauri::menu::IsMenuItem<R>,
        &separators[1],
        &open,
        &separators[2],
        &quit,
    ]);
    Menu::with_items(app, &items)
}

/// The extend item, shown while a timed session is inside its sunset
/// warning: the same moment the engine warned.
fn extend_label(snapshot: &Snapshot) -> Option<&'static str> {
    let warning = u64::from(snapshot.settings.sunset_warning_mins?) * 60;
    let inside = snapshot.mode == Mode::Timed
        && snapshot.remaining_secs? <= warning
        && snapshot.plan_secs? > warning;
    inside
        .then_some(EXTEND_LABEL)
        .filter(|_| !window_outlasts_timer(snapshot))
}

/// An open, unskipped schedule window ends after the timer, so the sun stays
/// up when the timer runs out: no sunset to warn about.
fn window_outlasts_timer(snapshot: &Snapshot) -> bool {
    let schedule = &snapshot.schedule;
    schedule.enabled
        && schedule.in_window
        && !schedule.skipped
        && matches!(
            (schedule.window_end_unix_ms, snapshot.ends_at_unix_ms),
            (Some(window), Some(timer)) if window > timer
        )
}

/// The schedule item under the toggle, shown only while a window is open.
fn skip_label(snapshot: &Snapshot) -> Option<&'static str> {
    let schedule = &snapshot.schedule;
    match (schedule.enabled && schedule.in_window, schedule.skipped) {
        (false, _) => None,
        (true, false) => Some("Skip today's schedule"),
        (true, true) => Some("Back on schedule"),
    }
}

/// The icon for `snapshot`: white on a dark taskbar, ink on a light one, a
/// template image in the macOS menu bar.
fn icon_for(snapshot: &Snapshot, light: bool) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(snapshot, light))
}

#[cfg(target_os = "macos")]
fn icon_bytes(snapshot: &Snapshot, _light: bool) -> &'static [u8] {
    match (snapshot.mode, snapshot.paused) {
        (Mode::Off, _) => ICON_OFF_TEMPLATE,
        (_, Some(_)) => ICON_PAUSED_TEMPLATE,
        (_, None) => ICON_ON_TEMPLATE,
    }
}

#[cfg(not(target_os = "macos"))]
fn icon_bytes(snapshot: &Snapshot, light: bool) -> &'static [u8] {
    match (snapshot.mode, snapshot.paused, light) {
        (Mode::Off, _, false) => ICON_OFF,
        (Mode::Off, _, true) => ICON_OFF_INK,
        (_, Some(_), false) => ICON_PAUSED,
        (_, Some(_), true) => ICON_PAUSED_INK,
        (_, None, false) => ICON_ON,
        (_, None, true) => ICON_ON_INK,
    }
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
    status_line_at(snapshot, now_unix_ms())
}

/// Wall clock now, Unix ms.
pub(crate) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// [`status_line`] against an explicit wall clock, for tests.
fn status_line_at(snapshot: &Snapshot, now_unix_ms: u64) -> String {
    let schedule = &snapshot.schedule;
    match (snapshot.mode, snapshot.paused) {
        (Mode::Off, _) => match (schedule.skipped, schedule.window_end_unix_ms) {
            (true, Some(end)) => {
                format!("Skipping today's schedule until {}", when(end, now_unix_ms))
            }
            _ => match schedule.next_start_unix_ms.filter(|_| schedule.enabled) {
                Some(next) => format!(
                    "Sleeping normally \u{b7} awake again at {}",
                    when(next, now_unix_ms)
                ),
                None => "Sleeping normally".to_owned(),
            },
        },
        (_, Some(reason)) => format!("Paused: {}", pause_text(reason)),
        (Mode::Indefinite, None) => "Awake indefinitely".to_owned(),
        (Mode::Scheduled, None) => match snapshot.ends_at_unix_ms {
            Some(end) => format!("Awake until {} \u{b7} on schedule", when(end, now_unix_ms)),
            None => "Awake on schedule".to_owned(),
        },
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
        PowerRequestFailed if cfg!(target_os = "macos") => "macOS refused the request",
        PowerRequestFailed => "Windows refused the request",
    }
}

/// A wall-clock moment as `6:00 pm`, with the weekday added (`9:00 am on
/// Monday`) when it is further away than tomorrow.
pub(crate) fn when(at_unix_ms: u64, now_unix_ms: u64) -> String {
    let (Some(at), Some(now)) = (local(at_unix_ms), local(now_unix_ms)) else {
        return String::new();
    };
    let clock = at.format("%-I:%M %P").to_string();
    let days_ahead = (at.date_naive() - now.date_naive()).num_days();
    if days_ahead > 1 {
        format!("{clock} on {}", at.format("%A"))
    } else {
        clock
    }
}

fn local(unix_ms: u64) -> Option<DateTime<Local>> {
    Local
        .timestamp_millis_opt(i64::try_from(unix_ms).ok()?)
        .single()
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
    use crate::core::{BatteryState, GuardState, PauseReason, ScheduleState, Settings};

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
            schedule: ScheduleState::default(),
            guard_rests_at_unix_ms: None,
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

    fn local_ms(day: u32, h: u32, m: u32) -> u64 {
        // March 2026: the 2nd is a Monday.
        let t = Local
            .with_ymd_and_hms(2026, 3, day, h, m, 0)
            .single()
            .unwrap();
        u64::try_from(t.timestamp_millis()).unwrap()
    }

    fn scheduled(in_window: bool, skipped: bool) -> Snapshot {
        let mut s = snap(Mode::Off, None, None);
        s.schedule = ScheduleState {
            enabled: true,
            in_window,
            skipped,
            window_start_unix_ms: in_window.then(|| local_ms(2, 9, 0)),
            window_end_unix_ms: in_window.then(|| local_ms(2, 18, 0)),
            next_start_unix_ms: Some(local_ms(3, 9, 0)),
        };
        if in_window && !skipped {
            s.mode = Mode::Scheduled;
            s.ends_at_unix_ms = Some(local_ms(2, 18, 0));
        }
        s
    }

    #[test]
    fn status_lines_cover_the_schedule() {
        let noon = local_ms(2, 12, 0);
        assert_eq!(
            status_line_at(&scheduled(true, false), noon),
            "Awake until 6:00 pm \u{b7} on schedule"
        );
        assert_eq!(
            status_line_at(&scheduled(true, true), noon),
            "Skipping today's schedule until 6:00 pm"
        );
        assert_eq!(
            status_line_at(&scheduled(false, false), local_ms(2, 19, 0)),
            "Sleeping normally \u{b7} awake again at 9:00 am"
        );
        let mut paused = scheduled(true, false);
        paused.paused = Some(PauseReason::OnBattery);
        assert_eq!(status_line_at(&paused, noon), "Paused: running on battery");
    }

    #[test]
    fn far_off_times_name_the_day() {
        // Friday evening, next window Monday morning.
        assert_eq!(
            when(local_ms(9, 9, 0), local_ms(6, 19, 0)),
            "9:00 am on Monday"
        );
        assert_eq!(when(local_ms(3, 9, 0), local_ms(2, 19, 0)), "9:00 am");
    }

    #[test]
    fn skip_item_only_while_a_window_is_open() {
        assert_eq!(skip_label(&snap(Mode::Off, None, None)), None);
        assert_eq!(skip_label(&scheduled(false, false)), None);
        assert_eq!(
            skip_label(&scheduled(true, false)),
            Some("Skip today's schedule")
        );
        assert_eq!(skip_label(&scheduled(true, true)), Some("Back on schedule"));
    }

    #[test]
    fn toggle_label_while_scheduled_lets_it_sleep() {
        assert_eq!(toggle_label(&scheduled(true, false)), "Let it sleep");
        assert_eq!(toggle_label(&scheduled(true, true)), "Keep awake");
    }

    #[test]
    fn a_refused_request_names_this_os() {
        let line = status_line(&snap(
            Mode::Indefinite,
            None,
            Some(PauseReason::PowerRequestFailed),
        ));
        let os = if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Windows"
        };
        assert_eq!(line, format!("Paused: {os} refused the request"));
    }

    #[test]
    fn the_extend_item_shows_only_inside_the_warning() {
        let mut timed = snap(Mode::Timed, Some(300), None);
        timed.plan_secs = Some(600);
        assert_eq!(extend_label(&timed), Some("Give it another 30m"));
        timed.remaining_secs = Some(301);
        assert_eq!(extend_label(&timed), None, "not yet");
        timed.remaining_secs = Some(60);
        timed.plan_secs = Some(120);
        assert_eq!(extend_label(&timed), None, "a plan too short to warn");
        let mut off = snap(Mode::Timed, Some(60), None);
        off.plan_secs = Some(600);
        off.settings.sunset_warning_mins = None;
        assert_eq!(extend_label(&off), None, "warning off");
        assert_eq!(extend_label(&snap(Mode::Indefinite, None, None)), None);

        // A window that outlasts the timer keeps the sun up: no sunset.
        let mut held = snap(Mode::Timed, Some(300), None);
        held.plan_secs = Some(600);
        held.ends_at_unix_ms = Some(1_000_000);
        held.schedule = ScheduleState {
            enabled: true,
            in_window: true,
            window_end_unix_ms: Some(2_000_000),
            ..ScheduleState::default()
        };
        assert_eq!(extend_label(&held), None);
        held.schedule.skipped = true;
        assert_eq!(extend_label(&held), Some("Give it another 30m"), "skipped");
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

    /// macOS tints a template image for the menu bar itself, so the theme
    /// never picks the icon there.
    #[cfg(target_os = "macos")]
    #[test]
    fn three_template_icons_whatever_the_menu_bar() {
        let states = [
            snap(Mode::Off, None, None),
            snap(Mode::Indefinite, None, None),
            snap(Mode::Indefinite, None, Some(PauseReason::OnBattery)),
        ];
        let expected = [ICON_OFF_TEMPLATE, ICON_ON_TEMPLATE, ICON_PAUSED_TEMPLATE];
        for (state, want) in states.iter().zip(expected) {
            assert!(icon_for(state, false).is_ok());
            assert_eq!(icon_bytes(state, false), want);
            assert_eq!(icon_bytes(state, true), want);
        }
        assert_ne!(ICON_ON_TEMPLATE, ICON_OFF_TEMPLATE);
        assert_ne!(ICON_ON_TEMPLATE, ICON_PAUSED_TEMPLATE);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn six_icons_decode_and_differ() {
        let states = [
            snap(Mode::Off, None, None),
            snap(Mode::Indefinite, None, None),
            snap(Mode::Indefinite, None, Some(PauseReason::OnBattery)),
        ];
        let mut seen: Vec<&[u8]> = Vec::new();
        for light in [false, true] {
            for state in &states {
                assert!(icon_for(state, light).is_ok());
                let bytes = icon_bytes(state, light);
                assert!(
                    !seen.contains(&bytes),
                    "each state and theme has its own icon"
                );
                seen.push(bytes);
            }
        }
        assert_eq!(seen.len(), 6);
        // A scheduled window shows the awake sun, like any other session.
        assert_eq!(
            icon_bytes(&snap(Mode::Scheduled, None, None), true),
            ICON_ON_INK
        );
    }
}
