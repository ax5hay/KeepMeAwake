# Small things

Status: built on Windows; waits for the Mac.

## Why

The app works without any of these. They are here because software used to
have them: a detail that shows someone looked, a thing to find, a way to
make it one's own. Each is small, optional where it changes anything, and
costs nothing at rest.

## Experience

| Thing                     | Where                                  | What it does                                                                                                                                                           |
| ------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A sun shaped by its theme | Home                                   | Pocket draws its sun in square pixels, Thermal as a ring around a white-hot core, Risograph as a halftone dot. Every other theme draws a disc.                         |
| Tonight's moon            | Home, resting, in Night                | The moon as it is tonight, from the date alone. Click it: `Waxing gibbous, 81% lit.`                                                                                   |
| A shooting star           | Home, resting                          | About once in thirty openings of the window, one star falls. Never with reduced motion.                                                                                |
| Look before you choose    | Settings → Theme                       | Pointing at a swatch wears that theme on the whole window until the pointer leaves. A click keeps it.                                                                  |
| A theme of your own       | Settings → Theme → Yours               | Five colours: Paper, Ink, Sun, Sky, Ground. The rest is worked out. One line shares it: `sun:f4f0e7-1c1b18-dca544-dfe8ea-e8dfca`; paste a line to wear someone else's. |
| After dark                | Settings → Theme → After dark          | Another theme from one time until another (7 pm until 7 am, Night, by default). Off unless you ask.                                                                    |
| Every key, on one sheet   | `?` anywhere                           | The keys and what they do, laid over the window. Esc or Back lifts it.                                                                                                 |
| A year, looked back on    | Almanac → the year → Look back         | Hours, days and leave; twelve months as bars; the fullest month, longest day, earliest start, latest finish, longest run, time outside your hours, usual night.        |
| Print month               | Almanac, beside Copy month as CSV      | The open month as a timesheet, to the system's print dialog. Black on white whatever theme is worn.                                                                    |
| Sounds                    | Settings → Theme → Sounds              | A soft rising fifth as the sun rises, falling as it sets. Off by default, and only from an open window.                                                                |
| Mini window               | Title bar, first button                | The scene and one line, 220 by 178, above other windows. The sun still switches. Its button, or Esc, brings the rest back.                                             |
| Credits                   | Settings → About → Credits and history | The versions as a line of dots, newest first, and whose work this is made with: the typeface, the frameworks, and the author of each borrowed palette.                 |

## Settings

| Field        | Type                                               | Default               | Notes                                                                                |
| ------------ | -------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------ |
| `theme`      | gains `own`                                        |                       | the fifth shelf                                                                      |
| `own_theme`  | `{ paper, ink, sun, sky, ground }`, each `#rrggbb` | Paper's five          | anything that is not a colour falls back to its default, in Rust, before it is saved |
| `after_dark` | `{ enabled, theme, from_minutes, until_minutes }`  | off, Night, 1140, 420 | past midnight is fine                                                                |
| `sounds`     | `bool`                                             | `false`               |                                                                                      |

All default, none changes the meaning of an old field: no version bump.

## Rust

- `core::settings`: `Theme::Own`, `OwnTheme` (sanitised to colours only),
  `AfterDark`, `sounds`.
- `app::window::set_mini`: limits moved before the size on the way down and
  after it on the way up; always-on-top with it.
- Commands `set_mini(on)` and `print_window()`.

## UI

- `theme.ts`: `ownTokens` (twenty tokens from five colours), `themeCode` /
  `parseThemeCode`, `themeAt` and `msToTurn` for the evening, `wear`,
  `sunShape`.
- `moon.ts`: `moonPhase`, `moonLit`, `moonName`, `moonPath`.
- `sound.ts`: two sine notes from the Web Audio API; no sound file.
- `Sheets.svelte` (keys, credits, the year), `Mini.svelte`,
  `Timesheet.svelte` (shown only by the print stylesheet).
- `almanac.ts`: `yearReview`.

## IPC

`set_mini`, `print_window`. The rest is `update_settings`.

## Tests

- Rust: an own theme keeps only colours; after dark defaults and is clamped.
- UI: the moon against three almanac dates and its four path shapes; the
  evening across midnight and the wait to its next turn; own tokens, the
  shared line both ways and against nonsense; `wear` on and off; `?` opens
  the sheet but not while typing; the year review.

## Resource impact

Nothing at rest: all of this lives in the window, which does not exist when
closed. With After dark on, one timeout to the next turn of the evening
while the window is open. The shooting star is one CSS animation played
once. Sounds build an audio context on the first note and not before.

## Acceptance

- [x] Each thing seen in the harness (Windows).
- [x] The mini window in the real app on Windows: 220 by 178 and back to
      380 by 600.
- [ ] Always on top, and dragging the mini window by its line, by hand.
- [ ] Print month through a real print dialog, on both machines.
- [ ] Sounds heard; the shooting star seen.
- [ ] A colour picked in the real window (the system's colour dialog).
- [ ] All of it on the Mac.

## Out of scope

Themes that change type or layout, a library of shared themes, sounds for
anything but the sun, the moon as seen from the southern hemisphere.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
