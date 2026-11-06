//! KeepMeAwake: keeps a Windows laptop awake, politely.
//!
//! The crate is split into layers so that every decision can be unit tested
//! without touching the operating system:
//!
//! * [`core`] holds pure domain logic: the session state machine, settings and
//!   the snapshot handed to the UI. No I/O.
//! * [`platform`] defines the OS capabilities the engine needs (power requests,
//!   battery state) as traits, with the real Win32 implementation behind
//!   `cfg(windows)` and fakes for tests.
//! * [`engine`] drives the state machine on a background thread. It is the only
//!   component that owns timing, and it sleeps until the next deadline instead
//!   of polling.
//! * [`app`] is the thin Tauri shell: commands, tray icon, window lifecycle.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod app;
pub mod core;
pub mod engine;
pub mod persistence;
pub mod platform;

/// Starts the desktop application. Blocks until the app exits.
pub fn run() {
    app::run();
}
