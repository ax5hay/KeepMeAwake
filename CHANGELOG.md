# Changelog

Notable changes, by version. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions: [SemVer](https://semver.org).

## [Unreleased]

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

[Unreleased]: https://github.com/BuiltByBajpai/KeepMeAwake/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/BuiltByBajpai/KeepMeAwake/releases/tag/v0.1.0
