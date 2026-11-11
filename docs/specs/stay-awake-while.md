# Stay awake while…

Keep the laptop awake on its own while something is going on: an app is
running, a download is moving, the processor is busy, something is full
screen, a display is connected, or it is plugged in. Off until chosen;
each condition has its own switch.

## Why

The sun is a switch you have to remember. The moments people forget it are
the ones that cost the most: a long download that dies when the laptop
sleeps, a build, an export, a talk. Paid keep-awake apps sell exactly this
as "triggers". Here it is free, and it costs nothing while no condition is
on.

## Experience

Settings gains a section, **Stay awake while…**, after Staying awake:

| Row                              | Control                      | Hint                                                   | Shown          |
| -------------------------------- | ---------------------------- | ------------------------------------------------------ | -------------- |
| These apps are running           | chips + **Add an app** field | `Pick from what is running now, or type a name.`       | always         |
| A download or upload is going    | switch                       | `Over 64 KB/s, until it has been quiet for 2 minutes.` | always         |
| The computer is busy             | switch                       | `Processor over 30%: a build, an export, a render.`    | always         |
| Something is full screen         | switch                       | `A video, a game or a presentation.`                   | always         |
| An external display is connected | switch                       | `At a desk with a monitor.`                            | with a battery |
| Plugged in                       | switch                       | `On battery it sleeps as usual.`                       | with a battery |

The **Add an app** field suggests the names of apps running now (a
`datalist` filled by `running_apps` when the field gets focus) and accepts
any name typed. A chip's `×` removes it.

While a condition holds and nothing else is keeping the laptop awake, the
sun is up, the word is **Awake** and the line names the reason:

| Reason   | Line                                   |
| -------- | -------------------------------------- |
| app      | `While Xcode runs`                     |
| process  | `While process 4120 runs` (the CLI)    |
| transfer | `While the network is busy · 2.4 MB/s` |
| busy     | `While the computer is busy · 54%`     |
| full     | `While something is full screen`       |
| display  | `While a display is connected`         |
| power    | `While plugged in`                     |

Clicking the sun then lets the laptop sleep until that condition clears
(`Skipping until Xcode quits.`, and so on); clicking again undoes it. A
manual session (a stop on the rule, Always, the tray, the hotkey) and an
open schedule window both outrank a condition. The tray's status line
reads `Awake while Xcode runs`.

No toast: this happens on its own, many times a day, and a toast each time
would nag. History logs each stretch, ending as `watch`.

## Settings

```rust
#[serde(default)]
pub struct WatchSettings {
    pub apps: Vec<String>,   // at most 16, each at most 64 chars, trimmed, deduplicated
    pub transfers: bool,
    pub busy: bool,
    pub full_screen: bool,
    pub display: bool,
    pub plugged_in: bool,
}
```

All off and empty by default; `Settings::watch`, defaulted. No version
bump. Asked for after the first build: the thresholds are settings too,
`transfer_kbps` (64, 1–100 000), `busy_percent` (30, 5–95) and
`linger_mins` (2, 1–60), each shown under its switch as **Counts from**,
**Busy above** and **Then waits**.

## Rust

**core::watch** (pure): `Watcher` keeps the previous transfer and processor
samples and when each last crossed its threshold. `Watcher::observe(&mut
self, &WatchSettings, &Readings, battery, now) -> WatchView` returns the
reason (first match, in the table's order) and the live numbers. Rates are
computed between consecutive samples; a counter that went backwards (an
adapter reset) is a fresh start, not a negative rate. A transfer or busy
condition holds for `WATCH_LINGER` (2 minutes) after its last sample over
the threshold, so a pause between files or compile steps does not let the
laptop sleep.

**platform::ActivityProbe** (new trait, read-only):

| Method                                 | Windows                                                                  | macOS                                                                            |
| -------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `running(&[String]) -> Option<String>` | `CreateToolhelp32Snapshot` + `Process32*W`                               | `proc_listallpids` + `proc_name`                                                 |
| `running_apps() -> Vec<String>`        | same, deduplicated, `.exe` dropped                                       | same, this user's processes only (`proc_listpids(PROC_UID_ONLY)`)                |
| `alive(pid) -> bool`                   | `OpenProcess` + `GetExitCodeProcess == STILL_ACTIVE`                     | `proc_pidpath` succeeds                                                          |
| `transferred() -> Option<u64>`         | `GetIfTable2`, `InOctets + OutOctets`, loopback and non-hardware skipped | `sysctl(NET_RT_IFLIST2)`, `if_data64`, loopback skipped                          |
| `cpu() -> Option<CpuTicks>`            | `GetSystemTimes`                                                         | `host_statistics(HOST_CPU_LOAD_INFO)`                                            |
| `full_screen() -> bool`                | `SHQueryUserNotificationState`: busy, D3D full screen, presentation      | `CGWindowListCopyWindowInfo`: a layer-0 window of another app covering a display |
| `external_display() -> bool`           | `QueryDisplayConfig`, a path whose `outputTechnology` is not internal    | `CGGetActiveDisplayList`, a display that is not `CGDisplayIsBuiltin`             |

Only the readings for switched-on conditions are taken.

**engine**: `State` gains a `Watcher`, the CLI's `pids`, and `watch_skip`.
`Mode::Watching` is derived like `Scheduled`, below it in precedence. The
worker wakes every `WATCH_POLL` (30 s, the battery poll) while any
condition is on, sun up or down. `Engine::watch_pid(pid)` adds a pid until
it exits. `Ended::Watch` when the condition clears.

**Snapshot** gains `watch: WatchState { reason, skipped, transfer_bps,
cpu_percent }`; `WatchReason` is tagged by `kind`.

## IPC

`running_apps() -> string[]`. Settings travel with `update_settings`.

## Tests

- core: each condition, the order, linger, counter reset, the 16-app cap,
  `.exe` and case ignored.
- engine (fakes): a condition raises the sun as `Watching` and lowers it
  after linger; a manual session outranks it; the sun click skips until it
  clears; a pid holds until it exits; nothing is read with nothing on.
- UI: the status line for each reason; rows hidden without a battery.

## Resource impact

With every condition off: nothing (no reading, no wake). With any on: one
wake every 30 s reading only what is switched on, each a single system
call measured in microseconds (process list: one snapshot of a few hundred
entries). Recorded in `resource-budget.md`.

## Acceptance

- Add an app, start it: the sun rises within 30 s; quit it: it sets.
- Download a large file with transfers on: awake; two minutes after it
  finishes: resting.
- Present full screen: awake; leave full screen: resting.
- With everything off, the engine thread wakes no more than before.

## Commits

1. `feat(core): watch settings and the pure watcher`
2. `feat(platform): read running apps, transfers, processor, displays`
3. `feat(engine): stay awake while a condition holds`
4. `feat(app): running_apps, watch in the tray and history`
5. `feat(ui): the Stay awake while section and its status lines`

## Out of scope

Wi-Fi networks, Bluetooth devices, mounted drives, per-app thresholds.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
