# When it ends

Choose what happens when a timer, the schedule or a condition lets the
laptop go: nothing (today), lock it, turn off the screen, or sleep.
The roadmap called this _After the sun sets_.

## Why

"Stay awake until the download finishes, then sleep" is the most asked-for
thing a keep-awake app can do. With [stay-awake-while](stay-awake-while.md)
the app knows when the download finishes; this decides what follows.

## Experience

Settings → Staying awake gains **When it ends**, a segmented control:

| Windows                             | macOS                        |
| ----------------------------------- | ---------------------------- |
| Nothing · Lock · Screen off · Sleep | Nothing · Screen off · Sleep |

Hint: `After a timer, the schedule or a condition ends, if you've been away a minute.`
macOS has no public way to lock; turning the screen off locks it when the
Mac asks for a password on wake, which the hint under **Screen off** says
on a Mac: `Locks too, if your Mac asks for a password on wake.`

It acts only when the stretch ended on its own (`timer`, `schedule`,
`watch`) and nobody has touched the machine for a minute: someone at the
keyboard when the schedule ends at six is not to be put to sleep. Ending
it yourself, the battery, or the overnight guard never acts (the guard
already lets the OS sleep in its own time).

The timer warning toast says what is coming: `5 minutes left. Add 30
minutes from the tray or the window. Then it will sleep.`

## Settings

`after_end: AfterEnd` (`nothing` | `lock` | `screen_off` | `sleep`),
default `nothing`. On macOS a stored `lock` acts as `screen_off`.

## Rust

- `core::AfterEnd` and `AfterEnd::applies(ended: Ended) -> bool`.
- `platform::after_end(action) -> Result<(), PlatformError>`:

| Action     | Windows                                                           | macOS                   |
| ---------- | ----------------------------------------------------------------- | ----------------------- |
| lock       | `LockWorkStation`                                                 | as screen off           |
| screen off | `PostMessageW(HWND_BROADCAST, WM_SYSCOMMAND, SC_MONITORPOWER, 2)` | `pmset displaysleepnow` |
| sleep      | `SetSuspendState(false, false, false)`                            | `pmset sleepnow`        |

None of these is input to another application (ADR 0002): each is the
documented request for that state.

- `app/notify` (the listener that already hears `SessionEnded`) reads the
  idle time and, when it applies, acts on a short-lived thread three
  seconds later, so the guard release and the toast land first.

## Tests

`applies` for every `Ended`; the idle rule; the toast text with each action.

## Resource impact

None at rest; one call when it acts.

## Acceptance

Set a 2-minute timer with **Sleep**, walk away: the machine sleeps when it
ends. Do the same while typing: nothing happens.

## Commits

1. `feat(core): what happens when a stretch ends on its own`
2. `feat(platform): lock, screen off and sleep`
3. `feat(app): act when a stretch ends and nobody is there`
4. `feat(ui): the When it ends setting`

## Out of scope

Hibernate, shut down, running a script.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
