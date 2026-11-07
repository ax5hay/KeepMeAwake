# Handoff

The one page every agent or session reads first and updates last. Keep it
short; link out for detail.

**Last updated:** 2026-11-07 · by Claude Opus 5.5 (sunset warning built on the Mac, reviewed)
**Release:** [v0.2.0](https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.2.0), pre-release.
**Repo state:** `main` is green (`npm run verify` on the Mac: vitest 91, cargo test 157;
Windows runs its own platform tests in place of the Mac ones). **v0.2.0 is published as a
pre-release** with Windows (`.exe`, `.msi`) and macOS (`.dmg`, `.app.tar.gz`, Apple Silicon,
ad-hoc signed, not notarized) assets. Since then: **session history** (0.3) and the **sunset
warning** (0.4) are built on the Mac and under Unreleased in `CHANGELOG.md`. The project is under its own license, not MIT (see `LICENSE`).

## Two tracks, two machines

```mermaid
flowchart LR
    main["main (ax5hay/KeepMeAwake, public)"]
    win["Windows PC<br/>0.3: auto-update → winget"]
    mac["Mac<br/>macOS + history built; hand acceptance left"]
    win -->|small commits| main
    mac -->|small commits| main
    main -->|pull first| win
    main -->|pull first| mac
```

Pull before you start. Platform work stays in `platform/` and `cfg` blocks;
anything shared (`core`, `engine`, `copy.ts`, docs) is where the two tracks
can collide, so commit it small and push it soon.

## Where to start

### On the Mac (macOS track)

Setup is done on the Mac (rustup via Homebrew, librsvg). Two things
the next session needs that this one did not have:

1. `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"` in `~/.zshrc` (Homebrew's
   rustup is keg-only; without it `npm run verify` and the pre-commit hook
   cannot find cargo). This session could not edit `~/.zshrc`.
2. Screen Recording and Accessibility for the terminal (System Settings →
   Privacy & Security), so screenshots and menu bar clicks can be scripted.

Then walk the by-hand acceptance in [`specs/macos.md`](specs/macos.md):

| Item                                                             | Status                                                     |
| ---------------------------------------------------------------- | ---------------------------------------------------------- |
| `npm run verify` green on the Mac                                | done                                                       |
| `pmset -g assertions` shows both, gone on toggle off and on quit | done (release app, quit via Apple Event = ⌘Q path)         |
| No Dock icon                                                     | done (`lsappinfo` type UIElement)                          |
| Window drops down centred under the icon, reopens there          | **by hand**: placement fixed in review, unit tested only   |
| Icon legible on light, dark and busy menu bars, three states     | **by hand**                                                |
| `⌥⌘K` with window closed, record `⇧⌘K`, `⌘Space` refused         | **by hand** (`Cmd+Alt+K` registers, per the log)           |
| `⌘,` `⌘W` `⌘M` `⌘Q` in the real window                           | **by hand**; in a browser harness ⌘W/⌘M/⌘, route correctly |
| Timer toast from the bundled `.app`                              | **by hand**                                                |
| Launch at sign-in writes a LaunchAgent, starts hidden            | **by hand**                                                |
| Unplug below the threshold pauses (needs a MacBook)              | **by hand**; this Mac has no battery                       |
| Schedule window raises the sun                                   | **by hand**                                                |
| Paper and Night screenshots, `-mac` suffix, under `docs/assets/` | **by hand** (needs Screen Recording)                       |
| DMG background shows at 540 × 380 in Finder                      | **by hand**                                                |
| Resource numbers recorded                                        | done; menu bar footprint 16–17 MB is over the 12 MB budget |
| CI matrix green on both runners                                  | done                                                       |

### On the Windows PC (Windows track)

0. **History was rewritten on 2026-11-07**: every commit is now authored by
   the ax5hay GitHub account (code unchanged), and `v0.2.0` moved with it.
   On a clone made before that:
   `git fetch && git reset --hard origin/main && git fetch --tags --force`.
   Commit with the ax5hay GitHub noreply address as
   `user.email` (`git config --local`) so commits link to the account.
1. Pull: session history and the sunset warning landed (shared code). Run `npm run verify` on
   Windows once; the Rust count there differs from the Mac's because each OS
   runs its own platform tests. Then walk both acceptance lists on Windows
   (below) and tick them off under **Waiting for the other machine**.
2. Install the v0.2.0 pre-release from the releases page and check it
   once on Windows (toasts name the app when installed, per the old open
   question). When both platforms are happy, untick **pre-release** on GitHub.
3. Then [`specs/auto-update.md`](specs/auto-update.md) at commit 1. The
   updater needs only Tauri's own free key pair (`tauri signer generate`,
   stored as `TAURI_SIGNING_PRIVATE_KEY`); a paid OS certificate removes the
   install warnings but is not a prerequisite. Check the spec still agrees.
4. `taskkill /IM keepmeawake.exe /F` before testing a dev build;
   `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` before
   `npm run dev` lets you drive the UI over the DevTools protocol.

## Waiting for the other machine

Built on one machine, not yet built and checked on the other. A row goes
when the other machine has run `npm run verify` and the acceptance items that
apply there (see [`development.md`](development.md#two-machines-one-main)).

| What            | Built on | Waits for | Check there                                                                    |
| --------------- | -------- | --------- | ------------------------------------------------------------------------------ |
| macOS track     | Mac      | Windows   | `npm run verify`; the Windows app still behaves as 0.2 (tray, toasts, hotkey)  |
| Session history | Mac      | Windows   | `npm run verify`; the history acceptance (below) on Windows                    |
| Sunset warning  | Mac      | Windows   | `npm run verify`; the sunset acceptance (below) on Windows, tray item included |

## In progress

| Spec                     | Track   | Commit reached | Notes                                                                                                           |
| ------------------------ | ------- | -------------- | --------------------------------------------------------------------------------------------------------------- |
| schedule                 | Windows | 7 of 7         | done; shipped in 0.2.0                                                                                          |
| notifications            | Windows | 5 of 5         | done; battery toasts verified by unit tests only                                                                |
| global-hotkey            | Windows | 3 of 3         | done                                                                                                            |
| tray-icons-light-taskbar | Windows | 4 of 4         | done; paused-state ink icon verified by unit test only                                                          |
| macos                    | macOS   | 12 of 12       | built, reviewed, four review fixes in; shipped in 0.2.0 as a pre-release; hand acceptance above is what is left |
| auto-update              | Windows | 0 of 6         | ready: needs the free updater key                                                                               |
| session-history          | shared  | 5 of 5         | built on the Mac, reviewed, two review fixes in; acceptance below                                               |
| sunset-warning           | shared  | 6 of 6         | spec written and built on the Mac, reviewed, two review fixes in; acceptance below                              |

## Done this cycle

| Date       | What                                                                                                 | By               |
| ---------- | ---------------------------------------------------------------------------------------------------- | ---------------- |
| 2026-11-07 | 0.1.0 built end to end; docs, CI, specs for 0.2 and 0.3; roadmap to 0.5                              | Claude Fable 5.1 |
| 2026-11-07 | 0.2 built: schedule, notifications, global hotkey, taskbar-aware tray icons; engine wake-up fix      | Claude Opus 5.5  |
| 2026-11-07 | Repo moved to `ax5hay` and made public; every link updated; bundle metadata made honest              | Claude Fable 5.1 |
| 2026-11-07 | macOS scoped: `specs/macos.md`, Mac setup in `development.md`, two-track handoff                     | Claude Fable 5.1 |
| 2026-11-07 | macOS built (12 commits), reviewed and fixed; own license; GitHub repo extras; v0.2.0 pre-release    | Claude Opus 5.5  |
| 2026-11-07 | Session history built (5 commits) on the Mac, reviewed; stretches end when due across sleep          | Claude Opus 5.5  |
| 2026-11-07 | Sunset warning: spec, built (6 commits) on the Mac, reviewed; two-machine rule written into the docs | Claude Opus 5.5  |

## Decisions made on the macOS track

Each is in its commit body; the ones a person might disagree with:

- Mac glyphs follow menu order, so the default shows as `⌥⌘K` (spec: `⌘⌥K`).
- The Mac Esc tip says "Back, or close the window" (the spec's text wraps).
- No `[data-platform]` letter-spacing: SF Pro is ~14% wider than Segoe, so
  long hints wrap to two lines instead of being squeezed.
- The bundle is signed ad hoc (`signingIdentity: "-"`): an unsigned one reads
  as "damaged" when downloaded on Apple Silicon.
- Control-click on the menu bar icon opens the window, not the menu
  (tray-icon reports it as a left click). Right click shows the menu.
- Clicking the icon while the window is open focuses it rather than hiding it.

## Session history: acceptance and what is deferred

| Acceptance (spec)                                         | Status                                                                 |
| --------------------------------------------------------- | ---------------------------------------------------------------------- |
| 30 m timer ends: a `timer` row with the right span        | done on the real app (60 s startup timer → `timer`, 60.01 s)           |
| Unplug below threshold, plug in, stop: one row, one pause | **by hand** on a laptop; unit tested                                   |
| Kill while Always, relaunch: a `sleep` row, end `–`       | done on the real app (`kill -9`, relaunch)                             |
| Clear history empties the list and the file               | **by hand** (needs a click); command and store unit tested             |
| Screenshots of Settings → History, Paper and Night        | **by hand** from the real window; checked in a browser at 340 / 380 px |

Decisions, each in its commit body: History sits after Hotkey (the spec
predates that section); `battery` applies to any stretch that ends while
paused for the battery; `SessionStarted` carries `paused` and both events
carry the engine's time; `Engine::start_with` attaches the listener before
the first step; a stretch ended by its timer or window is logged as ending
when it was due, even if the machine slept through it.

Deferred from review, all minor:

- A machine that sleeps and wakes inside the next window shows one row
  across both days; needs the 0.4 sleep detection.
- Turning the schedule off (or moving its hours) mid-window logs `schedule`,
  not `you`.
- The list does not refresh while Settings stays open, and a running row
  shows `–` for its length instead of a live one.
- The rows are rebuilt on each 1 Hz tick while a countdown runs, even with
  Settings off screen (cheap, but wasted).
- An unreadable `history.json` (corrupt, or from a newer build) is replaced
  on the next save without a backup; `Entry` has no `serde(default)` yet.

## Sunset warning: acceptance and what is deferred

| Acceptance (spec)                                             | Status                                                                                       |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| 6-minute timer: at 5 left a toast, the status line, the link  | engine event checked on the real app (320 s timer warns at 20 s); toast and tray **by hand** |
| Give it another 30m from the window moves the end, re-warns   | **by hand**; engine, store and harness click tested                                          |
| The same from the tray                                        | **by hand**                                                                                  |
| 2-minute timer never warns; Off never warns; timer switch off | unit tested; **by hand** on the real app                                                     |

Decisions (each in its commit body): the toast follows the existing
**When a timer ends** switch; no warning while an open schedule window
outlasts the timer; `Session::extend` keeps the plan's start, so the arc
grows (progress jumps back) rather than restarting.

Deferred from review, all minor: the link also shows while paused ("Holding
off while … Give it another 30m"); the click always whispers "Another 30
minutes." and keyboard focus drops to the page; a hand-edited warning
outside 2/5/10 selects no segment; under a minute the toast says "1 minute"
while the line says "under a minute".

## Open questions for Akshay

- Paid signing, to drop the first-open warnings: Apple Developer Program
  ($99 a year, includes notarization) and a Windows certificate (Azure
  Artifact Signing about $10 a month, but for individuals only in the US or
  Canada; otherwise an OV certificate, roughly $130–440 a year on a hardware
  token or cloud HSM). Neither blocks auto-update. Prices checked 2026-11-07.
- The macOS menu bar footprint (16–17 MB `phys_footprint`) is over the 12 MB
  budget written for Windows. A macOS line in the budget, or work to cut it?
- Should a second click on the menu bar icon hide the window, like a popover?
- Upload `design/social-preview.png` under GitHub → Settings → General →
  Social preview (the API cannot).
- The new `LICENSE` is a plain-words personal license, not legal advice; if
  the project ever earns money, have a lawyer read it.
- Confirm the default for **Check for updates daily** (auto-update spec says on).

## How to update this page

At the end of every session: set **Last updated**, move your spec's row in
**In progress** (commit number reached, anything half-done), append to **Done
this cycle**, add what you built to **Waiting for the other machine** (and
tick off what you checked from the other side), and rewrite **Where to
start** for both machines so either can begin in one step. Commit it as `docs(handoff): …` and push.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
