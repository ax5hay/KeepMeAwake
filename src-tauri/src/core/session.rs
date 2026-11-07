//! The session state machine: off, awake indefinitely, or awake until a deadline.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::plan::PlanRequest;

/// What the laptop is doing and why. The first three are shapes of a manual
/// session; `Scheduled` is the schedule holding it awake with no manual
/// session on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Not keeping anything awake.
    Off,
    /// Awake until the user turns it off.
    Indefinite,
    /// Awake until a deadline, then back to `Off` automatically.
    Timed,
    /// No manual session, but a schedule window is open and not skipped.
    /// Never produced by [`Session::mode`]; the engine derives it.
    Scheduled,
}

/// Something notable that happened inside the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// A timed session reached its deadline and switched itself off.
    Expired,
}

/// Monotonic view of the current session. Wall-clock values for the UI are
/// derived by the caller, so this type never depends on the system clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Session {
    mode: Option<ActiveMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveMode {
    started: Instant,
    planned: Instant,
    deadline: Option<Instant>,
}

impl Session {
    /// A session that is switched off.
    pub fn off() -> Self {
        Self::default()
    }

    /// Current mode.
    pub fn mode(&self) -> Mode {
        match self.mode {
            None => Mode::Off,
            Some(ActiveMode { deadline: None, .. }) => Mode::Indefinite,
            Some(ActiveMode {
                deadline: Some(_), ..
            }) => Mode::Timed,
        }
    }

    /// `true` while the session wants the machine awake.
    pub fn is_active(&self) -> bool {
        self.mode.is_some()
    }

    /// When the current session began, if active.
    pub fn started_at(&self) -> Option<Instant> {
        self.mode.map(|m| m.started)
    }

    /// The deadline of a timed session.
    pub fn deadline(&self) -> Option<Instant> {
        self.mode.and_then(|m| m.deadline)
    }

    /// Full length of the current timed plan, measured from when it was set.
    /// Lets the UI show progress even when the plan came from the tray.
    pub fn plan_duration(&self) -> Option<Duration> {
        self.mode.and_then(|m| {
            m.deadline
                .map(|end| end.saturating_duration_since(m.planned))
        })
    }

    /// Time left in a timed session. `None` for off or indefinite sessions.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline()
            .map(|end| end.saturating_duration_since(now))
    }

    /// How long the session has been running.
    pub fn elapsed(&self, now: Instant) -> Option<Duration> {
        self.started_at()
            .map(|start| now.saturating_duration_since(start))
    }

    /// Applies a user request. `Until` requests must be resolved to `For`
    /// first (see [`PlanRequest::resolve`]); passing one here is treated as `Off`
    /// because the state machine has no wall clock.
    ///
    /// Returns `true` when the session actually changed.
    pub fn apply(&mut self, request: PlanRequest, now: Instant) -> bool {
        let started = self.started_at().unwrap_or(now);
        let next = match request {
            PlanRequest::Off | PlanRequest::Until { .. } => None,
            PlanRequest::Indefinite => Some(ActiveMode {
                started,
                planned: now,
                deadline: None,
            }),
            PlanRequest::For { seconds } => Some(ActiveMode {
                started,
                planned: now,
                deadline: Some(now + Duration::from_secs(seconds.max(1))),
            }),
        };
        let changed = match (self.mode, next) {
            (Some(a), Some(b)) => a.deadline != b.deadline,
            (a, b) => a.is_some() != b.is_some(),
        };
        self.mode = next;
        changed
    }

    /// Flips between off and the given "on" request.
    pub fn toggle(&mut self, on: PlanRequest, now: Instant) -> bool {
        if self.is_active() {
            self.apply(PlanRequest::Off, now)
        } else {
            self.apply(on, now)
        }
    }

    /// Advances time. A timed session whose deadline has passed turns off and
    /// reports [`SessionEvent::Expired`].
    pub fn tick(&mut self, now: Instant) -> Option<SessionEvent> {
        match self.deadline() {
            Some(end) if now >= end => {
                self.mode = None;
                Some(SessionEvent::Expired)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn starts_off() {
        let s = Session::default();
        assert_eq!(s.mode(), Mode::Off);
        assert!(!s.is_active());
        assert_eq!(s.remaining(t0()), None);
        assert_eq!(s.elapsed(t0()), None);
    }

    #[test]
    fn indefinite_has_no_deadline() {
        let now = t0();
        let mut s = Session::off();
        assert!(s.apply(PlanRequest::Indefinite, now));
        assert_eq!(s.mode(), Mode::Indefinite);
        assert_eq!(s.deadline(), None);
        assert_eq!(s.started_at(), Some(now));
        assert_eq!(
            s.elapsed(now + Duration::from_secs(7)),
            Some(Duration::from_secs(7))
        );
    }

    #[test]
    fn timed_counts_down_and_expires() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::For { seconds: 60 }, now);
        assert_eq!(s.mode(), Mode::Timed);
        assert_eq!(s.remaining(now), Some(Duration::from_secs(60)));
        assert_eq!(
            s.remaining(now + Duration::from_secs(45)),
            Some(Duration::from_secs(15))
        );
        assert_eq!(s.tick(now + Duration::from_secs(59)), None);
        assert_eq!(
            s.tick(now + Duration::from_secs(60)),
            Some(SessionEvent::Expired)
        );
        assert!(!s.is_active());
    }

    #[test]
    fn remaining_saturates_at_zero() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::For { seconds: 1 }, now);
        assert_eq!(
            s.remaining(now + Duration::from_secs(5)),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn switching_plan_keeps_original_start() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::Indefinite, now);
        let later = now + Duration::from_secs(30);
        assert!(s.apply(PlanRequest::For { seconds: 10 }, later));
        assert_eq!(s.started_at(), Some(now));
        assert_eq!(s.deadline(), Some(later + Duration::from_secs(10)));
        assert_eq!(s.plan_duration(), Some(Duration::from_secs(10)));
    }

    #[test]
    fn plan_duration_is_none_unless_timed() {
        let now = t0();
        let mut s = Session::off();
        assert_eq!(s.plan_duration(), None);
        s.apply(PlanRequest::Indefinite, now);
        assert_eq!(s.plan_duration(), None);
    }

    #[test]
    fn apply_reports_no_change_when_identical() {
        let now = t0();
        let mut s = Session::off();
        assert!(!s.apply(PlanRequest::Off, now));
        s.apply(PlanRequest::Indefinite, now);
        assert!(!s.apply(PlanRequest::Indefinite, now));
    }

    #[test]
    fn toggle_flips_state() {
        let now = t0();
        let mut s = Session::off();
        assert!(s.toggle(PlanRequest::Indefinite, now));
        assert!(s.is_active());
        assert!(s.toggle(PlanRequest::Indefinite, now));
        assert!(!s.is_active());
    }

    #[test]
    fn unresolved_until_is_treated_as_off() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::Indefinite, now);
        s.apply(PlanRequest::Until { unix_ms: 1 }, now);
        assert!(!s.is_active());
    }

    #[test]
    fn zero_second_timer_lasts_at_least_one_second() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::For { seconds: 0 }, now);
        assert_eq!(s.remaining(now), Some(Duration::from_secs(1)));
    }

    #[test]
    fn tick_on_indefinite_never_expires() {
        let now = t0();
        let mut s = Session::off();
        s.apply(PlanRequest::Indefinite, now);
        assert_eq!(s.tick(now + Duration::from_secs(1_000_000)), None);
        assert!(s.is_active());
    }
}
