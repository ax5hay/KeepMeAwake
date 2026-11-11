import { describe, expect, it } from 'vitest';

import themes from '../styles/themes.css?raw';
import tokens from '../styles/tokens.css?raw';

import { resolveTheme, SHELVES, themeLabel, THEMES } from './theme';

const css = `${tokens}
${themes}`;

/** The body of the block that sets `theme`'s tokens. */
function blockOf(theme: string): string | null {
  const at = css.indexOf(`[data-theme='${theme}'] {`);
  if (at < 0) return null;
  return css.slice(at, css.indexOf('}', at));
}

const TOKENS = [
  'paper',
  'paper-2',
  'ink',
  'ink-2',
  'ink-3',
  'hair',
  'hair-strong',
  'sun',
  'sun-dim',
  'sky-off',
  'sky-on',
  'sky-paused',
  'ground-off',
  'ground-on',
  'grain-blend',
  'grain-opacity',
  'afterglow-off',
  'afterglow-on',
  'stars',
  'shadow',
];

describe('resolveTheme', () => {
  it('follows the OS only for system', () => {
    expect(resolveTheme('system', true)).toBe('dark');
    expect(resolveTheme('system', false)).toBe('light');
    expect(resolveTheme('light', true)).toBe('light');
    expect(resolveTheme('dark', false)).toBe('dark');
    expect(resolveTheme('hot_dog_stand', true)).toBe('hot_dog_stand');
  });
});

describe('the themes', () => {
  it('are each listed once, with a name', () => {
    const ids = THEMES.map((theme) => theme.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.slice(0, 3)).toEqual(['system', 'light', 'dark']);
    expect(themeLabel('dark')).toBe('Night');
    expect(themeLabel('tokyo_night')).toBe('Tokyo Night');
    for (const shelf of SHELVES) expect(shelf.themes.length).toBeGreaterThan(0);
  });

  it('each have a palette in the stylesheet that sets every token', () => {
    for (const { id } of THEMES) {
      if (id === 'system') continue;
      const block = blockOf(id);
      expect(block, id).not.toBeNull();
      for (const token of TOKENS) expect(block, `${id} --${token}`).toContain(`--${token}:`);
      expect(block, id).toMatch(/color-scheme: (light|dark);/);
    }
  });

  it('have no palette in the stylesheet that is not on the picker', () => {
    const named = [...css.matchAll(/\[data-theme='([a-z_]+)'\] \{/g)].map((m) => m[1]);
    const ids: string[] = THEMES.map((theme) => theme.id);
    for (const name of named) expect(ids).toContain(name);
  });
});
