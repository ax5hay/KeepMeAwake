# 0004. Destroy the window on close

Date: 2026-11-07 · Status: accepted

## Context

Tray apps usually hide their window on close so it reopens instantly. A hidden
WebView2 still owns several processes and tens of megabytes.

## Decision

The window is declared with `create: false` and built on demand. On close the
default path lets Tauri destroy it; the app stays alive because
`ExitRequested` without an exit code is vetoed. A setting ("Free memory when
closed", on by default) switches to hide-on-close for an instant reopen.

The UI reveals the window itself after its first render; a 2.5 s fallback in
Rust shows it anyway if the UI never reports ready. During `setup` the window
is created directly on the main thread because the deferred path waits for an
event loop that has not started.

## Consequences

- Idle memory in the tray is the Rust process alone.
- Reopening costs a WebView2 spin-up, a few hundred milliseconds.
- The UI boots from nothing every time: it always fetches a snapshot on mount.
