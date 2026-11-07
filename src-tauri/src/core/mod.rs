//! Pure domain logic. Nothing in this module performs I/O or talks to the OS.

pub mod plan;
pub mod schedule;
pub mod session;
pub mod settings;
pub mod snapshot;

pub use plan::PlanRequest;
pub use session::{Mode, Session, SessionEvent};
pub use settings::{Schedule, Settings};
pub use snapshot::{BatteryState, GuardState, PauseReason, Snapshot};
