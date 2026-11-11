<div align="center">

<a href="https://github.com/ax5hay/KeepMeAwake/releases/latest">
  <img src="docs/assets/cover.png" alt="Five Keep me awake windows on one horizon: Paper awake, Night resting, the Almanac in Risograph, Mars, and Thermal" width="100%" />
</a>

<br/><br/>

# Keep me awake

**A sun for your laptop. Click it and your laptop stays awake; click it again and it sleeps as usual.**

It asks Windows or macOS to stay awake, the same way a video player does.<br/>
No fake keypresses, no mouse jiggling, and it stops the moment you turn it off.

<br/>

[![Download the latest release](https://img.shields.io/badge/download-the_latest_release-1c1b18?style=for-the-badge&labelColor=dca544)](https://github.com/ax5hay/KeepMeAwake/releases/latest)

[![CI](https://github.com/ax5hay/KeepMeAwake/actions/workflows/ci.yml/badge.svg)](https://github.com/ax5hay/KeepMeAwake/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/ax5hay/KeepMeAwake?style=flat-square&label=release&color=dca544&labelColor=1c1b18)](https://github.com/ax5hay/KeepMeAwake/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/ax5hay/KeepMeAwake/total?style=flat-square&color=5f5b52&labelColor=1c1b18)](https://github.com/ax5hay/KeepMeAwake/releases)
![Windows 10 and 11](https://img.shields.io/badge/Windows-10_%C2%B7_11-0078D4?style=flat-square)
![macOS 12+](https://img.shields.io/badge/macOS-12%2B_%C2%B7_Apple_Silicon-1c1b18?style=flat-square&logo=apple)
![Tauri 2](https://img.shields.io/badge/Tauri_2-24C8D8?style=flat-square&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1c1b18?style=flat-square&logo=rust)
![Svelte 5](https://img.shields.io/badge/Svelte_5-FF3E00?style=flat-square&logo=svelte&logoColor=white)
![Fake keypresses](https://img.shields.io/badge/fake_keypresses-none-983729?style=flat-square)
![At rest](https://img.shields.io/badge/in_the_Windows_tray-about_5_MB-2d6a4f?style=flat-square)
![Themes](https://img.shields.io/badge/themes-27_and_your_own-dca544?style=flat-square&labelColor=1c1b18)
[![License](https://img.shields.io/badge/license-own,_see_LICENSE-5f5b52?style=flat-square)](LICENSE)

**Jump to ·**
[Install](#install) ·
[A tour](#a-tour-in-pictures) ·
[Themes](#themes) ·
[How it works](#how-it-works) ·
[Docs](#documentation)

</div>

<br/>

## Contents

| Section                                   | What it covers                                       |
| :---------------------------------------- | :--------------------------------------------------- |
| [The idea](#the-idea)                     | Why it asks the system, and why a sun                |
| [At a glance](#at-a-glance)               | Everything it does, in one table                     |
| [Install](#install)                       | Windows and macOS, and what the installer looks like |
| [A tour in pictures](#a-tour-in-pictures) | Home, the Almanac, Settings, History, AI, updates    |
| [Themes](#themes)                         | All twenty-six palettes, each on the same horizon    |
| [How it works](#how-it-works)             | The engine, the sun's states, one click end to end   |
| [What it never does](#what-it-never-does) | The promises, and where each is kept                 |
| [Use](#use)                               | The short version of the field guide                 |
| [Development](#development)               | Build it, test it, where things live                 |
| [Documentation](#documentation)           | Every doc, and what to read it for                   |

---

## The idea

This started as a script that pressed a key every few minutes; it is still in
[`legacy/`](legacy/keepmeawake.py). It worked, and it was wrong: it typed into whatever had
focus, it ran in a console, and Windows had no idea why the laptop never slept.

Keep me awake holds a Windows **power request** (on a Mac, an IOKit **power assertion**):
the API the system provides for exactly this. It shows up by name in `powercfg /requests` or
`pmset -g assertions`, and it is released the instant you turn the sun off or quit the app.

<table>
<tr>
<td width="33%" valign="top">

### One sun

The whole app is one button: a sun on a horizon. While the sun is up, your laptop stays
awake. Click it and the sun goes down, and your laptop sleeps as usual. Set a timer and you
can watch the sun sink as the time runs out.

</td>
<td width="33%" valign="top">

### Out of the way

One small process in the tray (the menu bar on a Mac), about 5 MB on Windows, that never
polls. The window only exists while it is open, and is freed when you close it.

</td>
<td width="33%" valign="top">

### Yours

No account and no tracking. Nothing goes over the network unless it is checking for updates
or you ask an AI feature something. What it records about your days stays in a file on your
machine. AI is off until you turn it on, with your own key.

</td>
</tr>
</table>

## At a glance

| Area               | What you get                                                                                                                                                                                                                                            |
| :----------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Control            | One sun. Click it, press <kbd>Space</kbd>, or use the tray (the menu bar on a Mac).                                                                                                                                                                     |
| Hotkey             | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>K</kbd> (<kbd>⌥</kbd><kbd>⌘</kbd><kbd>K</kbd> on a Mac) turns it on or off from any app; you can pick your own keys.                                                                                                |
| Time               | Stay awake for 30m · 1h · 2h · 4h · 8h, or until a time of day. A few minutes before the time runs out it warns you, so you can add 30 minutes.                                                                                                         |
| Schedule           | Stays awake during set hours on set days; outside them the laptop sleeps as usual.                                                                                                                                                                      |
| Battery            | Stops keeping it awake below a battery level (20% by default), or whenever unplugged; starts again when you plug in.                                                                                                                                    |
| Overnight          | Left on **Always** with no input for hours, it asks if you are still there, then lets the laptop sleep. If you like, it can also let the laptop sleep when you leave for the night and turn back on when you return.                                    |
| Almanac            | A diary it keeps for you: the hours you worked each day, extra hours, days off, and when you were away overnight, worked out from when you used the keyboard and mouse.                                                                                 |
| AI, if you want it | A sentence about your day, a few about your week, answers to questions about your month, and timers typed in plain words ("until 6:30 pm"). Claude, OpenAI or any compatible service, with your own key. Off by default.                                |
| Themes             | Paper and Night, plus twenty-four more: colour schemes from code editors, places, and loud ones. Or make your own.                                                                                                                                      |
| Small things       | A sun drawn to suit its theme, tonight's moon, your own theme to share as one line of text, a separate evening theme, a mini window, a year in review, a printable month, and `?` for all keyboard shortcuts. [All twelve](docs/guide.md#small-things). |
| Tray               | Left click opens the window; right click opens a menu with everything else. Starts quietly when you sign in.                                                                                                                                            |
| Updates            | Checks GitHub once a day and when you ask; installs only when you say.                                                                                                                                                                                  |
| Footprint          | About 5 MB in the tray on Windows, 16 MB on a Mac. By default, the window takes no memory while it is closed.                                                                                                                                           |

## Install

| System                     | Get                                                                                                               | Notes                                                                                 |
| :------------------------- | :---------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------ |
| Windows 10 (1809+) and 11  | the `-setup.exe` from the [latest release](https://github.com/ax5hay/KeepMeAwake/releases/latest) (or the `.msi`) | per user, no administrator rights                                                     |
| macOS 12+ on Apple Silicon | the `.dmg`; drag the sun to Applications                                                                          | not notarized yet, so macOS asks once: [the steps](docs/guide.md#installing-on-a-mac) |
| From source                | [Development](#development)                                                                                       | Node 22+, Rust 1.90+                                                                  |

The Windows setup uses the same horizon as the app, and says plainly what it is installing:

<p align="center">
  <img src="docs/assets/installer-welcome.png" alt="The Windows setup's welcome page: the horizon down the left, and a paper-coloured page that says what Keep me awake does" width="62%"/>
</p>

## A tour in pictures

Every pane, in Paper and in Night.

### Home

The sun is the switch. Under it: today's hours as a strip, what the laptop is doing right
now, a row of lengths to choose from (30m, 1h, … Until…), and two switches for the screen and
the battery.

<img src="docs/assets/home-light.png" alt="Home, Paper" width="48%"/> <img src="docs/assets/resting-dark.png" alt="Home, Night" width="48%"/>

A schedule keeps it awake during your hours. Left on overnight with no one around, it asks
if you are still there before it lets the laptop sleep.

<img src="docs/assets/schedule-home-light.png" alt="Home on a schedule, and asking overnight, Paper" width="48%"/> <img src="docs/assets/overnight-home-dark.png" alt="Home on a schedule, and asking overnight, Night" width="48%"/>

### The Almanac

The open book in the title bar. It records your working hours, days off and nights away,
from when you used the keyboard and mouse; there is nothing to start or stop. Weekends count
as days off, a missed working day counts as leave, and a weekday you never work stops
counting as leave on its own.

<img src="docs/assets/almanac-light.png" alt="The Almanac, Paper" width="48%"/> <img src="docs/assets/almanac-dark.png" alt="The Almanac, Night" width="48%"/>

The month is a calendar: a bar under each day worked, a ring for leave, the sun on today.
You can correct any day by hand.

<img src="docs/assets/almanac-month-light.png" alt="The month, and correcting a day, Paper" width="48%"/> <img src="docs/assets/almanac-correct-dark.png" alt="The month, and correcting a day, Night" width="48%"/>

### Settings

Everything has a sensible default, so you can click the sun without setting anything first.

<img src="docs/assets/overnight-settings-light.png" alt="Settings, Paper" width="48%"/> <img src="docs/assets/schedule-settings-dark.png" alt="Settings, Night" width="48%"/>

<img src="docs/assets/almanac-settings-light.png" alt="Almanac settings and the hotkey, Paper" width="48%"/> <img src="docs/assets/hotkey-dark.png" alt="Almanac settings and the hotkey, Night" width="48%"/>

### Small things

Extras you never need but might enjoy: suns drawn to match their theme, the
moon as it is tonight, a theme of your own, all keyboard shortcuts on one
sheet, the credits, and a year in review:

<img src="docs/assets/small-things.png" alt="Eight windows: a pixel sun, a ring sun, a halftone sun, the moon over Night, a theme of your own, the keyboard shortcuts, the credits, and a year in review" width="100%"/>

The mini window shows just the sun, above your other windows, and a month prints as a
timesheet:

<img src="docs/assets/small-mini-print.png" alt="The mini window, and a month laid out for the printer" width="100%"/>

### History and notifications

The last thirty times it kept your laptop awake, and notifications for the moments you choose.

<img src="docs/assets/history-light.png" alt="History and notifications, Paper" width="48%"/> <img src="docs/assets/notifications-dark.png" alt="History and notifications, Night" width="48%"/>

### AI, as a choice

Nothing is sent until three things are true, and you can see each one: you added a key of
your own, the main AI switch is on, and that feature's own switch is on. Every feature says
what it sends, and to whom, before you click.

<img src="docs/assets/claude-settings-light.png" alt="AI settings, and a week in words, Paper" width="48%"/> <img src="docs/assets/claude-week-dark.png" alt="AI settings, and a week in words, Night" width="48%"/>

<img src="docs/assets/claude-tell-light.png" alt="A plan said in a sentence, and a feature shown locked, Paper" width="48%"/> <img src="docs/assets/claude-locked-light.png" alt="A plan said in a sentence, and a feature shown locked, Night" width="48%"/>

### Updates

<img src="docs/assets/update-about-light.png" alt="Checking for updates, Paper" width="48%"/> <img src="docs/assets/update-home-dark.png" alt="Checking for updates, Night" width="48%"/>

## Themes

Settings → Theme. A theme changes the colours and nothing else: everything stays where it
is, and the sun is still the one bright spot. A test checks every theme, so the text is
readable and the sun stands out in all of them.

<p align="center">
  <img src="docs/assets/themes-picker-light.png" alt="The theme picker: four shelves of small horizons, one per theme" width="38%"/>
  <img src="docs/assets/themes-picker-dark.png" alt="The same picker, shown in Deep Sea" width="38%"/>
</p>

### Standard

Paper for daytime, Night for after dark. **System** follows your light or dark mode,
switching between the two whenever Windows or macOS does.

<table>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/light.png" alt="Home in the Paper theme" width="100%"/><br/><b>Paper</b><br/><sub>warm paper, one sun</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/dark.png" alt="Home in the Night theme" width="100%"/><br/><b>Night</b><br/><sub>the same horizon after dark</sub></td>
</tr>
</table>

### Editors

Colour schemes from code editors, which you may already know.

<table>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/solarized_light.png" alt="Home in the Solarized Light theme" width="100%"/><br/><b>Solarized Light</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/solarized_dark.png" alt="Home in the Solarized Dark theme" width="100%"/><br/><b>Solarized Dark</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/gruvbox.png" alt="Home in the Gruvbox theme" width="100%"/><br/><b>Gruvbox</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/nord.png" alt="Home in the Nord theme" width="100%"/><br/><b>Nord</b></td>
</tr>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/dracula.png" alt="Home in the Dracula theme" width="100%"/><br/><b>Dracula</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/catppuccin.png" alt="Home in the Catppuccin theme" width="100%"/><br/><b>Catppuccin</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/tokyo_night.png" alt="Home in the Tokyo Night theme" width="100%"/><br/><b>Tokyo Night</b></td>
<td align="center" width="25%"><img src="docs/assets/themes/monokai.png" alt="Home in the Monokai theme" width="100%"/><br/><b>Monokai</b></td>
</tr>
</table>

### Places

The same horizon, somewhere else: an old terminal, a blueprint, a handheld game, the sea floor.

<table>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/terminal.png" alt="Home in the Terminal theme" width="100%"/><br/><b>Terminal</b><br/><sub>green phosphor, amber sun</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/blueprint.png" alt="Home in the Blueprint theme" width="100%"/><br/><b>Blueprint</b><br/><sub>white line on cyanotype</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/pocket.png" alt="Home in the Pocket theme" width="100%"/><br/><b>Pocket</b><br/><sub>four greens of a handheld</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/newsprint.png" alt="Home in the Newsprint theme" width="100%"/><br/><b>Newsprint</b><br/><sub>black on white, a red sun</sub></td>
</tr>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/risograph.png" alt="Home in the Risograph theme" width="100%"/><br/><b>Risograph</b><br/><sub>two inks on cream</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/dial_up.png" alt="Home in the Dial-up theme" width="100%"/><br/><b>Dial-up</b><br/><sub>grey panels, teal desktop sky</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/mars.png" alt="Home in the Mars theme" width="100%"/><br/><b>Mars</b><br/><sub>where the sun sets blue</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/deep_sea.png" alt="Home in the Deep Sea theme" width="100%"/><br/><b>Deep Sea</b><br/><sub>something glowing below</sub></td>
</tr>
</table>

### Loud

Bright and bold, on purpose.

<table>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/vaporwave.png" alt="Home in the Vaporwave theme" width="100%"/><br/><b>Vaporwave</b><br/><sub>pink and cyan on violet</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/thermal.png" alt="Home in the Thermal theme" width="100%"/><br/><b>Thermal</b><br/><sub>as a heat camera sees it</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/negative.png" alt="Home in the Negative theme" width="100%"/><br/><b>Negative</b><br/><sub>film: orange base, cyan sun</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/miami.png" alt="Home in the Miami theme" width="100%"/><br/><b>Miami</b><br/><sub>aqua, sand, a flamingo sun</sub></td>
</tr>
<tr>
<td align="center" width="25%"><img src="docs/assets/themes/bubblegum.png" alt="Home in the Bubblegum theme" width="100%"/><br/><b>Bubblegum</b><br/><sub>pink, all of it</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/highlighter.png" alt="Home in the Highlighter theme" width="100%"/><br/><b>Highlighter</b><br/><sub>marker yellow, pink sun</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/grape_soda.png" alt="Home in the Grape Soda theme" width="100%"/><br/><b>Grape Soda</b><br/><sub>purple, a lime sun</sub></td>
<td align="center" width="25%"><img src="docs/assets/themes/hi_vis.png" alt="Home in the Hi-Vis theme" width="100%"/><br/><b>Hi-Vis</b><br/><sub>safety orange over asphalt</sub></td>
</tr>
</table>

## How it works

Rust owns the behaviour; the Svelte window is a view over one `Snapshot`. The full picture
is in [docs/architecture.md](docs/architecture.md).

### The parts

```mermaid
%%{init: {"theme": "base", "themeVariables": {"fontFamily": "Georgia, serif", "primaryColor": "#f4f0e7", "primaryTextColor": "#1c1b18", "primaryBorderColor": "#1c1b18", "lineColor": "#9a948a", "secondaryColor": "#ebe6da", "tertiaryColor": "#dfe8ea", "clusterBkg": "#ebe6da", "clusterBorder": "#cdb98c", "edgeLabelBackground": "#f4f0e7", "actorBkg": "#f4f0e7", "actorBorder": "#1c1b18", "actorTextColor": "#1c1b18", "signalColor": "#9a948a", "signalTextColor": "#9a948a", "labelBoxBkgColor": "#ebe6da", "labelBoxBorderColor": "#cdb98c", "labelTextColor": "#1c1b18", "loopTextColor": "#9a948a", "noteBkgColor": "#dca544", "noteTextColor": "#1c1b18", "noteBorderColor": "#1c1b18", "activationBkgColor": "#dca544", "activationBorderColor": "#1c1b18", "transitionColor": "#9a948a", "transitionLabelColor": "#9a948a", "stateLabelColor": "#1c1b18", "stateBkg": "#f4f0e7", "compositeBackground": "#ebe6da", "altBackground": "#ebe6da", "compositeTitleBackground": "#dfe8ea", "innerEndBackground": "#1c1b18", "specialStateColor": "#dca544"}}}%%
flowchart LR
    You(("You")):::sun
    subgraph app ["One native process"]
        Tray["Tray icon"]
        Engine["Engine thread<br/>one Session, one policy"]
        Keeper["Almanac keeper<br/>one look a minute"]
    end
    UI["Svelte window<br/>exists only while open"]:::sky
    OS["Power manager<br/>Windows · macOS"]
    Battery["Battery"]
    Idle["Last input time<br/>read only"]

    You -->|sun · Space| UI
    You -->|right click| Tray
    You -->|hotkey| Engine
    UI -->|invoke| Engine
    Tray --> Engine
    Engine -->|hold · release| OS
    Battery -->|level · plugged| Engine
    Idle --> Engine
    Idle --> Keeper
    Engine -->|Snapshot| UI
    Engine -->|Snapshot| Tray
    classDef sun fill:#dca544,stroke:#1c1b18,color:#1c1b18
    classDef sky fill:#dfe8ea,stroke:#1c1b18,color:#1c1b18
```

### The sun's states

```mermaid
%%{init: {"theme": "base", "themeVariables": {"fontFamily": "Georgia, serif", "primaryColor": "#f4f0e7", "primaryTextColor": "#1c1b18", "primaryBorderColor": "#1c1b18", "lineColor": "#9a948a", "secondaryColor": "#ebe6da", "tertiaryColor": "#dfe8ea", "clusterBkg": "#ebe6da", "clusterBorder": "#cdb98c", "edgeLabelBackground": "#f4f0e7", "actorBkg": "#f4f0e7", "actorBorder": "#1c1b18", "actorTextColor": "#1c1b18", "signalColor": "#9a948a", "signalTextColor": "#9a948a", "labelBoxBkgColor": "#ebe6da", "labelBoxBorderColor": "#cdb98c", "labelTextColor": "#1c1b18", "loopTextColor": "#9a948a", "noteBkgColor": "#dca544", "noteTextColor": "#1c1b18", "noteBorderColor": "#1c1b18", "activationBkgColor": "#dca544", "activationBorderColor": "#1c1b18", "transitionColor": "#9a948a", "transitionLabelColor": "#9a948a", "stateLabelColor": "#1c1b18", "stateBkg": "#f4f0e7", "compositeBackground": "#ebe6da", "altBackground": "#ebe6da", "compositeTitleBackground": "#dfe8ea", "innerEndBackground": "#1c1b18", "specialStateColor": "#dca544"}}}%%
stateDiagram-v2
    direction LR
    [*] --> Resting
    Resting --> Awake: click · Space · hotkey
    Awake --> Resting: click again
    Awake --> Resting: the timer runs out
    Awake --> Paused: battery low, or unplugged
    Paused --> Awake: plugged in again
    Paused --> Resting: click
    Awake --> Asking: overnight, nobody there
    Asking --> Awake: still here
    Asking --> Resting: no answer
    Resting --> Awake: a schedule window opens
```

### One click, end to end

```mermaid
%%{init: {"theme": "base", "themeVariables": {"fontFamily": "Georgia, serif", "primaryColor": "#f4f0e7", "primaryTextColor": "#1c1b18", "primaryBorderColor": "#1c1b18", "lineColor": "#9a948a", "secondaryColor": "#ebe6da", "tertiaryColor": "#dfe8ea", "clusterBkg": "#ebe6da", "clusterBorder": "#cdb98c", "edgeLabelBackground": "#f4f0e7", "actorBkg": "#f4f0e7", "actorBorder": "#1c1b18", "actorTextColor": "#1c1b18", "signalColor": "#9a948a", "signalTextColor": "#9a948a", "labelBoxBkgColor": "#ebe6da", "labelBoxBorderColor": "#cdb98c", "labelTextColor": "#1c1b18", "loopTextColor": "#9a948a", "noteBkgColor": "#dca544", "noteTextColor": "#1c1b18", "noteBorderColor": "#1c1b18", "activationBkgColor": "#dca544", "activationBorderColor": "#1c1b18", "transitionColor": "#9a948a", "transitionLabelColor": "#9a948a", "stateLabelColor": "#1c1b18", "stateBkg": "#f4f0e7", "compositeBackground": "#ebe6da", "altBackground": "#ebe6da", "compositeTitleBackground": "#dfe8ea", "innerEndBackground": "#1c1b18", "specialStateColor": "#dca544"}}}%%
sequenceDiagram
    actor You
    participant UI as Window
    participant Engine
    participant OS as Power manager
    You->>UI: click the sun
    UI->>Engine: start(Always)
    Engine->>Engine: policy: session on, battery allows
    Engine->>OS: hold a system request (and display)
    Note over OS: listed by name in powercfg /requests
    Engine-->>UI: Snapshot
    UI-->>You: the sun rises
    You->>UI: click again
    UI->>Engine: stop
    Engine->>OS: release
    Engine-->>UI: Snapshot
```

### The layers

```mermaid
%%{init: {"theme": "base", "themeVariables": {"fontFamily": "Georgia, serif", "primaryColor": "#f4f0e7", "primaryTextColor": "#1c1b18", "primaryBorderColor": "#1c1b18", "lineColor": "#9a948a", "secondaryColor": "#ebe6da", "tertiaryColor": "#dfe8ea", "clusterBkg": "#ebe6da", "clusterBorder": "#cdb98c", "edgeLabelBackground": "#f4f0e7", "actorBkg": "#f4f0e7", "actorBorder": "#1c1b18", "actorTextColor": "#1c1b18", "signalColor": "#9a948a", "signalTextColor": "#9a948a", "labelBoxBkgColor": "#ebe6da", "labelBoxBorderColor": "#cdb98c", "labelTextColor": "#1c1b18", "loopTextColor": "#9a948a", "noteBkgColor": "#dca544", "noteTextColor": "#1c1b18", "noteBorderColor": "#1c1b18", "activationBkgColor": "#dca544", "activationBorderColor": "#1c1b18", "transitionColor": "#9a948a", "transitionLabelColor": "#9a948a", "stateLabelColor": "#1c1b18", "stateBkg": "#f4f0e7", "compositeBackground": "#ebe6da", "altBackground": "#ebe6da", "compositeTitleBackground": "#dfe8ea", "innerEndBackground": "#1c1b18", "specialStateColor": "#dca544"}}}%%
flowchart TB
    app["app<br/>tray · commands · window · keeper · updater"]
    engine["engine<br/>the worker thread and its policy"]:::sun
    core["core<br/>session · settings · schedule · almanac<br/>no I/O"]
    platform["platform<br/>power · battery · idle · clock<br/>the only unsafe code"]:::sky
    app --> engine
    engine --> core
    engine --> platform
    classDef sun fill:#dca544,stroke:#1c1b18,color:#1c1b18
    classDef sky fill:#dfe8ea,stroke:#1c1b18,color:#1c1b18
```

### AI's three locks

```mermaid
%%{init: {"theme": "base", "themeVariables": {"fontFamily": "Georgia, serif", "primaryColor": "#f4f0e7", "primaryTextColor": "#1c1b18", "primaryBorderColor": "#1c1b18", "lineColor": "#9a948a", "secondaryColor": "#ebe6da", "tertiaryColor": "#dfe8ea", "clusterBkg": "#ebe6da", "clusterBorder": "#cdb98c", "edgeLabelBackground": "#f4f0e7", "actorBkg": "#f4f0e7", "actorBorder": "#1c1b18", "actorTextColor": "#1c1b18", "signalColor": "#9a948a", "signalTextColor": "#9a948a", "labelBoxBkgColor": "#ebe6da", "labelBoxBorderColor": "#cdb98c", "labelTextColor": "#1c1b18", "loopTextColor": "#9a948a", "noteBkgColor": "#dca544", "noteTextColor": "#1c1b18", "noteBorderColor": "#1c1b18", "activationBkgColor": "#dca544", "activationBorderColor": "#1c1b18", "transitionColor": "#9a948a", "transitionLabelColor": "#9a948a", "stateLabelColor": "#1c1b18", "stateBkg": "#f4f0e7", "compositeBackground": "#ebe6da", "altBackground": "#ebe6da", "compositeTitleBackground": "#dfe8ea", "innerEndBackground": "#1c1b18", "specialStateColor": "#dca544"}}}%%
flowchart LR
    Click(["You click a feature"]):::sun --> Key{"A key of your own?"}
    Key -- no --> Locked["Shown locked<br/>nothing is sent"]
    Key -- yes --> Master{"Use AI on?"}
    Master -- no --> Off["Shown off<br/>nothing is sent"]
    Master -- yes --> Feature{"This feature on?"}
    Feature -- no --> Off
    Feature -- yes --> Send["One request<br/>only what the feature says"]:::sky
    Send --> Answer["A line or two back"]
    classDef sun fill:#dca544,stroke:#1c1b18,color:#1c1b18
    classDef sky fill:#dfe8ea,stroke:#1c1b18,color:#1c1b18
```

## What it never does

| It never                                     | Because                                                                                               |
| :------------------------------------------- | :---------------------------------------------------------------------------------------------------- |
| types, clicks or moves anything              | it holds a power request; input injection is ruled out by [ADR 0002](docs/adr/0002-power-requests.md) |
| keeps the laptop awake after you turn it off | the request is released when you turn it off or quit, and the system drops it if the app crashes      |
| drains your battery without asking           | it pauses below 20% by default, or whenever unplugged if you choose                                   |
| phones home                                  | nothing goes over the network except the update check and AI features you click                       |
| sends your days anywhere                     | the almanac is a file on your machine; AI gets only what a feature says, when you click it            |
| shows your API key again                     | it goes into Windows Credential Manager or the macOS Keychain, and never back to the window           |
| installs an update on its own                | it tells you one exists; you say when                                                                 |

## Use

The [field guide](docs/guide.md) covers every control, including the things that are not
labelled. The short version:

| Want to…                           | Do…                                                                                                    |
| :--------------------------------- | :----------------------------------------------------------------------------------------------------- |
| keep the laptop awake              | click the sun, or press <kbd>Space</kbd>                                                               |
| for a fixed time                   | pick a length (30m, 1h, …) in the row under the status                                                 |
| until a time of day                | pick **Until…** and type a time                                                                        |
| let it sleep again                 | click the sun again                                                                                    |
| keep it awake during working hours | Settings → **Schedule**                                                                                |
| turn it on or off from another app | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>K</kbd> (<kbd>⌥</kbd><kbd>⌘</kbd><kbd>K</kbd> on a Mac)            |
| get the window out of the way      | <kbd>Esc</kbd> or ×; it keeps running in the tray (the menu bar on a Mac)                              |
| do all of this without a window    | right-click the sun in the tray or menu bar                                                            |
| change settings                    | the sliders icon in the title bar, or <kbd>Ctrl</kbd>+<kbd>,</kbd> (<kbd>⌘</kbd><kbd>,</kbd> on a Mac) |
| see your days, hours and leave     | the open book in the title bar: the Almanac                                                            |
| change the theme                   | Settings → **Theme**                                                                                   |

<details>
<summary><b>Where it keeps things</b></summary>

<br/>

| What                       | Windows                                      | macOS                                                   |
| :------------------------- | :------------------------------------------- | :------------------------------------------------------ |
| Settings, almanac, history | `%APPDATA%\dev.bajpai.keepmeawake\`          | `~/Library/Application Support/dev.bajpai.keepmeawake/` |
| Logs                       | `%LOCALAPPDATA%\dev.bajpai.keepmeawake\logs` | `~/Library/Logs/dev.bajpai.keepmeawake/`                |
| API key, if you add one    | Credential Manager                           | Keychain                                                |

</details>

## Development

Prerequisites: Node 22+, Rust stable 1.90+, and the MSVC build tools with a Windows SDK or,
on a Mac, the Xcode Command Line Tools. The [development guide](docs/development.md) has the
setup for both, the daily commands and the conventions.

```powershell
npm install        # also wires the git hooks
npm run dev        # Vite + Tauri dev build; the UI hot-reloads
npm run verify     # lint, type-check and test both halves
npm run build      # installers (or .app and .dmg) in src-tauri/target/release/bundle
```

<details>
<summary><b>Where things live</b></summary>

<br/>

| Path                      | What                                                             |
| :------------------------ | :--------------------------------------------------------------- |
| `src-tauri/src/core/`     | sessions, settings, schedule, almanac: pure, no I/O              |
| `src-tauri/src/engine/`   | the worker thread and its policy                                 |
| `src-tauri/src/platform/` | power, battery, idle and clock per OS; the only `unsafe`         |
| `src-tauri/src/app/`      | tray, commands, window, almanac keeper, updater, AI              |
| `src/lib/`                | the Svelte view: components, copy, the almanac's reading         |
| `src/styles/`             | tokens and themes                                                |
| `design/`                 | SVG sources for the icons, the DMG background and the installers |
| `docs/`                   | guide, architecture, design, specs, decisions                    |

</details>

## Documentation

| Doc                                        | Read it for                                         |
| :----------------------------------------- | :-------------------------------------------------- |
| [Field guide](docs/guide.md)               | every control, shortcut and hidden thing            |
| [Architecture](docs/architecture.md)       | layers, the engine loop, IPC, window lifecycle      |
| [Design](docs/design.md)                   | the horizon, palette, type, motion, and the reasons |
| [Roadmap](docs/roadmap.md)                 | what is next and what was decided against           |
| [Development](docs/development.md)         | setup, commands, style, testing, commits            |
| [Releasing](docs/releasing.md)             | versions, changelog, the release workflow           |
| [Resource budget](docs/resource-budget.md) | what "tiny" means and how it is kept                |
| [Decisions](docs/adr/README.md)            | the five choices that shaped the project            |
| [Specs](docs/specs/README.md)              | the spec each feature was built from                |
| [Handoff](docs/HANDOFF.md)                 | where the last session stopped and where to start   |
| [Changelog](CHANGELOG.md)                  | what changed, by version                            |
| [Contributing](CONTRIBUTING.md)            | issues, pull requests, and what a change agrees to  |
| [Security](SECURITY.md)                    | how to report a problem privately                   |

## License

Keep me awake is mine, a personal project by [Akshay Bajpai](https://akshaybajpai.com), under
its own [license](LICENSE). Use the builds, read the code, build it for yourself; copying it
or publishing your own version needs a yes from me first. It comes as is and you use it at
your own risk: it keeps your machine awake, and what that means for your battery, your desk
or your employer's rules is yours to judge.

---

<div align="center">

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_

</div>
