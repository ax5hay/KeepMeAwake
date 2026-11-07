# Notifications

Version 0.2 · Size S · Touches app, ui

## Why

The window is usually closed. The moments that matter (a timer ended, the
battery rule paused the sun, it resumed, a schedule started) should reach the
desktop as a quiet toast, each behind its own switch.

## Experience

Windows toasts, app icon on the left, one title line and one body line. No
buttons in this version. Copy, verbatim:

| Event            | Title                    | Body                                            | Default |
| ---------------- | ------------------------ | ----------------------------------------------- | ------- |
| timer ended      | `The sun has set`        | `Resting now. {duration}, as asked.`            | on      |
| battery paused   | `Paused for the battery` | `Holding off at {percent}%. Plug in to resume.` | off     |
| battery resumed  | `Back up`                | `Charger connected. The sun is up again.`       | off     |
| schedule started | `On schedule`            | `Awake until {time}.`                           | off     |
| schedule ended   | `Off schedule`           | `Resting until {next start}.`                   | off     |

Settings → new section **Notifications** between **Window** and **Tips**:

| Row                        | Control | Hint |
| -------------------------- | ------- | ---- |
| When a timer ends          | switch  | none |
| Battery pauses and resumes | switch  | none |
| Schedule starts and ends   | switch  | none |

Nothing is shown when the window is open and focused; the whisper line already
says it. (Check `WebviewWindow::is_focused` on the Rust side.)

## Settings

```rust
pub struct Notifications { pub timer: bool, pub battery: bool, pub schedule: bool }
pub notifications: Notifications,   // defaults true, false, false
```

## Rust

- Add `tauri-plugin-notification = "2"`; register after `opener`. Capability:
  `notification:default`.
- `app/notify.rs` (new): `pub fn send(app: &AppHandle, title: &str, body: &str)`
  using `app.notification().builder().title(..).body(..).show()`; log and
  swallow errors (toasts failing must never affect the engine).
- In the existing engine listener in `app/mod.rs`, map events:
  `TimerEnded` → timer toast; `Paused(LowBattery|OnBattery)` → battery toast;
  `Resumed` → battery toast; `ScheduleStarted/Ended` (from the schedule spec)
  → schedule toast. Read the switches from `engine.settings()` at send time.
- `TimerEnded` currently carries no duration. Change it to
  `TimerEnded { plan_secs: u64 }` so the body can say `2h 00m`. Reuse
  `tray::format_duration`.
- `Paused` needs the percentage: the listener has the snapshot in the same
  event burst; simplest is to extend `EngineEvent::Paused(reason, BatteryState)`.

Windows requires an AppUserModelID for toasts; Tauri sets it from the
identifier for installed builds. In `tauri dev` toasts may not appear; verify
with a release build or `tauri build --debug`.

## UI

Settings section as above; copy strings for toasts live in Rust (they are
sent from Rust), so add the table above to `app/notify.rs` as constants with
tests.

## IPC

None.

## Tests

Rust: `notify::body_for(event, settings)` pure helper returns `None` when the
switch is off and the exact strings above when on; `format_duration` reuse.
UI: none beyond the settings switch wiring already covered by the store test
pattern.

## Docs

Guide (Settings table), security posture in architecture (toasts are local;
no network), changelog.

## Resource impact

None at idle. The plugin adds ~300 KB to the binary; record it.

## Acceptance

- [ ] Pick 30m, close the window, wait; a toast `The sun has set` appears.
- [ ] With the battery switch on, unplug below the threshold: `Paused for the
battery`; plug in: `Back up`.
- [ ] No toast while the window is focused.
- [ ] Switches persist across restart.

## Commits

1. `feat(app): add the notification plugin and a send helper`
2. `feat(engine): carry plan length and battery state in events`
3. `feat(app): toast on timer end, battery pause/resume and schedule changes`
4. `feat(ui): add the Notifications section`
5. `docs: notifications in the guide and changelog`

## Out of scope

Toast action buttons, sounds, a notification history.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
