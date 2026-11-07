# Field guide

Everything the app can do, including the parts that are not labelled.

## The window

```mermaid
flowchart TB
    subgraph Window["380 × 600, frameless"]
        direction TB
        T["Title bar · wordmark · gear · minimise · close"]
        S["Scene · sky, sun, horizon, ground"]
        ST["Status · Resting / Awake / Paused + one line of detail"]
        R["Rule · Always · 30m · 1h · 2h · 4h · 8h · Until…"]
        RW["Rows · Keep the screen on · Pause on low battery"]
    end
    T --> S --> ST --> R --> RW
```

### Scene

| Element      | Click                     | Means                                                                      |
| ------------ | ------------------------- | -------------------------------------------------------------------------- |
| Sun          | toggles awake / resting   | above the line: awake; below it: resting; dimmed with a bar across: paused |
|              | during a schedule window  | skips the rest of the window; click again to come back to it               |
| Horizon line | shows a tip               | there are ten; they cycle                                                  |
| Stars        | lights one (resting only) | find all seven                                                             |
| Reflection   | ripples (awake only)      |                                                                            |
| Dotted arc   |                           | the path the sun still has to travel in a timed session                    |

### Status

The word is what the laptop is doing: **Resting**, **Awake**, **Paused**. The
line under it says until when, or why it is paused. Between midnight and five
in the morning the awake line adds a second sentence.

| State                     | Word    | Line                                                   |
| ------------------------- | ------- | ------------------------------------------------------ |
| resting                   | Resting | `Your laptop sleeps as usual.`                         |
| resting, schedule waiting | Resting | `Your laptop sleeps as usual. Awake again at 9:00 am.` |
| always                    | Awake   | `Until you say otherwise.`                             |
| timed                     | Awake   | `Until 5:30 pm · 1h 30m left`                          |
| schedule window           | Awake   | `Until 6:00 pm · on schedule`                          |
| window skipped            | Resting | `Skipping today's schedule until 6:00 pm.`             |
| paused                    | Paused  | `Holding off while battery is low.`                    |

When the next window is further off than tomorrow the line names the day:
`Awake again at 9:00 am on Monday.`

### Schedule

One window of time on chosen days, set in Settings → Schedule. While it is
open the sun rises on its own and sets as the window closes, exactly like a
timed session; outside it the laptop sleeps as usual. Overnight windows
(10:00 pm until 6:00 am) belong to the day they start on.

| You do                         | Happens                                                      |
| ------------------------------ | ------------------------------------------------------------ |
| nothing                        | awake inside the window, resting outside it                  |
| click the sun inside a window  | `Skipped until 6:00 pm.` Resting until the window closes     |
| click it again                 | `Back on schedule.`                                          |
| pick a stop or Always          | that session wins, and is not cut short when the window ends |
| a session ends inside a window | the schedule takes over without letting go                   |
| launch inside a window         | starts awake                                                 |

![Awake on schedule, Paper](assets/schedule-home-light.png) ![Awake on schedule, Night](assets/schedule-home-dark.png)

### Rule

Stops are durations measured from now. **Always** has no end. **Until…** opens
a time field; the time is today if still ahead, otherwise tomorrow. The active
stop carries a dot; while a custom time is set the last stop reads
"Until 5:30 pm".

### Rows

| Row                  | On                                           | Off                                 |
| -------------------- | -------------------------------------------- | ----------------------------------- |
| Keep the screen on   | display and system are both held             | display may dim; system stays awake |
| Pause on low battery | below 20% the sun pauses; resumes on charger | battery level is ignored            |

The hint under the battery row shows the live reading ("Plugged in · 100%").

## Keyboard

| Windows    | macOS    | Does                                                                      |
| ---------- | -------- | ------------------------------------------------------------------------- |
| Space      | Space    | toggle awake / resting                                                    |
| Ctrl+Alt+K | ⌥⌘K      | toggle from any app, window open or not (recordable in Settings → Hotkey) |
| Esc        | Esc      | back from settings, else close to the tray or menu bar                    |
|            | ⌘W       | close to the menu bar                                                     |
|            | ⌘M       | minimise                                                                  |
|            | ⌘Q       | quit                                                                      |
| Ctrl+,     | ⌘,       | settings                                                                  |
| Tab / ←→   | Tab / ←→ | move between stops and controls                                           |

## Tray and menu bar

On Windows the sun lives in the tray. On a Mac it lives in the menu bar, and
the app has no Dock icon and no place in Cmd-Tab: it is a menu bar app.

| Action                         | Does                                                                      |
| ------------------------------ | ------------------------------------------------------------------------- |
| click (left click on Windows)  | open the window; on a Mac it drops down centred beneath the menu bar icon |
| right click (or Control-click) | status line · Keep awake / Let it sleep · Keep awake for ▸ · Open · Quit  |
|                                | while a schedule window is open: Skip today's schedule / Back on schedule |
| hover                          | tooltip with the same status line as the menu                             |

Icon states: sun above the line (awake), sun set (resting), bar across the sun
(paused). On Windows the icons follow the taskbar theme: white on a dark
taskbar, ink on a light one, switching within a second when Windows changes
mode. On a Mac they are template images, which macOS tints for a light or
dark menu bar itself. Nothing to set.

## Settings

| Section       | Setting                    | Notes                                                                     |
| ------------- | -------------------------- | ------------------------------------------------------------------------- |
| Startup       | Launch at sign-in          | registers under the current user; starts hidden                           |
|               | Open the window at launch  | only applies when you start it yourself                                   |
|               | Begin as                   | Resting · Awake · Default length                                          |
| Waking        | Default length             | what the sun, tray toggle and Space use. Always, or a stop                |
|               | Keep the screen on         | same switch as on the home pane                                           |
| Schedule      | Keep a schedule            | awake during these hours, resting outside them. Off by default            |
|               | Days                       | Monday first; weekdays by default                                         |
|               | From / Until               | 9:00 am until 6:00 pm by default; overnight is fine                       |
| Battery       | Pause below                | Off, or 10–50%                                                            |
|               | Pause whenever unplugged   | strictest: on battery means resting                                       |
| Window        | Closing keeps it running   | off quits on close                                                        |
|               | Free memory when closed    | destroys the UI on close (default); off hides it for an instant reopen    |
|               | Appearance                 | System · Paper · Night                                                    |
| Notifications | When a timer ends          | a toast when a timed session sets on its own. On by default               |
|               | Battery pauses and resumes | when the battery rule pauses the sun, and when the charger brings it back |
|               | Schedule starts and ends   | when a schedule window opens or closes                                    |
| Hotkey        | Hotkey                     | `Ctrl+Alt+K` by default. Change records a new one; Off removes it         |
| Tips          |                            | the keyboard and tray table                                               |
| About         |                            | version, how it works, author, Source, Quit                               |

![Settings → Schedule, Paper](assets/schedule-settings-light.png) ![Settings → Schedule, Night](assets/schedule-settings-dark.png)

## Notifications

Quiet Windows toasts for the moments that matter while the window is closed,
each behind its own switch in Settings → Notifications. Nothing is shown
while the window is open and focused; the status line already says it.

| Moment           | Title                    | Body                                      | Default |
| ---------------- | ------------------------ | ----------------------------------------- | ------- |
| timer ended      | `The sun has set`        | `Resting now. 2h 00m, as asked.`          | on      |
| battery paused   | `Paused for the battery` | `Holding off at 18%. Plug in to resume.`  | off     |
| battery resumed  | `Back up`                | `Charger connected. The sun is up again.` | off     |
| schedule started | `On schedule`            | `Awake until 6:00 pm.`                    | off     |
| schedule ended   | `Off schedule`           | `Resting until 9:00 am.`                  | off     |

A toast is skipped when it would no longer be true: no "Resting now" if a
schedule window took over, no schedule toasts while your own session hides
the change.

![Settings → Notifications, Paper](assets/notifications-light.png) ![Settings → Notifications, Night](assets/notifications-dark.png)

## Hotkey

`Ctrl+Alt+K` (on a Mac, `⌥⌘K`) does what the sun does, from any app, with the
window open or closed: the tray or menu bar icon changes and nothing opens.
Settings → Hotkey shows the combination with **Change** and **Off**. Change
listens for the next keys you press with at least one of Ctrl, Alt or Win (on
a Mac ⌘, ⌃ or ⌥) (`Press keys…`), shows them as you hold them and saves when
you let go; Esc cancels. A Mac shows the combination in glyphs, in the order
its menus use. If another app or the system already owns the combination
(`⌘Space`, say), the old one stays and the window says
`That combination is already taken.`

The hotkey is registered with the OS (`RegisterHotKey` on Windows, a Carbon
hot key on macOS), which only listens. Nothing is ever typed into other
applications.

![Settings → Hotkey, Paper](assets/hotkey-light.png) ![Settings → Hotkey, Night](assets/hotkey-dark.png)

## Where things are

| What      | Windows                                                      | macOS                                                                |
| --------- | ------------------------------------------------------------ | -------------------------------------------------------------------- |
| Settings  | `%APPDATA%\dev.bajpai.keepmeawake\settings.json`             | `~/Library/Application Support/dev.bajpai.keepmeawake/settings.json` |
| Log       | `%LOCALAPPDATA%\dev.bajpai.keepmeawake\logs\keepmeawake.log` | `~/Library/Logs/dev.bajpai.keepmeawake/keepmeawake.log`              |
| Autostart | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`         | a LaunchAgent in `~/Library/LaunchAgents/`                           |
| Proof     | `powercfg /requests` (elevated) while awake                  | `pmset -g assertions` while awake                                    |

## When it will not hold

- Lid closed, power button, Start → Sleep: Windows releases every request on
  purpose. That is correct.
- Modern Standby laptop on battery: Windows terminates system requests five
  minutes after the sleep timeout. Plug in, or accept it.
- "Paused: Windows refused the request": something blocked
  `PowerCreateRequest`; check the log.
- Mac lid closed with no external display: macOS sleeps regardless of any
  assertion, by design. With a display, power and a keyboard attached
  (clamshell mode) it stays up.
- "Paused: macOS refused the request": IOKit turned the assertion down;
  check the log for the `IOReturn` code.

## Installing on a Mac

The Mac build is not notarized yet, so the first open needs one extra step:
drag the sun to Applications from the disk image, then right-click
**KeepMeAwake** → **Open** → **Open**, once. Or, in Terminal:
`xattr -d com.apple.quarantine /Applications/KeepMeAwake.app`. It runs on
macOS 12 or later on Apple Silicon. An Intel Mac needs a universal build
(`npm run build -- --target universal-apple-darwin`); the releases do not
include one yet.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
