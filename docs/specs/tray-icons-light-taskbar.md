# Tray icons for light taskbars

Version 0.2 · Size S · Touches platform, app, brand

## Why

The tray icons are white. On a light Windows taskbar they vanish. Windows
exposes the taskbar theme in the registry and can notify on change, so the
app can swap to an ink set at zero idle cost.

## Experience

Nothing to configure. On a light taskbar the icons are ink (`#1c1b18`); on a
dark one, white as today. Switching the Windows theme swaps them within a
second. Paused keeps its amber pupil, awake its sun; only the line and disc
colours change.

## Settings

None.

## Rust

### platform

`platform/windows.rs` gains:

```rust
/// `true` when the taskbar uses the light theme (SystemUsesLightTheme = 1).
pub fn taskbar_is_light() -> bool;

/// Blocks until the Personalize key changes, then returns. Returns Err if the
/// key cannot be opened. Used by a dedicated thread.
pub fn wait_for_theme_change() -> Result<(), PlatformError>;
```

Implemented with `RegOpenKeyExW` / `RegGetValueW` on
`HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` and
`RegNotifyChangeKeyValue` (synchronous, `REG_NOTIFY_CHANGE_LAST_SET`). Feature
`Win32_System_Registry` in `Cargo.toml`. Both are `unsafe` with `SAFETY:`
comments; the key handle is closed on every path.

Add a trait `TaskbarTheme` in `platform/mod.rs` with `is_light()` and a fake
for tests; the notify loop is Windows-only and not trait-wrapped.

### brand

`design/tray-on-ink.svg`, `tray-off-ink.svg`, `tray-paused-ink.svg`: same
geometry, line and disc in `#1c1b18`, pupil colours unchanged. Generate
`src-tauri/icons/tray/{on,off,paused}-ink.png` at 64 px with `tauri icon --png 64`.

### app

- `tray::icon_for(snapshot, light: bool)` picks the set.
- `tray::refresh` reads `taskbar_is_light()` (one registry read, microseconds).
- In `setup`, spawn a thread named `keepmeawake-theme` that loops
  `wait_for_theme_change()` → `tray::refresh(app, &engine.snapshot())`. The
  thread blocks in the kernel; no polling.

## UI

None.

## IPC

None.

## Tests

Rust: `icon_for` returns distinct bytes for light/dark × three states and all
six decode; registry functions are exercised by a `#[test]` that only asserts
`taskbar_is_light()` returns without error on Windows.

## Docs

Guide (tray section: "icons follow the taskbar theme"), roadmap, changelog,
resource budget (one blocked thread).

## Resource impact

One thread blocked on a kernel notification: 0 CPU, ~64 KB stack. Three more
4 KB PNGs embedded.

## Acceptance

- [ ] Settings → Personalisation → Colours → Windows mode Light: the tray icon
      becomes ink within a second, in all three states.
- [ ] Back to Dark: white again.
- [ ] No CPU use from the app while idle (Task Manager, 1 minute).

## Commits

1. `feat(brand): add ink tray icons for light taskbars`
2. `feat(platform): read and watch the taskbar theme from the registry`
3. `feat(tray): pick the icon set from the taskbar theme`
4. `docs: tray icons follow the taskbar theme`

## Out of scope

A user override, custom icon packs.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
