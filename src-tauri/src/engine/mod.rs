//! The engine owns the session and is the only thing that touches the OS.
//!
//! It runs one background thread that sleeps on a condition variable until
//! either a command arrives or a deadline (timer expiry, battery poll) passes.
//! There is no periodic polling while idle, which is what lets the app sit in
//! the tray at a rounding error of CPU.

pub mod policy;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::core::{
    BatteryState, GuardState, PauseReason, PlanRequest, Session, SessionEvent, Settings, Snapshot,
};
use crate::platform::{BatteryProbe, Clock, PowerGuard};

/// Something the app shell may want to react to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// State changed; here is the new read model.
    Snapshot(Snapshot),
    /// A timed session finished on its own.
    TimerEnded,
    /// An active session stopped guarding because of power conditions.
    Paused(PauseReason),
    /// A paused session is guarding again.
    Resumed,
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
        let Platform {
            guard,
            battery,
            clock,
        } = platform;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                session: Session::off(),
                settings,
                battery: battery.read(),
                guards: GuardState::default(),
                paused: None,
                shutdown: false,
            }),
            wake: Condvar::new(),
            clock,
            listeners: Mutex::new(Vec::new()),
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

    /// Flips between off and the default "on" plan from settings.
    pub fn toggle(&self) -> Snapshot {
        let now = self.shared.clock.now();
        self.mutate(|st| {
            let on = default_plan(&st.settings);
            st.session.toggle(on, now);
        })
    }

    /// Replaces settings (sanitized here) and re-evaluates the guards.
    pub fn update_settings(&self, settings: Settings) -> Snapshot {
        self.mutate(|st| st.settings = settings.sanitized())
    }

    /// Asks the worker to re-read the battery and re-apply now. Used when the
    /// window opens so the UI shows fresh power data without constant polling.
    pub fn refresh(&self) {
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

    fn mutate(&self, f: impl FnOnce(&mut State)) -> Snapshot {
        let mut st = self.shared.state.lock();
        f(&mut st);
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

fn run_loop(shared: &Arc<Shared>, mut guard: Box<dyn PowerGuard>, battery: &dyn BatteryProbe) {
    let mut st = shared.state.lock();
    loop {
        if st.shutdown {
            if let Err(err) = guard.apply(GuardState::default()) {
                log::warn!("could not release power request on shutdown: {err}");
            }
            return;
        }
        let now = shared.clock.now();
        let events = step(&mut st, guard.as_mut(), battery, now, shared);
        let deadline = policy::next_deadline(&st.session, now);

        // Deliver events without holding the lock: listeners may call back.
        MutexGuard::unlocked(&mut st, || {
            let listeners = shared.listeners.lock();
            for event in &events {
                for listener in listeners.iter() {
                    listener(event);
                }
            }
        });

        match deadline {
            Some(d) => {
                shared.wake.wait_until(&mut st, d);
            }
            None => shared.wake.wait(&mut st),
        }
    }
}

/// One evaluation of the state machine. Separated from the thread so it can be
/// tested deterministically with fakes.
fn step(
    st: &mut State,
    guard: &mut dyn PowerGuard,
    battery: &dyn BatteryProbe,
    now: Instant,
    shared: &Shared,
) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    let was_active = st.session.is_active();
    if st.session.tick(now) == Some(SessionEvent::Expired) {
        events.push(EngineEvent::TimerEnded);
    }
    st.battery = battery.read();

    let (desired, paused) = policy::decide(st.session.is_active(), &st.settings, st.battery);
    let previously_paused = st.paused;
    match guard.apply(desired) {
        Ok(()) => st.paused = paused,
        Err(err) => {
            log::error!("power request failed: {err}");
            st.paused = st
                .session
                .is_active()
                .then_some(PauseReason::PowerRequestFailed);
        }
    }
    st.guards = guard.current();

    if st.session.is_active() && was_active {
        match (previously_paused, st.paused) {
            (None, Some(reason)) => events.push(EngineEvent::Paused(reason)),
            (Some(_), None) => events.push(EngineEvent::Resumed),
            _ => {}
        }
    }
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
    Snapshot {
        mode: st.session.mode(),
        started_at_unix_ms: st.session.started_at().map(to_unix),
        ends_at_unix_ms: st.session.deadline().map(to_unix),
        remaining_secs: st.session.remaining(now).map(|d| millis(d).div_ceil(1000)),
        plan_secs: st.session.plan_duration().map(|d| d.as_secs()),
        guards: st.guards,
        paused: st.paused,
        battery: st.battery,
        settings: st.settings.clone(),
        app_version: shared.app_version.clone(),
    }
}

fn millis(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::core::Mode;
    use crate::platform::fakes::{FakeBattery, FakePowerGuard, ManualClock};

    struct Rig {
        engine: Engine,
        guard: FakePowerGuard,
        battery: FakeBattery,
        clock: ManualClock,
        events: mpsc::Receiver<EngineEvent>,
    }

    fn rig(settings: Settings) -> Rig {
        let guard = FakePowerGuard::default();
        let battery = FakeBattery::new(BatteryState {
            on_ac: true,
            percent: Some(90),
        });
        let clock = ManualClock::new();
        let engine = Engine::start(
            Platform {
                guard: Box::new(guard.clone()),
                battery: Box::new(battery.clone()),
                clock: Arc::new(clock.clone()),
            },
            settings,
            "test",
        );
        let (tx, rx) = mpsc::channel();
        engine.subscribe(Box::new(move |e| {
            let _ = tx.send(e.clone());
        }));
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
        assert!(events.contains(&EngineEvent::TimerEnded), "{events:?}");
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
        assert!(drain(&r).contains(&EngineEvent::Paused(PauseReason::LowBattery)));

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

    #[test]
    fn shutdown_releases_guards_and_is_idempotent() {
        let r = rig(Settings::default());
        r.engine.set_plan(PlanRequest::Indefinite);
        wait_for_guard(&r, |g| g.system);
        r.engine.shutdown();
        assert_eq!(r.guard.current(), GuardState::default());
        r.engine.shutdown();
    }
}
