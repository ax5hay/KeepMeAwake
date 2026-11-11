# Changelog

Notable changes, by version. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions: [SemVer](https://semver.org).

## [Unreleased]

### Changed

- Plainer words wherever you read them: the window, the tray and menu bar,
  notifications, the Windows installer, the README and the guide. Each
  line now says what your laptop does rather than what the sun does:
  `Ends in 5 minutes.` instead of `The sun sets in 5 minutes.`,
  **Add 30 minutes** instead of **Give it another 30m**, **I'm here**
  instead of **Keep it up**, and toasts titled **Timer done**, **Timer
  ends soon**, **Still there?**. Settings → Waking is now **Staying awake**,
  and **Warn before sunset** is **Timer warning**. The
  Almanac and the other named features keep their names. Settings files
  are unchanged.

## [0.7.0] - 2026-11-12

The small things: twelve details that nothing needs and that are there
anyway. Built and checked on Windows; the Mac has compiled and tested it on
CI. Ten were seen in a harness and the mini window in the real app; the
shooting star, the sounds, a real print dialog and the system's colour
picker have not been tried by hand yet.

### Added

- A sun shaped by its theme: square pixels in Pocket, a ring around a core
  in Thermal, a halftone dot in Risograph.
- Tonight's moon in Night's resting sky, worked out from the date. Click it
  for its phase.
- A shooting star, now and then, on a resting sky.
- Pointing at a theme in Settings wears it on the whole window until the
  pointer leaves, so you can look before you choose.
- A theme of your own (Settings → Theme → Yours): five colours, the rest
  worked out, and one line of text to share it or to wear someone else's.
- After dark (Settings → Theme): another theme for the evening, with its
  hours. Off unless you ask.
- `?` lays every key over the window on one sheet.
- A year, looked back on (Almanac → the year → Look back): hours, days and
  leave, the months as bars, the longest day, the earliest start, the
  latest finish, the longest run.
- Print month, beside Copy month as CSV: the open month as a timesheet.
- Sounds (Settings → Theme, off by default): a soft note as the sun rises
  and as it sets, while the window is open.
- Mini window (the first button in the title bar): the sun alone, small and
  above other windows. Esc brings the window back.
- Credits and history (Settings → About): the versions as a timeline, and
  whose work the app is made with, the borrowed palettes' authors included.

## [0.6.0] - 2026-11-12

Themes, a Windows installer that looks like the app, and a leave count
that knows which weekdays you do not work. Built and checked on Windows;
the Mac has compiled and tested it on CI. The themes were checked on Home
in a harness, not yet in the real window on either machine.

### Added

- Themes (Settings → Theme): the app's own System, Paper and Night, and
  twenty-four more. Borrowed from palettes people already work in: Solarized
  Light and Dark, Gruvbox, Nord, Dracula, Catppuccin, Tokyo Night, Monokai.
  To play with: Terminal, Blueprint, Pocket, Newsprint, Risograph, Dial-up, Mars, Deep Sea. Loud on purpose: Vaporwave, Thermal, Negative, Miami, Bubblegum, Highlighter, Grape Soda, Hi-Vis. Each swatch is a small
  horizon in its own colours, and the arrow keys walk them. A theme changes
  colours only, and every one is measured to be readable.

- The Windows installers wear the horizon: the setup's welcome and finish
  pages, the header of the pages between, and the MSI's dialogs and banner
  are drawn in Paper, in place of the stock installer art. The setup's
  welcome and finish pages are paper-coloured and say what the app is, in
  its own words, in place of the stock text.

### Changed

- A working day you never actually work is a day off, not a leave every
  week: if that weekday has been empty in three or more of the weeks around
  it, in weeks you worked other days, it stops counting. One missed day and
  a whole week away still read as leave. Weekends never counted and still
  do not; the switch now says so (**Count missed working days as leave**).

## [0.5.0] - 2026-11-11

AI, as a choice: off until you turn it on, with your own key, by the
service you pick. Also checking for updates when you ask and when the
window opens. Built and checked on Windows; the Mac has compiled and
tested it on CI. No answer from a real service has been seen yet, for
want of a key: the first request with yours is the test.

### Added

- AI, as a choice, and a cheap one, by the service you choose: Claude,
  OpenAI, or any other that speaks the OpenAI chat format, including a
  model on your own machine (Settings → AI → Service). Four features, shown where they
  are used: in the Almanac, **The day, in a line**, **This week, in words**
  and **Ask the almanac** (a question about the month in view); on Home
  with Until…, **Tell the sun** (say the plan, `two hours` or `until half
  six`; it shows what it understood and starts only when you say). Three
  locks, each visible: with no API key the features are shown locked, with
  a way to add one; with a key they say AI is switched off, with the
  master switch right there (Settings → AI → Use AI, off until you
  turn it on); and each feature has its own switch beside its name.
  Nothing is sent without a click that says what it sends. The key is your
  own, kept in Windows Credential Manager or the macOS Keychain, never in a
  file and never shown again.
- Every request asks the service's smallest model unless you name
  another, with a hard cap on the answer (and, with Claude, thinking off);
  the largest costs about a thirtieth of a cent with Claude Haiku at
  today's list price, and the same question twice in a session
  costs nothing the second time.
- Hide AI (Settings → AI, off by default): takes every AI feature out
  of sight and leaves one button, Enable AI.
- Check for updates, a button in Settings → About: look for a newer
  release now. The app also looks whenever the window is opened, if it has
  not in the last hour, so a new release shows the next time you look
  rather than a day later. An update that is found stays shown as
  available until you install it; checking never installs anything.
- Break reminder (Settings → Almanac, off unless you ask): a toast after
  1 hour, 90 minutes or 2 hours at the machine without five minutes away.
- Copy month as CSV, at the foot of the Almanac: a row a day, for a
  timesheet or a spreadsheet.

## [0.4.0] - 2026-11-10

The Almanac: your days at this machine, the hours you worked, your leave
and your nights, with today on Home. Built and checked on Windows; the Mac
has compiled and tested it on CI but nobody has lived with it there yet,
and Sleep when I'm away (off unless you ask) has not yet had a real night.

### Changed

- The almanac and AI say facts plainly and keep their own names: Started and ended rather than
  first light, Away overnight rather than a quiet night, Keep an almanac,
  Morning whisper, Sleep when I'm away, Tell the sun, The day, in a line
  and week, Edit hours. The scene and its whispers keep their voice.

### Fixed

- A schedule set to start and end at the same time switched itself off and
  could not be switched back on, because its hours are greyed out while it
  is off. The same time twice now means the whole day, and the switch is
  never touched for you.

### Added

- The Almanac: a book of your days at this machine, behind the open book in
  the title bar. It shows how long you worked each day, what ran over your
  working hours, which days were leave, and the night: the quiet between
  the last touch of one day and the first of the next. The night is read
  backwards, so it is found with the sun on Always exactly as with the
  laptop asleep, and Home whispers it once on the first open after it.
  Each day is drawn as a small horizon (a band of sky over your working
  hours, ink where you were here, the night dotted under the line, the sun
  at now), with the week as seven strips and the month as a calendar.
  **Mark as leave** on any day, past or to come.
- Settings → Almanac: Keep an almanac (on by default), Working days,
  Working hours (past midnight is fine: a day turns over opposite the
  middle of them, so late work stays on the day it began) and Empty working
  days are leave.
- Today on Home: under the scene, your day as a small horizon, and beside
  the status word today's hours. Both open the Almanac. The first time a
  day reaches the length of your working hours, Home whispers it once.
- Any day can be put right by hand: **Mark as leave** or **Not a leave**,
  and **Edit hours** to set it to one stretch, to say nobody was
  here, or to go back to what was read. A day set by hand says so, and
  nothing that was read is thrown away.
- Sleep when I'm away (Settings → Almanac, off unless asked for): an Always session
  sets once the keys have been quiet at night for 30 minutes, 1 hour or 2,
  asking first as the overnight guard does, and rises again at your next
  touch.
- Every automatic thing in the almanac has a switch, including the morning
  whisper (Morning whisper).
- It reads only how long since the last input, once a minute, the same
  read-only call the overnight guard makes. Nothing is typed, and
  `almanac.json` never leaves the machine. With the switch off the app
  wakes for nothing at rest, as before.

## [0.3.0] - 2026-11-07

Session history, the sunset warning, the overnight guard and auto-update,
with a pass over every screen. The first full release, and the first that
updates itself: from here on, installs are offered each new release. Built
and checked on Windows; on a Mac some acceptance is still by hand.
Installers are not yet code-signed, so Windows SmartScreen and macOS
Gatekeeper ask once (see the guide); updates carry their own signature.

### Changed

- The settings button is a set of sliders rather than a gear, which read as
  a second sun at title bar size.

### Added

- Auto-update: about half a minute after launch and then once a day, the
  app asks GitHub for a newer full release (Settings → About → Check for
  updates daily, on by default; the only request it makes). A newer one
  shows on Home's line, in the tray and in About, with a whisper the first
  time; Install and restart downloads it with a progress line, verifies its
  signature, installs and relaunches. Pre-releases are never offered.
- Home ends with a quiet line of doors, `Schedule · Overnight · History ·
  Ctrl+Alt+K`, each opening Settings at that section, so the features that
  never announce themselves can be found from the main window. A waiting
  update leads the line.
- The Always line says how it can end while the overnight guard watches:
  "Until you say otherwise, or 6h untouched." Keep it up answers with a
  whisper and the line drops the promise.

### Fixed

- Esc in the Until… field closed the window; it closes the field.
- An error stayed on screen until a later action succeeded; it goes after
  a few seconds or on a click.
- Settings → History did not refresh while Settings stayed open.
- A warning or guard set by hand outside the presets selected no segment.
- "Give it another 30m" showed while paused for the battery.

- Overnight guard: when an Always session has had no keyboard, mouse or
  trackpad input for 6 hours (Off, 4, 6 or 8 in Settings → Waking), the sun
  asks once ("Nobody here for 6 hours. Resting at 3:12 am.") in the window,
  the tray or menu bar and a toast. Any input withdraws the question, Keep
  it up leaves that session alone, and with no answer in ten minutes the
  session ends and history says `idle`. It only reads how long since the
  last input; it never sends any.
- Sunset warning: a few minutes before a timed session ends (5 by default;
  Off, 2, 5 or 10 in Settings → Waking), the status line says so and offers
  Give it another 30m, the tray or menu bar gains the same item, and a toast
  says it while the window is closed. Only for timers longer than the
  warning.
- Session history: Settings → History lists the last 30 times the sun was
  up, newest first, with the span, the length and how each ended (you,
  timer, schedule, battery, or sleep when the machine slept or the app was
  killed while it was up), and the pauses inside each. Clear history
  empties it. Kept in `history.json` beside the settings, at most 200
  entries.

## [0.2.0] - 2026-11-07

The first published release, as a pre-release: Windows, and the first macOS
build.

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

- License: Keep me awake is no longer MIT. It has its own license, written
  plainly: the builds are free to use and the source is free to read and
  build for yourself, but copying it or publishing your own version needs
  the author's yes. It is provided as is, at your own risk, with no
  liability. Copies of the source taken while it was public under MIT keep
  that grant.

### Fixed

- A command that arrived while the engine was delivering events could sit
  unapplied until the next deadline (forever while resting), and quitting
  could hang waiting for the engine thread.

## 0.1.0 - 2026-11-07

Built, but never tagged or published.

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

[Unreleased]: https://github.com/ax5hay/KeepMeAwake/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.7.0
[0.6.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.6.0
[0.5.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.5.0
[0.4.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.4.0
[0.3.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.3.0
[0.2.0]: https://github.com/ax5hay/KeepMeAwake/releases/tag/v0.2.0
