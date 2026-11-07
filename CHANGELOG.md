# Changelog

Notable changes, by version. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions: [SemVer](https://semver.org).

## [Unreleased]

### Added

- Schedule: one window of time on chosen days (overnight allowed) during
  which the laptop stays awake on its own. Click the sun to skip the rest of
  a window and again to come back; a manual session always wins and is never
  cut short by a window closing. Settings → Schedule, a tray item while a
  window is open, and status lines that say when the sun rises again.
- Notifications: quiet toasts when a timer ends (on by default), when the
  battery rule pauses or the charger resumes, and when a schedule window
  opens or closes. Each has its own switch in Settings → Notifications;
  nothing is shown while the window is focused.
- Global hotkey: `Ctrl+Alt+K` toggles from any app, window open or not.
  Record a different one or turn it off in Settings → Hotkey; a combination
  another app owns is refused and the old one kept.
- Tray icons follow the taskbar: ink on a light taskbar, white on a dark
  one, switching within a second when Windows changes mode.
- macOS: the same app as a menu bar app (no Dock icon) for macOS 12 and
  later on Apple Silicon. It holds IOKit power assertions, visible by name in
  `pmset -g assertions`, reads the battery from IOKit power sources, drops
  the window down beneath the menu bar icon, uses template icons macOS tints
  itself, and defaults the hotkey to `⌥⌘K`. `⌘W`, `⌘M`, `⌘,` and `⌘Q` work
  in the window, and every string that named the tray, the taskbar or
  Windows reads right on a Mac. Ships as an unsigned `.dmg` with a horizon
  background; CI builds and tests on both platforms.

### Changed

- License: Keep me awake is no longer MIT. It is under its own license, all
  rights reserved: the builds are free to use and the source is free to read
  and build for yourself, but copying, redistributing or publishing changed
  versions needs the author's permission. Copies of the source taken while
  it was public under MIT keep that grant.

### Fixed

- A command that arrived while the engine was delivering events could sit
  unapplied until the next deadline (forever while resting), and quitting
  could hang waiting for the engine thread.

## [0.1.0] - 2026-11-07

First release. A twenty-line Python script became a desktop app.

### Added

- Keep the laptop awake with a Windows power request (system, and optionally
  display), visible by name in `powercfg /requests`.
- Sessions: resting, always, 30m / 1h / 2h / 4h / 8h, or until a time of day.
- Battery rules: pause below a chosen percentage (default 20%) or whenever
  unplugged; resume automatically.
- Tray icon with state (sun up, sun set, clouded), tooltip and a full menu.
- Window created on demand and torn down on close; the app idles as one
  process of about 5 MB.
- Launch at sign-in (hidden), state at launch, default length.
- Paper and Night themes following Windows, drawn as a horizon with a sun that
  sets as a timed session runs out.
- Tips on the horizon line, seven stars to find, a ripple, a credit, and a
  late-night line.
- Settings persisted atomically; rotating log file.

[Unreleased]: https://github.com/ax5hay/KeepMeAwake/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.1.0
