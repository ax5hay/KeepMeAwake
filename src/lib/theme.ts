/**
 * The themes: which there are, which is worn now (`system` follows the OS,
 * and an evening can have its own), and how one is put on. A theme becomes
 * a palette's name on `<html data-theme>`, and CSS only ever reads that.
 */
import type { AfterDark, OwnTheme, Theme } from './types';

/** A palette: every theme but `system`, which is one of two of them. */
export type Palette = Exclude<Theme, 'system'>;

/** One theme on the picker. */
export interface ThemeChoice {
  id: Theme;
  label: string;
}

/** A shelf of the picker. */
export interface ThemeShelf {
  /** Said above it. */
  title: string;
  /** What kind of theme is on it. */
  hint: string;
  themes: ReadonlyArray<ThemeChoice>;
}

/** Every theme, as the picker shelves them. */
export const SHELVES: ReadonlyArray<ThemeShelf> = [
  {
    title: 'Its own',
    hint: 'Paper by day, Night by night.',
    themes: [
      { id: 'system', label: 'System' },
      { id: 'light', label: 'Paper' },
      { id: 'dark', label: 'Night' },
    ],
  },
  {
    title: 'Borrowed',
    hint: 'From the editor you already live in.',
    themes: [
      { id: 'solarized_light', label: 'Solarized Light' },
      { id: 'solarized_dark', label: 'Solarized Dark' },
      { id: 'gruvbox', label: 'Gruvbox' },
      { id: 'nord', label: 'Nord' },
      { id: 'dracula', label: 'Dracula' },
      { id: 'catppuccin', label: 'Catppuccin' },
      { id: 'tokyo_night', label: 'Tokyo Night' },
      { id: 'monokai', label: 'Monokai' },
    ],
  },
  {
    title: 'Play',
    hint: 'Somewhere else the horizon could be.',
    themes: [
      { id: 'terminal', label: 'Terminal' },
      { id: 'blueprint', label: 'Blueprint' },
      { id: 'pocket', label: 'Pocket' },
      { id: 'newsprint', label: 'Newsprint' },
      { id: 'risograph', label: 'Risograph' },
      { id: 'dial_up', label: 'Dial-up' },
      { id: 'mars', label: 'Mars' },
      { id: 'deep_sea', label: 'Deep Sea' },
    ],
  },
  {
    title: 'Loud',
    hint: 'On purpose.',
    themes: [
      { id: 'vaporwave', label: 'Vaporwave' },
      { id: 'thermal', label: 'Thermal' },
      { id: 'negative', label: 'Negative' },
      { id: 'miami', label: 'Miami' },
      { id: 'bubblegum', label: 'Bubblegum' },
      { id: 'highlighter', label: 'Highlighter' },
      { id: 'grape_soda', label: 'Grape Soda' },
      { id: 'hi_vis', label: 'Hi-Vis' },
    ],
  },
  {
    title: 'Yours',
    hint: 'Five colours, and it is a theme.',
    themes: [{ id: 'own', label: 'Yours' }],
  },
];

/** Every theme, in picker order. */
export const THEMES: ReadonlyArray<ThemeChoice> = SHELVES.flatMap((shelf) => shelf.themes);

/** What a theme is called; Paper for one this build has never heard of. */
export function themeLabel(theme: Theme): string {
  return THEMES.find((choice) => choice.id === theme)?.label ?? 'Paper';
}

/** The media query `system` follows. */
export const SYSTEM_DARK = '(prefers-color-scheme: dark)';

/** The palette `theme` wears: itself, or for `system` what the OS is in. */
export function resolveTheme(theme: Theme, systemPrefersDark: boolean): Palette {
  if (theme === 'system') return systemPrefersDark ? 'dark' : 'light';
  return theme;
}

const DAY_MINUTES = 24 * 60;

/** Whether `minute` of the day is inside the evening `afterDark` describes. */
export function isAfterDark(afterDark: AfterDark, minute: number): boolean {
  if (!afterDark.enabled) return false;
  const { from_minutes: from, until_minutes: until } = afterDark;
  if (from === until) return false;
  return from < until ? minute >= from && minute < until : minute >= from || minute < until;
}

/** The theme worn at `minute` of the day: the evening's, inside the evening. */
export function themeAt(theme: Theme, afterDark: AfterDark, minute: number): Theme {
  return isAfterDark(afterDark, minute) ? afterDark.theme : theme;
}

/**
 * Milliseconds from `now` to the next time the evening starts or ends, or
 * `null` when there is no evening to wait for. One timeout is set to it
 * while the window is open, so the theme turns over without a poll.
 */
export function msToTurn(afterDark: AfterDark, now: Date): number | null {
  if (!afterDark.enabled || afterDark.from_minutes === afterDark.until_minutes) return null;
  const at = now.getHours() * 60 + now.getMinutes();
  const waits = [afterDark.from_minutes, afterDark.until_minutes].map(
    (turn) => (turn - at + DAY_MINUTES - 1) % DAY_MINUTES,
  );
  const minutes = Math.min(...waits) + 1;
  return minutes * 60_000 - now.getSeconds() * 1000 - now.getMilliseconds();
}

/** `#rrggbb` as three numbers, 0 to 255. */
function channels(hex: string): [number, number, number] {
  return [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16)) as [number, number, number];
}

/** `a` with `amount` (0 to 1) of `b` mixed in, as `#rrggbb`. */
export function mix(a: string, b: string, amount: number): string {
  const [x, y] = [channels(a), channels(b)];
  return `#${x
    .map((c, i) =>
      Math.round(c + ((y[i] ?? c) - c) * amount)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}

/** Relative luminance of `#rrggbb`, 0 to 1, as WCAG has it. */
export function luminance(hex: string): number {
  const [r, g, b] = channels(hex).map((c) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
}

const COLOUR = /^#[0-9a-f]{6}$/;

/** The five colours of a theme of one's own, in the order they are shared. */
export const OWN_PARTS = ['paper', 'ink', 'sun', 'sky', 'ground'] as const;

/**
 * Every token of a theme of one's own, worked out from its five colours:
 * the quieter inks are ink let down with paper, the resting sky and ground
 * are the awake ones let down the same way, and a dark paper turns the
 * grain, the stars and the shadow over as Night does.
 */
export function ownTokens(own: OwnTheme): Record<string, string> {
  const dark = luminance(own.paper) < 0.3;
  const ink = channels(own.ink).join(' ');
  return {
    '--paper': own.paper,
    '--paper-2': mix(own.paper, own.ink, 0.07),
    '--ink': own.ink,
    '--ink-2': mix(own.ink, own.paper, 0.32),
    '--ink-3': mix(own.ink, own.paper, 0.56),
    '--hair': `rgb(${ink} / ${dark ? 0.12 : 0.14})`,
    '--hair-strong': `rgb(${ink} / ${dark ? 0.3 : 0.34})`,
    '--sun': own.sun,
    '--sun-dim': mix(own.sun, own.paper, 0.5),
    '--sky-off': mix(own.sky, own.paper, 0.45),
    '--sky-on': own.sky,
    '--sky-paused': mix(own.sky, own.paper, 0.7),
    '--ground-off': mix(own.ground, own.paper, 0.45),
    '--ground-on': own.ground,
    '--grain-blend': dark ? 'screen' : 'multiply',
    '--grain-opacity': dark ? '0.05' : '0.08',
    '--afterglow-off': dark ? '0.1' : '0.16',
    '--afterglow-on': dark ? '0.12' : '0.1',
    '--stars': dark ? '0.5' : '0.14',
    '--shadow': dark ? '0 1px 2px rgb(0 0 0 / 0.4)' : `0 1px 2px rgb(${ink} / 0.08)`,
    'color-scheme': dark ? 'dark' : 'light',
  };
}

/** The same tokens as one `style` attribute, for a swatch. */
export function ownStyle(own: OwnTheme): string {
  return Object.entries(ownTokens(own))
    .map(([name, value]) => `${name}: ${value}`)
    .join('; ');
}

/** A theme of one's own as one line to share: `sun:f4f0e7-1c1b18-dca544-dfe8ea-e8dfca`. */
export function themeCode(own: OwnTheme): string {
  return `sun:${OWN_PARTS.map((part) => own[part].slice(1)).join('-')}`;
}

/** The theme a shared line holds, or `null` if it is not one. */
export function parseThemeCode(text: string): OwnTheme | null {
  const found = /^\s*sun:((?:[0-9a-f]{6}-){4}[0-9a-f]{6})\s*$/i.exec(text);
  if (!found?.[1]) return null;
  const [paper, ink, sun, sky, ground] = found[1]
    .toLowerCase()
    .split('-')
    .map((part) => `#${part}`);
  if (!paper || !ink || !sun || !sky || !ground) return null;
  return { paper, ink, sun, sky, ground };
}

/** Whether `own` is five colours, as the settings keep them. */
export function isOwnTheme(own: OwnTheme): boolean {
  return OWN_PARTS.every((part) => COLOUR.test(own[part]));
}

/**
 * Puts `palette` on `root`: its name on `data-theme`, which is all a theme
 * in the stylesheet needs, and for one's own the tokens themselves.
 */
export function wear(root: HTMLElement, palette: Palette, own: OwnTheme): void {
  root.dataset['theme'] = palette;
  const tokens = palette === 'own' && isOwnTheme(own) ? ownTokens(own) : null;
  for (const name of Object.keys(ownTokens(DEFAULT_OWN))) {
    if (tokens) root.style.setProperty(name, tokens[name] ?? '');
    else root.style.removeProperty(name);
  }
}

/** Paper's five, where a theme of one's own starts. */
export const DEFAULT_OWN: OwnTheme = {
  paper: '#f4f0e7',
  ink: '#1c1b18',
  sun: '#dca544',
  sky: '#dfe8ea',
  ground: '#e8dfca',
};

/** How a theme draws its sun. Most draw a disc. */
export type SunShape = 'disc' | 'pixel' | 'ring' | 'halftone';

/**
 * The sun a palette draws: square pixels on the handheld's screen, a ring
 * around a white-hot core as a heat camera sees it, a halftone dot off the
 * press. Colour is the theme's business; this is the one shape that is too.
 */
export function sunShape(palette: Palette): SunShape {
  if (palette === 'pocket') return 'pixel';
  if (palette === 'thermal') return 'ring';
  if (palette === 'risograph') return 'halftone';
  return 'disc';
}
