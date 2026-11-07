# Overnight guard

Version 0.4 · Size M · Touches platform, core, engine, app, tray, ui

## Why

**Always** is the easiest session to start and the easiest to forget. Leave
the laptop on Always on a Friday evening and it stays up all weekend. If
nobody has touched the keyboard or the mouse for hours, the sun should ask
once, and then let the machine sleep.

## Experience

While an **Always** session runs and nobody has used the keyboard, mouse or
trackpad for the guard's time (6 hours by default), the sun asks, once:

| Where       | What                                                                                       |
| ----------- | ------------------------------------------------------------------------------------------ |
| Toast       | title `Nobody around`, body `No input for 6 hours. Resting at 3:12 am unless you're back.` |
| Status line | `Nobody here for 6 hours. Resting at 3:12 am.` followed by a quiet link **Keep it up**     |
| Tray        | **Keep it up**, under the toggle                                                           |

Then, ten minutes later, one of three things:

| What happened in the ten minutes           | Result                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------- |
| any input (a key, the mouse, the trackpad) | the question is withdrawn silently; the clock starts again from that input |
| **Keep it up** (window or tray)            | the guard leaves this session alone until it ends                          |
| nothing                                    | the session ends; the laptop sleeps as usual; history says `idle`          |

Rules:

- **Always** sessions only. Timed sessions end on their own; schedule windows
  close on their own.
- The guard reads how long since the last input; it never sends any. This is
  read-only, as ADR 0002 requires, and needs no permission on either OS.
- A machine that cannot say (no idle reading) is never stopped.
- Paused sessions are guarded too: the session would resume when plugged in.

## Settings

| Field                   | Type         | Default   | UI                                                          |
| ----------------------- | ------------ | --------- | ----------------------------------------------------------- |
| `overnight_guard_hours` | `Option<u8>` | `Some(6)` | Settings → Waking → **Overnight guard**: Off · 4h · 6h · 8h |

Sanitized to 1–24. A new field with a default, so `SETTINGS_VERSION` stays.

## Rust

### platform

```rust
/// How long since the last keyboard, mouse or trackpad input. Read-only.
pub trait IdleProbe: Send { fn idle(&self) -> Option<Duration>; }
pub struct NoIdle;                 // None: unknown, so never acted on
```

| OS      | Implementation                                                                                                  |
| ------- | --------------------------------------------------------------------------------------------------------------- |
| macOS   | `CGEventSourceSecondsSinceLastEventType(kCGEventSourceStateHIDSystemState, kCGAnyInputEventType)`, CoreGraphics |
| Windows | `GetLastInputInfo` and `GetTickCount`, wrapping subtraction (the tick count wraps every 49.7 days)              |
| other   | `NoIdle`                                                                                                        |

`platform::fakes::FakeIdle` for tests. `Platform` gains `idle`.

### core

- `Settings::overnight_guard_hours`, its default and clamp.
- `history::Ended::Idle`: the session was let go by the guard.

### engine

- `State::guard_rests_at: Option<u64>` (Unix ms) while asking;
  `State::guard_dismissed_for: Option<Instant>` (the session start the user
  said Keep it up for).
- In `step`, while the session is **Always**, the guard is on and not
  dismissed for this session: read `idle`. At or past the guard's time with
  no question pending, ask (`GUARD_GRACE` = 10 minutes) and emit
  `OvernightAsk { idle_secs, rests_at_unix_ms }`. Below it, withdraw a
  pending question. With a question pending and its time reached, end the
  session; `SessionEnded` carries `Ended::Idle`.
- No new timer: the session is active, so the 30 s battery poll already
  wakes the worker; `next_wake` adds the grace deadline.
- `Engine::keep_it_up()`: dismisses the guard for the current session.
- `Snapshot::guard_rests_at_unix_ms: Option<u64>` so the window and tray can
  show the question.

### app

- Command `keep_it_up` → `Snapshot`.
- Toast for `OvernightAsk`; it is part of the guard, so no separate switch.
- Tray item **Keep it up** while `guard_rests_at_unix_ms` is set.
- `make_platform` builds the OS idle probe.

## UI

- `types.ts`: the setting, the snapshot field, `'idle'` in `Ended`.
- `format.ts`: `statusText` shows the question while it is pending.
- `copy.ts`: `nobodyHere(hours, time)`, `KEEP_IT_UP`.
- `Home.svelte`: the **Keep it up** link; store `keepItUp()`.
- Settings → Waking: **Overnight guard**, Off · 4h · 6h · 8h.
- History shows `idle` for a session the guard ended.

## IPC

| Command      | Args | Returns    |
| ------------ | ---- | ---------- |
| `keep_it_up` |      | `Snapshot` |

## Tests

Platform: the macOS idle reading answers and is sane; the Windows one is
covered by CI on `windows-latest`. Engine with `FakeIdle`: asks once at the
guard's time; withdraws on input; ends the session after the grace with
`Ended::Idle`; Keep it up stops it asking for this session only; never for
Timed, Scheduled, with the guard off, or with no idle reading; the next wake
includes the grace deadline. Core: the setting's default and clamp; `Idle`
serialises as `idle`. App: toast text; tray item. UI: status line while
asking; the history label.

## Docs

Guide (Status, Tray, Settings, Notifications, History, When it will not
hold), architecture (platform table, events, command), resource budget,
changelog, roadmap, handoff (what to check on Windows, and how).

## Resource impact

One idle reading per battery poll while an Always session runs (a single
cheap call), nothing at rest, and one extra wake at the grace deadline.

## Acceptance

Shorten the guard to test: set `overnight_guard_hours` to 1 in the settings
file, start Always, keep hands off for an hour.

- [ ] After an hour untouched: the toast, the status line and the tray item.
- [ ] Touch the mouse: the question goes away.
- [ ] Keep it up: no question again until the session ends.
- [ ] Ten minutes untouched after the question: the session ends, the
      machine sleeps normally afterwards, history shows `idle`.
- [ ] Off in Settings: never asks. Timed sessions: never asks.

## Commits

1. `docs(specs): overnight guard`
2. `feat(platform): read how long since the last input`
3. `feat(core): overnight guard setting and the idle ending`
4. `feat(engine): let an unattended Always session sleep`
5. `feat(app): overnight toast, tray item and keep command`
6. `feat(ui): overnight status line, Keep it up and its setting`
7. `docs: overnight guard in the guide, architecture and changelog`

## Out of scope

Guarding timed sessions, a configurable grace, detecting presence by camera
or anything other than input, an action button on the toast.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
