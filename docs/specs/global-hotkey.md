# Global hotkey

Version 0.2 · Size S · Touches app, ui

## Why

Toggling from anywhere, without finding the tray, is the fastest the app can
get.

## Experience

Default `Ctrl+Alt+K`. Pressing it does exactly what the sun does (`toggle`).
If the window is closed nothing opens; the tray icon changes and, if the timer
notification switch is on, nothing else happens (no toast for a deliberate
action).

Settings → **Tips** section gains the row `Ctrl+Alt+K · Toggle from anywhere`
and a new section **Hotkey** directly above Tips:

| Row    | Control                                                       | Hint                                                      |
| ------ | ------------------------------------------------------------- | --------------------------------------------------------- |
| Hotkey | a field showing the combo, a `Change` button, an `Off` button | `Works in any app. Needs at least one of Ctrl, Alt, Win.` |

Pressing `Change` turns the field into a recorder: it shows `Press keys…`,
captures the next keydown with at least one modifier, displays it as
`Ctrl+Shift+K`, and saves on release. Esc cancels. If registration fails (combo
taken by another app), the field shows the old value and the error toast reads
`That combination is already taken.`

## Settings

```rust
pub hotkey: Option<String>,   // Some("Ctrl+Alt+K") by default; None = off
```

Stored in the plugin's accelerator syntax (`Ctrl+Alt+K`, `Super+Shift+F9`).
`sanitized()` leaves it alone; validation happens on registration.

## Rust

- Add `tauri-plugin-global-shortcut = "2"`; register after `notification`.
  Capability: `global-shortcut:allow-is-registered` only if the UI needs it;
  registration itself is done in Rust so no JS permission is required.
- `app/hotkey.rs` (new): `apply(app, hotkey: Option<&str>) -> Result<(), String>`
  unregisters everything, then registers the new combo with a handler that
  calls `engine.toggle()` on `ShortcutState::Pressed`. Called at setup and from
  `update_settings` when the field changed.
- `update_settings` returns the registration error as the command error string
  (the settings are still applied and saved, except `hotkey`, which reverts to
  the previous value on failure).

## UI

- `HotkeyField.svelte` (new): display / record / off. Recording converts a
  `KeyboardEvent` to accelerator syntax: modifiers in the order
  `Ctrl`, `Alt`, `Shift`, `Super`, then the key (`event.code` mapped: `KeyK` →
  `K`, `Digit1` → `1`, `F9` → `F9`, `Space`, `Enter`, arrows). Letters are
  upper-case. A pure `toAccelerator(event-like) -> string | null` in
  `src/lib/hotkey.ts` with tests.
- The store's `updateSettings` already surfaces command errors in `store.error`.

## IPC

None new (goes through `update_settings`).

## Tests

UI: `toAccelerator` for letters, digits, function keys, rejects no-modifier
combos, orders modifiers. Rust: a unit test on `apply` with an invalid string
returns `Err`; registration on CI is not exercised (no desktop session).

## Docs

Guide (keyboard table, Settings table), changelog.

## Resource impact

None at idle (the plugin uses a Windows hook that sleeps). ~150 KB binary.

## Acceptance

- [ ] Fresh install: `Ctrl+Alt+K` toggles with the window closed; tray icon
      flips.
- [ ] Recording `Ctrl+Shift+K` and pressing it works; the old combo no longer
      does.
- [ ] Recording a combo another app owns shows the error and keeps the old one.
- [ ] `Off` removes the hotkey; restart keeps it off.

## Commits

1. `feat(app): register a global hotkey that toggles the session`
2. `feat(ui): add the hotkey recorder and accelerator parsing`
3. `docs: hotkey in the guide and changelog`

## Out of scope

Multiple hotkeys (e.g. one per duration), chords.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
