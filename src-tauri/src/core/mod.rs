//! Pure domain logic. Nothing in this module performs I/O or talks to the OS.

pub mod ai;
pub mod almanac;
pub mod history;
pub mod placement;
pub mod plan;
pub mod schedule;
pub mod session;
pub mod settings;
pub mod snapshot;
pub mod watch;

pub use plan::PlanRequest;
pub use session::{Mode, Session, SessionEvent};
pub use settings::{AiSettings, AlmanacSettings, Provider, Schedule, Settings};
pub use snapshot::{BatteryState, GuardState, PauseReason, ScheduleState, Snapshot};
pub use watch::{WatchReason, WatchSettings, WatchState};
