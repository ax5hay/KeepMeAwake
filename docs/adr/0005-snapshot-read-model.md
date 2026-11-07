# 0005. Snapshot as the only read model

Date: 2026-11-07 · Status: accepted

## Context

The tray, the window and the log all describe state. Letting each compute it
from pieces (mode, deadline, battery, settings) invites drift.

## Decision

The engine builds one `Snapshot` after every change. Every command returns it,
the `snapshot` event carries it, the tray renders from it, and the UI store
holds exactly one. Wall-clock fields are derived inside the engine from the
monotonic session so the UI never does time arithmetic beyond a countdown.

Field names are `snake_case` on both sides and `src/lib/types.ts` mirrors the
Rust struct one to one.

## Consequences

- Adding state means a field in two places and a test on each side.
- The UI is stateless across reopen; it cannot be stale for longer than one
  IPC round trip.
- Tray text and UI text share formatting rules, pinned by tests on both sides.
