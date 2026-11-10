//! The almanac: when someone was at the machine, and which days they marked.
//! Pure; the idle reading, the clock and the file live elsewhere.
//!
//! It keeps spans of presence as instants and nothing else. Days, hours
//! worked, leave and nights are all read from those in the window, in local
//! time. A night is never recorded as it happens: it is the quiet between
//! one span's end and the next one's start, so it is found the same way
//! whether the sun was up on Always or the laptop was asleep.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Version of the almanac file's shape.
pub const ALMANAC_VERSION: u32 = 1;

/// A quiet longer than this ends a span; a shorter one is bridged, because
/// reading and thinking leave no input.
pub const AWAY_MS: u64 = 10 * 60 * 1000;

/// How long spans are kept: a year of days and a month to look back from.
pub const KEEP_MS: u64 = 400 * 24 * 60 * 60 * 1000;

/// Marked days kept; the earliest dates go first.
pub const MAX_MARKS: usize = 1000;

/// The longest stretch a day can be corrected to by hand.
pub const MAX_FIX_MS: u64 = 36 * 60 * 60 * 1000;

/// How far before its first known input a new span may start. Only the
/// latest input is ever known, so someone back for a while before a look is
/// given at most this much.
pub const LOOK_BACK_MS: u64 = 60 * 1000;

/// Readings this close to "nothing new" count as nothing new.
const SLACK: Duration = Duration::from_secs(1);

/// A stretch during which someone was at the keyboard, mouse or trackpad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Its first input, Unix ms (within a look of it).
    pub from_unix_ms: u64,
    /// Its last input, Unix ms.
    pub to_unix_ms: u64,
}

/// What someone said about a day, over what the spans say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    /// A day of leave, whatever was or was not typed on it.
    Leave,
    /// Not a leave, though it is a working day with nothing in it.
    Clear,
}

/// One reading of the idle time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    /// Wall clock at the reading, Unix ms.
    pub at_unix_ms: u64,
    /// Monotonic clock at the reading.
    pub at: Instant,
    /// How long since the last input.
    pub idle: Duration,
}

/// What a look changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Noted {
    /// No input since the look before.
    Nothing,
    /// Someone is still here: the newest span grew.
    Extended,
    /// Someone is back after a quiet: a new span opened.
    Opened,
}

/// The whole almanac.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Almanac {
    /// File shape version.
    pub version: u32,
    /// Presence, oldest first, never overlapping.
    pub spans: Vec<Span>,
    /// Marked days by local date, `2026-11-10`.
    pub marks: BTreeMap<String, Mark>,
    /// Days corrected by hand, by local date: the one stretch someone says
    /// they were here, in place of whatever was read for that day.
    pub fixes: BTreeMap<String, Span>,
}

impl Default for Almanac {
    fn default() -> Self {
        Self {
            version: ALMANAC_VERSION,
            spans: Vec::new(),
            marks: BTreeMap::new(),
            fixes: BTreeMap::new(),
        }
    }
}

impl Almanac {
    /// Takes one look into account. `last` is the look before it in this
    /// run, if there was one.
    ///
    /// Whether anyone touched anything in between is judged on the
    /// monotonic clock: with no input, the idle time grows by at least the
    /// time that passed. Judged that way, an idle clock that stands still
    /// while the machine sleeps cannot pass a wake in the night off as a
    /// touch.
    pub fn note(&mut self, look: Look, last: Option<Look>) -> Noted {
        if let Some(prev) = last {
            let untouched = prev.idle + look.at.saturating_duration_since(prev.at);
            if look.idle + SLACK >= untouched {
                return Noted::Nothing;
            }
        }
        let now = look.at_unix_ms;
        self.settle(now);
        let input = now.saturating_sub(millis(look.idle));
        if let Some(span) = self.spans.last_mut() {
            if input <= span.to_unix_ms {
                return Noted::Nothing;
            }
            if input - span.to_unix_ms <= AWAY_MS {
                span.to_unix_ms = input;
                return Noted::Extended;
            }
        }
        // Back after a quiet. Only the latest input is known: the first one
        // came after the look before, and is given a look's length at most.
        let earliest = last.map_or(input, |prev| prev.at_unix_ms);
        let from_unix_ms = input.saturating_sub(LOOK_BACK_MS).max(earliest).min(input);
        self.spans.push(Span {
            from_unix_ms,
            to_unix_ms: input,
        });
        self.trim(now);
        Noted::Opened
    }

    /// Marks `day` (`2026-11-10`), or takes its mark away with `None`.
    /// Returns `false`, changing nothing, for a day that is not a date.
    pub fn mark(&mut self, day: &str, mark: Option<Mark>) -> bool {
        if !is_day(day) {
            return false;
        }
        match mark {
            Some(mark) => {
                self.marks.insert(day.to_owned(), mark);
                while self.marks.len() > MAX_MARKS {
                    self.marks.pop_first();
                }
            }
            None => {
                self.marks.remove(day);
            }
        }
        true
    }

    /// Corrects `day` (`2026-11-10`) to one stretch, or goes back to what
    /// was read with `None`. Returns `false`, changing nothing, for a day
    /// that is not a date or a stretch that ends before it starts or runs
    /// longer than [`MAX_FIX_MS`].
    pub fn fix(&mut self, day: &str, stretch: Option<Span>) -> bool {
        if !is_day(day) {
            return false;
        }
        match stretch {
            Some(span) => {
                let length = span.to_unix_ms.checked_sub(span.from_unix_ms);
                if length.is_none_or(|length| length > MAX_FIX_MS) {
                    return false;
                }
                self.fixes.insert(day.to_owned(), span);
                while self.fixes.len() > MAX_MARKS {
                    self.fixes.pop_first();
                }
            }
            None => {
                self.fixes.remove(day);
            }
        }
        true
    }

    /// Drops spans that ended more than [`KEEP_MS`] before `now`.
    pub fn trim(&mut self, now_unix_ms: u64) {
        let cutoff = now_unix_ms.saturating_sub(KEEP_MS);
        let old = self
            .spans
            .iter()
            .take_while(|span| span.to_unix_ms < cutoff)
            .count();
        self.spans.drain(..old);
    }

    /// Nothing in the almanac is in the future. A clock that was wrong for
    /// a while and came back would otherwise leave a span ahead of now, and
    /// nothing could be noted until the clock caught up with it.
    fn settle(&mut self, now_unix_ms: u64) {
        while self
            .spans
            .last()
            .is_some_and(|span| span.from_unix_ms > now_unix_ms)
        {
            self.spans.pop();
        }
        if let Some(span) = self.spans.last_mut() {
            span.to_unix_ms = span.to_unix_ms.min(now_unix_ms);
        }
    }
}

/// `2026-11-10`, and a date that exists.
fn is_day(day: &str) -> bool {
    day.len() == 10 && NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok()
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_800_000_000_000;
    const SEC: u64 = 1_000;
    const MIN: u64 = 60 * SEC;
    const HOUR: u64 = 60 * MIN;

    /// Looks taken along one timeline, so each knows the one before.
    struct Watch {
        almanac: Almanac,
        origin: Instant,
        last: Option<Look>,
    }

    impl Watch {
        fn new() -> Self {
            Self {
                almanac: Almanac::default(),
                origin: Instant::now(),
                last: None,
            }
        }

        /// A look `at_ms` after `T0` that reads `idle_ms` since the last
        /// input, on a machine whose clocks all kept running.
        fn look(&mut self, at_ms: u64, idle_ms: u64) -> Noted {
            self.look_awake(at_ms, at_ms, idle_ms)
        }

        /// The same, with the monotonic clock `mono_ms` after the start:
        /// less than the wall clock when the machine slept in between.
        fn look_awake(&mut self, at_ms: u64, mono_ms: u64, idle_ms: u64) -> Noted {
            let look = Look {
                at_unix_ms: T0 + at_ms,
                at: self.origin + Duration::from_millis(mono_ms),
                idle: Duration::from_millis(idle_ms),
            };
            let noted = self.almanac.note(look, self.last);
            self.last = Some(look);
            noted
        }

        fn spans(&self) -> Vec<(u64, u64)> {
            self.almanac
                .spans
                .iter()
                .map(|s| (s.from_unix_ms - T0, s.to_unix_ms - T0))
                .collect()
        }
    }

    #[test]
    fn the_first_look_opens_a_span_at_the_last_input() {
        let mut w = Watch::new();
        assert_eq!(w.look(HOUR, 4 * SEC), Noted::Opened);
        assert_eq!(w.spans(), vec![(HOUR - 4 * SEC, HOUR - 4 * SEC)]);
    }

    #[test]
    fn looks_while_someone_types_grow_the_span() {
        let mut w = Watch::new();
        w.look(0, 0);
        assert_eq!(w.look(MIN, 2 * SEC), Noted::Extended);
        assert_eq!(w.look(2 * MIN, 30 * SEC), Noted::Extended);
        assert_eq!(w.spans(), vec![(0, 2 * MIN - 30 * SEC)]);
    }

    #[test]
    fn a_look_with_no_new_input_changes_nothing() {
        let mut w = Watch::new();
        w.look(0, 0);
        assert_eq!(w.look(MIN, MIN), Noted::Nothing);
        assert_eq!(w.look(2 * MIN, 2 * MIN), Noted::Nothing);
        assert_eq!(w.spans(), vec![(0, 0)]);
    }

    #[test]
    fn a_short_quiet_is_bridged_and_the_span_ends_at_the_last_touch() {
        let mut w = Watch::new();
        w.look(0, 0);
        for minute in 1..=9 {
            w.look(minute * MIN, minute * MIN);
        }
        assert_eq!(w.look(10 * MIN, 5 * SEC), Noted::Extended, "9m 55s away");
        assert_eq!(w.spans(), vec![(0, 10 * MIN - 5 * SEC)]);
    }

    #[test]
    fn a_long_quiet_opens_a_new_span_no_earlier_than_the_look_before() {
        let mut w = Watch::new();
        w.look(0, 0);
        for minute in 1..=30 {
            w.look(minute * MIN, minute * MIN);
        }
        // Back 50 s ago and still typing: the span starts at the look
        // before, not at the latest keystroke.
        assert_eq!(w.look(31 * MIN, SEC), Noted::Opened);
        assert_eq!(w.spans(), vec![(0, 0), (30 * MIN, 31 * MIN - SEC)]);
    }

    #[test]
    fn a_span_opened_after_sleep_starts_at_most_a_look_before_its_input() {
        let mut w = Watch::new();
        w.look(0, 0);
        // Eight hours later, woken by a key a moment ago.
        assert_eq!(w.look(8 * HOUR, 3 * SEC), Noted::Opened);
        let (from, to) = w.spans()[1];
        assert_eq!(to, 8 * HOUR - 3 * SEC);
        assert_eq!(to - from, LOOK_BACK_MS, "not the eight hours before it");
    }

    #[test]
    fn a_wake_in_the_night_is_not_a_touch_when_the_idle_clock_stood_still() {
        let mut w = Watch::new();
        w.look(0, 0);
        w.look(MIN, 30 * SEC);
        // Asleep for four hours; the machine wakes itself for twenty
        // seconds, and its idle clock did not count the sleep.
        let woke = 4 * HOUR;
        assert_eq!(
            w.look_awake(woke, MIN + 20 * SEC, 50 * SEC),
            Noted::Nothing,
            "thirty seconds idle before, twenty awake since: nobody came"
        );
        assert_eq!(w.spans(), vec![(0, 30 * SEC)]);
        // A real key, later that same wake.
        assert_eq!(
            w.look_awake(woke + MIN, 2 * MIN + 20 * SEC, 2 * SEC),
            Noted::Opened
        );
    }

    #[test]
    fn a_clock_that_ran_ahead_and_came_back_does_not_stop_the_almanac() {
        let mut w = Watch::new();
        w.look(0, 0);
        // The clock jumps a year ahead for a minute, and someone types.
        let ahead = 365 * 24 * HOUR;
        w.look_awake(ahead, MIN, SEC);
        assert_eq!(w.spans().len(), 2);
        // It comes back: the span from the future goes, and now is noted
        // (from its input: the look before it is no guide to its start).
        assert_eq!(w.look_awake(20 * MIN, 2 * MIN, SEC), Noted::Opened);
        assert_eq!(w.spans(), vec![(0, 0), (20 * MIN - SEC, 20 * MIN - SEC)]);
    }

    #[test]
    fn marks_are_set_replaced_and_taken_away() {
        let mut a = Almanac::default();
        assert!(a.mark("2026-11-10", Some(Mark::Leave)));
        assert_eq!(a.marks.get("2026-11-10"), Some(&Mark::Leave));
        assert!(a.mark("2026-11-10", Some(Mark::Clear)));
        assert_eq!(a.marks.get("2026-11-10"), Some(&Mark::Clear));
        assert!(a.mark("2026-11-10", None));
        assert!(a.marks.is_empty());
    }

    #[test]
    fn a_day_that_is_not_a_date_is_refused() {
        let mut a = Almanac::default();
        for day in [
            "",
            "today",
            "2026-13-01",
            "2026-02-30",
            "2026-1-5",
            "10-11-2026",
        ] {
            assert!(!a.mark(day, Some(Mark::Leave)), "{day}");
        }
        assert!(a.marks.is_empty());
    }

    #[test]
    fn a_day_is_corrected_to_one_stretch_and_put_back() {
        let mut a = Almanac::default();
        let stretch = Span {
            from_unix_ms: T0,
            to_unix_ms: T0 + 9 * HOUR,
        };
        assert!(a.fix("2026-11-10", Some(stretch)));
        assert_eq!(a.fixes.get("2026-11-10"), Some(&stretch));
        assert!(a.fix("2026-11-10", None));
        assert!(a.fixes.is_empty());
    }

    #[test]
    fn a_correction_that_makes_no_sense_is_refused() {
        let mut a = Almanac::default();
        let backwards = Span {
            from_unix_ms: T0 + HOUR,
            to_unix_ms: T0,
        };
        let endless = Span {
            from_unix_ms: T0,
            to_unix_ms: T0 + MAX_FIX_MS + 1,
        };
        let fine = Span {
            from_unix_ms: T0,
            to_unix_ms: T0,
        };
        assert!(!a.fix("2026-11-10", Some(backwards)));
        assert!(!a.fix("2026-11-10", Some(endless)));
        assert!(!a.fix("the tenth", Some(fine)));
        assert!(a.fixes.is_empty());
        assert!(
            a.fix("2026-11-10", Some(fine)),
            "an empty day is a correction too"
        );
    }

    #[test]
    fn the_earliest_marks_go_first_beyond_the_cap() {
        let mut a = Almanac::default();
        let first = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        for day in first.iter_days().take(MAX_MARKS + 3) {
            assert!(a.mark(&day.format("%Y-%m-%d").to_string(), Some(Mark::Leave)));
        }
        assert_eq!(a.marks.len(), MAX_MARKS);
        assert!(!a.marks.contains_key("2020-01-03"));
        assert!(a.marks.contains_key("2020-01-04"));
    }

    #[test]
    fn spans_older_than_the_keep_are_dropped() {
        let mut a = Almanac {
            spans: vec![
                Span {
                    from_unix_ms: T0,
                    to_unix_ms: T0 + MIN,
                },
                Span {
                    from_unix_ms: T0 + HOUR,
                    to_unix_ms: T0 + 2 * HOUR,
                },
            ],
            ..Almanac::default()
        };
        a.trim(T0 + KEEP_MS + 30 * MIN);
        assert_eq!(a.spans.len(), 1);
        assert_eq!(a.spans[0].from_unix_ms, T0 + HOUR);
    }

    #[test]
    fn round_trips_through_json_in_snake_case() {
        let mut a = Almanac::default();
        a.spans.push(Span {
            from_unix_ms: T0,
            to_unix_ms: T0 + MIN,
        });
        a.mark("2026-11-10", Some(Mark::Leave));
        a.mark("2026-11-11", Some(Mark::Clear));
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["spans"][0]["from_unix_ms"], T0);
        assert_eq!(json["spans"][0]["to_unix_ms"], T0 + MIN);
        assert_eq!(json["marks"]["2026-11-10"], "leave");
        assert_eq!(json["marks"]["2026-11-11"], "clear");
        assert!(
            json["fixes"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
        );
        a.fix(
            "2026-11-12",
            Some(Span {
                from_unix_ms: T0,
                to_unix_ms: T0 + HOUR,
            }),
        );
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["fixes"]["2026-11-12"]["to_unix_ms"], T0 + HOUR);
        let back: Almanac = serde_json::from_value(json).unwrap();
        assert_eq!(back, a);
        let empty: Almanac = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Almanac::default());
    }
}
