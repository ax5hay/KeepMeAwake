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

use chrono::{DateTime, Local, TimeZone};
use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::core::history::Ended;
use crate::core::schedule;
use crate::core::{
    BatteryState, GuardState, Mode, PauseReason, PlanRequest, Schedule, ScheduleState, Session,
    SessionEvent, Settings, Snapshot,
};
use crate::platform::{BatteryProbe, Clock, PowerGuard};

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
    /// The sun set: nothing wants the laptop awake any more, or the app is
    /// quitting while something did.
    SessionEnded {
        /// Why.
        ended: Ended,
        /// When, Unix ms.
        at_unix_ms: u64,
    },
}

/// Receives engine events. Called off the engine lock, so it may call back
/// into the engine.
pub type Listener = Box<dyn Fn(&EngineEvent) + Send + Sync>;

/// OS capabilities handed to the engine at start.
pub struct Platform {
    /// Holds the wake requests.
    pub guard: Box<dyn PowerGuard>,
    /// Reads battery state.
    pub battery: Box<dyn BatteryProbe>,
    /// Time source.
    pub clock: Arc<dyn Clock + Sync>,
}

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
    shutdown: bool,
}

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
                .spawn(move || run_loop(&shared, guard, battery.as_ref()))
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
            if st.session.is_active() {
                st.session.apply(PlanRequest::Off, now);
                return;
            }
            let view = ScheduleView::at(&st.settings.schedule, st.skip_until, unix_now);
            match view.window {
                Some((_, end)) if !view.skipped => st.skip_until = Some(end),
                Some(_) => st.skip_until = None,
                None => {
                    st.session.apply(default_plan(&st.settings), now);
                }
            }
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
        f(&mut st);
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

fn run_loop(shared: &Arc<Shared>, mut guard: Box<dyn PowerGuard>, battery: &dyn BatteryProbe) {
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
        let events = step(&mut st, guard.as_mut(), battery, now, unix_now, shared);
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
    policy::next_deadline(&st.session, st.phase == Phase::Holding, boundary, now)
}

/// One evaluation of the state machine. Separated from the thread so it can be
/// tested deterministically with fakes.
fn step(
    st: &mut State,
    guard: &mut dyn PowerGuard,
    battery: &dyn BatteryProbe,
    now: Instant,
    unix_now: u64,
    shared: &Shared,
) -> Vec<EngineEvent> {
    let mut events = Vec::new();
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
    let active = st.session.is_active() || st.phase == Phase::Holding;

    st.battery = battery.read();
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
            } else {
                Mode::Scheduled
            },
            paused: st.paused,
            at_unix_ms: unix_now,
        }),
        (true, false) => {
            let cause = if expired {
                Ended::Timer
            } else if window_closed {
                Ended::Schedule
            } else {
                Ended::User
            };
            events.push(EngineEvent::SessionEnded {
                ended: ended_by(previously_paused, cause),
                at_unix_ms: unix_now,
            });
        }
        _ => {}
    }
    st.awake = active;
    events.insert(0, EngineEvent::Snapshot(build_snapshot(st, shared)));
    events
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
    use crate::platform::fakes::{FakeBattery, FakePowerGuard, ManualClock};

    struct Rig {
        engine: Engine,
        guard: FakePowerGuard,
        battery: FakeBattery,
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
        });
        let (tx, rx) = mpsc::channel();
        let engine = Engine::start_with(
            Platform {
                guard: Box::new(guard.clone()),
                battery: Box::new(battery.clone()),
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
            }
        )));

        r.battery.set(BatteryState {
            on_ac: true,
            percent: Some(10),
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
            sessions(&drain(&r)),
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
            sessions(&drain(&r)),
            vec![started(Mode::Timed, t0), ended(Ended::Timer, t0 + 11_000)]
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
            sessions(&drain(&r)),
            vec![
                started(Mode::Scheduled, local_ms(2, 9, 0)),
                ended(Ended::Schedule, local_ms(2, 18, 0))
            ]
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
        });
        r.engine.refresh();
        wait_for_guard(&r, |g| g == GuardState::default());
        let at = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Off);
        let events = drain(&r);
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
            sessions(&drain(&r)),
            vec![started(Mode::Scheduled, local_ms(2, 10, 0))]
        );
    }

    #[test]
    fn a_session_that_starts_unplugged_says_it_is_paused() {
        let r = rig(Settings::default());
        r.battery.set(BatteryState {
            on_ac: false,
            percent: Some(10),
        });
        let t0 = r.clock.unix_ms();
        r.engine.set_plan(PlanRequest::Indefinite);
        let events = drain(&r);
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
        let events = drain(&r);
        assert!(
            events.contains(&EngineEvent::TimerEnded { plan_secs: 60 }),
            "{events:?}"
        );
        assert_eq!(
            sessions(&events),
            vec![started(Mode::Scheduled, local_ms(2, 10, 0))],
            "the window still holds, so the sun never set"
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
                })),
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
}
