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
    /// Launch without showing the window (tray only).
    pub start_hidden: bool,
    /// Closing the window keeps the app running in the tray.
    pub close_to_tray: bool,
    /// Tear down the UI process when the window is closed so the app idles at
    /// a few megabytes. Reopening takes a moment longer.
    pub free_memory_when_hidden: bool,
    /// Colour scheme.
    pub theme: Theme,
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
            start_hidden: true,
            close_to_tray: true,
            free_memory_when_hidden: true,
            theme: Theme::System,
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
