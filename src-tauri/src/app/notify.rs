//! Desktop toasts for the moments that matter while the window is closed.
//!
//! Toasts are local (Windows' own notification centre, no network) and best
//! effort: a toast that fails to show is logged and forgotten, and can never
//! affect the engine. Each kind sits behind its own switch in settings
//! (the overnight guard's question belongs to the guard and has none), and
//! nothing is shown while the window is open and focused, because the
//! window already says it.

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use super::{AppState, tray, window};
use crate::core::{Mode, PauseReason, Snapshot};
use crate::engine::EngineEvent;

/// Title when a timed session ends.
pub const TIMER_TITLE: &str = "Timer done";
/// Title when the battery rule pauses the sun.
pub const PAUSED_TITLE: &str = "Paused for the battery";
/// Title when the charger brings it back.
pub const RESUMED_TITLE: &str = "Awake again";
/// Body when the charger brings it back.
pub const RESUMED_BODY: &str = "Charger connected. Keeping your laptop awake again.";
/// Title when a schedule window opens.
pub const SCHEDULE_STARTED_TITLE: &str = "Schedule started";
/// Title of the toast a few minutes before a timed session ends.
pub const SUNSET_TITLE: &str = "Timer ends soon";
/// Title when a schedule window closes.
pub const SCHEDULE_ENDED_TITLE: &str = "Schedule ended";
/// Title when the overnight guard asks whether anyone is there.
pub const OVERNIGHT_TITLE: &str = "Still there?";

/// Title of the toast when Sleep when I'm away is about to set the sun for the night.
pub const NIGHT_TITLE: &str = "Away for the night";

/// Title of the toast after a long stretch at the machine without a break.
pub const BREAK_TITLE: &str = "Time for a break";

/// The body of that toast: `90 minutes without a break.`
pub fn break_body(minutes: u64) -> String {
    let length = if minutes.is_multiple_of(60) {
        let hours = minutes / 60;
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else {
        format!("{minutes} minutes")
    };
    format!("{length} without a break.")
}

/// Shows the break toast. Asked for in Settings, so it is shown even with
/// the window in front.
pub fn break_due(app: &AppHandle, minutes: u64) {
    log::info!("{minutes} minutes without a break");
    send(app, BREAK_TITLE, &break_body(minutes));
}

/// One toast, ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    /// The bold first line.
    pub title: &'static str,
    /// The line under it.
    pub body: String,
}

/// The toast for `event`, if its switch is on and it is still true by the
/// time it would be shown. `snapshot` is the state after the event.
pub fn body_for(event: &EngineEvent, snapshot: &Snapshot, now_unix_ms: u64) -> Option<Toast> {
    let switches = snapshot.settings.notifications;
    let toast = |title, body: String| Some(Toast { title, body });
    match event {
        // "Resting now" is only true if no schedule window took over.
        EngineEvent::TimerEnded { plan_secs } if switches.timer && snapshot.mode == Mode::Off => {
            toast(
                TIMER_TITLE,
                format!(
                    "Kept awake for {}, as asked. Your laptop sleeps as usual now.",
                    tray::format_duration(*plan_secs)
                ),
            )
        }
        // About the same timer, so it follows the timer switch.
        EngineEvent::SunsetSoon { secs_left, .. }
            if switches.timer && snapshot.mode == Mode::Timed =>
        {
            let mins = secs_left.div_ceil(60).max(1);
            let unit = if mins == 1 { "minute" } else { "minutes" };
            let place = if cfg!(target_os = "macos") {
                "the menu bar"
            } else {
                "the tray"
            };
            toast(
                SUNSET_TITLE,
                format!("{mins} {unit} left. Add 30 minutes from {place} or the window."),
            )
        }
        EngineEvent::Paused(PauseReason::LowBattery | PauseReason::OnBattery, battery)
            if switches.battery =>
        {
            let at = battery
                .percent
                .map_or_else(|| "On battery".to_owned(), |p| format!("Battery at {p}%"));
            toast(PAUSED_TITLE, format!("{at}. Plug in to keep it awake."))
        }
        // Resuming because a rule was relaxed is not the charger; say nothing.
        EngineEvent::Resumed if switches.battery && snapshot.battery.on_ac => {
            toast(RESUMED_TITLE, RESUMED_BODY.to_owned())
        }
        // While a manual session runs the window changes nothing you can see.
        EngineEvent::ScheduleStarted { ends_at_unix_ms }
            if switches.schedule && snapshot.mode == Mode::Scheduled =>
        {
            toast(
                SCHEDULE_STARTED_TITLE,
                format!("Awake until {}.", tray::when(*ends_at_unix_ms, now_unix_ms)),
            )
        }
        EngineEvent::ScheduleEnded { next_start_unix_ms }
            if switches.schedule && snapshot.mode == Mode::Off =>
        {
            let body = next_start_unix_ms.map_or_else(
                || "Your laptop sleeps as usual.".to_owned(),
                |next| {
                    format!(
                        "Your laptop sleeps as usual until {}.",
                        tray::when(next, now_unix_ms)
                    )
                },
            );
            toast(SCHEDULE_ENDED_TITLE, body)
        }
        // Part of the guard, so no switch of its own; gone if someone
        // touched the machine before it could be shown.
        EngineEvent::OvernightAsk {
            idle_secs,
            rests_at_unix_ms,
        } if snapshot.guard_rests_at_unix_ms.is_some() => {
            let rests = tray::when(*rests_at_unix_ms, now_unix_ms);
            if snapshot.guard_night {
                // Sleep when I'm away: the keys went quiet in the resting hours.
                return toast(
                    NIGHT_TITLE,
                    format!(
                        "Your laptop can sleep at {rests} unless you're back. Always comes back on when you return."
                    ),
                );
            }
            let hours = snapshot
                .settings
                .overnight_guard_hours
                .map_or(idle_secs / 3600, u64::from);
            let unit = if hours == 1 { "hour" } else { "hours" };
            toast(
                OVERNIGHT_TITLE,
                format!(
                    "No input for {hours} {unit}. Your laptop can sleep at {rests} unless you're back."
                ),
            )
        }
        _ => None,
    }
}

/// Shows the toast for `event`, if any. Called from the engine listener.
pub fn on_event(app: &AppHandle, event: &EngineEvent) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let snapshot = state.engine.snapshot();
    let Some(toast) = body_for(event, &snapshot, tray::now_unix_ms()) else {
        return;
    };
    let focused = app
        .get_webview_window(window::MAIN)
        .is_some_and(|w| w.is_focused().unwrap_or(false));
    if focused {
        log::debug!("window is focused; not showing \"{}\"", toast.title);
        return;
    }
    send(app, toast.title, &toast.body);
}

/// Shows one toast: the app icon, a title line and a body line.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        log::warn!("could not show a notification: {err}");
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;
    use crate::core::settings::Notifications;
    use crate::core::{BatteryState, GuardState, ScheduleState, Settings};

    fn local_ms(day: u32, h: u32, m: u32) -> u64 {
        let t = Local
            .with_ymd_and_hms(2026, 3, day, h, m, 0)
            .single()
            .unwrap();
        u64::try_from(t.timestamp_millis()).unwrap()
    }

    fn snapshot(mode: Mode, switches: Notifications) -> Snapshot {
        Snapshot {
            mode,
            started_at_unix_ms: None,
            ends_at_unix_ms: None,
            remaining_secs: None,
            plan_secs: None,
            guards: GuardState::default(),
            paused: None,
            battery: BatteryState {
                on_ac: true,
                percent: Some(80),
            },
            schedule: ScheduleState::default(),
            guard_rests_at_unix_ms: None,
            guard_night: false,
            rises_on_touch: false,
            guard_watching: false,
            settings: Settings {
                notifications: switches,
                ..Settings::default()
            },
            app_version: String::new(),
        }
    }

    const ALL: Notifications = Notifications {
        timer: true,
        battery: true,
        schedule: true,
    };
    const NONE: Notifications = Notifications {
        timer: false,
        battery: false,
        schedule: false,
    };

    fn body(event: &EngineEvent, snap: &Snapshot) -> Option<(&'static str, String)> {
        body_for(event, snap, local_ms(2, 12, 0)).map(|t| (t.title, t.body))
    }

    #[test]
    fn timer_toast_names_the_length() {
        let ended = EngineEvent::TimerEnded { plan_secs: 7200 };
        assert_eq!(
            body(&ended, &snapshot(Mode::Off, ALL)),
            Some((
                "Timer done",
                "Kept awake for 2h 00m, as asked. Your laptop sleeps as usual now.".to_owned()
            ))
        );
        let short = EngineEvent::TimerEnded { plan_secs: 1800 };
        assert_eq!(
            body(&short, &snapshot(Mode::Off, ALL)).map(|t| t.1),
            Some("Kept awake for 30m, as asked. Your laptop sleeps as usual now.".to_owned())
        );
    }

    #[test]
    fn sunset_toast_says_how_long_and_where_to_carry_on() {
        let soon = EngineEvent::SunsetSoon {
            ends_at_unix_ms: 0,
            secs_left: 300,
        };
        let place = if cfg!(target_os = "macos") {
            "the menu bar"
        } else {
            "the tray"
        };
        assert_eq!(
            body(&soon, &snapshot(Mode::Timed, ALL)),
            Some((
                "Timer ends soon",
                format!("5 minutes left. Add 30 minutes from {place} or the window.")
            ))
        );
        let last = EngineEvent::SunsetSoon {
            ends_at_unix_ms: 0,
            secs_left: 41,
        };
        assert!(
            body(&last, &snapshot(Mode::Timed, ALL))
                .is_some_and(|t| t.1.starts_with("1 minute left."))
        );
        assert_eq!(
            body(&soon, &snapshot(Mode::Timed, NONE)),
            None,
            "timer switch"
        );
        assert_eq!(body(&soon, &snapshot(Mode::Off, ALL)), None, "already over");
    }

    #[test]
    fn timer_toast_stays_quiet_when_a_schedule_took_over() {
        let ended = EngineEvent::TimerEnded { plan_secs: 60 };
        assert_eq!(body(&ended, &snapshot(Mode::Scheduled, ALL)), None);
    }

    #[test]
    fn battery_toasts() {
        let paused = EngineEvent::Paused(
            PauseReason::LowBattery,
            BatteryState {
                on_ac: false,
                percent: Some(18),
            },
        );
        assert_eq!(
            body(&paused, &snapshot(Mode::Indefinite, ALL)),
            Some((
                "Paused for the battery",
                "Battery at 18%. Plug in to keep it awake.".to_owned()
            ))
        );
        assert_eq!(
            body(&EngineEvent::Resumed, &snapshot(Mode::Indefinite, ALL)),
            Some((
                "Awake again",
                "Charger connected. Keeping your laptop awake again.".to_owned()
            ))
        );
        let mut unplugged = snapshot(Mode::Indefinite, ALL);
        unplugged.battery.on_ac = false;
        assert_eq!(body(&EngineEvent::Resumed, &unplugged), None);
        let refused = EngineEvent::Paused(PauseReason::PowerRequestFailed, BatteryState::default());
        assert_eq!(body(&refused, &snapshot(Mode::Indefinite, ALL)), None);
    }

    #[test]
    fn schedule_toasts() {
        let started = EngineEvent::ScheduleStarted {
            ends_at_unix_ms: local_ms(2, 18, 0),
        };
        assert_eq!(
            body(&started, &snapshot(Mode::Scheduled, ALL)),
            Some(("Schedule started", "Awake until 6:00 pm.".to_owned()))
        );
        let ended = EngineEvent::ScheduleEnded {
            next_start_unix_ms: Some(local_ms(3, 9, 0)),
        };
        assert_eq!(
            body(&ended, &snapshot(Mode::Off, ALL)),
            Some((
                "Schedule ended",
                "Your laptop sleeps as usual until 9:00 am.".to_owned()
            ))
        );
        // A manual session outlasting the window: nothing changed for you.
        assert_eq!(body(&ended, &snapshot(Mode::Timed, ALL)), None);
    }

    #[test]
    fn the_break_toast_says_how_long_in_hours_when_it_is_whole_hours() {
        assert_eq!(break_body(90), "90 minutes without a break.");
        assert_eq!(break_body(60), "1 hour without a break.");
        assert_eq!(break_body(120), "2 hours without a break.");
    }

    #[test]
    fn the_overnight_toast_says_how_long_and_when_it_rests() {
        let ask = EngineEvent::OvernightAsk {
            idle_secs: 6 * 3600 + 20,
            rests_at_unix_ms: local_ms(3, 3, 12),
        };
        let mut asking = snapshot(Mode::Indefinite, NONE);
        asking.guard_rests_at_unix_ms = Some(local_ms(3, 3, 12));
        assert_eq!(
            body_for(&ask, &asking, local_ms(3, 3, 2)).map(|t| (t.title, t.body)),
            Some((
                "Still there?",
                "No input for 6 hours. Your laptop can sleep at 3:12 am unless you're back."
                    .to_owned()
            )),
            "shown with every switch off: it belongs to the guard"
        );
        asking.settings.overnight_guard_hours = Some(1);
        assert!(
            body_for(&ask, &asking, local_ms(3, 3, 2))
                .is_some_and(|t| t.body.starts_with("No input for 1 hour."))
        );
        asking.guard_night = true;
        assert_eq!(
            body_for(&ask, &asking, local_ms(3, 3, 2)).map(|t| (t.title, t.body)),
            Some((
                "Away for the night",
                "Your laptop can sleep at 3:12 am unless you're back. Always comes back on when you return.".to_owned()
            )),
            "Sleep when I'm away says what it will do, and that it comes back"
        );
        let withdrawn = snapshot(Mode::Indefinite, ALL);
        assert_eq!(
            body(&ask, &withdrawn),
            None,
            "someone came back before it showed"
        );
    }

    #[test]
    fn every_switch_off_means_no_toasts() {
        let events = [
            EngineEvent::TimerEnded { plan_secs: 60 },
            EngineEvent::Paused(PauseReason::OnBattery, BatteryState::default()),
            EngineEvent::Resumed,
            EngineEvent::ScheduleStarted { ends_at_unix_ms: 0 },
            EngineEvent::ScheduleEnded {
                next_start_unix_ms: None,
            },
        ];
        for mode in [Mode::Off, Mode::Indefinite, Mode::Scheduled] {
            for event in &events {
                assert_eq!(body(event, &snapshot(mode, NONE)), None, "{event:?}");
            }
        }
    }
}
