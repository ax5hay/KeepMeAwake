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

/** Relative luminance of `#rrggbb`, as WCAG has it. */
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((at) => {
    const c = parseInt(hex.slice(at, at + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (r ?? 0) + 0.7152 * (g ?? 0) + 0.0722 * (b ?? 0);
}

/** WCAG contrast of two colours, 1 to 21. */
function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
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
    expect(resolveTheme('deep_sea', true)).toBe('deep_sea');
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

  it('can each be read: ink on paper, and the sun against its sky and ground', () => {
    for (const { id } of THEMES) {
      if (id === 'system') continue;
      const block = blockOf(id) ?? '';
      const of = (token: string) => {
        const found = new RegExp(`--${token}: (#[0-9a-f]{6});`).exec(block);
        expect(found, `${id} --${token}`).not.toBeNull();
        return found?.[1] ?? '#000000';
      };
      const paper = of('paper');
      expect(contrast(of('ink'), paper), `${id} ink on paper`).toBeGreaterThanOrEqual(6);
      expect(contrast(of('ink'), of('paper-2')), `${id} ink on paper-2`).toBeGreaterThanOrEqual(5);
      expect(contrast(of('ink-2'), paper), `${id} ink-2 on paper`).toBeGreaterThanOrEqual(4);
      expect(contrast(of('ink-3'), paper), `${id} ink-3 on paper`).toBeGreaterThanOrEqual(2);
      // The horizon line and the tips are ink on the sky.
      expect(contrast(of('ink'), of('sky-on')), `${id} ink on sky`).toBeGreaterThanOrEqual(3.5);
      // The sun has to be seen where it is drawn, and so does its reflection.
      expect(contrast(of('sun'), of('sky-on')), `${id} sun on sky`).toBeGreaterThanOrEqual(1.5);
      expect(contrast(of('sun'), of('ground-on')), `${id} sun on ground`).toBeGreaterThanOrEqual(
        1.5,
      );
    }
  });

  it('have no palette in the stylesheet that is not on the picker', () => {
    const named = [...css.matchAll(/\[data-theme='([a-z_]+)'\] \{/g)].map((m) => m[1]);
    const ids: string[] = THEMES.map((theme) => theme.id);
    for (const name of named) expect(ids).toContain(name);
  });
});
