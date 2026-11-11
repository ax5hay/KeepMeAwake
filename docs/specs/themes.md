# Themes

Status: shipped in 0.6.0, built on Windows; waits for the Mac.

## Why

Paper and Night are the app's own, and some people would rather it wore
the colours they already work in, or something silly. A theme here is
colours and nothing else: the horizon, the type and the layout do not move.

## Experience

Settings → Theme. The row names the theme worn, with **Back to System**
when it is not System. Under it, four shelves of swatches. Each swatch is a
small horizon in that theme's own colours, its sun on the line; System is
Paper and Night side by side. A click wears it at once.

| Shelf    | Said beside it                         | Themes                                                                                                                                                                                                           |
| -------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Its own  | `Paper by day, Night by night.`        | System, Paper, Night                                                                                                                                                                                             |
| Borrowed | `From the editor you already live in.` | Solarized Light, Solarized Dark, Gruvbox, Nord, Dracula, Catppuccin, Tokyo Night, Monokai                                                                                                                        |
| Play     | `Somewhere else the horizon could be.` | Terminal (green phosphor, amber sun), Blueprint, Pocket (four greens), Newsprint (a red sun), Risograph (two inks on cream), Dial-up (grey panels, a teal desktop sky), Mars (where the sun sets blue), Deep Sea |
| Loud     | `On purpose.`                          | Vaporwave, Thermal (as a heat camera sees it), Negative (film: orange base, cyan sun), Miami, Bubblegum, Highlighter, Grape Soda (a lime sun), Hi-Vis (safety orange over asphalt)                               |

The borrowed ones take their colours from each palette's published values;
the sun is that palette's yellow or orange. The rules of
[design.md](../design.md) hold in every theme: tokens only, no gradients,
one accent. Loud themes are loud in the paper, not in a second accent.

## Settings

`theme` gains a value per theme (`solarized_light`, `tokyo_night`,
`deep_sea`, …). `system`, `light` and `dark` mean what they did, so no
version bump. Only `system` follows the OS.

## Rust

`core::settings::Theme` gains the variants and `Theme::ALL`. Nothing else in
Rust reads the theme: the tray icon follows the taskbar, not the window.

## UI

- `src/styles/themes.css`: a block of the same twenty tokens per theme, on a
  bare `[data-theme='…']` so a swatch can wear one. `tokens.css` keeps Paper
  and Night, as `[data-theme='light']` and `[data-theme='dark']`.
- `src/lib/theme.ts`: `SHELVES`, `THEMES`, `themeLabel`, and `resolveTheme`,
  which now returns the palette's name.
- `ThemePicker.svelte`: the shelves. One radio group to the keyboard: Tab
  reaches the theme worn, the arrows walk and wear the rest.

## IPC

None new: `update_settings`.

## Tests

- Rust: every theme serialises to a name of its own and reads back.
- UI: every theme on the picker has a block in the stylesheet that sets
  every token and a `color-scheme`; no block is missing from the picker.
- UI: every theme can be read. Measured as WCAG contrast from the
  stylesheet: ink on paper at least 6, on the second surface 5, secondary
  text 4, the line on the sky 3.5, and the sun at least 1.5 against its
  sky and its ground (Paper's own sun is 1.7), so its reflection shows.

## Docs

Guide (Settings, Themes), design (Palette), changelog, roadmap, handoff.

## Resource impact

About 14 KB of CSS before minifying. No timers, no images, nothing at rest.

## Acceptance

- [x] Each theme worn on Home in the harness: text legible, the sun the
      brightest thing, the strip and rule readable.
- [ ] Each theme in the real window on Windows, Almanac and Settings too.
- [ ] The same on the Mac.
- [ ] System still follows the OS when it changes while the window is open.

## Out of scope

Themes of one's own, a theme by time of day, fonts per theme, tray icons
per theme.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
