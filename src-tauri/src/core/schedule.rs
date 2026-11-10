//! Schedule arithmetic: which window, if any, contains a moment, and when the
//! next one opens. Pure: callers pass the moment, and the time zone travels
//! with it, so tests can use fixed zones (including ones with a DST change).

use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveDateTime, TimeDelta, TimeZone};

use super::settings::Schedule;

/// How far ahead [`next_window`] looks. A schedule with at least one day on
/// always has a window within a week; one extra day covers the edges.
const LOOKAHEAD_DAYS: u64 = 8;

/// Longest run of missing local minutes we step over when a window starts or
/// ends inside a daylight-saving gap. Real gaps are 30 or 60 minutes.
const MAX_GAP_MINUTES: i64 = 3 * 60;

/// One open stretch of the schedule, as real instants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window<Tz: TimeZone> {
    /// When the window opens.
    pub start: DateTime<Tz>,
    /// When it closes (exclusive).
    pub end: DateTime<Tz>,
}

/// The window containing `now`, if any, honouring overnight spans and
/// weekdays. An overnight window belongs to the day it starts on, so Friday
/// 22:00 until 06:00 is open early on Saturday even when Saturday is off.
pub fn current_window<Tz: TimeZone>(s: &Schedule, now: &DateTime<Tz>) -> Option<Window<Tz>> {
    if !s.enabled {
        return None;
    }
    let tz = now.timezone();
    let today = now.date_naive();
    [today.pred_opt(), Some(today)]
        .into_iter()
        .flatten()
        .filter_map(|day| window_on(s, day, &tz))
        .find(|w| w.start <= *now && *now < w.end)
}

/// The first window that opens strictly after `now`, within the next 8 days.
pub fn next_window<Tz: TimeZone>(s: &Schedule, now: &DateTime<Tz>) -> Option<Window<Tz>> {
    if !s.enabled {
        return None;
    }
    let tz = now.timezone();
    let today = now.date_naive();
    (0..=LOOKAHEAD_DAYS)
        .filter_map(|n| today.checked_add_days(Days::new(n)))
        .filter_map(|day| window_on(s, day, &tz))
        .find(|w| w.start > *now)
}

/// The next moment the answer to [`current_window`] can change: the end of
/// the current window, else the next start. The engine sleeps until then.
pub fn next_boundary<Tz: TimeZone>(s: &Schedule, now: &DateTime<Tz>) -> Option<DateTime<Tz>> {
    current_window(s, now)
        .map(|w| w.end)
        .or_else(|| next_window(s, now).map(|w| w.start))
}

/// The window that starts on `day`, if that weekday is on. An end at or
/// before the start is the next day's, so equal times make a whole day.
fn window_on<Tz: TimeZone>(s: &Schedule, day: NaiveDate, tz: &Tz) -> Option<Window<Tz>> {
    let index = usize::try_from(day.weekday().num_days_from_monday()).ok()?;
    if !s.days.get(index).copied().unwrap_or(false) {
        return None;
    }
    let end_day = if s.end_minutes > s.start_minutes {
        day
    } else {
        day.succ_opt()?
    };
    let start = at(tz, day, s.start_minutes)?;
    let end = at(tz, end_day, s.end_minutes)?;
    (end > start).then_some(Window { start, end })
}

fn at<Tz: TimeZone>(tz: &Tz, day: NaiveDate, minutes: u16) -> Option<DateTime<Tz>> {
    let naive = day.and_hms_opt(u32::from(minutes / 60), u32::from(minutes % 60), 0)?;
    resolve_local(tz, naive)
}

/// Turns a wall-clock time into an instant. When clocks go back and the time
/// happens twice, the earlier one wins; when clocks go forward and it never
/// happens, the first moment that does exist after it is used.
pub fn resolve_local<Tz: TimeZone>(tz: &Tz, naive: NaiveDateTime) -> Option<DateTime<Tz>> {
    (0..=MAX_GAP_MINUTES).find_map(|step| {
        let candidate = naive.checked_add_signed(TimeDelta::minutes(step))?;
        tz.from_local_datetime(&candidate).earliest()
    })
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, MappedLocalTime, NaiveDate, Timelike};

    use super::*;

    fn office() -> Schedule {
        Schedule {
            enabled: true,
            ..Schedule::default()
        }
    }

    fn overnight() -> Schedule {
        Schedule {
            enabled: true,
            days: [true, true, true, true, true, false, false],
            start_minutes: 22 * 60,
            end_minutes: 6 * 60,
        }
    }

    /// UTC+05:30: no DST, an odd half-hour offset to catch UTC assumptions.
    fn half_hour_zone() -> FixedOffset {
        FixedOffset::east_opt(5 * 3600 + 1800).unwrap()
    }

    /// 2026-03-02 is a Monday.
    fn local_at(day: u32, h: u32, m: u32) -> DateTime<FixedOffset> {
        half_hour_zone()
            .with_ymd_and_hms(2026, 3, day, h, m, 0)
            .unwrap()
    }

    #[test]
    fn inside_a_window() {
        let w = current_window(&office(), &local_at(2, 12, 0)).unwrap();
        assert_eq!(w.start, local_at(2, 9, 0));
        assert_eq!(w.end, local_at(2, 18, 0));
    }

    #[test]
    fn outside_a_window() {
        assert_eq!(current_window(&office(), &local_at(2, 8, 59)), None);
        assert_eq!(current_window(&office(), &local_at(2, 19, 0)), None);
    }

    #[test]
    fn start_is_inside_and_end_is_outside() {
        assert!(current_window(&office(), &local_at(2, 9, 0)).is_some());
        assert_eq!(current_window(&office(), &local_at(2, 18, 0)), None);
    }

    #[test]
    fn disabled_schedule_has_no_windows() {
        let s = Schedule::default();
        assert_eq!(current_window(&s, &local_at(2, 12, 0)), None);
        assert_eq!(next_window(&s, &local_at(2, 12, 0)), None);
        assert_eq!(next_boundary(&s, &local_at(2, 12, 0)), None);
    }

    #[test]
    fn weekday_off_has_no_window() {
        // 2026-03-07 is a Saturday.
        assert_eq!(current_window(&office(), &local_at(7, 12, 0)), None);
    }

    #[test]
    fn the_same_time_twice_is_the_whole_day() {
        let all_day = Schedule {
            enabled: true,
            start_minutes: 9 * 60,
            end_minutes: 9 * 60,
            ..Schedule::default()
        };
        let w = current_window(&all_day, &local_at(2, 3 + 12, 0)).unwrap();
        assert_eq!(w.start, local_at(2, 9, 0));
        assert_eq!(w.end, local_at(3, 9, 0), "until the same time tomorrow");
        // Friday's day runs into Saturday morning, then nothing until Monday.
        assert!(current_window(&all_day, &local_at(7, 8, 0)).is_some());
        assert!(current_window(&all_day, &local_at(7, 10, 0)).is_none());
        assert_eq!(
            next_window(&all_day, &local_at(7, 10, 0)).map(|w| w.start),
            Some(local_at(9, 9, 0))
        );
    }

    #[test]
    fn overnight_window_spans_midnight() {
        let s = overnight();
        let late = current_window(&s, &local_at(2, 23, 0)).unwrap();
        assert_eq!(late.start, local_at(2, 22, 0));
        assert_eq!(late.end, local_at(3, 6, 0));
        let early = current_window(&s, &local_at(3, 5, 59)).unwrap();
        assert_eq!(early, late);
        assert_eq!(current_window(&s, &local_at(3, 6, 0)), None);
        assert_eq!(current_window(&s, &local_at(2, 21, 59)), None);
    }

    #[test]
    fn overnight_window_belongs_to_the_day_it_starts() {
        let s = overnight();
        // Friday night runs into Saturday morning even though Saturday is off.
        assert!(current_window(&s, &local_at(7, 3, 0)).is_some());
        // Saturday night has no window, so early Sunday is resting.
        assert_eq!(current_window(&s, &local_at(8, 3, 0)), None);
        // Sunday is off, but Monday's early hours belong to Sunday night.
        assert_eq!(current_window(&s, &local_at(9, 3, 0)), None);
    }

    #[test]
    fn next_window_later_today() {
        let w = next_window(&office(), &local_at(2, 7, 0)).unwrap();
        assert_eq!(w.start, local_at(2, 9, 0));
    }

    #[test]
    fn next_window_is_strictly_after_now() {
        let w = next_window(&office(), &local_at(2, 9, 0)).unwrap();
        assert_eq!(w.start, local_at(3, 9, 0));
    }

    #[test]
    fn next_window_skips_the_weekend() {
        // Friday evening → Monday morning.
        let w = next_window(&office(), &local_at(6, 19, 0)).unwrap();
        assert_eq!(w.start, local_at(9, 9, 0));
        assert_eq!(w.end, local_at(9, 18, 0));
    }

    #[test]
    fn no_days_means_no_next_window() {
        let s = Schedule {
            days: [false; 7],
            ..office()
        };
        assert_eq!(next_window(&s, &local_at(2, 7, 0)), None);
    }

    #[test]
    fn next_boundary_is_the_end_inside_and_the_start_outside() {
        let s = office();
        assert_eq!(
            next_boundary(&s, &local_at(2, 12, 0)),
            Some(local_at(2, 18, 0))
        );
        assert_eq!(
            next_boundary(&s, &local_at(2, 18, 0)),
            Some(local_at(3, 9, 0))
        );
        assert_eq!(
            next_boundary(&s, &local_at(2, 8, 0)),
            Some(local_at(2, 9, 0))
        );
    }

    /// A zone shaped like Europe/London in 2026: GMT, then BST from
    /// 29 March 01:00 UTC until 25 October 01:00 UTC.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct London;

    fn spring() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 29)
            .unwrap()
            .and_hms_opt(1, 0, 0)
            .unwrap()
    }

    fn autumn() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 25)
            .unwrap()
            .and_hms_opt(1, 0, 0)
            .unwrap()
    }

    fn london_offset(utc: &NaiveDateTime) -> FixedOffset {
        let summer = *utc >= spring() && *utc < autumn();
        FixedOffset::east_opt(if summer { 3600 } else { 0 }).unwrap()
    }

    impl TimeZone for London {
        type Offset = FixedOffset;

        fn from_offset(_: &FixedOffset) -> Self {
            London
        }

        fn offset_from_local_date(&self, local: &NaiveDate) -> MappedLocalTime<FixedOffset> {
            self.offset_from_local_datetime(&local.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_local_datetime(
            &self,
            local: &NaiveDateTime,
        ) -> MappedLocalTime<FixedOffset> {
            // Larger offset first: it maps to the earlier instant.
            let fits: Vec<FixedOffset> = [3600, 0]
                .into_iter()
                .map(|secs| FixedOffset::east_opt(secs).unwrap())
                .filter(|off| {
                    london_offset(&(*local - TimeDelta::seconds(off.local_minus_utc().into())))
                        == *off
                })
                .collect();
            match fits.as_slice() {
                [] => MappedLocalTime::None,
                [one] => MappedLocalTime::Single(*one),
                [a, b, ..] => MappedLocalTime::Ambiguous(*a, *b),
            }
        }

        fn offset_from_utc_date(&self, utc: &NaiveDate) -> FixedOffset {
            london_offset(&utc.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> FixedOffset {
            london_offset(utc)
        }
    }

    fn night_shift() -> Schedule {
        // 01:30 until 02:30 every day: on the spring day both ends fall in or
        // around the missing hour.
        Schedule {
            enabled: true,
            days: [true; 7],
            start_minutes: 90,
            end_minutes: 150,
        }
    }

    #[test]
    fn dst_gap_moves_a_missing_start_to_the_first_real_minute() {
        // 29 March: 01:00 jumps to 02:00 local. 01:30 never happens.
        let before = London.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
        let w = next_window(&night_shift(), &before).unwrap();
        assert_eq!(w.start.naive_utc(), spring(), "starts at the jump");
        assert_eq!((w.start.hour(), w.start.minute()), (2, 0));
        assert_eq!((w.end.hour(), w.end.minute()), (2, 30));
        assert!(w.end > w.start);
    }

    #[test]
    fn dst_overlap_uses_the_earlier_of_the_two_times() {
        // 25 October: 02:00 BST falls back to 01:00 GMT, so 01:30 happens twice.
        let before = London.with_ymd_and_hms(2026, 10, 25, 0, 0, 0).unwrap();
        let w = next_window(&night_shift(), &before).unwrap();
        let first_0130 = NaiveDate::from_ymd_opt(2026, 10, 25)
            .unwrap()
            .and_hms_opt(0, 30, 0)
            .unwrap();
        assert_eq!(w.start.naive_utc(), first_0130);
    }

    #[test]
    fn resolve_local_passes_ordinary_times_through() {
        let naive = NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        let t = resolve_local(&London, naive).unwrap();
        assert_eq!(t.naive_local(), naive);
    }
}
