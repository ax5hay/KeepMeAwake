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
    /// Minutes before a timed session ends to warn that the sun is about to
    /// set. `None` turns the warning off.
    pub sunset_warning_mins: Option<u8>,
    /// Hours without keyboard, mouse or trackpad input after which an
    /// Always session asks once, then lets the laptop sleep. `None` turns
    /// the overnight guard off.
    pub overnight_guard_hours: Option<u8>,
    /// The almanac: whether presence is kept, and what a working day is.
    pub almanac: AlmanacSettings,
    /// Claude: the master switch and one switch per feature.
    pub ai: AiSettings,
    /// Ask GitHub once a day whether a newer release exists.
    pub check_for_updates: bool,
    /// When that was last asked, Unix ms; 0 for never. Written by the app,
    /// not the UI.
    pub last_update_check_unix_ms: u64,
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
/// overnight and belongs to the day it starts on; the same time twice is the
/// whole day, from that time until the same time the next day.
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

/// Which AI service is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Anthropic's Claude, through its Messages API.
    #[default]
    Anthropic,
    /// `OpenAI`, through Chat Completions.
    #[serde(rename = "openai")]
    OpenAi,
    /// Any other service that speaks the same chat protocol, at an address
    /// of one's own: a hosted one, or a model running on this machine.
    Custom,
}

/// AI's switches and the service it uses. `enabled` is the master: off,
/// nothing is ever sent, whatever the others say. It is off until someone
/// turns it on, and it does nothing without an API key, which is kept in
/// the OS credential store and not here. Each feature also has a switch of
/// its own, shown where the feature is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
#[allow(clippy::struct_excessive_bools)] // independent switches, one per feature, as in Settings
pub struct AiSettings {
    /// AI may be asked at all.
    pub enabled: bool,
    /// The Almanac can put a week into words.
    pub week_in_words: bool,
    /// Home can turn a sentence into a plan.
    pub tell_the_sun: bool,
    /// The Almanac can put the day being read into a line.
    pub day_in_a_line: bool,
    /// The Almanac can answer a question about the month in view.
    pub ask_the_almanac: bool,
    /// Everything of AI's is kept out of sight, but for one button in
    /// Settings that brings it back. Hidden, nothing can be asked either.
    pub hidden: bool,
    /// The service asked.
    pub provider: Provider,
    /// The model asked; empty for the service's smallest.
    pub model: String,
    /// For a service of one's own: its address, `https://…/v1`.
    pub base_url: String,
}

/// The longest a model's name or a service's address may be.
const MAX_AI_TEXT: usize = 200;

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            week_in_words: true,
            tell_the_sun: true,
            day_in_a_line: true,
            ask_the_almanac: true,
            hidden: false,
            provider: Provider::Anthropic,
            model: String::new(),
            base_url: String::new(),
        }
    }
}

impl AiSettings {
    /// Trims the model and the address, and bounds their length.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        let tidy = |text: &str| text.trim().chars().take(MAX_AI_TEXT).collect::<String>();
        self.model = tidy(&self.model);
        self.base_url = tidy(&self.base_url);
        self
    }
}

/// What the almanac keeps and how it reads a day. The hours are when work
/// is meant to happen, not when it may: time outside them still counts, and
/// is shown as over. An `end_minutes` at or before `start_minutes` runs
/// past midnight, and equal ones mean the whole day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct AlmanacSettings {
    /// Keep the almanac at all. Off, the idle time is never read for it.
    pub enabled: bool,
    /// Which weekdays are working days, Monday first.
    pub work_days: [bool; 7],
    /// Start of working hours, minutes after local midnight.
    pub start_minutes: u16,
    /// End of working hours, minutes after local midnight.
    pub end_minutes: u16,
    /// A past working day with nothing in it reads as leave.
    pub empty_days_are_leave: bool,
    /// Morning whisper once, on the first open after it.
    pub whisper_night: bool,
    /// Minutes without input, inside the resting hours, after which an
    /// Always session sets for the night and rises again at the next
    /// touch. `None` leaves the sun to the overnight guard alone.
    pub rest_after_mins: Option<u16>,
    /// Minutes at the machine without a break of five after which a toast
    /// says so, once. `None` never says anything.
    pub break_after_mins: Option<u16>,
}

/// Minutes in a day.
const DAY_MINUTES: u16 = 24 * 60;

/// How far the resting hours reach either side of the turn of the day.
pub const REST_REACH_MINUTES: u16 = 5 * 60;

impl Default for AlmanacSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            work_days: [true, true, true, true, true, false, false],
            start_minutes: 9 * 60,
            end_minutes: 18 * 60,
            empty_days_are_leave: true,
            whisper_night: true,
            rest_after_mins: None,
            break_after_mins: None,
        }
    }
}

impl AlmanacSettings {
    /// Clamps the hours into a day, and the wait for the night to between
    /// ten minutes and four hours.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.start_minutes = self.start_minutes.min(LAST_MINUTE);
        self.end_minutes = self.end_minutes.min(LAST_MINUTE);
        if let Some(mins) = self.rest_after_mins {
            self.rest_after_mins = Some(mins.clamp(10, 240));
        }
        if let Some(mins) = self.break_after_mins {
            self.break_after_mins = Some(mins.clamp(15, 480));
        }
        self
    }

    /// The hour at which a day turns over: opposite the middle of the
    /// working hours, on the hour. 9 am to 6 pm turns at 1; 3 pm to
    /// midnight at 7. The window reads days the same way.
    pub fn turn_hour(&self) -> u16 {
        let length = match (self.end_minutes + DAY_MINUTES - self.start_minutes) % DAY_MINUTES {
            0 => DAY_MINUTES,
            length => length,
        };
        let middle = self.start_minutes + length / 2;
        ((middle + DAY_MINUTES / 2) % DAY_MINUTES) / 60
    }

    /// Whether `minute` of the day falls in the resting hours: the ten
    /// hours centred on the turn of the day.
    pub fn is_resting(&self, minute: u16) -> bool {
        let turn = self.turn_hour() * 60;
        let apart = (minute % DAY_MINUTES).abs_diff(turn);
        apart.min(DAY_MINUTES - apart) <= REST_REACH_MINUTES
    }
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
    /// Clamps the minutes into a day. The switch is never touched: equal
    /// times once switched the schedule off here, and since its hours cannot
    /// be edited while it is off, that left no way to switch it back on.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.start_minutes = self.start_minutes.min(LAST_MINUTE);
        self.end_minutes = self.end_minutes.min(LAST_MINUTE);
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
            sunset_warning_mins: Some(5),
            overnight_guard_hours: Some(6),
            almanac: AlmanacSettings::default(),
            ai: AiSettings::default(),
            check_for_updates: true,
            last_update_check_unix_ms: 0,
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
        if let Some(mins) = self.sunset_warning_mins {
            self.sunset_warning_mins = Some(mins.clamp(1, 60));
        }
        if let Some(hours) = self.overnight_guard_hours {
            self.overnight_guard_hours = Some(hours.clamp(1, 24));
        }
        self.schedule = self.schedule.sanitized();
        self.almanac = self.almanac.sanitized();
        self.ai = self.ai.sanitized();
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
    fn sunset_warning_defaults_to_five_minutes_and_is_clamped() {
        assert_eq!(Settings::default().sunset_warning_mins, Some(5));
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.sunset_warning_mins, Some(5), "older files gain it");
        let off: Settings =
            serde_json::from_str(r#"{"version":1,"sunset_warning_mins":null}"#).unwrap();
        assert_eq!(off.sunset_warning_mins, None);
        let wild = Settings {
            sunset_warning_mins: Some(0),
            ..Settings::default()
        };
        assert_eq!(wild.sanitized().sunset_warning_mins, Some(1));
        let wild = Settings {
            sunset_warning_mins: Some(200),
            ..Settings::default()
        };
        assert_eq!(wild.sanitized().sunset_warning_mins, Some(60));
    }

    #[test]
    fn overnight_guard_defaults_to_six_hours_and_is_clamped() {
        assert_eq!(Settings::default().overnight_guard_hours, Some(6));
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.overnight_guard_hours, Some(6), "older files gain it");
        let off: Settings =
            serde_json::from_str(r#"{"version":1,"overnight_guard_hours":null}"#).unwrap();
        assert_eq!(
            off.overnight_guard_hours, None,
            "an explicit null stays off"
        );
        for (wild, clamped) in [(0, 1), (25, 24), (200, 24), (8, 8)] {
            let s = Settings {
                overnight_guard_hours: Some(wild),
                ..Settings::default()
            };
            assert_eq!(s.sanitized().overnight_guard_hours, Some(clamped));
        }
    }

    #[test]
    fn the_almanac_defaults_on_with_weekday_office_hours() {
        let a = Settings::default().almanac;
        assert!(a.enabled);
        assert_eq!(a.work_days, [true, true, true, true, true, false, false]);
        assert_eq!((a.start_minutes, a.end_minutes), (540, 1080));
        assert!(a.empty_days_are_leave);
        assert!(a.whisper_night);
        assert_eq!(a.rest_after_mins, None, "the sun follows nobody unasked");
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(
            old.almanac,
            AlmanacSettings::default(),
            "older files gain it"
        );
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["almanac"]["enabled"], true);
        assert_eq!(json["almanac"]["start_minutes"], 540);
        assert_eq!(json["almanac"]["empty_days_are_leave"], true);
        assert_eq!(
            json["almanac"]["work_days"].as_array().map(Vec::len),
            Some(7)
        );
    }

    #[test]
    fn almanac_hours_are_clamped_and_may_run_past_midnight() {
        let late: Settings = serde_json::from_str(
            r#"{"almanac":{"start_minutes":900,"end_minutes":0,"enabled":false}}"#,
        )
        .unwrap();
        let late = late.sanitized().almanac;
        assert_eq!((late.start_minutes, late.end_minutes), (900, 0));
        assert!(!late.enabled);
        assert!(late.empty_days_are_leave, "a missing field defaults");
        let wild = AlmanacSettings {
            start_minutes: 5000,
            end_minutes: 2000,
            ..AlmanacSettings::default()
        }
        .sanitized();
        assert_eq!(
            (wild.start_minutes, wild.end_minutes),
            (LAST_MINUTE, LAST_MINUTE)
        );
    }

    #[test]
    fn the_resting_hours_sit_opposite_the_working_hours() {
        let office = AlmanacSettings::default();
        assert_eq!(office.turn_hour(), 1);
        assert!(office.is_resting(23 * 60));
        assert!(office.is_resting(3 * 60));
        assert!(office.is_resting(6 * 60));
        assert!(!office.is_resting(6 * 60 + 1));
        assert!(!office.is_resting(14 * 60));
        let evening = AlmanacSettings {
            start_minutes: 15 * 60,
            end_minutes: 0,
            ..AlmanacSettings::default()
        };
        assert_eq!(evening.turn_hour(), 7);
        assert!(evening.is_resting(3 * 60));
        assert!(!evening.is_resting(23 * 60), "still at work");
        let whole = AlmanacSettings {
            start_minutes: 600,
            end_minutes: 600,
            ..AlmanacSettings::default()
        };
        assert_eq!(whole.turn_hour(), 10);
    }

    #[test]
    fn claude_is_off_until_someone_turns_it_on() {
        let ai = Settings::default().ai;
        assert!(!ai.enabled, "a choice, never a default");
        assert!(ai.week_in_words && ai.tell_the_sun, "ready once it is on");
        assert!(ai.day_in_a_line && ai.ask_the_almanac);
        assert!(!ai.hidden, "shown, locked, until someone hides it");
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.ai, AiSettings::default(), "older files gain it, off");
        let json = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(json["ai"]["enabled"], false);
        assert_eq!(json["ai"]["week_in_words"], true);
        assert!(
            json["ai"].get("key").is_none(),
            "the key is never a setting"
        );
    }

    #[test]
    fn the_service_defaults_to_claude_and_its_text_is_tidied() {
        let ai = Settings::default().ai;
        assert_eq!(ai.provider, Provider::Anthropic);
        assert!(ai.model.is_empty() && ai.base_url.is_empty());
        let json = serde_json::to_value(&ai).unwrap();
        assert_eq!(json["provider"], "anthropic");
        let other: AiSettings = serde_json::from_str(
            r#"{"provider":"openai","model":"  gpt-5.4-nano ","base_url":" https://x.test/v1 "}"#,
        )
        .unwrap();
        let other = other.sanitized();
        assert_eq!(other.provider, Provider::OpenAi);
        assert_eq!(other.model, "gpt-5.4-nano");
        assert_eq!(other.base_url, "https://x.test/v1");
        assert_eq!(
            serde_json::to_value(Provider::Custom).unwrap(),
            serde_json::json!("custom")
        );
        let long = AiSettings {
            model: "m".repeat(500),
            ..AiSettings::default()
        };
        assert_eq!(long.sanitized().model.len(), 200);
    }

    #[test]
    fn the_break_reminder_is_off_by_default_and_clamped() {
        assert_eq!(Settings::default().almanac.break_after_mins, None);
        for (wild, clamped) in [(1, 15), (90, 90), (9000, 480)] {
            let a = AlmanacSettings {
                break_after_mins: Some(wild),
                ..AlmanacSettings::default()
            };
            assert_eq!(a.sanitized().break_after_mins, Some(clamped));
        }
    }

    #[test]
    fn the_wait_for_the_night_is_clamped() {
        for (wild, clamped) in [(0, 10), (5, 10), (60, 60), (900, 240)] {
            let a = AlmanacSettings {
                rest_after_mins: Some(wild),
                ..AlmanacSettings::default()
            };
            assert_eq!(a.sanitized().rest_after_mins, Some(clamped));
        }
    }

    #[test]
    fn update_checks_default_on_and_never_checked() {
        let s = Settings::default();
        assert!(s.check_for_updates);
        assert_eq!(s.last_update_check_unix_ms, 0);
        let old: Settings = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert!(old.check_for_updates, "older files gain it, on");
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["check_for_updates"], true);
        assert_eq!(json["last_update_check_unix_ms"], 0);
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
    fn sanitize_clamps_schedule_minutes_and_leaves_the_switch_alone() {
        let s = Schedule {
            enabled: true,
            start_minutes: 5000,
            end_minutes: 60,
            ..Schedule::default()
        }
        .sanitized();
        assert_eq!(s.start_minutes, LAST_MINUTE);
        assert!(s.enabled);
        let all_day = Schedule {
            enabled: true,
            start_minutes: 600,
            end_minutes: 600,
            ..Schedule::default()
        }
        .sanitized();
        assert!(
            all_day.enabled,
            "the same time twice is the whole day, not a reason to switch off"
        );
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
