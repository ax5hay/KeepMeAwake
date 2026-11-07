//! The read model the UI and tray render. Produced by the engine after every
//! change; the UI never computes state on its own beyond ticking a countdown.

use serde::{Deserialize, Serialize};

use super::session::Mode;
use super::settings::Settings;

/// Why an active session is currently not holding the machine awake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    /// Battery dropped below the configured threshold.
    LowBattery,
    /// The user asked to pause whenever the laptop is unplugged.
    OnBattery,
    /// The operating system refused the power request.
    PowerRequestFailed,
}

/// What the OS is currently being asked to keep on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GuardState {
    /// The system is prevented from sleeping.
    pub system: bool,
    /// The display is prevented from turning off.
    pub display: bool,
}

/// Power source and charge, as last observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct BatteryState {
    /// Plugged into mains power.
    pub on_ac: bool,
    /// Charge percentage, when a battery is present and the value is known.
    pub percent: Option<u8>,
}

/// Where the schedule stands right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ScheduleState {
    /// The schedule is switched on.
    pub enabled: bool,
    /// A window is open right now (whether or not it was skipped).
    pub in_window: bool,
    /// The open window was skipped; the laptop rests until it closes.
    pub skipped: bool,
    /// When the open window started, Unix ms.
    pub window_start_unix_ms: Option<u64>,
    /// When the open window ends, Unix ms.
    pub window_end_unix_ms: Option<u64>,
    /// When the next window (after any open one) starts, Unix ms.
    pub next_start_unix_ms: Option<u64>,
}

/// Everything the UI needs, in one message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Snapshot {
    /// Current session mode.
    pub mode: Mode,
    /// When the session started, Unix ms. `None` when off.
    pub started_at_unix_ms: Option<u64>,
    /// When a timed session ends, Unix ms. `None` when off or indefinite.
    pub ends_at_unix_ms: Option<u64>,
    /// Seconds left in a timed session, rounded up.
    pub remaining_secs: Option<u64>,
    /// Full length of the current timed plan in seconds, for progress display.
    pub plan_secs: Option<u64>,
    /// Guards actually applied right now (may differ from the plan when paused).
    pub guards: GuardState,
    /// Why the session is active but not guarding, if so.
    pub paused: Option<PauseReason>,
    /// Power source and charge.
    pub battery: BatteryState,
    /// The schedule's state; `mode` is `Scheduled` when it is what holds the
    /// laptop awake, and the timing fields then describe the window.
    pub schedule: ScheduleState,
    /// While the overnight guard is asking whether anyone is there: when the
    /// session will end unless someone answers, Unix ms. `None` otherwise.
    pub guard_rests_at_unix_ms: Option<u64>,
    /// The overnight guard is watching this session: an Always session,
    /// the guard on, and nobody said Keep it up for it.
    pub guard_watching: bool,
    /// User preferences, so the UI has a single source of truth.
    pub settings: Settings,
    /// Application version, for the About section.
    pub app_version: String,
}

impl Snapshot {
    /// `true` when the session is on and the guard is actually held.
    pub fn is_guarding(&self) -> bool {
        self.mode != Mode::Off && self.paused.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Snapshot {
        Snapshot {
            mode: Mode::Off,
            started_at_unix_ms: None,
            ends_at_unix_ms: None,
            remaining_secs: None,
            plan_secs: None,
            guards: GuardState::default(),
            paused: None,
            battery: BatteryState::default(),
            schedule: ScheduleState::default(),
            guard_rests_at_unix_ms: None,
            guard_watching: false,
            settings: Settings::default(),
            app_version: "0.0.0".into(),
        }
    }

    #[test]
    fn guarding_requires_active_and_not_paused() {
        let mut s = base();
        assert!(!s.is_guarding());
        s.mode = Mode::Indefinite;
        assert!(s.is_guarding());
        s.paused = Some(PauseReason::LowBattery);
        assert!(!s.is_guarding());
        s.paused = None;
        s.mode = Mode::Scheduled;
        assert!(s.is_guarding(), "a schedule window guards like a session");
    }

    #[test]
    fn schedule_state_serialises_snake_case() {
        let json = serde_json::to_value(base()).unwrap();
        assert_eq!(json["schedule"]["in_window"], false);
        assert!(json["schedule"].get("next_start_unix_ms").is_some());
        assert_eq!(
            serde_json::to_value(Mode::Scheduled).unwrap(),
            serde_json::json!("scheduled")
        );
    }

    #[test]
    fn serialises_snake_case() {
        let json = serde_json::to_value(base()).unwrap();
        assert!(json.get("started_at_unix_ms").is_some());
        assert_eq!(json["mode"], "off");
        assert!(json["guard_rests_at_unix_ms"].is_null());
        assert_eq!(json["guard_watching"], false);
        assert_eq!(json["settings"]["overnight_guard_hours"], 6);
    }
}
