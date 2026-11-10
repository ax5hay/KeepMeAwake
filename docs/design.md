# Design

The app has one job and one state. The interface is built so that the state is
legible from across the room and nothing on screen is decoration.

## The horizon

The face of the app is a landscape: a sky, a line, the ground. A sun sits
above the line while the machine is kept awake and below it while it rests. A
timed session is a sunset: the sun starts at its zenith and travels a quarter
circle to the horizon, arriving exactly when the session ends. A dotted arc
shows the path still to go.

This is not a metaphor laid over a dashboard; it is the status display.

```mermaid
stateDiagram-v2
    direction LR
    Resting: Resting<br/>sun below the line, crown showing<br/>dusk sky, stars
    Awake: Awake<br/>sun at zenith, still<br/>day sky, reflection
    Timed: Awake, timed<br/>sun on the arc, sinking<br/>dotted path to the horizon
    Paused: Paused<br/>sun in place, dimmed, cloud bar<br/>overcast sky
    Resting --> Awake: Always
    Resting --> Timed: a stop
    Awake --> Timed: a stop
    Timed --> Resting: sunset
    Awake --> Paused: battery rule
    Timed --> Paused: battery rule
    Paused --> Awake: charger
    Awake --> Resting: sun
    Timed --> Resting: sun
```

The sun is also the toggle. Its hit target is a real `<button>` positioned
over it, so keyboard users and screen readers get a button named "Keep awake"
or "Let it sleep" rather than an SVG.

## Palette

Two palettes, **Paper** and **Night**, both built from warm neutrals. There is
one accent, the sun, and nothing else in the UI is coloured. Switches, active
stops and the primary button are ink on paper, not accent on white; that is
what keeps the sun the brightest thing on screen.

| Token          | Paper     | Night     | Used for                             |
| -------------- | --------- | --------- | ------------------------------------ |
| `--paper`      | `#f4f0e7` | `#0f1216` | window background                    |
| `--paper-2`    | `#ebe6da` | `#161b21` | code chips, secondary surfaces       |
| `--ink`        | `#1c1b18` | `#ebe4d6` | text, the horizon line, active state |
| `--ink-2`      | `#5f5b52` | `#a79f91` | secondary text                       |
| `--ink-3`      | `#9a948a` | `#6d685f` | tertiary text, inactive stops        |
| `--sun`        | `#dca544` | `#e3b458` | the sun, afterglow, lit stars        |
| `--sky-on`     | `#dfe8ea` | `#1f2a38` | sky while awake                      |
| `--sky-off`    | `#cfd5d8` | `#131920` | sky while resting                    |
| `--sky-paused` | `#d5d3cd` | `#1b1e23` | sky while paused                     |

No gradients. Depth comes from three stepped bands of afterglow above the
line (the sun's colour at 10–16% opacity, like a risograph) and a static
film-grain overlay (`feTurbulence`, 8% multiply on Paper, 5% screen on Night)
that is rendered once and composited.

Theme follows the system (Windows or macOS) by default; Settings offers Paper
and Night by name.

## Type

| Face                | Where                                                                                  | Why                                                   |
| ------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Fraunces (variable) | the status word, the wordmark, whispers, the day's figure (in the Almanac and on Home) | one voice with character, used in exactly four places |
| Segoe UI Variable   | everything else, on Windows                                                            | native to Windows 11, perfect at 12–13 px             |
| SF Pro (system)     | everything else, on macOS                                                              | `-apple-system` in the same stack; native to the Mac  |

Fraunces is bundled (latin subset, upright and italic, 150 KB together) and
set at `opsz 72`, `SOFT 50`. Tabular numerals wherever a number can change.
No icon font: the few icons are 1.3 px stroked SVG paths drawn to a 16 px grid.

SF Pro sets about 14% wider than Segoe at 12 px. Rather than tighten its
tracking, which makes the system face look pinched, long hints are allowed to
wrap to a second line; the Tips rows and the rule's stop labels still fit at
380 px. Shortcuts on a Mac are shown in glyphs (`⌥⌘K`), modifiers in the order
macOS menus use: ⌃ ⌥ ⇧ ⌘.

The menu bar icon on macOS is a **template image**: the same three tray
drawings in pure black with alpha, 22 pt at 2x, which macOS tints for a light
or dark menu bar. No colour, so the sun's amber stays in the window.

## Layout

380 × 600 by default, frameless, custom 40 px title bar. Top to bottom:

1. **Title bar**: wordmark in italic Fraunces; almanac, settings, minimise,
   close. The close button is red only on hover, the one concession to
   platform habit.
2. **Scene**: 380 × 240, scales with width, capped at 440.
3. **Today** (with the almanac on): the day as a small horizon directly
   under the scene, the picture and then its ledger. Its figure stands at
   the right of the status word. Both open the Almanac.
4. **Status**: the word (Resting / Awake / Paused) and one line of detail.
   Whispers (tips, easter eggs) briefly replace the detail in italic.
5. **Rule**: durations as stops on a hairline. The active stop has a filled
   dot and a doubled tick. "Until…" opens an inline time field.
6. **Rows**: the two switches people flip daily.
7. **Doors**: one quiet line, `Schedule · Overnight · History · Ctrl+Alt+K`,
   each opening Settings at its section, with a tooltip saying what is
   behind it. The schedule is off until set, the guard is quiet until it
   acts, history only records, and the hotkey shows its own combination, so
   the line says each exists. A waiting update leads it (`Update 0.3.1`, in
   full ink) and the hotkey door makes room: one line, always. The line
   gives way below 560 px of height, where the two rows need the room.

Everything else is behind the settings button (sliders, not a gear: at
16 px a gear is one more sun, in an app where the sun is the one accent and
the switch), in a pane that slides in from the right
and uses the same Row primitive with switches, a two-arrow stepper for lists
and a segmented control for three-way choices. No dropdowns, no modals.

### The Almanac

A third pane, behind the open book in the title bar. It is built from the
scene's own parts, so it reads as the same place seen over time:

| Part          | Drawn as                                                                                         |
| ------------- | ------------------------------------------------------------------------------------------------ |
| the day       | a figure in Fraunces at the size of Home's status word, with one line of detail                  |
| a day's strip | a horizon line; a band of `--sky-on` over the working hours; ink on the line where you were here |
| the night     | a dotted run under the line, as the scene dots the path the sun has yet to travel                |
| now           | the sun, on the line, at the leading edge of today's ink: the one accent, where the sun is       |
| a day worked  | in the month, a 2 px bar under the date, longer for a longer day                                 |
| leave         | a ring: a sun that did not rise                                                                  |

The strips are positioned boxes, not an SVG: they stretch with the pane, and
boxes stretch without bending their dots or their sun. Nothing in the pane
is a chart with axes; the only labels are four hours under the day's strip.

Spacing is a 4 px grid with 24 px side margins. Hairlines are 1 px at 14% ink.
Radii: 6 px for controls, 10 px for surfaces.

## Motion

| Thing              | Duration | Easing                     |
| ------------------ | -------- | -------------------------- |
| sun travel, sky    | 720 ms   | `cubic-bezier(.4,0,.2,1)`  |
| pane slide         | 320 ms   | `cubic-bezier(.2,.7,.2,1)` |
| hover, switch knob | 140 ms   | standard                   |

The sun moves on `transform` only. `prefers-reduced-motion` zeroes all three.

## Copy

Short, lower-case where Windows is, no exclamation marks. States are named for
what the laptop is doing ("Resting", "Awake"), not what the app is doing. Hints
explain consequences, never restate the label. All copy that is more than a
label lives in `src/lib/copy.ts` and is tested for length and tone.

## Hidden things

They exist so the app rewards curiosity without cluttering the surface. Each
is harmless, discoverable by touching the scene, and never required.

| Touch                  | Happens                                                        |
| ---------------------- | -------------------------------------------------------------- |
| the horizon line       | a tip, nine in rotation                                        |
| a star (resting)       | it lights in sun colour; the seventh draws the constellation   |
| the reflection (awake) | the water ripples                                              |
| the wordmark           | a credit line                                                  |
| 00:00–05:00            | the awake line gains "It is late. The sun will hold the line." |

## Decided against

| Idea                        | Why not                                                            |
| --------------------------- | ------------------------------------------------------------------ |
| Mica / acrylic window       | the scene paints its own sky; translucency fights it               |
| a big primary button        | the sun is the button; a second one would make the sun decorative  |
| a gear for settings         | at 16 px its spokes read as a sun, the app's switch and its mark   |
| a third row on Home         | no room at 340 × 520; the doors point into Settings instead        |
| chips for durations         | a rule reads as an instrument and uses a quarter of the height     |
| a calendar icon             | any app's; an almanac is a book, so the button is an open one      |
| a fifth door on Home        | the line must stay one line; the almanac is a pane, not a setting  |
| today's strip by the rule   | two hairlines with marks, a finger apart, read as one instrument   |
| today in the scene's ground | the scene is the status display; a second reading there muddies it |
| bar charts of hours         | a day is a horizon here, not a column; the strip shows when        |
| "you slept 7h"              | it knows the keys went quiet, not what you did: it says "night"    |
| accent-coloured controls    | one accent, used once                                              |
| toasts in 0.1               | a sunset you can see is enough; on the roadmap behind a setting    |

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
