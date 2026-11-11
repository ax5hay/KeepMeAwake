//! Stay awake while…: the conditions that keep the laptop awake on their
//! own, and the pure reckoning from what the machine reports to a reason.
//!
//! The engine reads the machine (only what is switched on) and hands the
//! readings to a [`Watcher`], which keeps the previous samples so rates can
//! be worked out between two readings. Nothing here performs I/O.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::snapshot::BatteryState;

/// How long a transfer or a busy processor keeps holding after its last
/// reading over the threshold, by default, so a pause between two files or
/// two compile steps does not let the laptop sleep in the middle.
pub const LINGER_MINS: u8 = 2;

/// Network traffic, in and out together, above which a transfer is going,
/// by default, in kilobytes (1000 bytes) a second. Background chatter (sync
/// heartbeats, mail) stays well under it.
pub const TRANSFER_KBPS: u32 = 64;

/// Processor use, across all cores, above which the computer is busy, by
/// default.
pub const BUSY_PERCENT: u8 = 30;

/// Two readings closer than this are not compared: a command can wake the
/// engine a moment after a poll, and a rate over a tenth of a second is
/// noise. The older sample is kept until enough time has passed.
pub const MIN_SAMPLE_GAP: Duration = Duration::from_secs(5);

/// At most this many apps are watched; each name at most this long.
pub const MAX_APPS: usize = 16;
/// Longest app name kept, in characters.
pub const MAX_APP_NAME: usize = 64;

/// What keeps the laptop awake on its own. Every condition is off, and the
/// app list empty, until chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
#[allow(clippy::struct_excessive_bools)] // independent switches, one per row
pub struct WatchSettings {
    /// Awake while any of these apps runs. Matched without case, and
    /// without a trailing `.exe`.
    pub apps: Vec<String>,
    /// Awake while the network moves more than `transfer_kbps`.
    pub transfers: bool,
    /// Awake while the processor is over `busy_percent`.
    pub busy: bool,
    /// Awake while another app is full screen: a video, a game, slides.
    pub full_screen: bool,
    /// Awake while an external display is connected. Laptops only.
    pub display: bool,
    /// Awake while plugged in. Laptops only.
    pub plugged_in: bool,
    /// A transfer is going above this many kilobytes a second, in and out.
    pub transfer_kbps: u32,
    /// The computer is busy above this share of the processor.
    pub busy_percent: u8,
    /// A transfer or a busy spell keeps holding this long after it drops.
    pub linger_mins: u8,
}

impl Default for WatchSettings {
    fn default() -> Self {
        Self {
            apps: Vec::new(),
            transfers: false,
            busy: false,
            full_screen: false,
            display: false,
            plugged_in: false,
            transfer_kbps: TRANSFER_KBPS,
            busy_percent: BUSY_PERCENT,
            linger_mins: LINGER_MINS,
        }
    }
}

impl WatchSettings {
    /// How long a transfer or a busy spell keeps holding after it drops.
    #[must_use]
    pub fn linger(&self) -> Duration {
        Duration::from_secs(u64::from(self.linger_mins) * 60)
    }

    /// Any condition is switched on, so the machine has to be read.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.apps.is_empty()
            || self.transfers
            || self.busy
            || self.full_screen
            || self.display
            || self.plugged_in
    }

    /// Names trimmed, emptied ones dropped, duplicates (ignoring case and
    /// `.exe`) removed, at most [`MAX_APPS`] of at most [`MAX_APP_NAME`].
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        // A kilobyte a second to 100 MB/s; 5% to 95%; a minute to an hour.
        self.transfer_kbps = self.transfer_kbps.clamp(1, 100_000);
        self.busy_percent = self.busy_percent.clamp(5, 95);
        self.linger_mins = self.linger_mins.clamp(1, 60);
        let mut seen: Vec<String> = Vec::new();
        self.apps = self
            .apps
            .into_iter()
            .map(|name| name.trim().chars().take(MAX_APP_NAME).collect::<String>())
            .filter(|name| !name.is_empty())
            .filter(|name| {
                let key = app_key(name);
                let fresh = !seen.contains(&key);
                if fresh {
                    seen.push(key);
                }
                fresh
            })
            .take(MAX_APPS)
            .collect();
        self
    }
}

/// How an app name is compared: lower case, with a trailing `.exe` and
/// `.app` dropped, so `Xcode`, `xcode.app` and `XCODE` are one app.
#[must_use]
pub fn app_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    lower
        .strip_suffix(".exe")
        .or_else(|| lower.strip_suffix(".app"))
        .unwrap_or(&lower)
        .to_owned()
}

/// The names from `wanted` (as written there) that one of `processes`
/// matches, compared by [`app_key`]. Pure, so both platforms list
/// processes and nothing more.
#[must_use]
pub fn matching(wanted: &[String], processes: &[String]) -> Vec<String> {
    if wanted.is_empty() {
        return Vec::new();
    }
    let running: Vec<String> = processes.iter().map(|p| app_key(p)).collect();
    wanted
        .iter()
        .filter(|name| running.contains(&app_key(name)))
        .cloned()
        .collect()
}

/// Process names worth offering in the app picker: deduplicated by
/// [`app_key`], extensions dropped, sorted without case.
#[must_use]
pub fn app_choices(processes: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for process in processes {
        let trimmed = process.trim();
        let shown = trimmed
            .strip_suffix(".exe")
            .or_else(|| trimmed.strip_suffix(".EXE"))
            .unwrap_or(trimmed);
        let key = app_key(shown);
        if !key.is_empty() && !seen.contains(&key) {
            seen.push(key);
            names.push(shown.to_owned());
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// Why a condition is holding the laptop awake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchReason {
    /// An app from the list is running; `name` as the user wrote it.
    App {
        /// The name from the list.
        name: String,
    },
    /// A process the command line asked to watch is still running.
    Process {
        /// Its id.
        pid: u32,
    },
    /// The network is moving more than the threshold.
    Transfer,
    /// The processor is over the threshold.
    Busy,
    /// Another app is full screen.
    FullScreen,
    /// An external display is connected.
    Display,
    /// The laptop is plugged in.
    PluggedIn,
}

/// Where the watched conditions stand, for the window and the tray.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "snake_case")]
pub struct WatchState {
    /// The condition that holds, first in the settings' order; `None` when
    /// none does (or none is on).
    pub reason: Option<WatchReason>,
    /// The sun was clicked while a condition held: resting until it clears.
    pub skipped: bool,
    /// The network rate at the last reading, bytes a second, while
    /// transfers are watched.
    pub transfer_bps: Option<u64>,
    /// Processor use at the last reading, while busy is watched.
    pub cpu_percent: Option<u8>,
    /// The apps from the list that were running at the last reading.
    pub running: Vec<String>,
}

/// Processor time since boot: time spent working, and in all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuTicks {
    /// Time not idle.
    pub busy: u64,
    /// Time in all, idle included.
    pub total: u64,
}

/// One reading of the machine. Only switched-on conditions are read; the
/// rest stay at their defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Readings {
    /// Names from the app list that are running, as written in the list.
    pub running: Vec<String>,
    /// The first watched process still alive.
    pub pid: Option<u32>,
    /// Bytes in and out since boot, all real interfaces together.
    pub transferred: Option<u64>,
    /// Processor time since boot.
    pub cpu: Option<CpuTicks>,
    /// Another app is full screen.
    pub full_screen: bool,
    /// An external display is connected.
    pub display: bool,
}

/// Keeps the samples rates are worked out from, and how long a transfer or
/// a busy spell keeps holding.
#[derive(Debug, Default, Clone)]
pub struct Watcher {
    bytes: Option<(u64, Instant)>,
    cpu: Option<(CpuTicks, Instant)>,
    transfer_until: Option<Instant>,
    busy_until: Option<Instant>,
    rate: Option<u64>,
    percent: Option<u8>,
}

impl Watcher {
    /// Takes one reading and says which condition, if any, holds now.
    pub fn observe(
        &mut self,
        settings: &WatchSettings,
        readings: &Readings,
        battery: BatteryState,
        now: Instant,
    ) -> WatchState {
        self.sample_transfer(
            settings.transfers.then_some(readings.transferred).flatten(),
            settings,
            now,
        );
        self.sample_cpu(
            settings.busy.then_some(readings.cpu).flatten(),
            settings,
            now,
        );
        let holding = |until: Option<Instant>| until.is_some_and(|until| now < until);
        // Without a battery these two would hold forever: a desktop is
        // always plugged in, and its only displays are external.
        let laptop = battery.present;
        let reason = readings
            .running
            .first()
            .map(|name| WatchReason::App { name: name.clone() })
            .or_else(|| readings.pid.map(|pid| WatchReason::Process { pid }))
            .or_else(|| {
                (settings.transfers && holding(self.transfer_until))
                    .then_some(WatchReason::Transfer)
            })
            .or_else(|| (settings.busy && holding(self.busy_until)).then_some(WatchReason::Busy))
            .or_else(|| {
                (settings.full_screen && readings.full_screen).then_some(WatchReason::FullScreen)
            })
            .or_else(|| {
                (settings.display && laptop && readings.display).then_some(WatchReason::Display)
            })
            .or_else(|| {
                (settings.plugged_in && laptop && battery.on_ac).then_some(WatchReason::PluggedIn)
            });
        WatchState {
            reason,
            skipped: false,
            transfer_bps: settings.transfers.then_some(self.rate).flatten(),
            cpu_percent: settings.busy.then_some(self.percent).flatten(),
            running: readings.running.clone(),
        }
    }

    fn sample_transfer(&mut self, reading: Option<u64>, settings: &WatchSettings, now: Instant) {
        let Some(bytes) = reading else {
            self.bytes = None;
            self.rate = None;
            self.transfer_until = None;
            return;
        };
        match self.bytes {
            Some((before, at)) if now.saturating_duration_since(at) < MIN_SAMPLE_GAP => {
                // Too soon to compare; keep the older sample.
                let _ = before;
            }
            Some((before, at)) => {
                self.bytes = Some((bytes, now));
                // A counter that went backwards was reset (an adapter came
                // and went): start again from here rather than guess.
                self.rate = bytes
                    .checked_sub(before)
                    .map(|moved| per_second(moved, now.saturating_duration_since(at)));
                let threshold = u64::from(settings.transfer_kbps) * 1000;
                if self.rate.is_some_and(|rate| rate > threshold) {
                    self.transfer_until = Some(now + settings.linger());
                }
            }
            None => self.bytes = Some((bytes, now)),
        }
    }

    fn sample_cpu(&mut self, reading: Option<CpuTicks>, settings: &WatchSettings, now: Instant) {
        let Some(ticks) = reading else {
            self.cpu = None;
            self.percent = None;
            self.busy_until = None;
            return;
        };
        match self.cpu {
            Some((_, at)) if now.saturating_duration_since(at) < MIN_SAMPLE_GAP => {}
            Some((before, _)) => {
                self.cpu = Some((ticks, now));
                self.percent = busy_percent(before, ticks);
                if self.percent.is_some_and(|p| p > settings.busy_percent) {
                    self.busy_until = Some(now + settings.linger());
                }
            }
            None => self.cpu = Some((ticks, now)),
        }
    }
}

/// Bytes moved over `over`, as bytes a second.
fn per_second(moved: u64, over: Duration) -> u64 {
    let millis = u64::try_from(over.as_millis()).unwrap_or(u64::MAX).max(1);
    moved.saturating_mul(1000) / millis
}

/// Share of processor time spent working between two readings, or `None`
/// when the counters did not move forwards.
fn busy_percent(before: CpuTicks, after: CpuTicks) -> Option<u8> {
    let total = after.total.checked_sub(before.total).filter(|t| *t > 0)?;
    let busy = after.busy.checked_sub(before.busy)?.min(total);
    u8::try_from(busy * 100 / total).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laptop(on_ac: bool) -> BatteryState {
        BatteryState {
            on_ac,
            percent: Some(80),
            present: true,
            minutes_left: None,
        }
    }

    fn desktop() -> BatteryState {
        BatteryState {
            on_ac: true,
            percent: None,
            present: false,
            minutes_left: None,
        }
    }

    fn all_on() -> WatchSettings {
        WatchSettings {
            apps: vec!["Xcode".into()],
            transfers: true,
            busy: true,
            full_screen: true,
            display: true,
            plugged_in: true,
            ..WatchSettings::default()
        }
    }

    #[test]
    fn off_by_default_and_reads_nothing() {
        let settings = WatchSettings::default();
        assert!(!settings.any());
        let mut watcher = Watcher::default();
        let state = watcher.observe(
            &settings,
            &Readings::default(),
            laptop(true),
            Instant::now(),
        );
        assert_eq!(state, WatchState::default());
    }

    #[test]
    fn the_first_condition_in_order_is_the_reason() {
        let mut watcher = Watcher::default();
        let now = Instant::now();
        let mut readings = Readings {
            running: vec!["Xcode".into()],
            pid: Some(42),
            full_screen: true,
            display: true,
            ..Readings::default()
        };
        let reason =
            |w: &mut Watcher, r: &Readings| w.observe(&all_on(), r, laptop(true), now).reason;
        assert_eq!(
            reason(&mut watcher, &readings),
            Some(WatchReason::App {
                name: "Xcode".into()
            })
        );
        readings.running.clear();
        assert_eq!(
            reason(&mut watcher, &readings),
            Some(WatchReason::Process { pid: 42 })
        );
        readings.pid = None;
        assert_eq!(
            reason(&mut watcher, &readings),
            Some(WatchReason::FullScreen)
        );
        readings.full_screen = false;
        assert_eq!(reason(&mut watcher, &readings), Some(WatchReason::Display));
        readings.display = false;
        assert_eq!(
            reason(&mut watcher, &readings),
            Some(WatchReason::PluggedIn)
        );
    }

    #[test]
    fn a_desktop_is_never_held_for_power_or_displays() {
        let mut watcher = Watcher::default();
        let readings = Readings {
            display: true,
            ..Readings::default()
        };
        let state = watcher.observe(&all_on(), &readings, desktop(), Instant::now());
        assert_eq!(state.reason, None);
    }

    #[test]
    fn plugged_in_holds_only_on_mains() {
        let settings = WatchSettings {
            plugged_in: true,
            ..WatchSettings::default()
        };
        let mut watcher = Watcher::default();
        let now = Instant::now();
        assert_eq!(
            watcher
                .observe(&settings, &Readings::default(), laptop(true), now)
                .reason,
            Some(WatchReason::PluggedIn)
        );
        assert_eq!(
            watcher
                .observe(&settings, &Readings::default(), laptop(false), now)
                .reason,
            None
        );
    }

    fn transfers() -> WatchSettings {
        WatchSettings {
            transfers: true,
            ..WatchSettings::default()
        }
    }

    fn bytes(n: u64) -> Readings {
        Readings {
            transferred: Some(n),
            ..Readings::default()
        }
    }

    #[test]
    fn a_transfer_holds_and_lingers_after_it_stops() {
        let mut watcher = Watcher::default();
        let start = Instant::now();
        let at = |secs| start + Duration::from_secs(secs);
        let mut observe =
            |n, secs| watcher.observe(&transfers(), &bytes(n), laptop(true), at(secs));
        // First reading: nothing to compare with.
        assert_eq!(observe(0, 0).reason, None);
        // 30 MB in 30 s is 1 MB/s.
        let state = observe(30_000_000, 30);
        assert_eq!(state.reason, Some(WatchReason::Transfer));
        assert_eq!(state.transfer_bps, Some(1_000_000));
        // Quiet again: still held inside the linger...
        let quiet = observe(30_000_000, 60);
        assert_eq!(quiet.reason, Some(WatchReason::Transfer));
        assert_eq!(quiet.transfer_bps, Some(0));
        // ...and let go once it has passed.
        assert_eq!(observe(30_000_000, 30 + 121).reason, None);
    }

    #[test]
    fn background_chatter_is_not_a_transfer() {
        let mut watcher = Watcher::default();
        let start = Instant::now();
        watcher.observe(&transfers(), &bytes(0), laptop(true), start);
        // 30 KB/s for 30 s.
        let state = watcher.observe(
            &transfers(),
            &bytes(900_000),
            laptop(true),
            start + Duration::from_secs(30),
        );
        assert_eq!(state.reason, None);
        assert_eq!(state.transfer_bps, Some(30_000));
    }

    #[test]
    fn a_counter_that_goes_backwards_starts_again() {
        let mut watcher = Watcher::default();
        let start = Instant::now();
        watcher.observe(&transfers(), &bytes(5_000_000_000), laptop(true), start);
        let state = watcher.observe(
            &transfers(),
            &bytes(10),
            laptop(true),
            start + Duration::from_secs(30),
        );
        assert_eq!(state.reason, None);
        assert_eq!(state.transfer_bps, None);
    }

    #[test]
    fn readings_too_close_together_are_not_compared() {
        let mut watcher = Watcher::default();
        let start = Instant::now();
        watcher.observe(&transfers(), &bytes(0), laptop(true), start);
        // A tenth of a second later a big jump would read as a huge rate.
        let soon = watcher.observe(
            &transfers(),
            &bytes(1_000_000),
            laptop(true),
            start + Duration::from_millis(100),
        );
        assert_eq!(soon.reason, None);
        // Compared against the first sample once enough time has passed.
        let later = watcher.observe(
            &transfers(),
            &bytes(1_000_000),
            laptop(true),
            start + Duration::from_secs(10),
        );
        assert_eq!(later.transfer_bps, Some(100_000));
    }

    #[test]
    fn a_busy_processor_holds_then_lets_go() {
        let settings = WatchSettings {
            busy: true,
            ..WatchSettings::default()
        };
        let cpu = |busy, total| Readings {
            cpu: Some(CpuTicks { busy, total }),
            ..Readings::default()
        };
        let mut watcher = Watcher::default();
        let start = Instant::now();
        let at = |secs| start + Duration::from_secs(secs);
        watcher.observe(&settings, &cpu(0, 0), laptop(true), at(0));
        let busy = watcher.observe(&settings, &cpu(540, 1000), laptop(true), at(30));
        assert_eq!(busy.reason, Some(WatchReason::Busy));
        assert_eq!(busy.cpu_percent, Some(54));
        let calm = watcher.observe(&settings, &cpu(560, 2000), laptop(true), at(60));
        assert_eq!(calm.cpu_percent, Some(2));
        assert_eq!(calm.reason, Some(WatchReason::Busy), "lingers");
        let gone = watcher.observe(&settings, &cpu(570, 3000), laptop(true), at(200));
        assert_eq!(gone.reason, None);
    }

    #[test]
    fn the_thresholds_and_the_wait_are_the_settings() {
        let settings = WatchSettings {
            transfers: true,
            transfer_kbps: 16,
            linger_mins: 5,
            ..WatchSettings::default()
        };
        let mut watcher = Watcher::default();
        let start = Instant::now();
        let at = |secs| start + Duration::from_secs(secs);
        watcher.observe(&settings, &bytes(0), laptop(true), at(0));
        // 20 KB/s: under the default, over a 16 KB/s threshold.
        let state = watcher.observe(&settings, &bytes(600_000), laptop(true), at(30));
        assert_eq!(state.reason, Some(WatchReason::Transfer));
        // Quiet: still held at four minutes, let go after five.
        let held = watcher.observe(&settings, &bytes(600_000), laptop(true), at(30 + 240));
        assert_eq!(held.reason, Some(WatchReason::Transfer));
        let gone = watcher.observe(&settings, &bytes(600_000), laptop(true), at(30 + 301));
        assert_eq!(gone.reason, None);
    }

    #[test]
    fn thresholds_are_kept_in_range() {
        let wild = WatchSettings {
            transfer_kbps: 0,
            busy_percent: 100,
            linger_mins: 0,
            ..WatchSettings::default()
        }
        .sanitized();
        assert_eq!(
            (wild.transfer_kbps, wild.busy_percent, wild.linger_mins),
            (1, 95, 1)
        );
        let d = WatchSettings::default();
        assert_eq!(
            (d.transfer_kbps, d.busy_percent, d.linger_mins),
            (64, 30, 2)
        );
    }

    #[test]
    fn switching_a_condition_off_forgets_its_samples() {
        let mut watcher = Watcher::default();
        let start = Instant::now();
        watcher.observe(&transfers(), &bytes(0), laptop(true), start);
        watcher.observe(
            &transfers(),
            &bytes(30_000_000),
            laptop(true),
            start + Duration::from_secs(30),
        );
        let off = watcher.observe(
            &WatchSettings::default(),
            &Readings::default(),
            laptop(true),
            start + Duration::from_secs(31),
        );
        assert_eq!(off.reason, None);
        assert_eq!(off.transfer_bps, None);
    }

    #[test]
    fn app_names_are_tidied_and_capped() {
        let settings = WatchSettings {
            apps: vec![
                "  Xcode ".into(),
                "xcode.app".into(),
                "XCODE".into(),
                String::new(),
                "cargo.exe".into(),
                "Cargo".into(),
                "x".repeat(100),
            ],
            ..WatchSettings::default()
        }
        .sanitized();
        assert_eq!(settings.apps.len(), 3);
        assert_eq!(settings.apps[0], "Xcode");
        assert_eq!(settings.apps[1], "cargo.exe");
        assert_eq!(settings.apps[2].chars().count(), MAX_APP_NAME);
        let many = WatchSettings {
            apps: (0..40).map(|i| format!("app{i}")).collect(),
            ..WatchSettings::default()
        }
        .sanitized();
        assert_eq!(many.apps.len(), MAX_APPS);
    }

    #[test]
    fn running_apps_are_matched_as_written_in_the_list() {
        let wanted = vec!["Xcode".to_owned(), "cargo".to_owned(), "Blender".to_owned()];
        let processes = vec!["cargo.exe".to_owned(), "XCODE".to_owned(), "zsh".to_owned()];
        assert_eq!(matching(&wanted, &processes), vec!["Xcode", "cargo"]);
        assert_eq!(matching(&[], &processes), Vec::<String>::new());
    }

    #[test]
    fn choices_are_deduplicated_and_sorted() {
        let processes = vec![
            "zsh".to_owned(),
            "Code.exe".to_owned(),
            "code.exe".to_owned(),
            "Arc".to_owned(),
            " ".to_owned(),
        ];
        assert_eq!(app_choices(&processes), vec!["Arc", "Code", "zsh"]);
    }

    #[test]
    fn names_compare_without_case_or_extension() {
        assert_eq!(app_key("Cargo.EXE"), "cargo");
        assert_eq!(app_key("Xcode.app"), "xcode");
        assert_eq!(app_key(" Final Cut Pro "), "final cut pro");
    }

    #[test]
    fn reasons_serialize_tagged_by_kind() {
        let json = serde_json::to_string(&WatchReason::App {
            name: "Xcode".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"app","name":"Xcode"}"#);
        assert_eq!(
            serde_json::to_string(&WatchReason::FullScreen).unwrap(),
            r#"{"kind":"full_screen"}"#
        );
    }
}
