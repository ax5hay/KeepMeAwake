# Battery aware

Only show battery rules on a machine that has a battery, and say how long
the charge will last.

## Why

A Mac mini, an iMac or a desktop PC has no battery, yet Home, Settings, the
tray and the tips all offer to pause below 20%. The rules can never fire,
and offering them says the app does not know where it is running. On a
laptop, the question people ask while it stays awake on battery is "how
long have I got?": the OS already knows, so the app should say.

## Experience

| Where                                                  | With a battery                                      | Without one |
| ------------------------------------------------------ | --------------------------------------------------- | ----------- |
| Home → Pause on low battery                            | shown; detail `Below 20% · 62% · about 3h 10m left` | hidden      |
| Settings → Battery                                     | shown                                               | hidden      |
| Settings → Notifications → Battery pauses and resumes  | shown                                               | hidden      |
| Tips (horizon)                                         | the battery tip is in the rotation                  | left out    |
| Stay awake while… → plugged in, a display is connected | shown (see [stay-awake-while](stay-awake-while.md)) | hidden      |

The estimate is shown only on battery and only when the OS has one
(`about 3h 10m left`, `about 45m left`). Plugged in, the row reads
`Plugged in · 80%` as today.

Rules already set on a machine that loses its battery reading (a desktop
that once ran the app's settings from a laptop) stay in the settings file
untouched; they cannot fire, because a machine without a battery is always
on mains.

## Rust

`core::BatteryState` gains two fields, both `#[serde(default)]`:

```rust
/// A battery is fitted. `false` on a desktop: no battery rules apply.
pub present: bool,
/// The OS's estimate of the charge left at the current drain, in
/// minutes, while on battery. `None` when plugged in or unknown.
pub minutes_left: Option<u32>,
```

| Platform | `present`                                                                | `minutes_left`                                                         |
| -------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Windows  | `GetSystemPowerStatus`: `BatteryFlag & 128 == 0` and `!= 255`            | `BatteryLifeTime` in seconds, `u32::MAX` is unknown; only off mains    |
| macOS    | a source in `IOPSCopyPowerSourcesList` whose `Type` is `InternalBattery` | `IOPSGetTimeRemainingEstimate()`: seconds, `-1` unknown, `-2` on mains |

`AlwaysOnMains` reports `present: false`. Policy is unchanged: no battery
means on mains, so nothing pauses.

## UI

`types.ts` mirrors the two fields. `batteryText` adds the estimate.
`horizonTips(mac, battery)` leaves the battery tip out without a battery.
Rows and sections are wrapped in `{#if snapshot.battery.present}`.

## Tests

- `decode_power_status` (Windows) and `decode_power_source` (macOS): no
  battery, unknown, plugged in, estimate present and absent.
- `batteryText` with and without an estimate; tips without a battery.
- A component-level check that Settings has no Battery section when
  `present` is false.

## Resource impact

None: the same calls, read at the same times, return two more numbers.

## Acceptance

- On a Mac mini or desktop PC: no battery row, section, notification switch
  or tip anywhere.
- On a laptop on battery: Home's battery row shows the estimate within 30 s
  of unplugging.

## Commits

1. `feat(core): battery presence and time left in BatteryState`
2. `feat(platform): read battery presence and time left`
3. `feat(ui): hide battery rules without a battery, show time left`

## Out of scope

Drain rate in watts; charging time.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
