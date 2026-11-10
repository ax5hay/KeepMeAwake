# Architecture

Keep me awake is a [Tauri 2](https://v2.tauri.app) application: a Rust core
that owns all behaviour and a Svelte UI rendered by the system webview
(WebView2 on Windows, WKWebView on macOS) that exists only while the window is
open. Everything above `platform/` is shared between the two.

## Layers

```mermaid
flowchart TB
    subgraph exe["keepmeawake.exe · KeepMeAwake.app"]
        direction TB
        app["app/<br/>commands · tray · window"]
        engine["engine/<br/>worker thread · policy"]
        core["core/<br/>Session · PlanRequest · Settings · Schedule · Snapshot · Almanac<br/><i>pure, no I/O</i>"]
        platform["platform/<br/>PowerGuard · BatteryProbe · IdleProbe · Clock · TaskbarTheme"]
        win["platform/windows.rs<br/>PowerCreateRequest · PowerSetRequest<br/>GetSystemPowerStatus · GetLastInputInfo · registry theme<br/><i>unsafe, Windows only</i>"]
        mac["platform/macos.rs<br/>IOPMAssertionCreateWithName · Release<br/>IOPSCopyPowerSourcesInfo · CGEventSource idle time<br/><i>unsafe, macOS only</i>"]
        persist["persistence.rs<br/>settings.json · history.json · almanac.json, atomic save"]
        app --> engine --> core
        engine --> platform --> win
        platform --> mac
        app --> persist
    end
    subgraph webview["WebView2 · WKWebView (while the window is open)"]
        direction TB
        api["lib/api.ts<br/>the only file that calls invoke()"]
        store["lib/state.svelte.ts<br/>one store, 1 Hz clock only during a countdown"]
        pure["lib/format.ts · sun.ts · copy.ts · platform.ts · almanac.ts<br/><i>pure, unit tested</i>"]
        ui["components/<br/>Scene · DurationRule · Settings · Almanac · …"]
        ui --> store --> api
        store --> pure
    end
    api <-->|"invoke / event"| app
```

Dependencies point down only: `app → engine → core` and `engine → platform`.
`core` imports nothing from the other layers and performs no I/O; that is what
makes the state machine, the policy and the snapshot trivially testable, and it
is the first thing to check in a review.

### core

| Type          | Role                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------ |
| `PlanRequest` | the only mutation the UI can send: `off`, `indefinite`, `for {seconds}`, `until {unix_ms}`             |
| `Session`     | off, indefinite, or timed with a deadline; remembers when it started and when the current plan was set |
| `Settings`    | versioned, every field defaulted, `sanitized()` clamps ranges                                          |
| `schedule`    | `current_window`, `next_window`, `next_boundary`: pure local-time arithmetic, generic over the zone    |
| `Snapshot`    | everything the UI and tray render, in one message                                                      |
| `Almanac`     | spans of presence as instants, and a mark per local date; `note` takes one idle reading at a time      |

`until` is resolved to `for` against the wall clock before it reaches the state
machine, which only knows monotonic time.

```mermaid
stateDiagram-v2
    [*] --> Off
    Off --> Indefinite: indefinite
    Off --> Timed: for / until
    Indefinite --> Timed: for / until
    Timed --> Indefinite: indefinite
    Timed --> Off: deadline passes (Expired)
    Indefinite --> Off: off
    Timed --> Off: off
```

The schedule sits beside the session rather than inside it. The engine
derives a fourth mode, `Scheduled`, when no manual session is on and a window
is open and not skipped. Precedence, highest first: manual session → skip →
schedule → resting.

```mermaid
stateDiagram-v2
    direction LR
    Resting --> Scheduled: window opens
    Scheduled --> Resting: window closes
    Scheduled --> Skipped: sun clicked
    Skipped --> Scheduled: sun clicked again
    Skipped --> Resting: window closes
    Scheduled --> Manual: a stop, Always, tray
    Manual --> Scheduled: session ends inside the window
    Manual --> Resting: session ends outside it
```

A skip is stored as the window's end in wall-clock milliseconds, so it lapses
with the window even across sleep.

### platform

Five traits so the engine and the tray never call the OS directly.

| Trait          | Windows                                            | macOS                                                                     | Test double      |
| -------------- | -------------------------------------------------- | ------------------------------------------------------------------------- | ---------------- |
| `PowerGuard`   | `PowerCreateRequest` + `PowerSetRequest` / `Clear` | two IOPMAssertions: `PreventUserIdleSystemSleep`, `…DisplaySleep`         | `FakePowerGuard` |
| `BatteryProbe` | `GetSystemPowerStatus`                             | first source of `IOPSCopyPowerSourcesInfo`; none means mains              | `FakeBattery`    |
| `IdleProbe`    | `GetLastInputInfo` − `GetTickCount`, wrapping      | `CGEventSourceSecondsSinceLastEventType`, HID state, any input            | `FakeIdle`       |
| `Clock`        | `Instant::now`, `SystemTime`                       | same                                                                      | `ManualClock`    |
| `TaskbarTheme` | `RegGetValueW` on `SystemUsesLightTheme`           | `DarkTaskbar` (unused: the menu bar icon is a template image macOS tints) | `FakeTaskbar`    |

`unsafe` lives only in `windows.rs` and `macos.rs`, each compiled only on its
own OS. Every block carries a `SAFETY:` comment; clippy's
`undocumented_unsafe_blocks` is `deny`.

`windows.rs` also exposes `wait_for_theme_change`, which blocks in
`RegNotifyChangeKeyValue` on the Personalize key. A thread named
`keepmeawake-theme` loops on it and refreshes the tray, so the icon set
follows the taskbar with no polling.

### engine

One thread, one loop, no polling while idle.

```mermaid
flowchart TD
    wake(["wake: command, deadline or shutdown"]) --> tick["session.tick(now)<br/>timed session may expire"]
    tick --> sched["schedule: window open? skipped?<br/>emit ScheduleStarted / ScheduleEnded"]
    sched --> bat["battery.read()"]
    bat --> decide["policy::decide(active, settings, battery)<br/>→ guards to hold, pause reason"]
    decide --> apply["guard.apply(desired)<br/>idempotent"]
    apply --> sun["sun up or down since last step?<br/>emit SessionStarted / SessionEnded"]
    sun --> snap["build Snapshot"]
    snap --> emit["deliver events to listeners<br/><i>lock released</i>"]
    emit --> next{"next deadline?"}
    next -->|"session end, battery poll in 30 s<br/>or schedule boundary (≤ 5 min)"| waitUntil["condvar.wait_until(deadline)"]
    next -->|"none (resting)"| waitAny["condvar.wait()"]
    waitUntil --> wake
    waitAny --> wake
```

Commands from the UI and tray mutate state under the lock, mark it dirty,
notify the condvar and return the fresh snapshot immediately, so callers never
wait on the worker. The loop re-evaluates instead of waiting when a change
landed while it was delivering events. The thread is named
`keepmeawake-engine`.

With a schedule on and no window open, the wait for the next window is capped
at five minutes: Windows stops counting wait timeouts while the machine
sleeps, and the cap bounds how late a window can be noticed after the lid
opens.

The policy, as a table:

`active` is a manual session, or an open schedule window that was not skipped.

| Session | Schedule               | On mains           | On battery, above threshold | On battery, below threshold | "Pause whenever unplugged" |
| ------- | ---------------------- | ------------------ | --------------------------- | --------------------------- | -------------------------- |
| off     | off, closed or skipped | nothing            | nothing                     | nothing                     | nothing                    |
| off     | window open            | system (+ display) | system (+ display)          | nothing · `LowBattery`      | nothing · `OnBattery`      |
| active  | any (the session wins) | system (+ display) | system (+ display)          | nothing · `LowBattery`      | nothing · `OnBattery`      |

If the OS refuses the request the snapshot says `PowerRequestFailed` rather
than pretending.

The engine also reports each stretch with the sun up exactly once:
`SessionStarted { mode, paused, at_unix_ms }` when it rises and
`SessionEnded { ended, at_unix_ms }` when it sets, including on quit.
`SunsetSoon { ends_at_unix_ms, secs_left }` comes once per deadline when a
timed session reaches its warning (`policy::sunset_warning_at`); the worker
wakes for it, and `Engine::extend` re-arms it by moving the deadline.
`OvernightAsk { idle_secs, rests_at_unix_ms }` comes once per question when
an Always session has had no input for the guard's hours. The idle time is
read on the steps the 30 s battery poll already makes, and only while it can
matter. Input withdraws the question; `Engine::keep_it_up` dismisses the
guard for that session; an unanswered question ends the session after
`policy::GUARD_GRACE` (10 minutes, a wake the worker adds) as `Idle`. A
`State::awake` flag holds what was last reported, so a startup plan or a
window already open at launch is reported too. `Ended` is `Idle` when the
guard let it go, else `Timer`, `Schedule` or `User`, or `Battery` if the
stretch was still paused for the battery as it ended.

### app

`app/mod.rs` builds the Tauri app. Plugins in order: single-instance (must be
first), autostart (passes `--hidden`), opener, notification, global-shortcut,
updater, log. `setup` loads settings and the session log (marking an entry left open by
the last run as `sleep`), starts the engine with its listener already
attached (`Engine::start_with`, so the first events are heard), creates the
tray, registers the hotkey (`app/hotkey.rs`), and opens the window unless
launched hidden. The listener writes session events to the log
(`app/history.rs`, saved as each stretch starts and ends), refreshes the
tray, emits `snapshot` to the window and hands the other events to
`app/notify.rs` for toasts.
`RunEvent::ExitRequested` with no exit code (the last window closed) is vetoed
so the app lives in the tray.

`app/almanac.rs` keeps the almanac beside the engine, not in it: who is at
the machine is not a power decision, and has to be noted with the sun up or
down alike. A thread, `keepmeawake-almanac`, reads the idle time once a
minute while **Keep an almanac** is on and blocks with no timeout while it
is off. Each reading goes to `core::almanac::Almanac::note`, which grows the
newest span or opens a new one after ten quiet minutes. The almanac is saved
as a span opens, two minutes after the keys go quiet, every quarter hour
while a span grows, and on exit.

```mermaid
flowchart LR
    wake(["once a minute, while on"]) --> read["idle.idle()<br/>read-only"]
    read --> touched{"idle shrank against<br/>the monotonic clock?"}
    touched -->|no| nothing["nothing: nobody touched anything"]
    touched -->|"yes, within 10 min of the newest span"| grow["the span grows to that input"]
    touched -->|"yes, after a longer quiet"| open["a new span opens · save"]
    grow --> save{"quiet 2 min,<br/>or 15 min since the last save?"}
    save -->|yes| disk["almanac.json"]
```

The window does the rest (`lib/almanac.ts`, pure): days that turn over
opposite the middle of the working hours, time over, nights as the quiet
between two spans, leave. Nothing about a night is decided as it happens.

`app/update.rs` starts a thread, `keepmeawake-update`, that asks the updater
plugin about 30 s after launch and then 24 h after the last check
(`last_update_check_unix_ms`), sleeping at most 6 h at a time so a laptop
that slept through the due time catches up within hours. With
`check_for_updates` off it never asks. A newer release is kept in
`AppState.update`, sent as the `update` event and shown by the tray.
`install_update` checks again, downloads with `update-progress` events,
lets the plugin verify the signature against the public key in
`tauri.conf.json`, installs and relaunches (`AppHandle::restart`). On
Windows the plugin runs the passive NSIS installer and exits the app
itself, so `on_before_exit` shuts the engine down first.

```mermaid
sequenceDiagram
    participant T as update thread
    participant G as GitHub Releases
    participant UI as Window
    T->>G: GET releases/latest/download/latest.json (once a day)
    G-->>T: version, per-platform URL and signature
    T-->>UI: event "update" · tray "Update ready"
    UI->>T: invoke install_update
    T->>G: download the installer or .app.tar.gz
    T-->>UI: event "update-progress" (per percent)
    T->>T: verify signature, install, restart
```

## IPC

```mermaid
sequenceDiagram
    actor U as You
    participant UI as Svelte UI
    participant E as Engine
    participant W as Windows
    U->>UI: click the 2h stop
    UI->>E: invoke set_plan {kind: for, seconds: 7200}
    E->>E: session.apply, notify worker
    E-->>UI: Snapshot (returned)
    E->>W: PowerSetRequest(System), PowerSetRequest(Display)
    E-->>UI: event "snapshot"
    E-->>E: tray.refresh(snapshot)
```

| Command           | Arguments              | Returns                                                                                 |
| ----------------- | ---------------------- | --------------------------------------------------------------------------------------- |
| `get_snapshot`    |                        | `Snapshot` (also re-reads battery)                                                      |
| `set_plan`        | `request: PlanRequest` | `Snapshot`                                                                              |
| `toggle_session`  |                        | `Snapshot`; skips or resumes an open schedule window                                    |
| `skip_schedule`   |                        | `Snapshot`                                                                              |
| `resume_schedule` |                        | `Snapshot`                                                                              |
| `update_settings` | `settings: Settings`   | `Snapshot`, or an error string if the file could not be saved or the hotkey was refused |
| `quit_app`        |                        |                                                                                         |
| `extend_session`  |                        | `Snapshot`: a timed session's end moved 30 minutes later                                |
| `keep_it_up`      |                        | `Snapshot`: the overnight guard leaves this Always session alone                        |
| `get_update`      |                        | `UpdateInfo \| null`: a newer release the daily check found                             |
| `install_update`  |                        | nothing on success (the app relaunches); events `update-progress`; an error string      |
| `get_history`     |                        | `History`: the session log, oldest entry first                                          |
| `clear_history`   |                        | `History` (empty), also written to disk                                                 |
| `get_almanac`     |                        | `Almanac`: presence and marked days, after one fresh look                               |
| `mark_day`        | `day`, `mark \| null`  | `Almanac`; an error string for a day that is not a date                                 |
| `clear_almanac`   |                        | `Almanac` (empty), also written to disk                                                 |

Field names are `snake_case` on both sides and cross the boundary untouched.
`src/lib/types.ts` mirrors `core` one to one; a test on each side pins the
JSON shape, because a mismatch here would be silent.

## Window lifecycle

```mermaid
stateDiagram-v2
    [*] --> Tray: launch
    Tray --> Hidden: setup (manual launch) or tray click or second instance
    Hidden --> Visible: UI renders, calls show()<br/>(2.5 s fallback in Rust)
    Visible --> Tray: close (default: destroy, webview gone)
    Visible --> HiddenKept: close with "Free memory when closed" off
    HiddenKept --> Visible: tray click
    Tray --> [*]: Quit
```

The window is declared with `create: false` and built on demand. During
`setup` it is created directly on the main thread; from tray and plugin
callbacks it is created on a spawned thread on Windows, because creating
windows from a synchronous event handler can deadlock there, and queued with
`run_on_main_thread` on macOS, because AppKit wants windows made on the main
thread itself.

On macOS the app runs with `ActivationPolicy::Accessory` (no Dock icon, no
Cmd-Tab entry), and a click on the menu bar icon places the window centred
beneath it, inside the work area of that screen, every time
(`core::placement::under_icon`, pure and tested). Windows reopens the window
where it last was.

## Security posture

- Runs as the signed-in user, never elevated.
- One outbound request a day, from Rust: HTTPS to GitHub Releases for
  `latest.json`, and the download when you click Install. Off with
  **Check for updates daily**, and then there is no network access at all.
  Updates are verified against the public key built into the app before
  they run; the webview has no updater or process permission. Fonts and
  assets are bundled. Toasts go to the local notification centre; they are
  sent from Rust, so the webview has no notification permission.
- One settings file, one log and one almanac, per user (see **On disk**).
  The almanac holds only when someone was at the machine, never what they
  did, and never leaves it.
- A handful of OS calls, one file per OS: on Windows power requests, power
  status, the last input time, two registry reads and a registry change
  notification; on macOS two power assertions, the power sources snapshot
  and the HID idle time.
- No input is ever sent to other applications. The overnight guard and
  the almanac only read how long ago the last input was; neither OS asks
  permission for it. The global hotkey is
  registered from Rust (`RegisterHotKey` on Windows, a Carbon hot key on
  macOS), which only listens, so the webview has no shortcut permission.
- CSP is `default-src 'self'`; the capability file grants only window
  dragging and visibility, autostart toggling, the commands above, and
  opening two allow-listed `https` origins.
- Dependencies are locked (`package-lock.json`, `Cargo.lock`) and bumped by
  Dependabot in grouped PRs.

## On disk

| What      | Windows                                                      | macOS                                                                   |
| --------- | ------------------------------------------------------------ | ----------------------------------------------------------------------- |
| Settings  | `%APPDATA%\dev.bajpai.keepmeawake\settings.json`             | `~/Library/Application Support/dev.bajpai.keepmeawake/settings.json`    |
| Log       | `%LOCALAPPDATA%\dev.bajpai.keepmeawake\logs\keepmeawake.log` | `~/Library/Logs/dev.bajpai.keepmeawake/keepmeawake.log`                 |
| History   | `%APPDATA%\dev.bajpai.keepmeawake\history.json`              | `~/Library/Application Support/dev.bajpai.keepmeawake/history.json`     |
| Almanac   | `%APPDATA%\dev.bajpai.keepmeawake\almanac.json`              | `~/Library/Application Support/dev.bajpai.keepmeawake/almanac.json`     |
| Autostart | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`         | a LaunchAgent plist in `~/Library/LaunchAgents/`, written by the plugin |

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
