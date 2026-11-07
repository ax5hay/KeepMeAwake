//! User preferences. Persisted as JSON; every field has a default so older
//! files keep loading after new fields are added.

use serde::{Deserialize, Serialize};

/// Current on-disk schema version. Bump when a field changes meaning.
pub const SETTINGS_VERSION: u32 = 1;

/// Which colour scheme the UI uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow Windows.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// What happens when the app launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StartupPlan {
    /// Start idle; the user turns it on.
    #[default]
    Off,
    /// Start awake indefinitely.
    Indefinite,
    /// Start awake for the default duration.
    DefaultDuration,
}

/// Everything the user can tune. Grouped the same way the settings UI is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
#[allow(clippy::struct_excessive_bools)] // they are independent switches, not a state machine
pub struct Settings {
    /// Schema version of this document.
    pub version: u32,
    /// Keep the display on as well as the system. Off lets the screen dim
    /// while the machine keeps running.
    pub keep_display_on: bool,
    /// Duration used by the toggle (tray, orb) when no explicit duration was
    /// chosen. `None` means indefinite.
    pub default_duration_secs: Option<u64>,
    /// Session to start when the app launches.
    pub startup_plan: StartupPlan,
    /// Release the wake guard while the laptop runs on battery below this
    /// percentage. `None` disables the rule.
    pub pause_below_battery_percent: Option<u8>,
    /// Release the wake guard whenever the laptop is unplugged.
    pub pause_on_battery: bool,
    /// Launch without showing the window (tray only). Launches triggered by
    /// autostart are always hidden regardless of this.
    pub start_hidden: bool,
    /// Closing the window keeps the app running in the tray.
    pub close_to_tray: bool,
    /// Tear down the UI process when the window is closed so the app idles at
    /// a few megabytes. Reopening takes a moment longer.
    pub free_memory_when_hidden: bool,
    /// Colour scheme.
    pub theme: Theme,
    /// Hours during which the laptop is kept awake without being asked.
    pub schedule: Schedule,
    /// Which moments get a desktop toast.
    pub notifications: Notifications,
    /// Global hotkey that toggles from any app, in accelerator syntax
    /// (`Ctrl+Alt+K`, `Super+Shift+F9`). `None` turns it off. Validated when
    /// it is registered, not here.
    pub hotkey: Option<String>,
}

/// The hotkey a fresh install gets: `⌘⌥K` on a Mac, `Ctrl+Alt+K` elsewhere.
/// A settings file carried across keeps whatever it stored.
pub const DEFAULT_HOTKEY: &str = if cfg!(target_os = "macos") {
    "Cmd+Alt+K"
} else {
    "Ctrl+Alt+K"
};

/// One switch per kind of toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct Notifications {
    /// A timed session ended on its own.
    pub timer: bool,
    /// The battery rule paused the sun, or the charger brought it back.
    pub battery: bool,
    /// A schedule window opened or closed.
    pub schedule: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            timer: true,
            battery: false,
            schedule: false,
        }
    }
}

/// One window of time on chosen weekdays, e.g. 09:00 until 18:00 Monday to
/// Friday. An `end_minutes` earlier than `start_minutes` means the window runs
/// overnight and belongs to the day it starts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct Schedule {
    /// The schedule applies at all.
    pub enabled: bool,
    /// Which weekdays have a window, Monday first.
    pub days: [bool; 7],
    /// Start of the window, minutes after local midnight.
    pub start_minutes: u16,
    /// End of the window, minutes after local midnight.
    pub end_minutes: u16,
}

/// Last minute of a day, as minutes after midnight.
pub const LAST_MINUTE: u16 = 24 * 60 - 1;

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            days: [true, true, true, true, true, false, false],
            start_minutes: 9 * 60,
            end_minutes: 18 * 60,
        }
    }
}

impl Schedule {
    /// Clamps the minutes into a day and switches off a zero-length window,
    /// which would otherwise mean nothing at all.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.start_minutes = self.start_minutes.min(LAST_MINUTE);
        self.end_minutes = self.end_minutes.min(LAST_MINUTE);
        if self.start_minutes == self.end_minutes {
            self.enabled = false;
        }
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            keep_display_on: true,
            default_duration_secs: None,
            startup_plan: StartupPlan::Off,
            pause_below_battery_percent: Some(20),
            pause_on_battery: false,
            start_hidden: false,
            close_to_tray: true,
            free_memory_when_hidden: true,
            theme: Theme::System,
            schedule: Schedule::default(),
            notifications: Notifications::default(),
            hotkey: Some(DEFAULT_HOTKEY.to_owned()),
        }
    }
}

impl Settings {
    /// Clamps values into their valid ranges. Called after loading from disk
    /// and after every update from the UI so the engine never sees nonsense.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.version = SETTINGS_VERSION;
        if let Some(pct) = self.pause_below_battery_percent {
            self.pause_below_battery_percent = Some(pct.clamp(5, 95));
        }
        if let Some(secs) = self.default_duration_secs {
            // Between one minute and one week.
            self.default_duration_secs = Some(secs.clamp(60, 7 * 24 * 60 * 60));
        }
        self.schedule = self.schedule.sanitized();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let s = Settings::default();
        assert!(s.keep_display_on);
        assert_eq!(s.pause_below_battery_percent, Some(20));
        assert!(!s.pause_on_battery);
        assert!(s.close_to_tray);
        assert!(s.free_memory_when_hidden);
        assert!(!s.start_hidden, "a manual launch should show the window");
        assert_eq!(s.startup_plan, StartupPlan::Off);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"version":1,"keep_display_on":false}"#).unwrap();
        assert!(!s.keep_display_on);
        assert_eq!(s.pause_below_battery_percent, Some(20));
        assert_eq!(s.theme, Theme::System);
    }

    #[test]
    fn unknown_fields_are_ignored_for_forward_compatibility() {
        let s: Settings = serde_json::from_str(r#"{"version":9,"future_field":true}"#).unwrap();
        assert_eq!(s.sanitized().version, SETTINGS_VERSION);
    }

    #[test]
    fn round_trips_through_json() {
        let s = Settings {
            theme: Theme::Dark,
            default_duration_secs: Some(3600),
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn sanitize_clamps_ranges() {
        let s = Settings {
            pause_below_battery_percent: Some(100),
            default_duration_secs: Some(1),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.pause_below_battery_percent, Some(95));
        assert_eq!(s.default_duration_secs, Some(60));
    }

    #[test]
    fn schedule_defaults_to_weekday_office_hours_switched_off() {
        let s = Schedule::default();
        assert!(!s.enabled);
        assert_eq!(s.days, [true, true, true, true, true, false, false]);
        assert_eq!((s.start_minutes, s.end_minutes), (540, 1080));
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.schedule, Schedule::default());
    }

    #[test]
    fn sanitize_clamps_schedule_minutes_and_disables_empty_windows() {
        let s = Schedule {
            enabled: true,
            start_minutes: 5000,
            end_minutes: 60,
            ..Schedule::default()
        }
        .sanitized();
        assert_eq!(s.start_minutes, LAST_MINUTE);
        assert!(s.enabled);
        let empty = Schedule {
            enabled: true,
            start_minutes: 600,
            end_minutes: 600,
            ..Schedule::default()
        }
        .sanitized();
        assert!(!empty.enabled);
    }

    #[test]
    fn schedule_serialises_snake_case() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["schedule"]["start_minutes"], 540);
        assert_eq!(json["schedule"]["end_minutes"], 1080);
        assert_eq!(json["schedule"]["days"].as_array().map(Vec::len), Some(7));
        assert_eq!(json["schedule"]["enabled"], false);
    }

    #[test]
    fn notifications_default_to_the_timer_only() {
        let n = Settings::default().notifications;
        assert!(n.timer);
        assert!(!n.battery);
        assert!(!n.schedule);
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.notifications, Notifications::default());
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["notifications"]["timer"], true);
    }

    #[test]
    fn hotkey_defaults_on_and_off_survives_a_round_trip() {
        let native = if cfg!(target_os = "macos") {
            "Cmd+Alt+K"
        } else {
            "Ctrl+Alt+K"
        };
        assert_eq!(Settings::default().hotkey.as_deref(), Some(native));
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.hotkey.as_deref(), Some(native));
        let off = Settings {
            hotkey: None,
            ..Settings::default()
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&off).unwrap()).unwrap();
        assert_eq!(back.hotkey, None, "an explicit null stays off");
    }

    #[test]
    fn enums_use_snake_case() {
        assert_eq!(
            serde_json::to_string(&StartupPlan::DefaultDuration).unwrap(),
            r#""default_duration""#
        );
        assert_eq!(
            serde_json::to_string(&Theme::System).unwrap(),
            r#""system""#
        );
    }
}
