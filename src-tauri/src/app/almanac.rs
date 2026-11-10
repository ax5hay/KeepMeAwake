//! Keeps the almanac: one look at the idle time a minute, written down.
//!
//! This is apart from the engine on purpose. Whether someone is at the
//! machine is not a power decision, and it has to be noted with the sun up
//! or down alike. The reading is the one the overnight guard uses: it asks
//! how long since the last input and never sends any (ADR 0002).

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

use crate::core::almanac::{Almanac, Look, Mark, Noted, Span};
use crate::persistence;
use crate::platform::{Clock, IdleProbe};

/// How often the idle time is read while the almanac is on. The OS stamps
/// the last input itself, so a minute between looks still gives the moment
/// someone left to the second; only a return is rounded to a look.
pub const LOOK_EVERY: Duration = Duration::from_secs(60);

/// The longest a growing span goes unsaved while someone is here.
pub const SAVE_EVERY_MS: u64 = 15 * 60 * 1000;

/// Idle this long is a break: the count towards a break reminder starts
/// again from the next touch.
pub const BREAK: Duration = Duration::from_secs(5 * 60);

/// Called with the minutes someone has been at the machine without a break.
pub type OnBreakDue = Box<dyn Fn(u64) + Send + Sync>;

/// Idle this long, whoever was here has stopped for now: what is unsaved is
/// written, so the file is final soon after the last touch.
pub const QUIET: Duration = Duration::from_secs(2 * 60);

struct Inner {
    almanac: Almanac,
    /// The look before, in this run.
    last: Option<Look>,
    /// Changed since it was last written.
    dirty: bool,
    /// When it was last written, Unix ms.
    saved_unix_ms: u64,
    /// The almanac is being kept.
    on: bool,
    /// Say when someone has gone this long without a break; `None` never.
    break_after: Option<Duration>,
    /// When the stretch without a break began, Unix ms; `None` on a break.
    streak_from: Option<u64>,
    /// The reminder has been given for this stretch.
    reminded: bool,
}

/// The almanac and the thread that keeps it.
pub struct Keeper {
    inner: Mutex<Inner>,
    wake: Condvar,
    path: PathBuf,
    idle: Mutex<Box<dyn IdleProbe>>,
    clock: Arc<dyn Clock + Sync>,
}

impl Keeper {
    /// A keeper for `almanac`, saved at `path`. Nothing runs until
    /// [`Keeper::start`].
    pub fn new(
        almanac: Almanac,
        path: PathBuf,
        idle: Box<dyn IdleProbe>,
        clock: Arc<dyn Clock + Sync>,
        on: bool,
    ) -> Self {
        Self {
            inner: Mutex::new(Inner {
                almanac,
                last: None,
                dirty: false,
                saved_unix_ms: clock.unix_ms(),
                on,
                break_after: None,
                streak_from: None,
                reminded: false,
            }),
            wake: Condvar::new(),
            path,
            idle: Mutex::new(idle),
            clock,
        }
    }

    /// Starts the thread that looks once every [`LOOK_EVERY`] while the
    /// almanac is on, and blocks without a timeout while it is off.
    /// `on_break_due` is told when a break reminder is due.
    pub fn start(self: &Arc<Self>, on_break_due: OnBreakDue) {
        let keeper = Arc::clone(self);
        let spawned = thread::Builder::new()
            .name("keepmeawake-almanac".into())
            .spawn(move || {
                loop {
                    {
                        let mut inner = keeper.inner.lock();
                        if !inner.on {
                            keeper.wake.wait(&mut inner);
                            continue;
                        }
                    }
                    if let Some(minutes) = keeper.look() {
                        on_break_due(minutes);
                    }
                    let mut inner = keeper.inner.lock();
                    if inner.on {
                        keeper.wake.wait_for(&mut inner, LOOK_EVERY);
                    }
                }
            });
        if let Err(err) = spawned {
            log::warn!("the almanac is not being kept: {err}");
        }
    }

    /// Says when someone has gone `minutes` without a break, or never.
    pub fn set_break_after(&self, minutes: Option<u16>) {
        let mut inner = self.inner.lock();
        inner.break_after = minutes.map(|m| Duration::from_secs(u64::from(m) * 60));
        // A new length is a new promise: it may be given again.
        inner.reminded = false;
    }

    /// Reads the idle time once and notes it, saving if it is time to.
    /// Does nothing while the almanac is off or the OS cannot say. Returns
    /// the minutes at the machine without a break when a reminder is due,
    /// which is once per stretch.
    pub fn look(&self) -> Option<u64> {
        let mut inner = self.inner.lock();
        if !inner.on {
            return None;
        }
        let idle = self.idle.lock().idle()?;
        let look = Look {
            at_unix_ms: self.clock.unix_ms(),
            at: self.clock.now(),
            idle,
        };
        if observe(&mut inner, look) {
            self.save(&mut inner);
        }
        break_due(&mut inner, look)
    }

    /// The almanac as it stands, after one fresh look.
    pub fn read(&self) -> Almanac {
        // A reminder that falls due on this look is left to the next one.
        let _ = self.look();
        self.inner.lock().almanac.clone()
    }

    /// Marks `day`, or takes its mark away, and saves. `None` for a day
    /// that is not a date.
    pub fn mark(&self, day: &str, mark: Option<Mark>) -> Option<Almanac> {
        let mut inner = self.inner.lock();
        if !inner.almanac.mark(day, mark) {
            return None;
        }
        self.save(&mut inner);
        Some(inner.almanac.clone())
    }

    /// Corrects `day` to one stretch, or goes back to what was read, and
    /// saves. `None` for a correction the almanac refuses.
    pub fn fix(&self, day: &str, stretch: Option<Span>) -> Option<Almanac> {
        let mut inner = self.inner.lock();
        if !inner.almanac.fix(day, stretch) {
            return None;
        }
        self.save(&mut inner);
        Some(inner.almanac.clone())
    }

    /// Empties the almanac, in memory and on disk.
    pub fn clear(&self) -> io::Result<Almanac> {
        let mut inner = self.inner.lock();
        inner.almanac = Almanac::default();
        // The next look starts again from whoever is here now.
        inner.last = None;
        inner.dirty = false;
        inner.saved_unix_ms = self.clock.unix_ms();
        persistence::save_almanac(&self.path, &inner.almanac)?;
        Ok(inner.almanac.clone())
    }

    /// Switches the keeping on or off. Off, what is unsaved is written and
    /// the thread stops waking; on, it looks at once.
    pub fn set_on(&self, on: bool) {
        let mut inner = self.inner.lock();
        if inner.on == on {
            return;
        }
        inner.on = on;
        // A look from before the switch is no guide to one after it.
        inner.last = None;
        if !on && inner.dirty {
            self.save(&mut inner);
        }
        self.wake.notify_all();
    }

    /// One last look and a save, for when the app is about to exit.
    pub fn flush(&self) {
        let _ = self.look();
        let mut inner = self.inner.lock();
        if inner.dirty {
            self.save(&mut inner);
        }
    }

    /// Writes the almanac, logging rather than failing: it is never worth
    /// interrupting anything for.
    fn save(&self, inner: &mut Inner) {
        match persistence::save_almanac(&self.path, &inner.almanac) {
            Ok(()) => {
                inner.dirty = false;
                inner.saved_unix_ms = self.clock.unix_ms();
            }
            Err(err) => log::warn!("could not save the almanac: {err}"),
        }
    }
}

/// Notes `look`. Returns `true` when the almanac should be written now: as
/// a span opens (so one cut short is still on disk), once the keys have
/// gone quiet, and every [`SAVE_EVERY_MS`] while a span grows.
fn observe(inner: &mut Inner, look: Look) -> bool {
    let noted = inner.almanac.note(look, inner.last);
    inner.last = Some(look);
    if noted != Noted::Nothing {
        inner.dirty = true;
    }
    let overdue = look.at_unix_ms.saturating_sub(inner.saved_unix_ms) >= SAVE_EVERY_MS;
    inner.dirty && (noted == Noted::Opened || look.idle >= QUIET || overdue)
}

/// The break reminder, for one look. A stretch runs from a touch until the
/// keys have been idle for [`BREAK`]; when it reaches the chosen length the
/// reminder is due, once. Returns the stretch's length in minutes then.
fn break_due(inner: &mut Inner, look: Look) -> Option<u64> {
    if look.idle >= BREAK {
        inner.streak_from = None;
        inner.reminded = false;
        return None;
    }
    let idle_ms = u64::try_from(look.idle.as_millis()).unwrap_or(0);
    let from = *inner
        .streak_from
        .get_or_insert(look.at_unix_ms.saturating_sub(idle_ms));
    let after = inner.break_after?;
    let length = Duration::from_millis(look.at_unix_ms.saturating_sub(from));
    if inner.reminded || length < after {
        return None;
    }
    inner.reminded = true;
    Some(length.as_secs() / 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fakes::{FakeIdle, ManualClock};

    struct Rig {
        keeper: Keeper,
        idle: FakeIdle,
        clock: ManualClock,
        path: PathBuf,
        _dir: tempfile::TempDir,
    }

    fn rig(on: bool) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let path = persistence::almanac_path(dir.path());
        let idle = FakeIdle::default();
        let clock = ManualClock::new();
        let keeper = Keeper::new(
            Almanac::default(),
            path.clone(),
            Box::new(idle.clone()),
            Arc::new(clock.clone()),
            on,
        );
        Rig {
            keeper,
            idle,
            clock,
            path,
            _dir: dir,
        }
    }

    impl Rig {
        /// Moves the clock on and looks, with `idle` since the last input.
        fn after(&self, secs: u64, idle_secs: u64) -> Option<u64> {
            self.clock.advance(Duration::from_secs(secs));
            self.idle.set(Some(Duration::from_secs(idle_secs)));
            self.keeper.look()
        }

        fn spans(&self) -> usize {
            self.keeper.inner.lock().almanac.spans.len()
        }

        fn on_disk(&self) -> Almanac {
            persistence::load_almanac(&self.path)
        }
    }

    #[test]
    fn a_span_is_on_disk_as_soon_as_it_opens() {
        let r = rig(true);
        r.after(0, 1);
        assert_eq!(r.spans(), 1);
        assert_eq!(r.on_disk().spans.len(), 1, "a kill now still leaves it");
    }

    #[test]
    fn a_growing_span_is_saved_when_the_keys_go_quiet_not_every_minute() {
        let r = rig(true);
        r.after(0, 1);
        let opened = r.on_disk().spans[0].to_unix_ms;
        for _ in 0..5 {
            r.after(60, 1);
        }
        assert_eq!(r.on_disk().spans[0].to_unix_ms, opened, "not yet rewritten");
        // Two minutes with nothing typed: nothing new to note, but what
        // grew is written.
        r.after(60, 61);
        r.after(60, 121);
        let in_memory = r.keeper.inner.lock().almanac.clone();
        assert_eq!(r.on_disk(), in_memory);
        assert!(r.on_disk().spans[0].to_unix_ms > opened);
    }

    #[test]
    fn a_span_that_keeps_growing_is_saved_every_quarter_hour() {
        let r = rig(true);
        r.after(0, 1);
        let opened = r.on_disk().spans[0].to_unix_ms;
        for _ in 0..14 {
            r.after(60, 1);
        }
        assert_eq!(r.on_disk().spans[0].to_unix_ms, opened);
        r.after(60, 1);
        assert!(r.on_disk().spans[0].to_unix_ms > opened);
    }

    #[test]
    fn an_idle_time_the_os_cannot_give_notes_nothing() {
        let r = rig(true);
        r.idle.set(None);
        assert_eq!(r.keeper.look(), None);
        assert_eq!(r.spans(), 0);
    }

    #[test]
    fn switched_off_it_does_not_look_and_what_it_had_is_saved() {
        let r = rig(true);
        r.after(0, 1);
        r.after(60, 1);
        r.keeper.set_on(false);
        let kept = r.keeper.inner.lock().almanac.clone();
        assert_eq!(r.on_disk(), kept, "written as it was switched off");
        r.after(3600, 1);
        assert_eq!(r.keeper.read(), kept, "no look while off");
        r.keeper.set_on(true);
        r.after(60, 1);
        assert_eq!(r.spans(), 2, "back on, whoever is here opens a new span");
    }

    #[test]
    fn it_starts_off_when_the_setting_is_off() {
        let r = rig(false);
        r.after(0, 1);
        assert_eq!(r.spans(), 0);
    }

    #[test]
    fn a_break_reminder_comes_once_after_the_chosen_time_without_one() {
        let r = rig(true);
        r.keeper.set_break_after(Some(30));
        assert_eq!(r.after(0, 1), None);
        for _ in 0..29 {
            assert_eq!(r.after(60, 1), None);
        }
        assert_eq!(r.after(60, 1), Some(30), "thirty minutes at it");
        assert_eq!(r.after(60, 1), None, "said once");
        assert_eq!(r.after(600, 1), None, "and not again in the same stretch");
    }

    #[test]
    fn five_idle_minutes_are_a_break_and_the_count_starts_again() {
        let r = rig(true);
        r.keeper.set_break_after(Some(30));
        r.after(0, 1);
        for _ in 0..20 {
            r.after(60, 1);
        }
        // Away for six minutes.
        assert_eq!(r.after(360, 360), None);
        // The first look back starts the count; thirty minutes on, it is due.
        for _ in 0..30 {
            assert_eq!(r.after(60, 1), None, "counting from the return");
        }
        assert_eq!(r.after(60, 1), Some(30));
    }

    #[test]
    fn no_reminder_unless_one_was_asked_for() {
        let r = rig(true);
        r.after(0, 1);
        for _ in 0..200 {
            assert_eq!(r.after(60, 1), None);
        }
    }

    #[test]
    fn marking_a_day_saves_and_a_bad_day_is_refused() {
        let r = rig(true);
        let marked = r.keeper.mark("2026-11-10", Some(Mark::Leave)).unwrap();
        assert_eq!(marked.marks.get("2026-11-10"), Some(&Mark::Leave));
        assert_eq!(r.on_disk(), marked);
        assert!(r.keeper.mark("someday", Some(Mark::Leave)).is_none());
        let cleared = r.keeper.mark("2026-11-10", None).unwrap();
        assert!(cleared.marks.is_empty());
    }

    #[test]
    fn correcting_a_day_saves_and_works_with_the_almanac_off() {
        let r = rig(false);
        let stretch = Span {
            from_unix_ms: 1_000,
            to_unix_ms: 9_000,
        };
        let fixed = r.keeper.fix("2026-11-10", Some(stretch)).unwrap();
        assert_eq!(fixed.fixes.get("2026-11-10"), Some(&stretch));
        assert_eq!(r.on_disk(), fixed, "by hand needs no reading");
        let backwards = Span {
            from_unix_ms: 9_000,
            to_unix_ms: 1_000,
        };
        assert!(r.keeper.fix("2026-11-10", Some(backwards)).is_none());
        assert!(r.keeper.fix("2026-11-10", None).unwrap().fixes.is_empty());
    }

    #[test]
    fn clearing_empties_it_here_and_on_disk_and_starts_again() {
        let r = rig(true);
        r.after(0, 1);
        r.keeper.mark("2026-11-10", Some(Mark::Leave));
        assert_eq!(r.keeper.clear().unwrap(), Almanac::default());
        assert_eq!(r.on_disk(), Almanac::default());
        r.after(60, 1);
        assert_eq!(r.spans(), 1, "today starts again from now");
    }

    #[test]
    fn exiting_writes_what_was_not_yet_saved() {
        let r = rig(true);
        r.after(0, 1);
        r.after(60, 1);
        r.clock.advance(Duration::from_secs(30));
        r.idle.set(Some(Duration::from_secs(1)));
        r.keeper.flush();
        let kept = r.keeper.inner.lock().almanac.clone();
        assert_eq!(r.on_disk(), kept);
    }
}
