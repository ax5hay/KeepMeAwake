/**
 * The themes, and the one place `system` is resolved: the choice becomes a
 * palette's name on `<html data-theme>`, and CSS only ever reads that.
 */
import type { Theme } from './types';

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
    ],
  },
  {
    title: 'Loud',
    hint: 'On purpose.',
    themes: [
      { id: 'vaporwave', label: 'Vaporwave' },
      { id: 'hot_dog_stand', label: 'Hot Dog Stand' },
      { id: 'bubblegum', label: 'Bubblegum' },
      { id: 'highlighter', label: 'Highlighter' },
    ],
  },
];

/** Every theme, in picker order. */
export const THEMES: ReadonlyArray<ThemeChoice> = SHELVES.flatMap((shelf) => shelf.themes);

/** What a theme is called; Paper for one this build has never heard of. */
export function themeLabel(theme: Theme): string {
  return THEMES.find((choice) => choice.id === theme)?.label ?? 'Paper';
}

const query = '(prefers-color-scheme: dark)';

/** The palette `theme` wears: itself, or for `system` what the OS is in. */
export function resolveTheme(theme: Theme, systemPrefersDark: boolean): Palette {
  if (theme === 'system') return systemPrefersDark ? 'dark' : 'light';
  return theme;
}

/** Applies `theme` now and keeps following the OS while `theme` is `system`. */
export function watchTheme(getTheme: () => Theme): () => void {
  const media = window.matchMedia(query);
  const apply = () => {
    document.documentElement.dataset['theme'] = resolveTheme(getTheme(), media.matches);
  };
  apply();
  media.addEventListener('change', apply);
  return () => media.removeEventListener('change', apply);
}
