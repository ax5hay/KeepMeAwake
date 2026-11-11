//! The session log: when the laptop was held awake, how each stretch ended,
//! and where it paused. Pure; persistence and the clock live elsewhere.
//!
//! An entry is one awake stretch as the user saw it: from the moment the sun
//! rose (a manual session or a schedule window) to the moment it set, even if
//! the plan changed in between. The newest entry is last.

use serde::{Deserialize, Serialize};

use super::session::Mode;
use super::snapshot::PauseReason;

/// Version of the history file's shape.
pub const HISTORY_VERSION: u32 = 1;

/// Entries kept; older ones are dropped as new ones open.
pub const MAX_ENTRIES: usize = 200;

/// How an awake stretch ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ended {
    /// Turned off: the sun, the rule, the tray, the hotkey, or quitting.
    User,
    /// A timed session ran out.
    Timer,
    /// The schedule window it belonged to closed.
    Schedule,
    /// It was paused for the battery and never resumed before it ended.
    Battery,
    /// The overnight guard let it go: nobody had touched the machine for
    /// hours and nobody answered the question.
    Idle,
    /// It never ended cleanly: the next launch found it still open, so the
    /// machine slept, shut down or the app was killed while it was up.
    Sleep,
    /// No clean end for a reason the app cannot name.
    Crash,
    /// The condition it was kept awake for (stay awake while…) cleared.
    Watch,
}

/// A stretch inside an entry during which the sun was up but not holding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pause {
    /// When it paused, Unix ms.
    pub from_unix_ms: u64,
    /// When it resumed; `None` if it never did.
    pub to_unix_ms: Option<u64>,
    /// Why it paused.
    pub reason: PauseReason,
}

/// One awake stretch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// When the sun rose, Unix ms.
    pub started_unix_ms: u64,
    /// When it set; `None` while running, or when it never ended cleanly.
    pub ended_unix_ms: Option<u64>,
    /// The mode it started in.
    pub mode: Mode,
    /// How it ended; `None` while running.
    pub ended: Option<Ended>,
    /// Pauses, oldest first.
    pub pauses: Vec<Pause>,
}

impl Entry {
    /// Still running.
    pub fn is_open(&self) -> bool {
        self.ended.is_none()
    }
}

/// The whole log, oldest entry first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    /// File shape version.
    pub version: u32,
    /// Entries, oldest first, at most [`MAX_ENTRIES`].
    pub entries: Vec<Entry>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            version: HISTORY_VERSION,
            entries: Vec::new(),
        }
    }
}

impl History {
    /// Starts an entry. One left open (which the engine never does) is
    /// marked [`Ended::Crash`] first, so there is only ever one running.
    pub fn open(&mut self, started_unix_ms: u64, mode: Mode) {
        self.mark_unfinished_as(Ended::Crash);
        self.entries.push(Entry {
            started_unix_ms,
            ended_unix_ms: None,
            mode,
            ended: None,
            pauses: Vec::new(),
        });
        self.trim();
    }

    /// The running entry paused. Ignored if it is already paused.
    pub fn pause(&mut self, at_unix_ms: u64, reason: PauseReason) {
        let Some(entry) = self.running() else {
            return;
        };
        if entry.pauses.last().is_some_and(|p| p.to_unix_ms.is_none()) {
            return;
        }
        entry.pauses.push(Pause {
            from_unix_ms: at_unix_ms,
            to_unix_ms: None,
            reason,
        });
    }

    /// The running entry resumed.
    pub fn resume(&mut self, at_unix_ms: u64) {
        if let Some(pause) = self
            .running()
            .and_then(|entry| entry.pauses.last_mut())
            .filter(|pause| pause.to_unix_ms.is_none())
        {
            pause.to_unix_ms = Some(at_unix_ms);
        }
    }

    /// The running entry ended. A pause still open ends with it.
    pub fn close(&mut self, at_unix_ms: u64, ended: Ended) {
        self.resume(at_unix_ms);
        if let Some(entry) = self.running() {
            entry.ended_unix_ms = Some(at_unix_ms);
            entry.ended = Some(ended);
        }
    }

    /// Marks an entry left open by the previous run, with no end time.
    /// Returns `true` if there was one.
    pub fn mark_unfinished_as(&mut self, ended: Ended) -> bool {
        match self.running() {
            Some(entry) => {
                entry.ended = Some(ended);
                true
            }
            None => false,
        }
    }

    /// Drops the oldest entries beyond [`MAX_ENTRIES`].
    pub fn trim(&mut self) {
        let excess = self.entries.len().saturating_sub(MAX_ENTRIES);
        self.entries.drain(..excess);
    }

    /// The entry still running, which is always the newest if any.
    fn running(&mut self) -> Option<&mut Entry> {
        self.entries.last_mut().filter(|entry| entry.is_open())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_800_000_000_000;
    const MIN: u64 = 60_000;

    fn only(history: &History) -> &Entry {
        assert_eq!(history.entries.len(), 1);
        &history.entries[0]
    }

    #[test]
    fn open_then_close_records_the_span_and_the_reason() {
        let mut h = History::default();
        h.open(T0, Mode::Timed);
        assert!(only(&h).is_open());
        h.close(T0 + 30 * MIN, Ended::Timer);
        assert_eq!(
            only(&h),
            &Entry {
                started_unix_ms: T0,
                ended_unix_ms: Some(T0 + 30 * MIN),
                mode: Mode::Timed,
                ended: Some(Ended::Timer),
                pauses: vec![],
            }
        );
    }

    #[test]
    fn pauses_are_recorded_inside_the_running_entry() {
        let mut h = History::default();
        h.open(T0, Mode::Indefinite);
        h.pause(T0 + MIN, PauseReason::LowBattery);
        h.pause(T0 + 2 * MIN, PauseReason::OnBattery);
        h.resume(T0 + 3 * MIN);
        h.pause(T0 + 4 * MIN, PauseReason::OnBattery);
        h.close(T0 + 5 * MIN, Ended::Battery);
        assert_eq!(
            only(&h).pauses,
            vec![
                Pause {
                    from_unix_ms: T0 + MIN,
                    to_unix_ms: Some(T0 + 3 * MIN),
                    reason: PauseReason::LowBattery,
                },
                Pause {
                    from_unix_ms: T0 + 4 * MIN,
                    to_unix_ms: Some(T0 + 5 * MIN),
                    reason: PauseReason::OnBattery,
                },
            ],
            "a second pause while paused is ignored; closing ends an open pause"
        );
    }

    #[test]
    fn nothing_happens_without_a_running_entry() {
        let mut h = History::default();
        h.pause(T0, PauseReason::LowBattery);
        h.resume(T0);
        h.close(T0, Ended::User);
        assert_eq!(h, History::default());
        h.open(T0, Mode::Timed);
        h.close(T0 + MIN, Ended::User);
        h.close(T0 + 2 * MIN, Ended::Timer);
        assert_eq!(
            only(&h).ended,
            Some(Ended::User),
            "a closed entry stays closed"
        );
    }

    #[test]
    fn opening_over_a_running_entry_marks_it_crashed() {
        let mut h = History::default();
        h.open(T0, Mode::Indefinite);
        h.open(T0 + MIN, Mode::Scheduled);
        assert_eq!(h.entries[0].ended, Some(Ended::Crash));
        assert_eq!(h.entries[0].ended_unix_ms, None);
        assert!(h.entries[1].is_open());
    }

    #[test]
    fn an_entry_left_open_by_the_last_run_is_marked_without_an_end() {
        let mut h = History::default();
        h.open(T0, Mode::Indefinite);
        h.pause(T0 + MIN, PauseReason::OnBattery);
        assert!(h.mark_unfinished_as(Ended::Sleep));
        let e = only(&h);
        assert_eq!(e.ended, Some(Ended::Sleep));
        assert_eq!(e.ended_unix_ms, None);
        assert_eq!(e.pauses[0].to_unix_ms, None);
        assert!(!h.mark_unfinished_as(Ended::Sleep), "nothing left open");
    }

    #[test]
    fn keeps_the_newest_two_hundred() {
        let mut h = History::default();
        for i in 0..(MAX_ENTRIES as u64 + 5) {
            h.open(T0 + i * MIN, Mode::Timed);
            h.close(T0 + i * MIN + 1, Ended::Timer);
        }
        assert_eq!(h.entries.len(), MAX_ENTRIES);
        assert_eq!(h.entries[0].started_unix_ms, T0 + 5 * MIN);
    }

    #[test]
    fn round_trips_through_json_in_snake_case() {
        let mut h = History::default();
        h.open(T0, Mode::Scheduled);
        h.pause(T0 + MIN, PauseReason::LowBattery);
        h.close(T0 + 2 * MIN, Ended::Schedule);
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["entries"][0]["mode"], "scheduled");
        assert_eq!(json["entries"][0]["ended"], "schedule");
        assert_eq!(json["entries"][0]["pauses"][0]["reason"], "low_battery");
        let back: History = serde_json::from_value(json).unwrap();
        assert_eq!(back, h);
        let empty: History = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, History::default());
    }

    #[test]
    fn the_overnight_guard_ending_serialises_as_idle() {
        assert_eq!(
            serde_json::to_value(Ended::Idle).unwrap(),
            serde_json::json!("idle")
        );
        let back: Ended = serde_json::from_str(r#""idle""#).unwrap();
        assert_eq!(back, Ended::Idle);
    }
}
