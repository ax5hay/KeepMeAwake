//! The narrow slice of the operating system the engine depends on.
//!
//! Everything is a trait so the engine can be tested with fakes; the real
//! implementations live in `windows` (Win32) and `macos` (IOKit and
//! CoreGraphics), each compiled only on its own OS.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::core::watch::CpuTicks;
use crate::core::{BatteryState, GuardState};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

/// Errors from the OS layer. Kept small on purpose; the engine only needs to
/// know that a call failed and what to log.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// A Win32 or IOKit call failed. Carries the formatted OS error.
    #[error("{call} failed: {detail}")]
    Os {
        /// Name of the API that failed, for logs.
        call: &'static str,
        /// Human readable detail: the Win32 message or the `IOReturn` in hex.
        detail: String,
    },
    /// This platform cannot provide the capability.
    #[error("unsupported on this platform")]
    Unsupported,
}

/// Holds and releases the "keep awake" requests. Implementations must be
/// idempotent: applying the same state twice is a no-op.
pub trait PowerGuard: Send {
    /// Make the OS requests match `want`. On error the implementation should
    /// leave itself in a consistent state and report what it actually holds
    /// via [`PowerGuard::current`].
    fn apply(&mut self, want: GuardState) -> Result<(), PlatformError>;
    /// What is currently requested from the OS.
    fn current(&self) -> GuardState;
}

/// Reads mains/battery status.
pub trait BatteryProbe: Send {
    /// Current power source and charge. Must be cheap; it is polled.
    fn read(&self) -> BatteryState;
}

/// Whether the taskbar is light, so the tray can pick icons that show on it.
pub trait TaskbarTheme: Send + Sync {
    /// `true` for a light taskbar.
    fn is_light(&self) -> bool;
}

/// For platforms without a notion of taskbar theme: always dark, which is
/// what the white tray icons were drawn for.
#[derive(Debug, Default, Clone, Copy)]
pub struct DarkTaskbar;

impl TaskbarTheme for DarkTaskbar {
    fn is_light(&self) -> bool {
        false
    }
}

/// How long since the last keyboard, mouse or trackpad input, to any
/// application. Read-only: it only ever asks, and never sends input
/// (ADR 0002).
pub trait IdleProbe: Send {
    /// Time since the last input, or `None` when the OS cannot say. The
    /// engine never acts on `None`: unknown means someone may be there.
    fn idle(&self) -> Option<Duration>;
}

/// For platforms that cannot report idle time: always unknown, so nothing
/// that depends on it ever happens.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoIdle;

impl IdleProbe for NoIdle {
    fn idle(&self) -> Option<Duration> {
        None
    }
}

/// What the machine is doing, for stay awake while…. Read-only: it lists
/// and counts, and never touches another process.
pub trait ActivityProbe: Send {
    /// The names of the processes running now, as the OS gives them
    /// (`cargo.exe`, `Xcode`). Duplicates are fine.
    fn processes(&self) -> Vec<String>;
    /// The process with this id is still running.
    fn alive(&self, pid: u32) -> bool;
    /// Bytes received and sent since boot over real interfaces (loopback
    /// and virtual adapters left out), or `None` when unknown.
    fn transferred(&self) -> Option<u64>;
    /// Processor time since boot, or `None` when unknown.
    fn cpu(&self) -> Option<CpuTicks>;
    /// Another app is full screen: a video, a game, a presentation.
    fn full_screen(&self) -> bool;
    /// A display other than the built-in one is connected.
    fn external_display(&self) -> bool;
}

/// For platforms that cannot say: nothing is running, nothing moves, so
/// no condition ever holds.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoActivity;

impl ActivityProbe for NoActivity {
    fn processes(&self) -> Vec<String> {
        Vec::new()
    }
    fn alive(&self, _pid: u32) -> bool {
        false
    }
    fn transferred(&self) -> Option<u64> {
        None
    }
    fn cpu(&self) -> Option<CpuTicks> {
        None
    }
    fn full_screen(&self) -> bool {
        false
    }
    fn external_display(&self) -> bool {
        false
    }
}

/// Monotonic and wall clocks, injectable for tests.
pub trait Clock: Send {
    /// Monotonic now.
    fn now(&self) -> Instant;
    /// Wall clock now, Unix epoch milliseconds.
    fn unix_ms(&self) -> u64;
}

/// The production clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

/// A guard that holds nothing. Used where the real guard is unavailable so
/// the rest of the app still works (and reports `PowerRequestFailed`).
#[derive(Debug, Default)]
pub struct NullPowerGuard;

impl PowerGuard for NullPowerGuard {
    fn apply(&mut self, want: GuardState) -> Result<(), PlatformError> {
        if want == GuardState::default() {
            Ok(())
        } else {
            Err(PlatformError::Unsupported)
        }
    }

    fn current(&self) -> GuardState {
        GuardState::default()
    }
}

/// A probe for machines without a battery: always on mains.
#[derive(Debug, Default)]
pub struct AlwaysOnMains;

impl BatteryProbe for AlwaysOnMains {
    fn read(&self) -> BatteryState {
        BatteryState {
            on_ac: true,
            percent: None,
            present: false,
            minutes_left: None,
        }
    }
}

/// Test doubles shared by engine and app tests.
#[cfg(test)]
#[allow(missing_docs, clippy::new_without_default)]
pub mod fakes {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{
        ActivityProbe, BatteryProbe, BatteryState, Clock, CpuTicks, GuardState, IdleProbe,
        PlatformError, PowerGuard, TaskbarTheme,
    };

    /// What the fake machine is doing; the test sets it.
    #[derive(Debug, Clone, Default)]
    pub struct Activity {
        pub processes: Vec<String>,
        pub alive: Vec<u32>,
        pub transferred: Option<u64>,
        pub cpu: Option<CpuTicks>,
        pub full_screen: bool,
        pub display: bool,
        /// How many times the machine was read, any reading.
        pub reads: u32,
    }

    /// An activity probe the test controls, counting its reads.
    #[derive(Debug, Clone, Default)]
    pub struct FakeActivity(pub Arc<Mutex<Activity>>);

    impl FakeActivity {
        pub fn set(&self, f: impl FnOnce(&mut Activity)) {
            f(&mut self.0.lock().unwrap());
        }

        pub fn reads(&self) -> u32 {
            self.0.lock().unwrap().reads
        }

        fn read<T>(&self, f: impl FnOnce(&Activity) -> T) -> T {
            let mut a = self.0.lock().unwrap();
            a.reads += 1;
            f(&a)
        }
    }

    impl ActivityProbe for FakeActivity {
        fn processes(&self) -> Vec<String> {
            self.read(|a| a.processes.clone())
        }
        fn alive(&self, pid: u32) -> bool {
            self.read(|a| a.alive.contains(&pid))
        }
        fn transferred(&self) -> Option<u64> {
            self.read(|a| a.transferred)
        }
        fn cpu(&self) -> Option<CpuTicks> {
            self.read(|a| a.cpu)
        }
        fn full_screen(&self) -> bool {
            self.read(|a| a.full_screen)
        }
        fn external_display(&self) -> bool {
            self.read(|a| a.display)
        }
    }

    /// An idle reading the test sets; starts unknown.
    #[derive(Debug, Clone, Default)]
    pub struct FakeIdle(pub Arc<Mutex<Option<Duration>>>);

    impl FakeIdle {
        pub fn set(&self, idle: Option<Duration>) {
            *self.0.lock().unwrap() = idle;
        }
    }

    impl IdleProbe for FakeIdle {
        fn idle(&self) -> Option<Duration> {
            *self.0.lock().unwrap()
        }
    }

    /// A taskbar whose theme the test sets.
    #[derive(Debug, Clone, Default)]
    pub struct FakeTaskbar(pub Arc<Mutex<bool>>);

    impl FakeTaskbar {
        pub fn set_light(&self, light: bool) {
            *self.0.lock().unwrap() = light;
        }
    }

    impl TaskbarTheme for FakeTaskbar {
        fn is_light(&self) -> bool {
            *self.0.lock().unwrap()
        }
    }

    /// Records every state applied and can be told to fail.
    #[derive(Debug, Default, Clone)]
    pub struct FakePowerGuard {
        inner: Arc<Mutex<FakeGuardInner>>,
    }

    #[derive(Debug, Default)]
    struct FakeGuardInner {
        current: GuardState,
        history: Vec<GuardState>,
        fail: bool,
    }

    impl FakePowerGuard {
        /// Every distinct state that was applied, in order.
        pub fn history(&self) -> Vec<GuardState> {
            self.inner.lock().unwrap().history.clone()
        }

        /// Make subsequent `apply` calls fail.
        pub fn set_failing(&self, fail: bool) {
            self.inner.lock().unwrap().fail = fail;
        }
    }

    impl PowerGuard for FakePowerGuard {
        fn apply(&mut self, want: GuardState) -> Result<(), PlatformError> {
            let mut inner = self.inner.lock().unwrap();
            if inner.fail {
                inner.current = GuardState::default();
                return Err(PlatformError::Os {
                    call: "fake",
                    detail: "forced failure".into(),
                });
            }
            if inner.current != want {
                inner.history.push(want);
                inner.current = want;
            }
            Ok(())
        }

        fn current(&self) -> GuardState {
            self.inner.lock().unwrap().current
        }
    }

    /// A battery whose readings the test controls.
    #[derive(Debug, Clone)]
    pub struct FakeBattery(pub Arc<Mutex<BatteryState>>);

    impl FakeBattery {
        pub fn new(state: BatteryState) -> Self {
            Self(Arc::new(Mutex::new(state)))
        }

        pub fn set(&self, state: BatteryState) {
            *self.0.lock().unwrap() = state;
        }
    }

    impl BatteryProbe for FakeBattery {
        fn read(&self) -> BatteryState {
            *self.0.lock().unwrap()
        }
    }

    /// A clock that only moves when told to.
    #[derive(Debug, Clone)]
    pub struct ManualClock {
        inner: Arc<Mutex<(Instant, u64)>>,
    }

    impl ManualClock {
        pub fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new((Instant::now(), 1_700_000_000_000))),
            }
        }

        /// A clock whose wall time starts at `unix_ms`.
        pub fn at(unix_ms: u64) -> Self {
            Self {
                inner: Arc::new(Mutex::new((Instant::now(), unix_ms))),
            }
        }

        pub fn advance(&self, by: Duration) {
            let mut g = self.inner.lock().unwrap();
            g.0 += by;
            g.1 += u64::try_from(by.as_millis()).unwrap_or(0);
        }
    }

    impl Clock for ManualClock {
        fn now(&self) -> Instant {
            self.inner.lock().unwrap().0
        }

        fn unix_ms(&self) -> u64 {
            self.inner.lock().unwrap().1
        }
    }
}
