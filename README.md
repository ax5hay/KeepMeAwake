# Keep me awake

A small Windows and macOS app that keeps my laptop from sleeping while I need
it up, and gets out of the way the rest of the time. By [Akshay Bajpai](https://akshaybajpai.com).

It holds a Windows **power request** (on a Mac, an IOKit **power
assertion**), the same mechanism a video player uses, instead of faking
keystrokes. It shows up by name in `powercfg /requests` or
`pmset -g assertions`, and is released the instant it is turned off or the
app exits. Nothing is typed, nothing moves, and nothing fights you when you
actually want the machine to sleep.

![Resting, Paper theme](docs/assets/resting-light.png) ![Awake for two hours, Night theme](docs/assets/home-dark.png)

## At a glance

| Area      | What you get                                                                                                                |
| --------- | --------------------------------------------------------------------------------------------------------------------------- |
| Control   | One sun. Click it, press Space, or use the tray (the menu bar on a Mac).                                                    |
| Hotkey    | `Ctrl+Alt+K` (`⌥⌘K` on a Mac) toggles from any app; record your own.                                                        |
| Time      | 30m · 1h · 2h · 4h · 8h · until a time of day. The sun sets as the time runs out.                                           |
| Schedule  | Awake during chosen hours on chosen days, resting outside them. Click the sun to skip a day.                                |
| Safety    | Pauses below a battery level (20% by default) or whenever unplugged; resumes when plugged in.                               |
| Screen    | Keep the display on, or let it dim while the machine keeps running.                                                         |
| Tray      | Left click opens the window. Right click does everything else. Starts at sign-in, quietly. Follows a light or dark taskbar. |
| Menu bar  | On a Mac: no Dock icon; the window drops down beneath the sun. Template icons, so macOS tints them.                         |
| Toasts    | Optional: when a timer ends, the battery pauses or resumes, or a schedule starts or ends.                                   |
| Footprint | One native process of about 5 MB while in the tray. No polling. The UI exists only while it is open.                        |
| Look      | Paper and Night, following the system. A horizon, a sun, a serif, and nothing else.                                         |

## How it works

```mermaid
flowchart LR
    You(("You")) -->|sun · tray · Space| UI["Svelte UI<br/>(WebView2, only while open)"]
    You -->|Ctrl+Alt+K| Engine
    You -->|right click| Tray["Tray icon"]
    UI -->|invoke| Engine["Engine thread<br/>(Rust)"]
    Tray --> Engine
    Engine -->|PowerSetRequest| Win["Windows power manager"]
    Engine -->|GetSystemPowerStatus| Bat["Battery"]
    Engine -->|snapshot| UI
    Engine -->|snapshot| Tray
```

The engine owns one `Session` (off, indefinite, or timed) and applies a policy
each time something changes: if the session is active and the battery rules
allow it, hold a system request, plus a display request when "Keep the screen
on" is set. Everything the UI and tray show comes from a single `Snapshot` the
engine produces. The full picture is in [docs/architecture.md](docs/architecture.md).

## Install

Grab the latest installer from the
[releases page](https://github.com/ax5hay/KeepMeAwake/releases). It is
per-user and needs no administrator rights. Windows 10 (1809+) or Windows 11.

On a Mac (macOS 12+, Apple Silicon), open the `.dmg` and drag the sun to
Applications. The build is not notarized yet, so open it the first time with
right-click → **Open**; the [field guide](docs/guide.md#installing-on-a-mac)
has the details.

Or build it from source: see [Development](#development).

## Use

The [field guide](docs/guide.md) covers every control, including the things
that are not labelled. The short version:

| Want to…                        | Do…                                        |
| ------------------------------- | ------------------------------------------ |
| keep the laptop awake           | click the sun, or press **Space**          |
| for a fixed time                | pick a stop on the rule under the status   |
| until a time of day             | pick **Until…** and type a time            |
| let it sleep again              | click the sun again                        |
| keep it up during working hours | Settings → **Schedule**                    |
| toggle from another app         | **Ctrl+Alt+K** (**⌥⌘K** on a Mac)          |
| get the window out of the way   | **Esc** or ×; the sun stays up in the tray |
| do all of this without a window | right-click the sun in the tray            |
| change defaults                 | gear, or **Ctrl+,** (**⌘,** on a Mac)      |

Settings live at `%APPDATA%\dev.bajpai.keepmeawake\settings.json`, logs at
`%LOCALAPPDATA%\dev.bajpai.keepmeawake\logs`. On a Mac:
`~/Library/Application Support/dev.bajpai.keepmeawake/` and
`~/Library/Logs/dev.bajpai.keepmeawake/`.

## Why not just press Alt every fifteen minutes

That is how this started; the script is in [`legacy/`](legacy/keepmeawake.py).
It works, but it injects input into whatever has focus, it has to run visibly
in a console, it cannot be turned off from the tray, and Windows has no idea
why the laptop never sleeps. A power request is the API Windows provides for
exactly this: explicit, inspectable, self-cleaning.

## Development

Prerequisites: Node 22+, Rust stable 1.90+, and the MSVC build tools with a
Windows SDK or, on a Mac, the Xcode Command Line Tools. The
[development guide](docs/development.md) has setup for both, the daily
commands and the conventions this repo follows.

```powershell
npm install        # also wires the git hooks
npm run dev        # Vite + Tauri dev build; UI hot-reloads
npm run verify     # lint, type-check and test both halves
npm run build      # installers (or .app and .dmg) in src-tauri/target/release/bundle
```

## Documentation

| Doc                                        | Read it for                                         |
| ------------------------------------------ | --------------------------------------------------- |
| [Field guide](docs/guide.md)               | every control, shortcut and hidden thing            |
| [Architecture](docs/architecture.md)       | layers, the engine loop, IPC, window lifecycle      |
| [Design](docs/design.md)                   | the horizon, palette, type, motion, and the reasons |
| [Roadmap](docs/roadmap.md)                 | what is next and what was decided against           |
| [Development](docs/development.md)         | setup, commands, style, testing, commits            |
| [Releasing](docs/releasing.md)             | versions, changelog, the release workflow           |
| [Resource budget](docs/resource-budget.md) | what "tiny" means and how it is kept                |
| [Decisions](docs/adr/README.md)            | the five choices that shaped the project            |
| [Specs](docs/specs/README.md)              | implementation specs for the next releases          |
| [Handoff](docs/HANDOFF.md)                 | where the last session stopped and where to start   |
| [Changelog](CHANGELOG.md)                  | what changed, by version                            |

## License

Keep me awake is a personal project by
[Akshay Bajpai](https://akshaybajpai.com), under its own
[license](LICENSE): all rights reserved. You may use the published builds and
read and build the source for yourself; copying, redistributing or publishing
changed versions needs his permission. The source is public so it can be
checked, not so it can be reused.
