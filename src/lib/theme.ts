/**
 * Resolves the user's theme choice to a concrete light/dark value and writes
 * it to `<html data-theme>`. CSS only ever reads that attribute, so the
 * "system" case is handled in one place.
 */
import type { Theme } from './types';

export type ResolvedTheme = 'light' | 'dark';

const query = '(prefers-color-scheme: dark)';

export function resolveTheme(theme: Theme, systemPrefersDark: boolean): ResolvedTheme {
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
