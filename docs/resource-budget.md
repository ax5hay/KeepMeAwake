# Resource budget

"Tiny" is a feature, so it has numbers and a way to check them.

## Budget

| Metric                         | Budget              | Windows (0.2, release)                | macOS (release)                            | Why                                                              |
| ------------------------------ | ------------------- | ------------------------------------- | ------------------------------------------ | ---------------------------------------------------------------- |
| Private memory, tray only      | ≤ 12 MB             | 5.7 MB (0.1: 5.3 MB)                  | **16 MB resting, 17 MB awake** (over)      | one native process, no webview                                   |
| Private memory, window open    | ≤ 90 MB             | 5.4 MB app + WebView2                 | 26 MB app + 30 MB WebContent               | the system webview is the floor; the page is small               |
| CPU while resting              | 0 wake-ups          | 0 ms CPU in 60 s                      | 0 ms CPU in 60 s                           | the engine (and, on Windows, theme) threads block in the kernel  |
| CPU while awake                | 1 wake-up / 30 s    | 1 / 30 s                              | 20 ms CPU in 60 s (two polls)              | battery poll only                                                |
| CPU while resting, schedule on | ≤ 1 wake-up / 5 min | by design                             | by design                                  | the next window start, re-checked so sleep cannot make it late   |
| Threads at rest                | no polling threads  | 30 (Tauri, WebView2 loader, plugins)  | 14 resting, 18 awake                       | `keepmeawake-engine` (and `keepmeawake-theme`) sleep until woken |
| CPU, window open, idle         | ≤ 0.1%              | idle                                  | 10 ms CPU in 30 s                          | no animation loops; 1 Hz clock only in a timed session           |
| Binary                         | ≤ 12 MB             | 4.2 MB exe · 1.5 MB NSIS · 2.1 MB MSI | 4.1 MB binary · 4.0 MB `.app` · 2.0 MB DMG | `opt-level = "s"`, LTO, stripped                                 |
| UI bundle (JS + CSS, gzip)     | ≤ 60 KB             | 30 KB + 3.7 KB (0.1: 28 KB + 3.5 KB)  | 32 KB + 3.8 KB (both, from the next build) | Svelte compiles away; no framework runtime                       |
| Fonts                          | ≤ 160 KB            | 149 KB                                | 149 KB                                     | one family, two files, latin subset                              |
| Cold open of the window        | ≤ 400 ms            | under 1 s                             | 0.86 s from a cold launch                  | webview spin-up + first paint                                    |

0.2 adds `chrono` (schedule arithmetic), `tauri-plugin-notification` and
`tauri-plugin-global-shortcut`: together +0.4 MB on the exe and nothing at
idle. The taskbar watch is one thread blocked in `RegNotifyChangeKeyValue`
(about 64 KB of stack, no CPU), and the tray carries three more 64 px PNGs
(under 1 KB each).

macOS adds no thread and no timer: there is no taskbar theme to watch (the
menu bar icon is a template image), and the battery is read with the same
30 s poll while awake rather than an IOKit run-loop notification, which would
remove that poll and is noted under Later. The UI bundle grew by about 2 KB
gzipped when the build target gained `safari15` for WKWebView on macOS 12.

The macOS memory figures are `phys_footprint` from `footprint <pid>`, which
counts AppKit's and WebKit's dirty and compressed pages for the process; the
Windows figures are private bytes. They are not the same measure, but the
menu-bar-only footprint is over the 12 MB budget either way: AppKit itself is
most of it. Whether the budget gets a macOS line of its own is an open
question.

Windows measured on a Windows 11 laptop, 2026-11-07. macOS measured on an
Apple Silicon Mac, macOS 27, 2026-11-07. WebView2 processes are
shared with other apps on the machine and cannot be attributed cleanly from Task
Manager; the app process itself stays at ~5 MB whether the window is open or not.

## Measure

```sh
# macOS: memory, threads and CPU time for the app process
P=$(pgrep -x keepmeawake)
footprint $P | grep phys_footprint
ps -M $P | tail -n +2 | wc -l
ps -o time= -p $P; sleep 60; ps -o time= -p $P
```

```powershell
# Memory for the app process
Get-Process keepmeawake | Select-Object Id, @{n='PrivateMB';e={[math]::Round($_.PrivateMemorySize64/1MB,1)}}, Handles, Threads

# Wake-ups: Resource Monitor → CPU → keepmeawake.exe, or
# Windows Performance Recorder, CPU usage profile, one idle minute.

# UI bundle
npm run ui:build   # Vite prints gzip sizes per chunk
```

## Rules that protect it

```mermaid
flowchart TD
    A["No setInterval faster than 1 s,<br/>and none unless a countdown is on screen"] --> keep["Budget holds"]
    B["No CSS animation that runs while idle;<br/>transitions on state change only"] --> keep
    C["Grain is a static filter;<br/>never animate baseFrequency"] --> keep
    D["Engine waits are wait_until(deadline) or wait();<br/>the loop never spins"] --> keep
    E["New crate or plugin: say what it adds<br/>to the binary and to idle memory"] --> keep
    F["Window is destroyed on close by default;<br/>changing that needs a measured reason"] --> keep
    G["A schedule waits for its next boundary,<br/>capped at 5 min because waits stop counting while Windows sleeps"] --> keep
```

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
