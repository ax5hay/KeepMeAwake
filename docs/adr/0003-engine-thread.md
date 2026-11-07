# 0003. One engine thread that sleeps on deadlines

Date: 2026-11-07 · Status: accepted

## Context

A handful of time-based behaviours (timer expiry, battery polling) and
commands from two sources (window, tray). A naive design polls every second;
an async runtime is heavier than the problem.

## Decision

A single `std::thread` owns the session, settings and OS handles. It blocks on
a `parking_lot::Condvar` with `wait_until(next_deadline)`, where the deadline
is the earlier of the session end and the next battery poll (30 s, only while
active). Commands mutate state under the mutex and notify. Listeners are
called with the lock released.

## Consequences

- Zero wake-ups while resting; one per 30 s while awake.
- The loop body is a `step()` function tested with fakes and a manual clock.
- The engine is the only place that touches `PowerGuard`; the handle never
  crosses threads.
- No async runtime in the core; Tauri's stays on the UI side.
