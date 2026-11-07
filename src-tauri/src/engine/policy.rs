//! Pure decisions: given what the user wants and what the machine looks like,
//! what should we ask the OS for?

use std::time::{Duration, Instant};

use crate::core::{BatteryState, GuardState, PauseReason, Session, Settings};

/// How often the battery is re-read while a session is active. Unplugging a
/// laptop is the one external event that can change our decision, and thirty
/// seconds is well inside the window where a low battery still matters.
pub const BATTERY_POLL: Duration = Duration::from_secs(30);

/// Decides which guards to hold.
///
/// Returns the desired guard state plus, when the session is active but we
/// deliberately hold nothing, the reason, so the UI can explain itself.
pub fn decide(
    active: bool,
    settings: &Settings,
    battery: BatteryState,
) -> (GuardState, Option<PauseReason>) {
    if !active {
        return (GuardState::default(), None);
    }
    if !battery.on_ac {
        if settings.pause_on_battery {
            return (GuardState::default(), Some(PauseReason::OnBattery));
        }
        if let (Some(threshold), Some(percent)) =
            (settings.pause_below_battery_percent, battery.percent)
            && percent < threshold
        {
            return (GuardState::default(), Some(PauseReason::LowBattery));
        }
    }
    (
        GuardState {
            system: true,
            display: settings.keep_display_on,
        },
        None,
    )
}

/// Longest the engine sleeps while the schedule waits for its next window.
/// Windows stops counting wait timeouts while the machine sleeps, so one
/// long wait set at night could fire hours late after the lid opens in the
/// morning; re-checking every few minutes bounds that lateness.
pub const SCHEDULE_RECHECK: Duration = Duration::from_secs(5 * 60);

/// The next moment the engine must wake up on its own: the session deadline,
/// the next battery poll while anything is holding the laptop awake, or the
/// next schedule boundary (capped by [`SCHEDULE_RECHECK`]), whichever comes
/// first. `None` means "sleep until a command arrives".
pub fn next_deadline(
    session: &Session,
    scheduled: bool,
    schedule_boundary: Option<Instant>,
    now: Instant,
) -> Option<Instant> {
    let poll = (session.is_active() || scheduled).then(|| now + BATTERY_POLL);
    let boundary = schedule_boundary.map(|at| at.min(now + SCHEDULE_RECHECK));
    [session.deadline(), poll, boundary]
        .into_iter()
        .flatten()
        .min()
}

/// When a timed session should say the sun is about to set: `warning_mins`
/// before its deadline, and only if the plan is longer than the warning, so a
/// short timer is not announced the moment it starts.
pub fn sunset_warning_at(session: &Session, warning_mins: Option<u8>) -> Option<Instant> {
    let warning = Duration::from_secs(u64::from(warning_mins?) * 60);
    let deadline = session.deadline()?;
    if session.plan_duration()? > warning {
        deadline.checked_sub(warning)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PlanRequest;

    fn mains() -> BatteryState {
        BatteryState {
            on_ac: true,
            percent: Some(80),
        }
    }

    fn battery(percent: u8) -> BatteryState {
        BatteryState {
            on_ac: false,
            percent: Some(percent),
        }
    }

    #[test]
    fn inactive_holds_nothing() {
        let (g, p) = decide(false, &Settings::default(), battery(3));
        assert_eq!(g, GuardState::default());
        assert_eq!(p, None);
    }

    #[test]
    fn active_on_mains_holds_system_and_display() {
        let (g, p) = decide(true, &Settings::default(), mains());
        assert_eq!(
            g,
            GuardState {
                system: true,
                display: true
            }
        );
        assert_eq!(p, None);
    }

    #[test]
    fn display_guard_follows_setting() {
        let s = Settings {
            keep_display_on: false,
            ..Settings::default()
        };
        let (g, _) = decide(true, &s, mains());
        assert_eq!(
            g,
            GuardState {
                system: true,
                display: false
            }
        );
    }

    #[test]
    fn low_battery_pauses() {
        let (g, p) = decide(true, &Settings::default(), battery(19));
        assert_eq!(g, GuardState::default());
        assert_eq!(p, Some(PauseReason::LowBattery));
    }

    #[test]
    fn battery_at_threshold_still_guards() {
        let (g, p) = decide(true, &Settings::default(), battery(20));
        assert!(g.system);
        assert_eq!(p, None);
    }

    #[test]
    fn low_battery_rule_can_be_disabled() {
        let s = Settings {
            pause_below_battery_percent: None,
            ..Settings::default()
        };
        let (g, p) = decide(true, &s, battery(2));
        assert!(g.system);
        assert_eq!(p, None);
    }

    #[test]
    fn unknown_percent_never_pauses() {
        let (g, p) = decide(
            true,
            &Settings::default(),
            BatteryState {
                on_ac: false,
                percent: None,
            },
        );
        assert!(g.system);
        assert_eq!(p, None);
    }

    #[test]
    fn pause_on_battery_wins_over_percentage() {
        let s = Settings {
            pause_on_battery: true,
            ..Settings::default()
        };
        let (_, p) = decide(true, &s, battery(99));
        assert_eq!(p, Some(PauseReason::OnBattery));
        let (_, p) = decide(true, &s, mains());
        assert_eq!(p, None);
    }

    #[test]
    fn no_deadline_when_off() {
        assert_eq!(
            next_deadline(&Session::off(), false, None, Instant::now()),
            None
        );
    }

    #[test]
    fn indefinite_wakes_for_battery_poll() {
        let now = Instant::now();
        let mut s = Session::off();
        s.apply(PlanRequest::Indefinite, now);
        assert_eq!(
            next_deadline(&s, false, None, now),
            Some(now + BATTERY_POLL)
        );
    }

    #[test]
    fn timed_wakes_at_the_earlier_of_deadline_and_poll() {
        let now = Instant::now();
        let mut s = Session::off();
        s.apply(PlanRequest::For { seconds: 5 }, now);
        assert_eq!(
            next_deadline(&s, false, None, now),
            Some(now + Duration::from_secs(5))
        );
        s.apply(PlanRequest::For { seconds: 3600 }, now);
        assert_eq!(
            next_deadline(&s, false, None, now),
            Some(now + BATTERY_POLL)
        );
    }

    #[test]
    fn a_window_keeps_the_battery_poll_running() {
        let now = Instant::now();
        assert_eq!(
            next_deadline(&Session::off(), true, None, now),
            Some(now + BATTERY_POLL)
        );
    }

    #[test]
    fn schedule_boundary_wakes_exactly_when_near_and_is_capped_when_far() {
        let now = Instant::now();
        let soon = now + Duration::from_secs(60);
        assert_eq!(
            next_deadline(&Session::off(), false, Some(soon), now),
            Some(soon)
        );
        let tomorrow = now + Duration::from_secs(15 * 3600);
        assert_eq!(
            next_deadline(&Session::off(), false, Some(tomorrow), now),
            Some(now + SCHEDULE_RECHECK)
        );
        let edge = now + Duration::from_secs(10);
        assert_eq!(
            next_deadline(&Session::off(), true, Some(edge), now),
            Some(edge)
        );
    }

    #[test]
    fn warns_the_set_minutes_before_a_long_enough_timer_ends() {
        let now = Instant::now();
        let mut s = Session::off();
        s.apply(PlanRequest::For { seconds: 600 }, now);
        assert_eq!(
            sunset_warning_at(&s, Some(5)),
            Some(now + Duration::from_secs(300))
        );
        assert_eq!(sunset_warning_at(&s, None), None, "off");
        assert_eq!(sunset_warning_at(&s, Some(10)), None, "plan not longer");
        s.apply(PlanRequest::Indefinite, now);
        assert_eq!(sunset_warning_at(&s, Some(5)), None, "no end, no sunset");
        assert_eq!(sunset_warning_at(&Session::off(), Some(5)), None);
    }
}
