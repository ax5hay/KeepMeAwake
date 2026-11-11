# Sleep detection

Know when the machine sleeps and wakes anyway (a closed lid, the power
button, a forced sleep on low battery), so history tells the truth and
timers catch up the moment the lid opens.

## Why

A power request keeps idle sleep away, not a closed lid. Today a laptop
that sleeps with the sun up shows one history row across the night, and a
timer that ran out during sleep is only noticed when the worker next wakes
(up to five minutes later on Windows).

## Experience

Nothing new to set. History splits a stretch at a sleep: the row before
it ends `sleep` at the moment the machine slept; if anything still wants
the laptop awake on wake, a new row starts then. The window, opened after
a wake, is right straight away.

## Rust

- `platform::SleepWatch` starts a thread that calls back on sleep and
  wake:

| Platform | How                                                                                                                                                                                                        |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows  | `PowerRegisterSuspendResumeNotification(DEVICE_NOTIFY_CALLBACK)`: `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC`                                                                                               |
| macOS    | `IORegisterForSystemPower` on a thread running its own `CFRunLoop`: `kIOMessageSystemWillSleep` (answered with `IOAllowPowerChange`), `kIOMessageCanSystemSleep` (allowed), `kIOMessageSystemHasPoweredOn` |

The callbacks only record the wall-clock time and notify the engine; no
allocation, no locks held across the OS call.

- `Engine::slept(at)` / `Engine::woke(at)`: at wake the worker runs a step
  at once. A stretch that was up before the sleep ends as `Ended::Sleep`
  at the sleep's time, and starts again at wake if it still holds.

## Tests

Engine with fakes: sleep and wake around an Always session give two
stretches; a timer that ran out during sleep ends as `timer`, at its due
time, not `sleep`; a wake with nothing up reports nothing.

## Resource impact

One parked thread per OS (no wake-ups of its own); one step per sleep and
per wake.

## Acceptance

Always on, close the lid for a minute, open it: History shows two rows,
the first ending `sleep`.

## Commits

1. `feat(engine): split a stretch at a sleep`
2. `feat(platform): hear sleep and wake`
3. `feat(app): wire sleep and wake to the engine`

## Out of scope

Dark wakes (Power Nap) are not wakes: macOS sends no `HasPoweredOn` for
them.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
