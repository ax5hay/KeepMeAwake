//! The engine owns the session and is the only thing that touches the OS.
//!
//! It runs one background thread that sleeps on a condition variable until
//! either a command arrives or a deadline (timer expiry, battery poll,
//! schedule boundary) passes. There is no periodic polling while idle, which
//! is what lets the app sit in the tray at a rounding error of CPU.

pub mod policy;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, TimeZone, Timelike};
use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::core::history::Ended;
use crate::core::schedule;
use crate::core::watch::{self, Readings, Watcher};
use crate::core::{
    BatteryState, GuardState, Mode, PauseReason, PlanRequest, Schedule, ScheduleState, Session,
    SessionEvent, Settings, Snapshot, WatchState,
};
use crate::platform::{ActivityProbe, BatteryProbe, Clock, IdleProbe, PowerGuard};

/// Something the app shell may want to react to.
#[derive(Debug, Clone, PartialEq, Eq)]
// A handful of short-lived events per change, delivered by reference; boxing
// the snapshot would add an allocation to save nothing measurable.
#[allow(clippy::large_enum_variant)]
pub enum EngineEvent {
    /// State changed; here is the new read model.
    Snapshot(Snapshot),
    /// A timed session finished on its own.
    TimerEnded {
        /// How long the session was set for, in seconds.
        plan_secs: u64,
    },
    /// An active session stopped guarding because of power conditions, with
    /// the battery reading that caused it.
    Paused(PauseReason, BatteryState),
    /// A paused session is guarding again.
    Resumed,
    /// A schedule window opened and is holding the laptop awake.
    ScheduleStarted {
        /// When the window closes, Unix ms.
        ends_at_unix_ms: u64,
    },
    /// A schedule window that was holding the laptop awake closed.
    ScheduleEnded {
        /// When the next window opens, Unix ms.
        next_start_unix_ms: Option<u64>,
    },
    /// The sun rose: a manual session or a schedule window began holding
    /// the laptop awake after a stretch with neither. Reported once per
    /// stretch, including the first one after launch.
    SessionStarted {
        /// The mode it started in.
        mode: Mode,
        /// Already paused as it started (say, unplugged below the threshold).
        paused: Option<PauseReason>,
        /// When, Unix ms.
        at_unix_ms: u64,
    },
    /// A timed session is about to end: the warning set in settings has
    /// been reached. Sent once per deadline.
    SunsetSoon {
        /// When the session ends, Unix ms.
        ends_at_unix_ms: u64,
        /// Seconds left, rounded up.
        secs_left: u64,
    },
    /// An Always session has had no input for the overnight guard's time:
    /// the guard asks, once, whether anyone is there. Sent once per
    /// question; a later input withdraws it silently.
    OvernightAsk {
        /// How long since the last input, in seconds.
        idle_secs: u64,
        /// When the session ends unless someone answers, Unix ms.
        rests_at_unix_ms: u64,
    },
    /// The sun set: nothing wants the laptop awake any more, or the app is
    /// quitting while something did.
    SessionEnded {
        /// Why.
        ended: Ended,
        /// When, Unix ms.
        at_unix_ms: u64,
    },
}

/// How much "Add 30 minutes" adds to a timed session.
pub const EXTEND_BY: Duration = Duration::from_secs(30 * 60);

/// Receives engine events. Called off the engine lock, so it may call back
/// into the engine.
pub type Listener = Box<dyn Fn(&EngineEvent) + Send + Sync>;

/// OS capabilities handed to the engine at start.
pub struct Platform {
    /// Holds the wake requests.
    pub guard: Box<dyn PowerGuard>,
    /// Reads battery state.
    pub battery: Box<dyn BatteryProbe>,
    /// Reads how long since the last input, for the overnight guard.
    pub idle: Box<dyn IdleProbe>,
    /// Reads what the machine is doing, for stay awake while….
    pub activity: Box<dyn ActivityProbe>,
    /// Time source.
    pub clock: Arc<dyn Clock + Sync>,
}

// Independent facts the worker remembers between steps, not a state machine.
#[allow(clippy::struct_excessive_bools)]
struct State {
    session: Session,
    settings: Settings,
    battery: BatteryState,
    guards: GuardState,
    paused: Option<PauseReason>,
    /// The open window was skipped until this moment (its end), Unix ms.
    /// Wall clock rather than `Instant`, because windows are wall-clock times
    /// and a monotonic instant drifts from them across sleep.
    skip_until: Option<u64>,
    /// The schedule at the last evaluation; used to report each window
    /// opening and closing once.
    phase: Phase,
    /// Something changed since the worker last evaluated. A notify that
    /// lands while the worker is delivering events (lock released, not yet
    /// waiting) would otherwise be lost until the next deadline.
    dirty: bool,
    /// The sun was up at the last evaluation, as reported to listeners.
    /// Kept apart from the session and the schedule because those can
    /// change between evaluations; this is what makes every stretch report
    /// exactly one start and one end.
    awake: bool,
    /// When the current stretch was due to end on its own, Unix ms, as of
    /// the last evaluation: a timer's deadline or a window's end. A stretch
    /// that ran out while the machine slept is logged as ending then, not
    /// when the worker next woke.
    due_end_unix: Option<u64>,
    /// The deadline the last sunset warning was for, so each deadline warns
    /// once and an extended one warns again.
    warned_for: Option<Instant>,
    /// While the overnight guard is asking: when the session ends unless
    /// someone answers, Unix ms.
    guard_rests_at: Option<u64>,
    /// The start of the session someone said I'm here for; the guard
    /// leaves that session alone.
    guard_dismissed_for: Option<Instant>,
    /// Whose question is pending: the guard's hours, or the night's.
    guard_question: Question,
    /// The sun set with the night and is waiting for the next touch.
    rise: Option<Rise>,
    /// Stay awake while…: the samples rates are worked out from.
    watcher: Watcher,
    /// Where the watched conditions stood at the last evaluation.
    watch: WatchState,
    /// Processes the command line asked to stay awake for, until they exit.
    pids: Vec<u32>,
    /// The sun was clicked while a condition held: rest until it clears.
    watch_skip: bool,
    /// A condition was what held the laptop awake at the last evaluation,
    /// so a stretch that ends now ended because it cleared.
    watch_held: bool,
    shutdown: bool,
}

/// At most this many processes from the command line are watched at once.
const MAX_PIDS: usize = 32;

/// Why the overnight guard is asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Question {
    /// Nobody has touched the machine for the guard's hours.
    Hours,
    /// Sleep when I'm away: the keys went quiet inside the resting hours.
    Night,
}

/// A session that set with the night, waiting to rise at the next touch.
#[derive(Debug, Clone, Copy)]
struct Rise {
    /// When it set, Unix ms.
    since: u64,
    /// The last look for a touch: when, and the idle time read then.
    at: Instant,
    idle: Duration,
}

/// How long a session that set with the night waits for a touch. Past
/// this nobody came back the next day, and it stays down.
const RISE_WITHIN_MS: u64 = 24 * 60 * 60 * 1000;

/// Idle readings this close to "nothing new" count as nothing new.
const TOUCH_SLACK: Duration = Duration::from_secs(1);

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
    clock: Arc<dyn Clock + Sync>,
    listeners: Mutex<Vec<Listener>>,
    app_version: String,
}

/// Handle to the running engine. Cheap to clone via `Arc`.
pub struct Engine {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Engine {
    /// Starts the background thread. `settings` should already be sanitized.
    pub fn start(platform: Platform, settings: Settings, app_version: impl Into<String>) -> Self {
        Self::start_with(platform, settings, app_version, Vec::new())
    }

    /// [`Engine::start`] with `listeners` attached before the worker's first
    /// step, so they hear the first events too: the startup plan, or a
    /// schedule window already open at launch.
    pub fn start_with(
        platform: Platform,
        settings: Settings,
        app_version: impl Into<String>,
        listeners: Vec<Listener>,
    ) -> Self {
        let Platform {
            guard,
            battery,
            idle,
            activity,
            clock,
        } = platform;
        // Launching inside a window is not the window opening: no event.
        let view = ScheduleView::at(&settings.schedule, None, clock.unix_ms());
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                session: Session::off(),
                settings,
                battery: battery.read(),
                guards: GuardState::default(),
                paused: None,
                skip_until: None,
                phase: view.phase(),
                dirty: false,
                awake: false,
                due_end_unix: None,
                warned_for: None,
                guard_rests_at: None,
                guard_dismissed_for: None,
                guard_question: Question::Hours,
                rise: None,
                watcher: Watcher::default(),
                watch: WatchState::default(),
                pids: Vec::new(),
                watch_skip: false,
                watch_held: false,
                shutdown: false,
            }),
            wake: Condvar::new(),
            clock,
            listeners: Mutex::new(listeners),
            app_version: app_version.into(),
        });
        let worker = {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("keepmeawake-engine".into())
                .spawn(move || {
                    let probes = Probes {
                        battery: battery.as_ref(),
                        idle: idle.as_ref(),
                        activity: activity.as_ref(),
                    };
                    run_loop(&shared, guard, probes);
                })
                .ok()
        };
        Self {
            shared,
            worker: Mutex::new(worker),
        }
    }

    /// Registers a listener for future events.
    pub fn subscribe(&self, listener: Listener) {
        self.shared.listeners.lock().push(listener);
    }

    /// Current read model.
    pub fn snapshot(&self) -> Snapshot {
        let st = self.shared.state.lock();
        build_snapshot(&st, &self.shared)
    }

    /// Current settings.
    pub fn settings(&self) -> Settings {
        self.shared.state.lock().settings.clone()
    }

    /// Changes the plan and returns the resulting snapshot.
    pub fn set_plan(&self, request: PlanRequest) -> Snapshot {
        let request = request.resolve(self.shared.clock.unix_ms());
        let now = self.shared.clock.now();
        self.mutate(|st| {
            // Said by hand, either way: the night has no say after that.
            st.rise = None;
            st.session.apply(request, now);
        })
    }

    /// What the sun does. A manual session turns off; otherwise an open
    /// schedule window is skipped, a skipped one resumes, and outside any
    /// window the default plan from settings starts.
    pub fn toggle(&self) -> Snapshot {
        let now = self.shared.clock.now();
        let unix_now = self.shared.clock.unix_ms();
        self.mutate(|st| {
            st.rise = None;
            if st.session.is_active() {
                st.session.apply(PlanRequest::Off, now);
                return;
            }
            let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
            match view.window {
                Some((_, end)) if !view.skipped => st.skip_until = Some(end),
                Some(_) => st.skip_until = None,
                // A condition holds: rest until it clears, or undo that.
                None if st.watch.reason.is_some() => st.watch_skip = !st.watch_skip,
                None => {
                    st.session.apply(default_plan(&st.settings), now);
                }
            }
        })
    }

    /// Stays awake while the process `pid` runs (the command line's
    /// `--run`), as a condition: below a manual session and a schedule
    /// window. Forgotten once it exits.
    pub fn watch_pid(&self, pid: u32) -> Snapshot {
        self.mutate(|st| {
            if !st.pids.contains(&pid) && st.pids.len() < MAX_PIDS {
                st.pids.push(pid);
            }
            st.watch_skip = false;
        })
    }

    /// Rests for the rest of the open schedule window. No-op outside one.
    pub fn skip_today(&self) -> Snapshot {
        let unix_now = self.shared.clock.unix_ms();
        self.mutate(|st| {
            let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
            if let Some((_, end)) = view.window {
                st.skip_until = Some(end);
            }
        })
    }

    /// Pushes a timed session's end `by` later. Does nothing unless a timed
    /// session is running.
    pub fn extend(&self, by: Duration) -> Snapshot {
        self.mutate(|st| {
            st.session.extend(by);
        })
    }

    /// Answers the overnight guard: someone is here. It withdraws its
    /// question and leaves the current Always session alone until it ends.
    /// Does nothing without an Always session.
    pub fn keep_it_up(&self) -> Snapshot {
        self.mutate(|st| {
            if st.session.mode() == Mode::Indefinite {
                st.guard_dismissed_for = st.session.started_at();
                st.guard_rests_at = None;
                st.guard_question = Question::Hours;
            }
        })
    }

    /// Undoes [`Engine::skip_today`].
    pub fn unskip(&self) -> Snapshot {
        self.mutate(|st| st.skip_until = None)
    }

    /// Replaces settings (sanitized here) and re-evaluates the guards.
    pub fn update_settings(&self, settings: Settings) -> Snapshot {
        self.mutate(|st| st.settings = settings.sanitized())
    }

    /// Asks the worker to re-read the battery and re-apply now. Used when the
    /// window opens so the UI shows fresh power data without constant polling.
    pub fn refresh(&self) {
        self.shared.state.lock().dirty = true;
        self.shared.wake.notify_all();
    }

    /// Releases all guards and stops the worker. Safe to call more than once.
    pub fn shutdown(&self) {
        self.shared.state.lock().shutdown = true;
        self.shared.wake.notify_all();
        if let Some(handle) = self.worker.lock().take() {
            let _ = handle.join();
        }
    }

    /// When the worker would next wake on its own, computed the same way the
    /// loop does.
    #[cfg(test)]
    fn next_wake(&self) -> Option<Instant> {
        let st = self.shared.state.lock();
        next_wake(&st, self.shared.clock.now(), self.shared.clock.unix_ms())
    }

    fn mutate(&self, f: impl FnOnce(&mut State)) -> Snapshot {
        let mut st = self.shared.state.lock();
        let started = st.session.started_at();
        f(&mut st);
        // A session that began or ended here takes no question with it: an
        // Off and an Always landing before the worker looks again must not
        // hand the new session the old one's deadline.
        if st.session.started_at() != started {
            st.guard_rests_at = None;
            st.guard_question = Question::Hours;
        }
        st.dirty = true;
        // Wake the worker so it applies the change on its own thread.
        self.shared.wake.notify_all();
        build_snapshot(&st, &self.shared)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The "on" request used by toggles.
pub fn default_plan(settings: &Settings) -> PlanRequest {
    match settings.default_duration_secs {
        Some(seconds) => PlanRequest::For { seconds },
        None => PlanRequest::Indefinite,
    }
}

/// Where the schedule stood when the worker last looked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No window open.
    Closed,
    /// A window is open and holding the laptop awake.
    Holding,
    /// A window is open but was skipped.
    Skipped,
}

/// The schedule as seen at one moment, in Unix ms.
#[derive(Debug, Clone, Copy, Default)]
struct ScheduleView {
    /// Start and end of the open window.
    window: Option<(u64, u64)>,
    /// Start of the first window after now.
    next_start: Option<u64>,
    /// The next moment any of this can change.
    boundary: Option<u64>,
    /// The open window was skipped.
    skipped: bool,
}

impl ScheduleView {
    fn at(schedule: &Schedule, skip_until: Option<u64>, unix_now: u64) -> Self {
        let Some(now) = local_time(unix_now) else {
            return Self::default();
        };
        let window =
            schedule::current_window(schedule, &now).map(|w| (unix_ms(&w.start), unix_ms(&w.end)));
        Self {
            window,
            next_start: schedule::next_window(schedule, &now).map(|w| unix_ms(&w.start)),
            boundary: schedule::next_boundary(schedule, &now).map(|t| unix_ms(&t)),
            skipped: window.is_some() && skip_until.is_some_and(|until| unix_now < until),
        }
    }

    /// The schedule is what wants the laptop awake right now.
    fn holds(&self) -> bool {
        self.window.is_some() && !self.skipped
    }

    fn phase(&self) -> Phase {
        match (self.window, self.skipped) {
            (None, _) => Phase::Closed,
            (Some(_), false) => Phase::Holding,
            (Some(_), true) => Phase::Skipped,
        }
    }

    fn state(&self, enabled: bool) -> ScheduleState {
        ScheduleState {
            enabled,
            in_window: self.window.is_some(),
            skipped: self.skipped,
            window_start_unix_ms: self.window.map(|(start, _)| start),
            window_end_unix_ms: self.window.map(|(_, end)| end),
            next_start_unix_ms: self.next_start,
        }
    }
}

fn local_time(unix_ms: u64) -> Option<DateTime<Local>> {
    Local
        .timestamp_millis_opt(i64::try_from(unix_ms).ok()?)
        .single()
}

fn unix_ms(at: &DateTime<Local>) -> u64 {
    u64::try_from(at.timestamp_millis()).unwrap_or(0)
}

fn run_loop(shared: &Arc<Shared>, mut guard: Box<dyn PowerGuard>, probes: Probes<'_>) {
    let mut st = shared.state.lock();
    loop {
        if st.shutdown {
            if let Err(err) = guard.apply(GuardState::default()) {
                log::warn!("could not release power request on shutdown: {err}");
            }
            if st.awake {
                st.awake = false;
                let quit = EngineEvent::SessionEnded {
                    ended: ended_by(st.paused, Ended::User),
                    at_unix_ms: shared.clock.unix_ms(),
                };
                MutexGuard::unlocked(&mut st, || deliver(shared, &[quit]));
            }
            return;
        }
        st.dirty = false;
        let now = shared.clock.now();
        let unix_now = shared.clock.unix_ms();
        let events = step(&mut st, guard.as_mut(), probes, now, unix_now, shared);
        let deadline = next_wake(&st, now, unix_now);

        // Deliver events without holding the lock: listeners may call back.
        MutexGuard::unlocked(&mut st, || deliver(shared, &events));

        // A command or shutdown that arrived while the lock was released has
        // already been notified; waiting now would sleep through it.
        if st.dirty || st.shutdown {
            continue;
        }
        match deadline {
            Some(d) => {
                shared.wake.wait_until(&mut st, d);
            }
            None => shared.wake.wait(&mut st),
        }
    }
}

fn deliver(shared: &Shared, events: &[EngineEvent]) {
    let listeners = shared.listeners.lock();
    for event in events {
        for listener in listeners.iter() {
            listener(event);
        }
    }
}

/// How a stretch ended: `cause`, unless it was paused for the battery and
/// never resumed, in which case the battery is what really ended it.
fn ended_by(paused: Option<PauseReason>, cause: Ended) -> Ended {
    match paused {
        Some(PauseReason::LowBattery | PauseReason::OnBattery) => Ended::Battery,
        _ => cause,
    }
}

/// The next moment the worker must wake on its own.
fn next_wake(st: &State, now: Instant, unix_now: u64) -> Option<Instant> {
    let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
    let boundary = view
        .boundary
        .map(|at| now + Duration::from_millis(at.saturating_sub(unix_now)));
    let usual = policy::next_deadline(&st.session, st.phase == Phase::Holding, boundary, now);
    let warning = pending_warning(st, &view, now, unix_now).filter(|at| *at > now);
    let guard = st
        .guard_rests_at
        .map(|at| now + Duration::from_millis(at.saturating_sub(unix_now)));
    // Waiting for a touch: the sun is down, so nothing else would look.
    let touch = st.rise.map(|_| now + policy::BATTERY_POLL);
    // Something is watched: look again, sun up or down.
    let watching = watches(st).then(|| now + policy::WATCH_POLL);
    [usual, warning, guard, touch, watching]
        .into_iter()
        .flatten()
        .min()
}

/// What the worker reads from the machine on each step.
#[derive(Clone, Copy)]
struct Probes<'a> {
    battery: &'a dyn BatteryProbe,
    idle: &'a dyn IdleProbe,
    activity: &'a dyn ActivityProbe,
}

/// Any condition is on, or a process from the command line is watched:
/// the machine has to be read.
fn watches(st: &State) -> bool {
    st.settings.watch.any() || !st.pids.is_empty()
}

/// Stay awake while…, for one step: reads only what is switched on, and
/// works out which condition holds. Returns whether one holds the laptop
/// awake (it holds, and the sun was not clicked to skip it).
fn watch_step(st: &mut State, activity: &dyn ActivityProbe, now: Instant) -> bool {
    if !watches(st) {
        // Nothing on: forget the samples, so switching one on starts clean.
        st.watcher = Watcher::default();
        st.watch = WatchState::default();
        st.watch_skip = false;
        return false;
    }
    let settings = &st.settings.watch;
    st.pids.retain(|pid| activity.alive(*pid));
    let readings = Readings {
        running: if settings.apps.is_empty() {
            Vec::new()
        } else {
            watch::matching(&settings.apps, &activity.processes())
        },
        pid: st.pids.first().copied(),
        transferred: settings.transfers.then(|| activity.transferred()).flatten(),
        cpu: settings.busy.then(|| activity.cpu()).flatten(),
        full_screen: settings.full_screen && activity.full_screen(),
        // Only a laptop has a built-in display to be docked away from.
        display: settings.display && st.battery.present && activity.external_display(),
    };
    let mut state = st.watcher.observe(settings, &readings, st.battery, now);
    if state.reason.is_none() {
        // The condition that was skipped cleared: the next one counts again.
        st.watch_skip = false;
    }
    state.skipped = st.watch_skip && state.reason.is_some();
    st.watch = state;
    st.watch.reason.is_some() && !st.watch_skip
}

/// The guard is watching the current session: Always, the guard or Set
/// with me on, and nobody said I'm here for this session.
fn guard_watches(st: &State) -> bool {
    st.session.mode() == Mode::Indefinite
        && (st.settings.overnight_guard_hours.is_some() || night_rest(&st.settings).is_some())
        && st.guard_dismissed_for != st.session.started_at()
}

/// How long the keys must be quiet in the resting hours before an Always
/// session sets for the night, if the almanac's Sleep when I'm away is on.
fn night_rest(settings: &Settings) -> Option<Duration> {
    settings
        .almanac
        .rest_after_mins
        .filter(|_| settings.almanac.enabled)
        .map(|mins| Duration::from_secs(u64::from(mins) * 60))
}

/// Whether `unix_now` falls in the almanac's resting hours, local time.
fn resting_now(settings: &Settings, unix_now: u64) -> bool {
    local_time(unix_now).is_some_and(|at| {
        let minute = u16::try_from(at.hour() * 60 + at.minute()).unwrap_or(0);
        settings.almanac.is_resting(minute)
    })
}

/// A session that set with the night rises at the next touch. Whether
/// anyone touched anything is judged on the monotonic clock, as the almanac
/// judges it: with no input the idle time grows by at least the time that
/// passed, so a machine that wakes itself in the night raises nothing.
fn rise_on_touch(st: &mut State, idle: &dyn IdleProbe, now: Instant, unix_now: u64) {
    let Some(rise) = st.rise else {
        return;
    };
    if st.session.is_active() || unix_now.saturating_sub(rise.since) > RISE_WITHIN_MS {
        st.rise = None;
        return;
    }
    let Some(reading) = idle.idle() else {
        return;
    };
    let untouched = rise.idle + now.saturating_duration_since(rise.at);
    if reading + TOUCH_SLACK < untouched {
        st.rise = None;
        st.session.apply(PlanRequest::Indefinite, now);
    } else {
        st.rise = Some(Rise {
            at: now,
            idle: reading,
            ..rise
        });
    }
}

/// The overnight guard, for one step. While an Always session runs, the
/// guard is on and nobody said I'm here for this session, it reads how
/// long since the last input: at the guard's time it asks (once), below it
/// any question is withdrawn, and a question left unanswered past its
/// grace ends the session. Returns when the session rested, Unix ms, if
/// this step ended it.
fn overnight_guard(
    st: &mut State,
    idle: &dyn IdleProbe,
    now: Instant,
    unix_now: u64,
    events: &mut Vec<EngineEvent>,
) -> Option<u64> {
    let watching = guard_watches(st);
    let hours = st
        .settings
        .overnight_guard_hours
        .filter(|_| watching)
        .map(|hours| Duration::from_secs(u64::from(hours) * 3600));
    // Inside the resting hours, Sleep when I'm away asks much sooner than the guard.
    let night =
        night_rest(&st.settings).filter(|_| watching && resting_now(&st.settings, unix_now));
    let limit = [hours, night].into_iter().flatten().min();
    // Read only while it can matter. No reading means nobody can say, and
    // the guard never acts on that.
    let reading = limit.and_then(|_| idle.idle());
    let (Some(limit), Some(idle)) = (limit, reading) else {
        st.guard_rests_at = None;
        st.guard_question = Question::Hours;
        return None;
    };
    if idle < limit {
        // Someone is here, or has been recently: the clock starts again
        // from their last input.
        st.guard_rests_at = None;
        st.guard_question = Question::Hours;
        return None;
    }
    match st.guard_rests_at {
        None => {
            let rests_at = unix_now.saturating_add(millis(policy::GUARD_GRACE));
            st.guard_rests_at = Some(rests_at);
            // The night's question, unless the guard's own hours are up too.
            st.guard_question = if hours.is_none_or(|hours| idle < hours) {
                Question::Night
            } else {
                Question::Hours
            };
            events.push(EngineEvent::OvernightAsk {
                idle_secs: idle.as_secs(),
                rests_at_unix_ms: rests_at,
            });
            None
        }
        Some(rests_at) if unix_now >= rests_at => {
            st.guard_rests_at = None;
            if std::mem::replace(&mut st.guard_question, Question::Hours) == Question::Night {
                // Set with the night: up again at the next touch.
                st.rise = Some(Rise {
                    since: rests_at,
                    at: now,
                    idle,
                });
            }
            st.session.apply(PlanRequest::Off, now);
            Some(rests_at)
        }
        Some(_) => None,
    }
}

/// The moment the current timed session should warn, unless it already has,
/// or unless an open window outlasts it: then the sun does not set when the
/// timer ends, and there is nothing to warn about.
fn pending_warning(
    st: &State,
    view: &ScheduleView,
    now: Instant,
    unix_now: u64,
) -> Option<Instant> {
    let at = policy::sunset_warning_at(&st.session, st.settings.sunset_warning_mins)?;
    let deadline = st.session.deadline()?;
    let ends = unix_now.saturating_add(millis(deadline.saturating_duration_since(now)));
    let held_past_it = view.holds() && view.window.is_some_and(|(_, end)| end > ends);
    (st.warned_for != Some(deadline) && !held_past_it).then_some(at)
}

/// One evaluation of the state machine. Separated from the thread so it can be
/// tested deterministically with fakes.
fn step(
    st: &mut State,
    guard: &mut dyn PowerGuard,
    probes: Probes<'_>,
    now: Instant,
    unix_now: u64,
    shared: &Shared,
) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    rise_on_touch(st, probes.idle, now, unix_now);
    let was_due = st.due_end_unix;
    let was_active = st.session.is_active() || st.phase == Phase::Holding;
    let plan_secs = st.session.plan_duration().map_or(0, |d| d.as_secs());
    let expired = st.session.tick(now) == Some(SessionEvent::Expired);
    if expired {
        events.push(EngineEvent::TimerEnded { plan_secs });
    }

    if st.skip_until.is_some_and(|until| unix_now >= until) {
        st.skip_until = None;
    }

    let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
    match (st.phase, view.phase(), view.window) {
        (Phase::Closed, Phase::Holding, Some((_, end))) => {
            events.push(EngineEvent::ScheduleStarted {
                ends_at_unix_ms: end,
            });
        }
        (Phase::Holding, Phase::Closed, _) => events.push(EngineEvent::ScheduleEnded {
            next_start_unix_ms: view.next_start,
        }),
        _ => {}
    }
    let window_closed = st.phase == Phase::Holding && view.phase() == Phase::Closed;
    st.phase = view.phase();
    if let (Some(at), Some(deadline)) = (
        pending_warning(st, &view, now, unix_now),
        st.session.deadline(),
    ) && now >= at
    {
        st.warned_for = Some(deadline);
        let left = deadline.saturating_duration_since(now);
        events.push(EngineEvent::SunsetSoon {
            ends_at_unix_ms: unix_now.saturating_add(millis(left)),
            secs_left: millis(left).div_ceil(1000),
        });
    }
    let rested = overnight_guard(st, probes.idle, now, unix_now, &mut events);
    st.battery = probes.battery.read();
    let watch_holds = watch_step(st, probes.activity, now);
    // Cleared, not skipped: a click on the sun is the user ending it.
    let watch_cleared = st.watch_held && st.watch.reason.is_none();
    let active = st.session.is_active() || st.phase == Phase::Holding || watch_holds;

    let (desired, paused) = policy::decide(active, &st.settings, st.battery);
    let previously_paused = st.paused;
    match guard.apply(desired) {
        Ok(()) => st.paused = paused,
        Err(err) => {
            log::error!("power request failed: {err}");
            st.paused = active.then_some(PauseReason::PowerRequestFailed);
        }
    }
    st.guards = guard.current();

    if active && was_active {
        match (previously_paused, st.paused) {
            (None, Some(reason)) => events.push(EngineEvent::Paused(reason, st.battery)),
            (Some(_), None) => events.push(EngineEvent::Resumed),
            _ => {}
        }
    }
    match (st.awake, active) {
        (false, true) => events.push(EngineEvent::SessionStarted {
            mode: if st.session.is_active() {
                st.session.mode()
            } else if st.phase == Phase::Holding {
                Mode::Scheduled
            } else {
                Mode::Watching
            },
            paused: st.paused,
            at_unix_ms: unix_now,
        }),
        (true, false) => events.push(sunset(
            Sunset {
                rested,
                expired,
                window_closed,
                watch_cleared,
                was_due,
                previously_paused,
            },
            unix_now,
        )),
        _ => {}
    }
    st.awake = active;
    // What holds it now is a condition, and nothing above it.
    st.watch_held = watch_holds && !st.session.is_active() && st.phase != Phase::Holding;
    st.due_end_unix = match st.session.deadline() {
        Some(deadline) => {
            Some(unix_now.saturating_add(millis(deadline.saturating_duration_since(now))))
        }
        None if st.phase == Phase::Holding => view.window.map(|(_, end)| end),
        None => None,
    };
    events.insert(0, EngineEvent::Snapshot(build_snapshot(st, shared)));
    events
}

/// What one step knows about a stretch that just ended.
#[derive(Debug, Clone, Copy)]
struct Sunset {
    /// The overnight guard let it go, due at this moment (Unix ms).
    rested: Option<u64>,
    /// A timed session ran out.
    expired: bool,
    /// The schedule window holding it closed.
    window_closed: bool,
    /// The condition holding it cleared.
    watch_cleared: bool,
    /// When it was due to end on its own, as of the step before (Unix ms).
    was_due: Option<u64>,
    /// Why it was paused at the step before, if it was.
    previously_paused: Option<PauseReason>,
}

/// The `SessionEnded` for a stretch that just ended: why, and when.
fn sunset(s: Sunset, unix_now: u64) -> EngineEvent {
    let cause = if s.rested.is_some() {
        Ended::Idle
    } else if s.expired {
        Ended::Timer
    } else if s.window_closed {
        Ended::Schedule
    } else if s.watch_cleared {
        Ended::Watch
    } else {
        Ended::User
    };
    // Ran out on its own: it ended when it was due, even if the machine
    // slept through that and the worker is only now awake.
    let at_unix_ms = match cause {
        Ended::Timer | Ended::Schedule => s.was_due.map_or(unix_now, |due| due.min(unix_now)),
        Ended::Idle => s.rested.map_or(unix_now, |at| at.min(unix_now)),
        _ => unix_now,
    };
    // The guard is what let it go, paused for the battery or not.
    let ended = if cause == Ended::Idle {
        Ended::Idle
    } else {
        ended_by(s.previously_paused, cause)
    };
    EngineEvent::SessionEnded { ended, at_unix_ms }
}

fn build_snapshot(st: &State, shared: &Shared) -> Snapshot {
    let now = shared.clock.now();
    let unix_now = shared.clock.unix_ms();
    let to_unix = |at: Instant| -> u64 {
        if at >= now {
            unix_now.saturating_add(millis(at - now))
        } else {
            unix_now.saturating_sub(millis(now - at))
        }
    };
    let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
    let mut snapshot = Snapshot {
        mode: st.session.mode(),
        started_at_unix_ms: st.session.started_at().map(to_unix),
        ends_at_unix_ms: st.session.deadline().map(to_unix),
        remaining_secs: st.session.remaining(now).map(|d| millis(d).div_ceil(1000)),
        plan_secs: st.session.plan_duration().map(|d| d.as_secs()),
        guards: st.guards,
        paused: st.paused,
        battery: st.battery,
        schedule: view.state(st.settings.schedule.enabled),
        // Only while an Always session can still be let go; a command that
        // just ended it is answered before the worker clears the question.
        guard_rests_at_unix_ms: st
            .guard_rests_at
            .filter(|_| st.session.mode() == Mode::Indefinite),
        guard_night: st.guard_question == Question::Night
            && st.guard_rests_at.is_some()
            && st.session.mode() == Mode::Indefinite,
        rises_on_touch: st.rise.is_some() && !st.session.is_active(),
        guard_watching: guard_watches(st),
        watch: st.watch.clone(),
        settings: st.settings.clone(),
        app_version: shared.app_version.clone(),
    };
    // With no manual session the window is the plan, so the sun's arc and the
    // countdown work exactly as they do for a timed session.
    if let (false, true, Some((start, end))) = (st.session.is_active(), view.holds(), view.window) {
        snapshot.mode = Mode::Scheduled;
        snapshot.started_at_unix_ms = Some(start);
        snapshot.ends_at_unix_ms = Some(end);
        snapshot.remaining_secs = Some(end.saturating_sub(unix_now).div_ceil(1000));
        snapshot.plan_secs = Some(end.saturating_sub(start) / 1000);
    } else if !st.session.is_active() && st.watch.reason.is_some() && !st.watch_skip {
        snapshot.mode = Mode::Watching;
    }
    snapshot
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::platform::fakes::{
        FakeActivity, FakeBattery, FakeIdle, FakePowerGuard, ManualClock,
    };

    struct Rig {
        engine: Engine,
        guard: FakePowerGuard,
        battery: FakeBattery,
        idle: FakeIdle,
        activity: FakeActivity,
        clock: ManualClock,
        events: mpsc::Receiver<EngineEvent>,
    }

    fn rig(settings: Settings) -> Rig {
        rig_with_clock(settings, ManualClock::new())
    }

    fn rig_with_clock(settings: Settings, clock: ManualClock) -> Rig {
        let guard = FakePowerGuard::default();
        let battery = FakeBattery::new(BatteryState {
            on_ac: true,
            percent: Some(90),
            present: true,
            minutes_left: None,
        });
        // Someone is at the keyboard unless a test says otherwise.
        let idle = FakeIdle::default();
        idle.set(Some(Duration::from_secs(1)));
        let activity = FakeActivity::default();
        let (tx, rx) = mpsc::channel();
        let engine = Engine::start_with(
            Platform {
                guard: Box::new(guard.clone()),
                battery: Box::new(battery.clone()),
                idle: Box::new(idle.clone()),
                activity: Box::new(activity.clone()),
                clock: Arc::new(clock.clone()),
            },
            settings,
            "test",
            vec![Box::new(move |e| {
                let _ = tx.send(e.clone());
            })],
        );
        Rig {
            engine,
            guard,
            battery,
            idle,
            activity,
            clock,
            events: rx,
        }
    }

    /// Waits until the worker has applied `pred` to the guard, or panics.
    fn wait_for_guard(rig: &Rig, pred: impl Fn(GuardState) -> bool) {
        let start = Instant::now();
        while !pred(rig.guard.current()) {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "guard never reached expected state"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn drain(rig: &Rig) -> Vec<EngineEvent> {
        let mut out = Vec::new();
        while let Ok(e) = rig.events.recv_timeout(Duration::from_millis(50)) {
            out.push(e);
        }
        out
    }

    #[test]
    fn starts_off_and_holds_nothing() {
        let r = rig(Settings::default());
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Off);
        assert_eq!(snap.app_version, "test");
        assert_eq!(r.guard.current(), GuardState::default());
    }

    #[test]
    fn indefinite_plan_holds_system_and_display() {
        let r = rig(Settings::default());
        let snap = r.engine.set_plan(PlanRequest::Indefinite);
        assert_eq!(snap.mode, Mode::Indefinite);
        wait_for_guard(&r, |g| {
            g == GuardState {
                system: true,
                display: true,
            }
        });
        let snap = r.engine.snapshot();
        assert!(snap.is_guarding());
        assert_eq!(
            snap.guards,
            GuardState {
                system: true,
                display: true
            }
        );
        assert!(snap.started_at_unix_ms.is_some());
        assert_eq!(snap.ends_at_unix_ms, None);
    }

    #[test]
    fn turning_off_releases_guards() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        r.engine.set_plan(PlanRequest::Off);
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(r.engine.snapshot().mode, Mode::Off);
    }

    #[test]
    fn timed_plan_reports_deadline_in_wall_clock() {
        let r = rig(Settings::default());
        let before = r.clock.unix_ms();
        let snap = r.engine.set_plan(PlanRequest::For { seconds: 90 });
        assert_eq!(snap.mode, Mode::Timed);
        assert_eq!(snap.remaining_secs, Some(90));
        assert_eq!(snap.plan_secs, Some(90));
        assert_eq!(snap.ends_at_unix_ms, Some(before + 90_000));
    }

    #[test]
    fn until_request_is_resolved_against_wall_clock() {
        let r = rig(Settings::default());
        let target = r.clock.unix_ms() + 30_000;
        let snap = r.engine.set_plan(PlanRequest::Until { unix_ms: target });
        assert_eq!(snap.mode, Mode::Timed);
        assert_eq!(snap.ends_at_unix_ms, Some(target));
    }

    #[test]
    fn timer_expires_and_emits_event() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::For { seconds: 10 });
        wait_for_guard(&r, |g| g.system);
        drain(&r);
        r.clock.advance(Duration::from_secs(11));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let events = drain(&r);
        assert!(
            events.contains(&EngineEvent::TimerEnded { plan_secs: 10 }),
            "{events:?}"
        );
        assert_eq!(r.engine.snapshot().mode, Mode::Off);
    }

    #[test]
    fn toggle_uses_default_duration() {
        let s = Settings {
            default_duration_secs: Some(600),
            ..Settings::default()
        };
        let r = rig(s);
        let snap = r.engine.toggle();
        assert_eq!(snap.mode, Mode::Timed);
        assert_eq!(snap.remaining_secs, Some(600));
        let snap = r.engine.toggle();
        assert_eq!(snap.mode, Mode::Off);
    }

    #[test]
    fn low_battery_pauses_then_resumes_when_plugged_in() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        drain(&r);

        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Indefinite, "session stays on while paused");
        assert_eq!(snap.paused, Some(PauseReason::LowBattery));
        assert!(drain(&r).contains(&EngineEvent::Paused(
            PauseReason::LowBattery,
            BatteryState {
                on_ac: false,
                percent: Some(10),
                present: true,
                minutes_left: None,
            }
        )));

        r.battery.set(BatteryState {
            on_ac: true,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        assert_eq!(r.engine.snapshot().paused, None);
        assert!(drain(&r).contains(&EngineEvent::Resumed));
    }

    #[test]
    fn settings_change_is_applied_immediately() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.display);
        let s = Settings {
            keep_display_on: false,
            ..Settings::default()
        };
        let snap = r.engine.update_settings(s);
        assert!(!snap.settings.keep_display_on);
        wait_for_guard(&r, |g| {
            g == GuardState {
                system: true,
                display: false,
            }
        });
    }

    #[test]
    fn os_failure_is_reported_not_hidden() {
        let r = rig(Settings::default());
        r.guard.set_failing(true);
        r.engine.set_plan(PlanRequest::Indefinite);
        let start = Instant::now();
        loop {
            let snap = r.engine.snapshot();
            if snap.paused == Some(PauseReason::PowerRequestFailed) {
                assert_eq!(snap.guards, GuardState::default());
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "failure never surfaced"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Collects events until one matches `pred` (kept), or fails after 5 s.
    /// Waiting on the event itself, not on a quiet spell, keeps a slow
    /// machine from failing the test.
    fn events_until(rig: &Rig, pred: impl Fn(&EngineEvent) -> bool) -> Vec<EngineEvent> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rig.events.recv_timeout(left) {
                Ok(event) => {
                    let done = pred(&event);
                    out.push(event);
                    if done {
                        return out;
                    }
                }
                Err(err) => panic!("the expected event never arrived ({err}): {out:?}"),
            }
        }
    }

    /// Everything up to and including the step whose snapshot `pred`
    /// accepts: proof the worker evaluated the state the test set up, so a
    /// check that nothing happened there means something. A step sends its
    /// snapshot first and its other events after, so the engine is shut
    /// down (which joins the worker, delivering the rest) and drained.
    fn settled(rig: &Rig, pred: impl Fn(&Snapshot) -> bool) -> Vec<EngineEvent> {
        let mut events = events_until(rig, |e| matches!(e, EngineEvent::Snapshot(s) if pred(s)));
        rig.engine.shutdown();
        events.extend(drain(rig));
        events
    }

    fn is_sunset(event: &EngineEvent) -> bool {
        matches!(event, EngineEvent::SunsetSoon { .. })
    }

    fn sunsets(events: &[EngineEvent]) -> usize {
        events.iter().filter(|e| is_sunset(e)).count()
    }

    #[test]
    fn warns_once_when_the_warning_is_reached() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(299));
        r.engine.refresh();
        r.clock.advance(Duration::from_secs(1));
        r.engine.refresh();
        let events = events_until(&r, is_sunset);
        assert_eq!(
            events.last(),
            Some(&EngineEvent::SunsetSoon {
                ends_at_unix_ms: t0 + 600_000,
                secs_left: 300,
            })
        );
        r.clock.advance(Duration::from_secs(60));
        r.engine.refresh();
        let after = settled(&r, |s| s.remaining_secs == Some(240));
        assert_eq!(sunsets(&after), 0, "once per deadline");
    }

    #[test]
    fn never_warns_for_a_plan_no_longer_than_the_warning() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::For { seconds: 120 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(60));
        r.engine.refresh();
        let events = settled(&r, |s| s.remaining_secs == Some(60));
        assert_eq!(sunsets(&events), 0);
    }

    #[test]
    fn never_warns_when_the_warning_is_off() {
        let r = rig(Settings {
            sunset_warning_mins: None,
            ..Settings::default()
        });
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(400));
        r.engine.refresh();
        let events = settled(&r, |s| s.remaining_secs == Some(200));
        assert_eq!(sunsets(&events), 0);
    }

    #[test]
    fn extending_moves_the_end_and_warns_again_before_it() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(300));
        r.engine.refresh();
        events_until(&r, is_sunset);
        let snap = r.engine.extend(Duration::from_secs(1800));
        assert_eq!(snap.ends_at_unix_ms, Some(t0 + 2_400_000));
        assert_eq!(snap.plan_secs, Some(2400), "the arc grows");
        r.clock.advance(Duration::from_secs(1800));
        r.engine.refresh();
        assert_eq!(
            events_until(&r, is_sunset).last(),
            Some(&EngineEvent::SunsetSoon {
                ends_at_unix_ms: t0 + 2_400_000,
                secs_left: 300,
            })
        );
    }

    #[test]
    fn extending_anything_but_a_timer_changes_nothing() {
        let r = rig(Settings::default());
        assert_eq!(r.engine.extend(Duration::from_secs(1800)).mode, Mode::Off);
        r.engine.set_plan(PlanRequest::Indefinite);
        let snap = r.engine.extend(Duration::from_secs(1800));
        assert_eq!(snap.mode, Mode::Indefinite);
        assert_eq!(snap.ends_at_unix_ms, None);
    }

    #[test]
    fn never_warns_for_a_schedule_window() {
        // Always has no deadline, so no warning: see the policy test.
        let w = rig_at(with_schedule(), 2, 17, 50);
        wait_for_guard(&w, |g| g.system);
        w.clock.advance(Duration::from_secs(8 * 60));
        w.engine.refresh();
        let events = settled(&w, |s| s.remaining_secs == Some(120));
        assert_eq!(sunsets(&events), 0);
    }

    #[test]
    fn a_timer_inside_a_window_that_outlasts_it_never_warns() {
        // 12:00 inside a 9-18 window: the timer ends at 12:10 but the window
        // keeps the sun up, so there is no sunset to warn about.
        let r = rig_at(with_schedule(), 2, 12, 0);
        wait_for_guard(&r, |g| g.system);
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        r.clock.advance(Duration::from_secs(300));
        r.engine.refresh();
        let events = settled(&r, |s| s.remaining_secs == Some(300));
        assert_eq!(sunsets(&events), 0);
    }

    #[test]
    fn a_timer_that_outlasts_the_window_still_warns() {
        // 17:50, a 20-minute timer: it ends at 18:10, after the window.
        let r = rig_at(with_schedule(), 2, 17, 50);
        wait_for_guard(&r, |g| g.system);
        r.engine.set_plan(PlanRequest::For { seconds: 1200 });
        r.clock.advance(Duration::from_secs(900));
        r.engine.refresh();
        assert_eq!(
            events_until(&r, is_sunset).last(),
            Some(&EngineEvent::SunsetSoon {
                ends_at_unix_ms: local_ms(2, 18, 10),
                secs_left: 300,
            })
        );
    }

    #[test]
    fn the_worker_wakes_for_the_warning_when_it_comes_before_a_poll() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::For { seconds: 320 });
        assert_eq!(
            r.engine.next_wake(),
            Some(r.clock.now() + Duration::from_secs(20))
        );
    }

    const HOUR: Duration = Duration::from_secs(3600);
    const GRACE_MS: u64 = 10 * 60 * 1000;

    fn is_ask(event: &EngineEvent) -> bool {
        matches!(event, EngineEvent::OvernightAsk { .. })
    }

    /// Makes the worker look again and returns the snapshot of the step
    /// that did, with everything received up to it. The battery reading
    /// carries `marker` as its percentage, so that step can be told apart
    /// from earlier ones; the power source is left as the test set it.
    fn look(r: &Rig, marker: u8) -> (Snapshot, Vec<EngineEvent>) {
        let mut state = *r.battery.0.lock().unwrap();
        state.percent = Some(marker);
        r.battery.set(state);
        r.engine.refresh();
        let events = events_until(
            r,
            |e| matches!(e, EngineEvent::Snapshot(s) if s.battery.percent == Some(marker)),
        );
        match events.last() {
            Some(EngineEvent::Snapshot(s)) => (s.clone(), events),
            other => panic!("{other:?}"),
        }
    }

    /// Starts Always and leaves the machine untouched for six hours: the
    /// guard asks. Returns when it said the session would rest.
    fn asked(r: &Rig) -> u64 {
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(6 * HOUR));
        let (snap, _) = look(r, 60);
        snap.guard_rests_at_unix_ms.expect("the guard asks")
    }

    #[test]
    fn asks_once_at_the_guards_time() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(Duration::from_secs(6 * 3600 - 1)));
        let (snap, mut seen) = look(&r, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "not yet");

        r.idle.set(Some(6 * HOUR));
        let (snap, events) = look(&r, 51);
        seen.extend(events);
        assert_eq!(snap.guard_rests_at_unix_ms, Some(t0 + GRACE_MS));

        r.clock.advance(Duration::from_secs(60));
        r.idle.set(Some(6 * HOUR + Duration::from_secs(60)));
        let (snap, events) = look(&r, 52);
        seen.extend(events);
        assert_eq!(
            snap.guard_rests_at_unix_ms,
            Some(t0 + GRACE_MS),
            "same question"
        );
        assert_eq!(snap.mode, Mode::Indefinite);

        r.engine.shutdown();
        seen.extend(drain(&r));
        let asks: Vec<_> = seen.iter().filter(|e| is_ask(e)).collect();
        assert_eq!(
            asks,
            vec![&EngineEvent::OvernightAsk {
                idle_secs: 6 * 3600,
                rests_at_unix_ms: t0 + GRACE_MS,
            }],
            "asked once"
        );
    }

    #[test]
    fn input_withdraws_the_question_and_the_clock_starts_again() {
        let r = rig(Settings::default());
        asked(&r);
        r.idle.set(Some(Duration::from_secs(5)));
        let (snap, _) = look(&r, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None);
        assert_eq!(snap.mode, Mode::Indefinite);

        r.clock.advance(6 * HOUR);
        r.idle.set(Some(6 * HOUR));
        let (snap, _) = look(&r, 51);
        assert_eq!(
            snap.guard_rests_at_unix_ms,
            Some(r.clock.unix_ms() + GRACE_MS),
            "six hours after that input, it asks again"
        );
    }

    #[test]
    fn an_unanswered_question_ends_the_session_as_idle_when_it_was_due() {
        let r = rig(Settings::default());
        let rests_at = asked(&r);
        // The worker looks again well after the grace ran out.
        r.clock.advance(HOUR / 2);
        r.idle.set(Some(6 * HOUR + HOUR / 2));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)).last(),
            Some(&ended(Ended::Idle, rests_at))
        );
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Off);
        assert_eq!(snap.guard_rests_at_unix_ms, None);
    }

    #[test]
    fn a_paused_session_let_go_by_the_guard_still_ends_as_idle() {
        let r = rig(Settings::default());
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(6 * HOUR));
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.paused, Some(PauseReason::LowBattery));
        let rests_at = snap
            .guard_rests_at_unix_ms
            .expect("paused sessions are guarded");
        r.clock.advance(Duration::from_secs(10 * 60));
        r.engine.refresh();
        assert_eq!(
            sessions(&events_until(&r, is_ended)).last(),
            Some(&ended(Ended::Idle, rests_at))
        );
    }

    #[test]
    fn keep_it_up_leaves_this_session_alone_but_not_the_next() {
        let r = rig(Settings::default());
        asked(&r);
        assert!(r.engine.snapshot().guard_watching);
        let kept = r.engine.keep_it_up();
        assert_eq!(kept.guard_rests_at_unix_ms, None);
        assert!(!kept.guard_watching, "the window stops promising the guard");
        r.clock.advance(Duration::from_secs(10 * 60));
        let (snap, _) = look(&r, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None);
        assert_eq!(snap.mode, Mode::Indefinite, "kept up past the grace");
        r.clock.advance(6 * HOUR);
        r.idle.set(Some(12 * HOUR));
        let (snap, _) = look(&r, 51);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "never asks again");

        r.engine.set_plan(PlanRequest::Off);
        r.clock.advance(Duration::from_secs(1));
        r.engine.set_plan(PlanRequest::Indefinite);
        let (snap, _) = look(&r, 52);
        assert!(snap.guard_rests_at_unix_ms.is_some(), "a new session asks");
        assert!(snap.guard_watching);
    }

    #[test]
    fn keep_it_up_without_an_always_session_does_nothing() {
        let r = rig(Settings::default());
        assert_eq!(r.engine.keep_it_up().mode, Mode::Off);
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        assert_eq!(r.engine.keep_it_up().mode, Mode::Timed);
    }

    /// Leaves `r` untouched for hours, looks, then looks again past a whole
    /// grace. Returns the mode it is still in and every event, so a guard
    /// that asked (even one the snapshot hides) or ended anything shows.
    fn untouched_past_a_grace(r: &Rig) -> (Mode, Vec<EngineEvent>) {
        r.idle.set(Some(7 * HOUR));
        let (_, mut seen) = look(r, 50);
        r.clock.advance(Duration::from_secs(11 * 60));
        r.idle.set(Some(7 * HOUR + Duration::from_secs(11 * 60)));
        let (snap, events) = look(r, 51);
        seen.extend(events);
        r.engine.shutdown();
        seen.extend(drain(r));
        (snap.mode, seen)
    }

    #[test]
    fn never_asks_for_a_timer_a_window_the_guard_off_or_no_reading() {
        let timed = rig(Settings::default());
        timed
            .engine
            .set_plan(PlanRequest::For { seconds: 8 * 3600 });
        let (mode, events) = untouched_past_a_grace(&timed);
        assert_eq!(mode, Mode::Timed, "a timer is never let go");
        assert!(!timed.engine.snapshot().guard_watching);
        assert!(!events.iter().any(is_ask), "timed: {events:?}");

        let window = rig_at(with_schedule(), 2, 10, 0);
        let (mode, events) = untouched_past_a_grace(&window);
        assert_eq!(mode, Mode::Scheduled, "a window is never let go");
        assert!(!events.iter().any(is_ask), "scheduled: {events:?}");

        let off = rig(Settings {
            overnight_guard_hours: None,
            ..Settings::default()
        });
        off.engine.set_plan(PlanRequest::Indefinite);
        let (mode, events) = untouched_past_a_grace(&off);
        assert_eq!(mode, Mode::Indefinite, "guard off");
        assert!(!events.iter().any(is_ask), "guard off: {events:?}");

        let unknown = rig(Settings::default());
        unknown.engine.set_plan(PlanRequest::Indefinite);
        unknown.idle.set(None);
        let (_, mut seen) = look(&unknown, 50);
        unknown.clock.advance(Duration::from_secs(11 * 60));
        let (snap, events) = look(&unknown, 51);
        seen.extend(events);
        assert_eq!(snap.mode, Mode::Indefinite, "no reading");
        assert!(!seen.iter().any(is_ask), "no reading: {seen:?}");
    }

    /// Office hours, so the resting hours are 8 pm to 6 am, and Set with
    /// me after half an hour.
    fn set_with_me() -> Settings {
        let mut settings = Settings::default();
        settings.almanac.rest_after_mins = Some(30);
        settings
    }

    #[test]
    fn sets_for_the_night_after_a_short_quiet_in_the_resting_hours() {
        let r = rig_at(set_with_me(), 2, 2, 0);
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(Duration::from_secs(30 * 60 - 1)));
        let (snap, _) = look(&r, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "not yet");
        assert!(snap.guard_watching);

        r.idle.set(Some(HOUR / 2));
        let (snap, _) = look(&r, 51);
        assert_eq!(snap.guard_rests_at_unix_ms, Some(t0 + GRACE_MS));
        assert!(
            snap.guard_night,
            "the night is asking, not the guard's six hours"
        );

        r.clock.advance(HOUR / 4);
        r.idle.set(Some(HOUR * 3 / 4));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)).last(),
            Some(&ended(Ended::Idle, t0 + GRACE_MS))
        );
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Off);
        assert!(snap.rises_on_touch);
    }

    /// Always, set for the night at two in the morning, the worker settled.
    fn set_for_the_night(r: &Rig) {
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(HOUR / 2));
        look(r, 50);
        r.clock.advance(HOUR / 4);
        r.idle.set(Some(HOUR * 3 / 4));
        r.engine.refresh();
        wait_for_guard(r, |g| g == GuardState::default());
        events_until(r, is_ended);
    }

    #[test]
    fn rises_at_the_next_touch_and_not_before() {
        let r = rig_at(set_with_me(), 2, 2, 0);
        set_for_the_night(&r);
        assert_eq!(
            r.engine.next_wake(),
            Some(r.clock.now() + policy::BATTERY_POLL),
            "it looks for a touch though the sun is down"
        );

        // Six hours on, nobody has touched anything.
        r.clock.advance(6 * HOUR);
        r.idle.set(Some(6 * HOUR + HOUR * 3 / 4));
        let (snap, _) = look(&r, 51);
        assert_eq!(snap.mode, Mode::Off);
        assert!(snap.rises_on_touch);

        // A key.
        r.clock.advance(Duration::from_secs(30));
        r.idle.set(Some(Duration::from_secs(2)));
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Indefinite);
        assert!(!snap.rises_on_touch);
    }

    #[test]
    fn a_wake_in_the_night_raises_nothing_when_the_idle_clock_stood_still() {
        let r = rig_at(set_with_me(), 2, 2, 0);
        set_for_the_night(&r);
        // The machine slept three hours and woke itself for twenty
        // seconds; its idle clock counted only the twenty seconds.
        r.clock.advance(Duration::from_secs(20));
        r.idle.set(Some(HOUR * 3 / 4 + Duration::from_secs(20)));
        let (snap, _) = look(&r, 51);
        assert_eq!(snap.mode, Mode::Off, "nobody came");
        assert!(snap.rises_on_touch);
    }

    #[test]
    fn switching_it_by_hand_cancels_the_rise() {
        let r = rig_at(set_with_me(), 2, 2, 0);
        set_for_the_night(&r);
        let snap = r.engine.set_plan(PlanRequest::Off);
        assert!(!snap.rises_on_touch);
        r.clock.advance(Duration::from_secs(30));
        r.idle.set(Some(Duration::from_secs(2)));
        let (snap, _) = look(&r, 51);
        assert_eq!(snap.mode, Mode::Off, "off by hand stays off");
    }

    #[test]
    fn outside_the_resting_hours_or_switched_off_it_leaves_always_to_the_guard() {
        let afternoon = rig_at(set_with_me(), 2, 14, 0);
        afternoon.engine.set_plan(PlanRequest::Indefinite);
        afternoon.idle.set(Some(2 * HOUR));
        let (snap, _) = look(&afternoon, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "an afternoon out");

        let unasked = rig_at(Settings::default(), 2, 2, 0);
        unasked.engine.set_plan(PlanRequest::Indefinite);
        unasked.idle.set(Some(2 * HOUR));
        let (snap, _) = look(&unasked, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "off by default");

        let mut closed = set_with_me();
        closed.almanac.enabled = false;
        let closed = rig_at(closed, 2, 2, 0);
        closed.engine.set_plan(PlanRequest::Indefinite);
        closed.idle.set(Some(2 * HOUR));
        let (snap, _) = look(&closed, 50);
        assert_eq!(snap.guard_rests_at_unix_ms, None, "no almanac, no night");
    }

    #[test]
    fn the_guards_own_hours_are_the_guards_question_and_raise_nothing() {
        let r = rig_at(set_with_me(), 2, 2, 0);
        r.engine.set_plan(PlanRequest::Indefinite);
        r.idle.set(Some(6 * HOUR));
        let (snap, _) = look(&r, 50);
        assert!(snap.guard_rests_at_unix_ms.is_some());
        assert!(!snap.guard_night, "six hours untouched is the guard's");
    }

    #[test]
    fn a_new_session_never_inherits_the_last_ones_question() {
        let r = rig(Settings::default());
        asked(&r);
        // Off and Always back to back, before the worker need look again.
        r.engine.set_plan(PlanRequest::Off);
        r.clock.advance(Duration::from_secs(1));
        let snap = r.engine.set_plan(PlanRequest::Indefinite);
        assert_eq!(snap.guard_rests_at_unix_ms, None);
    }

    #[test]
    fn the_worker_wakes_for_the_end_of_the_grace() {
        let r = rig(Settings::default());
        asked(&r);
        // Ten seconds before the grace runs out the poll (30 s) is later.
        r.clock.advance(Duration::from_secs(10 * 60 - 10));
        assert_eq!(
            r.engine.next_wake(),
            Some(r.clock.now() + Duration::from_secs(10))
        );
    }

    fn is_started(event: &EngineEvent) -> bool {
        matches!(event, EngineEvent::SessionStarted { .. })
    }

    fn is_ended(event: &EngineEvent) -> bool {
        matches!(event, EngineEvent::SessionEnded { .. })
    }

    /// The session events, in order, without the rest.
    fn sessions(events: &[EngineEvent]) -> Vec<EngineEvent> {
        events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    EngineEvent::SessionStarted { .. } | EngineEvent::SessionEnded { .. }
                )
            })
            .cloned()
            .collect()
    }

    fn started(mode: Mode, at_unix_ms: u64) -> EngineEvent {
        EngineEvent::SessionStarted {
            mode,
            paused: None,
            at_unix_ms,
        }
    }

    fn ended(ended: Ended, at_unix_ms: u64) -> EngineEvent {
        EngineEvent::SessionEnded { ended, at_unix_ms }
    }

    #[test]
    fn a_session_turned_off_by_the_user_starts_and_ends_once() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        // Changing the plan mid-session is the same stretch.
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        r.clock.advance(Duration::from_secs(60));
        r.engine.set_plan(PlanRequest::Off);
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)),
            vec![
                started(Mode::Indefinite, t0),
                ended(Ended::User, t0 + 60_000)
            ]
        );
    }

    #[test]
    fn a_timer_running_out_ends_the_session_as_timer() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::For { seconds: 10 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(11));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)),
            vec![started(Mode::Timed, t0), ended(Ended::Timer, t0 + 10_000)],
            "it ended when due, though the worker only looked at 11 s"
        );
    }

    #[test]
    fn a_schedule_window_closing_ends_the_session_as_schedule() {
        let r = rig_at(with_schedule(), 2, 8, 59);
        drain(&r);
        r.clock.advance(Duration::from_secs(60));
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(9 * 3600));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)),
            vec![
                started(Mode::Scheduled, local_ms(2, 9, 0)),
                ended(Ended::Schedule, local_ms(2, 18, 0))
            ]
        );
    }

    #[test]
    fn a_window_that_closed_while_asleep_ends_when_the_window_did() {
        let r = rig_at(with_schedule(), 2, 8, 59);
        drain(&r);
        r.clock.advance(Duration::from_secs(60));
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        // The lid closed at noon and opened at 8 the next morning: the first
        // step after that is long past the window's end.
        r.clock.advance(Duration::from_secs(23 * 3600));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)).last(),
            Some(&ended(Ended::Schedule, local_ms(2, 18, 0)))
        );
    }

    #[test]
    fn a_timer_that_ran_out_while_asleep_ends_at_its_deadline() {
        let r = rig(Settings::default());
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::For { seconds: 10 });
        wait_for_guard(&r, |g| g.system);
        r.clock.advance(Duration::from_secs(11 * 3600));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(
            sessions(&events_until(&r, is_ended)).last(),
            Some(&ended(Ended::Timer, t0 + 10_000))
        );
    }

    #[test]
    fn a_session_paused_for_the_battery_ends_as_battery() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let at = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Off);
        let events = events_until(&r, is_ended);
        assert_eq!(sessions(&events).last(), Some(&ended(Ended::Battery, at)));
    }

    #[test]
    fn quitting_ends_the_session_as_user_or_battery() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        let at = r.clock.unix_ms();
        r.engine.shutdown();
        assert_eq!(sessions(&drain(&r)).last(), Some(&ended(Ended::User, at)));

        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        r.engine.shutdown();
        assert_eq!(
            sessions(&drain(&r)).last(),
            Some(&ended(Ended::Battery, at))
        );
    }

    #[test]
    fn quitting_while_resting_reports_nothing() {
        let r = rig(Settings::default());
        drain(&r);
        r.engine.shutdown();
        assert_eq!(sessions(&drain(&r)), vec![]);
    }

    #[test]
    fn launching_inside_a_window_opens_a_session_for_the_log() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        assert_eq!(
            sessions(&events_until(&r, is_started)),
            vec![started(Mode::Scheduled, local_ms(2, 10, 0))]
        );
    }

    #[test]
    fn a_session_that_starts_unplugged_says_it_is_paused() {
        let r = rig(Settings::default());
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
            present: true,
            minutes_left: None,
        });
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Indefinite);
        let events = events_until(&r, is_started);
        assert_eq!(
            sessions(&events),
            vec![EngineEvent::SessionStarted {
                mode: Mode::Indefinite,
                paused: Some(PauseReason::LowBattery),
                at_unix_ms: t0,
            }]
        );
    }

    #[test]
    fn a_manual_session_ending_inside_a_window_keeps_the_stretch_open() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        r.engine.set_plan(PlanRequest::For { seconds: 60 });
        r.clock.advance(Duration::from_secs(61));
        r.engine.refresh();
        let mut events = events_until(&r, |e| matches!(e, EngineEvent::TimerEnded { .. }));
        // Shutdown joins the worker, so everything before it has arrived;
        // the only end is the one quitting reports.
        r.engine.shutdown();
        events.extend(drain(&r));
        assert_eq!(
            sessions(&events),
            vec![
                started(Mode::Scheduled, local_ms(2, 10, 0)),
                ended(Ended::User, local_ms(2, 10, 1) + 1_000)
            ],
            "the window still holds, so the sun never set until quitting"
        );
    }

    #[test]
    fn shutdown_releases_guards_and_is_idempotent() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        r.engine.shutdown();
        assert_eq!(r.guard.current(), GuardState::default());
        r.engine.shutdown();
    }

    /// Unix ms for a local wall time on Monday 2 March 2026 (`day` 2) or a
    /// following day, in whatever zone the test machine uses.
    fn local_ms(day: u32, h: u32, m: u32) -> u64 {
        let t = Local
            .with_ymd_and_hms(2026, 3, day, h, m, 0)
            .single()
            .unwrap();
        u64::try_from(t.timestamp_millis()).unwrap()
    }

    /// Every day, 09:00 until 18:00.
    fn with_schedule() -> Settings {
        Settings {
            schedule: Schedule {
                enabled: true,
                days: [true; 7],
                start_minutes: 9 * 60,
                end_minutes: 18 * 60,
            },
            ..Settings::default()
        }
    }

    fn rig_at(settings: Settings, day: u32, h: u32, m: u32) -> Rig {
        rig_with_clock(settings, ManualClock::at(local_ms(day, h, m)))
    }

    fn has_schedule_started(events: &[EngineEvent]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, EngineEvent::ScheduleStarted { .. }))
    }

    #[test]
    fn window_opening_holds_and_closing_releases() {
        let r = rig_at(with_schedule(), 2, 8, 59);
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Off);
        assert!(snap.schedule.enabled);
        assert!(!snap.schedule.in_window);
        assert_eq!(snap.schedule.next_start_unix_ms, Some(local_ms(2, 9, 0)));
        drain(&r);
        assert_eq!(r.guard.current(), GuardState::default());

        r.clock.advance(Duration::from_secs(60));
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Scheduled);
        assert!(snap.is_guarding());
        assert_eq!(snap.started_at_unix_ms, Some(local_ms(2, 9, 0)));
        assert_eq!(snap.ends_at_unix_ms, Some(local_ms(2, 18, 0)));
        assert_eq!(snap.plan_secs, Some(9 * 3600));
        assert_eq!(snap.remaining_secs, Some(9 * 3600));
        assert!(drain(&r).contains(&EngineEvent::ScheduleStarted {
            ends_at_unix_ms: local_ms(2, 18, 0)
        }));

        r.clock.advance(Duration::from_secs(9 * 3600));
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Off);
        assert_eq!(snap.schedule.next_start_unix_ms, Some(local_ms(3, 9, 0)));
        assert!(drain(&r).contains(&EngineEvent::ScheduleEnded {
            next_start_unix_ms: Some(local_ms(3, 9, 0))
        }));
    }

    #[test]
    fn launching_inside_a_window_starts_awake_without_an_event() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        assert_eq!(r.engine.snapshot().mode, Mode::Scheduled);
        assert!(!has_schedule_started(&drain(&r)));
    }

    #[test]
    fn manual_session_outlasts_the_window() {
        let r = rig_at(with_schedule(), 2, 17, 0);
        wait_for_guard(&r, |g| g.system);
        let snap = r.engine.set_plan(PlanRequest::For { seconds: 2 * 3600 });
        assert_eq!(snap.mode, Mode::Timed);
        r.clock.advance(Duration::from_secs(61 * 60));
        r.engine.refresh();
        let events = drain(&r);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EngineEvent::TimerEnded { .. })),
            "{events:?}"
        );
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Timed);
        assert!(!snap.schedule.in_window);
        assert!(r.guard.current().system);
        assert_eq!(r.guard.history().len(), 1, "never let go in between");
    }

    #[test]
    fn manual_session_ending_inside_a_window_returns_to_the_schedule() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        r.engine.set_plan(PlanRequest::For { seconds: 60 });
        r.clock.advance(Duration::from_secs(61));
        r.engine.refresh();
        let events = drain(&r);
        assert!(
            events.contains(&EngineEvent::TimerEnded { plan_secs: 60 }),
            "{events:?}"
        );
        assert_eq!(r.engine.snapshot().mode, Mode::Scheduled);
        assert!(r.guard.current().system);
        assert_eq!(r.guard.history().len(), 1, "never let go in between");
    }

    #[test]
    fn toggling_skips_the_window_and_toggling_again_resumes() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);

        let snap = r.engine.toggle();
        assert_eq!(snap.mode, Mode::Off);
        assert!(snap.schedule.skipped);
        assert!(snap.schedule.in_window);
        wait_for_guard(&r, |g| g == GuardState::default());

        let snap = r.engine.toggle();
        assert_eq!(snap.mode, Mode::Scheduled);
        assert!(!snap.schedule.skipped);
        wait_for_guard(&r, |g| g.system);
    }

    #[test]
    fn skip_and_unskip_commands() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        assert!(r.engine.skip_today().schedule.skipped);
        wait_for_guard(&r, |g| g == GuardState::default());
        assert_eq!(r.engine.unskip().mode, Mode::Scheduled);
        wait_for_guard(&r, |g| g.system);
    }

    #[test]
    fn skip_lasts_only_until_the_window_closes() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        r.engine.skip_today();
        wait_for_guard(&r, |g| g == GuardState::default());
        r.clock.advance(Duration::from_secs(8 * 3600));
        r.engine.refresh();
        drain(&r);
        assert!(!r.engine.snapshot().schedule.skipped);
        // The next morning the schedule holds again.
        r.clock.advance(Duration::from_secs(15 * 3600));
        r.engine.refresh();
        wait_for_guard(&r, |g| g.system);
        assert_eq!(r.engine.snapshot().mode, Mode::Scheduled);
    }

    #[test]
    fn skip_outside_a_window_does_nothing() {
        let r = rig_at(with_schedule(), 2, 7, 0);
        assert!(!r.engine.skip_today().schedule.skipped);
        assert_eq!(r.engine.toggle().mode, Mode::Indefinite);
    }

    #[test]
    fn worker_wakes_exactly_at_the_window_boundary() {
        let r = rig_at(with_schedule(), 2, 8, 59);
        assert_eq!(
            r.engine.next_wake(),
            Some(r.clock.now() + Duration::from_secs(60))
        );
        // Long before the window, sleep is capped by the re-check.
        let early = rig_at(with_schedule(), 2, 6, 0);
        assert_eq!(
            early.engine.next_wake(),
            Some(early.clock.now() + policy::SCHEDULE_RECHECK)
        );
        // Inside the window the battery poll comes first, until the end is
        // closer than a poll.
        let late = rig_at(with_schedule(), 2, 17, 59);
        assert_eq!(
            late.engine.next_wake(),
            Some(late.clock.now() + policy::BATTERY_POLL)
        );
        late.clock.advance(Duration::from_secs(50));
        assert_eq!(
            late.engine.next_wake(),
            Some(late.clock.now() + Duration::from_secs(10))
        );
    }

    #[test]
    fn schedule_off_never_wakes_the_worker() {
        let r = rig(Settings::default());
        assert_eq!(r.engine.next_wake(), None);
    }

    #[test]
    fn battery_pauses_a_scheduled_window_too() {
        let r = rig_at(with_schedule(), 2, 10, 0);
        wait_for_guard(&r, |g| g.system);
        drain(&r);
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(5),
            present: true,
            minutes_left: None,
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let snap = r.engine.snapshot();
        assert_eq!(snap.mode, Mode::Scheduled);
        assert_eq!(snap.paused, Some(PauseReason::LowBattery));
        assert!(drain(&r).contains(&EngineEvent::Paused(
            PauseReason::LowBattery,
            BatteryState {
                on_ac: false,
                percent: Some(5),
                present: true,
                minutes_left: None,
            }
        )));
    }

    #[test]
    fn command_sent_while_events_are_delivered_is_not_lost() {
        let guard = FakePowerGuard::default();
        let engine = Arc::new(Engine::start(
            Platform {
                guard: Box::new(guard.clone()),
                battery: Box::new(FakeBattery::new(BatteryState {
                    on_ac: true,
                    percent: Some(90),
                    present: true,
                    minutes_left: None,
                })),
                idle: Box::new(FakeIdle::default()),
                activity: Box::new(FakeActivity::default()),
                clock: Arc::new(ManualClock::new()),
            },
            Settings::default(),
            "test",
        ));
        // A listener that sends a command runs exactly in the window where
        // the worker has released the lock but is not yet waiting.
        let weak = Arc::downgrade(&engine);
        let sent = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&sent);
        engine.subscribe(Box::new(move |_| {
            if !flag.swap(true, std::sync::atomic::Ordering::SeqCst)
                && let Some(engine) = weak.upgrade()
            {
                engine.set_plan(PlanRequest::Indefinite);
            }
        }));
        engine.refresh();
        let start = Instant::now();
        while !guard.current().system {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "the command was never applied"
            );
            thread::sleep(Duration::from_millis(5));
        }
        engine.shutdown();
    }

    // Stay awake while…

    fn watching(f: impl FnOnce(&mut crate::core::WatchSettings)) -> Settings {
        // The guard is out of the way: these tests are about conditions.
        let mut settings = Settings {
            overnight_guard_hours: None,
            ..Settings::default()
        };
        f(&mut settings.watch);
        settings
    }

    fn endings(events: &[EngineEvent]) -> Vec<Ended> {
        events
            .iter()
            .filter_map(|e| match e {
                EngineEvent::SessionEnded { ended, .. } => Some(*ended),
                _ => None,
            })
            .collect()
    }

    /// Waits for the stretch to end, and says why it did. Events come after
    /// the step's snapshot, so `look` has returned before they arrive.
    fn next_ending(r: &Rig) -> Ended {
        let events = events_until(r, |e| matches!(e, EngineEvent::SessionEnded { .. }));
        endings(&events)[0]
    }

    #[test]
    fn a_running_app_raises_the_sun_and_its_exit_lowers_it() {
        let r = rig(watching(|w| w.apps = vec!["Xcode".into()]));
        r.activity
            .set(|a| a.processes = vec!["zsh".into(), "Xcode".into()]);
        let (snap, _) = look(&r, 10);
        assert_eq!(snap.mode, Mode::Watching);
        assert_eq!(
            snap.watch.reason,
            Some(crate::core::WatchReason::App {
                name: "Xcode".into()
            })
        );
        assert!(snap.guards.system);
        let started = events_until(&r, |e| matches!(e, EngineEvent::SessionStarted { .. }));
        assert!(matches!(
            started.last(),
            Some(EngineEvent::SessionStarted {
                mode: Mode::Watching,
                ..
            })
        ));

        r.activity.set(|a| a.processes = vec!["zsh".into()]);
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.mode, Mode::Off);
        assert!(!snap.guards.system);
        assert_eq!(next_ending(&r), Ended::Watch);
    }

    #[test]
    fn a_manual_session_outranks_a_condition() {
        let r = rig(watching(|w| w.apps = vec!["Xcode".into()]));
        r.activity.set(|a| a.processes = vec!["Xcode".into()]);
        look(&r, 10);
        r.engine.set_plan(PlanRequest::For { seconds: 600 });
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.mode, Mode::Timed);
        r.engine.set_plan(PlanRequest::Off);
        // Everything the Off step sent arrives before this look's snapshot.
        let (snap, events) = look(&r, 12);
        assert_eq!(snap.mode, Mode::Watching, "the condition takes over");
        assert_eq!(endings(&events), Vec::<Ended>::new(), "the sun never set");
    }

    #[test]
    fn the_sun_skips_a_condition_until_it_clears() {
        let r = rig(watching(|w| w.apps = vec!["Xcode".into()]));
        r.activity.set(|a| a.processes = vec!["Xcode".into()]);
        look(&r, 10);
        r.engine.toggle();
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.mode, Mode::Off);
        assert!(snap.watch.skipped);
        assert_eq!(next_ending(&r), Ended::User);

        // Undo by clicking again.
        r.engine.toggle();
        let (snap, _) = look(&r, 12);
        assert_eq!(snap.mode, Mode::Watching);
        r.engine.toggle();
        look(&r, 13);

        // The app quits: the skip is spent, and its next start counts.
        r.activity.set(|a| a.processes.clear());
        let (snap, _) = look(&r, 14);
        assert!(!snap.watch.skipped);
        r.activity.set(|a| a.processes = vec!["Xcode".into()]);
        let (snap, _) = look(&r, 15);
        assert_eq!(snap.mode, Mode::Watching);
    }

    #[test]
    fn a_process_from_the_command_line_holds_until_it_exits() {
        let r = rig(watching(|_| {}));
        r.activity.set(|a| a.alive = vec![4120]);
        r.engine.watch_pid(4120);
        let (snap, _) = look(&r, 10);
        assert_eq!(snap.mode, Mode::Watching);
        assert_eq!(
            snap.watch.reason,
            Some(crate::core::WatchReason::Process { pid: 4120 })
        );
        r.activity.set(|a| a.alive.clear());
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.mode, Mode::Off);
        assert_eq!(next_ending(&r), Ended::Watch);
        assert!(r.engine.shared.state.lock().pids.is_empty(), "forgotten");
    }

    #[test]
    fn a_transfer_holds_and_lingers_on_the_engines_clock() {
        let r = rig(watching(|w| w.transfers = true));
        r.activity.set(|a| a.transferred = Some(0));
        look(&r, 10);
        r.clock.advance(Duration::from_secs(30));
        r.activity.set(|a| a.transferred = Some(30_000_000));
        let (snap, _) = look(&r, 11);
        assert_eq!(snap.mode, Mode::Watching);
        assert_eq!(snap.watch.transfer_bps, Some(1_000_000));
        r.clock.advance(Duration::from_secs(30));
        let (snap, _) = look(&r, 12);
        assert_eq!(snap.mode, Mode::Watching, "lingers while quiet");
        r.clock.advance(Duration::from_secs(121));
        let (snap, _) = look(&r, 13);
        assert_eq!(snap.mode, Mode::Off);
        assert_eq!(next_ending(&r), Ended::Watch);
    }

    #[test]
    fn nothing_is_read_and_nothing_wakes_with_every_condition_off() {
        let r = rig(watching(|_| {}));
        look(&r, 10);
        r.engine.set_plan(PlanRequest::Indefinite);
        look(&r, 11);
        r.engine.set_plan(PlanRequest::Off);
        look(&r, 12);
        assert_eq!(r.activity.reads(), 0);
        assert_eq!(r.engine.next_wake(), None, "resting, nothing to look at");
    }

    #[test]
    fn a_watched_condition_looks_again_every_poll_while_resting() {
        let r = rig(watching(|w| w.busy = true));
        look(&r, 10);
        let now = r.clock.now();
        assert_eq!(r.engine.next_wake(), Some(now + policy::WATCH_POLL));
        assert!(r.activity.reads() > 0);
    }
}
