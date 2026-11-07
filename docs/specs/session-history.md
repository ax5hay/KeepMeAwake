# Session history

Version 0.3 · Size M · Touches core, engine, app, ui

## Why

"Did it actually stay up last night?" and "when did it pause?" are the two
questions the app cannot answer today. A short, honest log answers both and
is the foundation for the sky log later.

## Experience

Settings → new section **History** between **Notifications** and **Tips**.
A list of the last sessions, newest first, at most 30 shown:

```
Today        9:02 am – 11:30 am   2h 28m   timer
Today        7:41 am – 7:46 am    5m       you
Yesterday    9:00 am – 6:00 pm    9h       schedule
Mon 3 Nov    10:15 pm – 1:05 am   2h 50m   battery
```

Columns: day (`Today`, `Yesterday`, else `Mon 3 Nov`), span in local time,
length, how it ended: `you` (sun, rule, tray, hotkey, quit), `timer`,
`schedule`, `battery` (paused and never resumed before it ended), `sleep`
(Windows slept anyway: the next launch found an unfinished session), `crash`
(no clean end). Pauses inside a session are shown as a thin inset line under
the row: `paused 12:10 pm – 12:40 pm · battery`.

A `Clear history` link at the bottom with no confirmation (it is a log, not
data).

## Settings

None. History is a separate file.

## Rust

### core/history.rs (new, pure)

```rust
pub enum Ended { User, Timer, Schedule, Battery, Sleep, Crash }
pub struct Pause { pub from_unix_ms: u64, pub to_unix_ms: Option<u64>, pub reason: PauseReason }
pub struct Entry {
    pub started_unix_ms: u64,
    pub ended_unix_ms: Option<u64>,   // None while running
    pub mode: Mode,                   // the mode it started in
    pub ended: Option<Ended>,
    pub pauses: Vec<Pause>,
}
pub struct History { pub version: u32, pub entries: Vec<Entry> }  // capped at 200
impl History {
    pub fn open(&mut self, started: u64, mode: Mode);
    pub fn pause(&mut self, at: u64, reason: PauseReason);
    pub fn resume(&mut self, at: u64);
    pub fn close(&mut self, at: u64, ended: Ended);
    pub fn mark_unfinished_as(&mut self, ended: Ended);   // called at launch
    pub fn trim(&mut self);
}
```

### engine

The engine already knows every transition. Emit richer events:
`SessionStarted { mode }`, `SessionEnded { ended }`, and the existing
`Paused`/`Resumed`. `Ended::User` for `set_plan(Off)`/toggle, `Timer` for
expiry, `Schedule` for window close, `Battery` when `shutdown` happens while
paused, and `User` on quit.

### app

- `persistence.rs`: `history_path(config_dir)`, `load_history`, `save_history`
  (same atomic write). Save on every `SessionEnded`, and on every
  `SessionStarted` (so a crash leaves an open entry).
- At setup, after loading: `mark_unfinished_as(Ended::Sleep)` if the last
  snapshot of settings shows the previous run was active, else `Crash`.
  Simplest reliable signal: an open entry exists → `Sleep` (Windows slept or
  the machine was shut down; both mean "it did not hold"). Document the
  simplification.
- Commands: `get_history` → `History`, `clear_history`.

## UI

- `types.ts`: mirrors.
- `History.svelte` (new): the list; day grouping and span formatting as pure
  helpers in `src/lib/history.ts` with tests.
- Store: `history` loaded when the settings pane opens (not at launch).

## IPC

| Command         | Returns           |
| --------------- | ----------------- |
| `get_history`   | `History`         |
| `clear_history` | `History` (empty) |

## Tests

Rust: `History` open/pause/resume/close sequences, trim at 200, unfinished
marking, serde round trip; engine emits `SessionStarted`/`SessionEnded` with
the right `Ended` for each path (user, timer, schedule, battery).
UI: `formatDay`, `formatSpan`, grouping; empty state renders
`Nothing yet. The first sunset will be written here.`

## Docs

Guide (Settings table), architecture (on-disk table gains `history.json`),
changelog.

## Resource impact

One file write per session start and end. Nothing at idle.

## Acceptance

- [ ] Start 30m, let it end: a `timer` row appears with the right span.
- [ ] Start, unplug below threshold, plug in, stop: one row with a pause line.
- [ ] Start Always, kill the process from Task Manager, relaunch: a `sleep`
      row with no end time shown as `–`.
- [ ] Clear history empties the list and the file.

## Commits

1. `feat(core): add the History log model`
2. `feat(engine): emit session started and ended events with a reason`
3. `feat(app): persist history and expose get/clear commands`
4. `feat(ui): add the History section`
5. `docs: history in the guide, architecture and changelog`

## Out of scope

Visualisation (sky log), export, retention settings.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
