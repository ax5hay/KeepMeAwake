# Sunset warning

Version 0.4 · Size S · Touches core, engine, app, tray, ui

## Why

A timed session that ends while you are mid-download or mid-render ends
quietly: the next thing you notice is the screen dimming. A few minutes'
notice, and one click to carry on, turns "it slept on me" into a choice.

## Experience

A few minutes before a **timed** session ends (5 by default), three things
change, and nothing else:

| Where       | What                                                                                             |
| ----------- | ------------------------------------------------------------------------------------------------ |
| Toast       | title `The sun sets soon`, body `In 5 minutes. Give it another 30m from the tray or the window.` |
| Status line | the detail line turns to `The sun sets in 5 minutes.` (`in 1 minute`, `in under a minute`)       |
| Action      | under the status line, a quiet link **Give it another 30m**; the tray menu gains the same item   |

**Give it another 30m** pushes the deadline 30 minutes later. The rule's arc
grows to match, the warning goes away, and it can come again 5 minutes
before the new end.

Rules:

- Timed sessions only. A schedule window ending is expected, and Always has
  no end.
- Only when the plan is longer than the warning: a 2-minute timer with a
  5-minute warning never warns.
- One warning per deadline. Extending, or choosing a new stop, re-arms it.
- Not while an open, unskipped schedule window outlasts the timer: the sun
  stays up when the timer ends, so there is no sunset to warn about (the
  timer's own end toast stays quiet then too).
- The toast follows the existing **When a timer ends** switch (it is about
  the same timer); the status line and the action do not, they are in the
  window you are looking at.
- Paused sessions still warn: the deadline still comes.

## Settings

| Field                 | Type         | Default   | UI                                                              |
| --------------------- | ------------ | --------- | --------------------------------------------------------------- |
| `sunset_warning_mins` | `Option<u8>` | `Some(5)` | Settings → Waking → **Warn before sunset**: Off · 2m · 5m · 10m |

Sanitized to 1–60. A new field with a default, so `SETTINGS_VERSION` stays.

## Rust

### core

- `Session::extend(by: Duration) -> bool`: moves a timed session's
  deadline later, keeping when it was planned, so `plan_duration` grows and
  the arc stays continuous. No-op (returns `false`) when not timed.
- `settings.rs`: the field, its default, the clamp.

### engine

- `EngineEvent::SunsetSoon { ends_at_unix_ms, secs_left }`.
- `State::warned_for: Option<Instant>`: the deadline the last warning was
  for. In `step`, a timed session warns when
  `deadline - warning <= now < deadline`, `plan_duration > warning`, and
  `warned_for != Some(deadline)`.
- `next_wake` includes the warning moment while a warning is still due, so
  the worker wakes for it (one extra wake per timed session).
- `Engine::extend(by)`: `Session::extend`, then the usual mutate.

### app

- Command `extend_session` (no arguments): extends by 30 minutes.
- `notify.rs`: the toast for `SunsetSoon`, behind the timer switch.
- `tray.rs`: `Give it another 30m` after the toggle while a timed session
  has no more than the warning left. The menu is rebuilt on every snapshot,
  and the warning step produces one.

## UI

- `types.ts`: `sunset_warning_mins: number | null`.
- `format.ts`: `sunsetSoon(snapshot, nowMs)` → `true` inside the warning;
  `statusText` uses the copy below while it is.
- `copy.ts`: `sunsetIn(secs)`, `EXTEND_LABEL`.
- `Home.svelte`: the link under the status line while `sunsetSoon`.
- Store: `extend()`.
- Settings → Waking: **Warn before sunset**, segmented Off · 2m · 5m · 10m.

## IPC

| Command          | Args | Returns    |
| ---------------- | ---- | ---------- |
| `extend_session` |      | `Snapshot` |

## Tests

Rust: `Session::extend` keeps the plan start and grows the plan; no-op when
not timed. Engine: warns once at the right moment with the right
`secs_left`; never for a plan shorter than the warning; not when the
setting is off; re-arms after `extend`; never for Always or a schedule
window. Policy: next wake includes the warning moment. Tray: the item shows
only inside the warning. Toast text, and silence with the timer switch off.

UI: `sunsetSoon` boundaries; `statusText` inside and outside the warning;
`sunsetIn` wording; the setting round-trips.

## Docs

Guide (Status, Tray, Settings, Notifications), architecture (event, command),
resource budget (one wake per timed session), changelog, roadmap.

## Resource impact

One extra wake of the engine per timed session, at the warning moment. No
new timers in the UI: the status line uses the 1 Hz clock that already runs
during a timed session.

## Acceptance

- [ ] A 6-minute timer: at 5 minutes left a toast, the status line and the
      link; the tray menu has the item.
- [ ] **Give it another 30m** from the window: the deadline moves 30 minutes,
      the arc grows, the warning is gone; it returns 5 minutes before the new
      end.
- [ ] The same from the tray menu.
- [ ] A 2-minute timer never warns. Off in Settings never warns. The timer
      switch off: no toast, but the status line and the link still appear.

## Commits

1. `docs(specs): sunset warning`
2. `feat(core): extend a timed session and a sunset warning setting`
3. `feat(engine): warn before a timed session sets`
4. `feat(app): sunset toast, tray item and extend command`
5. `feat(ui): sunset status line, the extend link and its setting`
6. `docs: sunset warning in the guide, architecture and changelog`

## Out of scope

Warnings for schedule windows, a configurable extension length, snoozing
the toast, an action button on the toast itself (desktop toasts from the
plugin carry no actions).

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
