import { describe, expect, it } from 'vitest';

import themes from '../styles/themes.css?raw';
import tokens from '../styles/tokens.css?raw';

import {
  DEFAULT_OWN,
  isAfterDark,
  mix,
  msToTurn,
  ownTokens,
  parseThemeCode,
  resolveTheme,
  SHELVES,
  sunShape,
  themeAt,
  themeCode,
  themeLabel,
  THEMES,
  wear,
} from './theme';

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

describe('a theme after dark', () => {
  const evening = { enabled: true, theme: 'dark' as const, from_minutes: 1140, until_minutes: 420 };

  it('is worn from the evening until the morning, across midnight', () => {
    expect(themeAt('mars', evening, 1139)).toBe('mars');
    expect(themeAt('mars', evening, 1140)).toBe('dark');
    expect(themeAt('mars', evening, 30)).toBe('dark');
    expect(themeAt('mars', evening, 420)).toBe('mars');
    expect(themeAt('mars', { ...evening, enabled: false }, 30)).toBe('mars');
    expect(isAfterDark({ ...evening, from_minutes: 60, until_minutes: 120 }, 90)).toBe(true);
    expect(isAfterDark({ ...evening, from_minutes: 60, until_minutes: 60 }, 60)).toBe(false);
  });

  it('knows how long until it next turns over', () => {
    expect(msToTurn(evening, new Date(2026, 10, 12, 18, 59, 30))).toBe(30_000);
    expect(msToTurn(evening, new Date(2026, 10, 12, 19, 0, 0))).toBe(12 * 3_600_000);
    expect(msToTurn(evening, new Date(2026, 10, 12, 6, 0, 0))).toBe(3_600_000);
    expect(msToTurn({ ...evening, enabled: false }, new Date())).toBeNull();
  });
});

describe('a theme of your own', () => {
  it('works every token out from five colours', () => {
    const tokens = ownTokens(DEFAULT_OWN);
    for (const token of TOKENS) expect(tokens[`--${token}`], token).toBeTruthy();
    expect(tokens['--paper']).toBe('#f4f0e7');
    expect(tokens['color-scheme']).toBe('light');
    expect(mix('#000000', '#ffffff', 0.5)).toBe('#808080');
    const night = ownTokens({ ...DEFAULT_OWN, paper: '#101216', ink: '#ebe4d6' });
    expect(night['color-scheme']).toBe('dark');
    expect(night['--grain-blend']).toBe('screen');
    expect(contrast(night['--ink-2'] ?? '', '#101216')).toBeGreaterThan(4);
  });

  it('is shared as one line and read back from one', () => {
    const code = themeCode(DEFAULT_OWN);
    expect(code).toBe('sun:f4f0e7-1c1b18-dca544-dfe8ea-e8dfca');
    expect(parseThemeCode(`  ${code.toUpperCase().replace('SUN', 'sun')} `)).toEqual(DEFAULT_OWN);
    expect(parseThemeCode('sun:f4f0e7-1c1b18')).toBeNull();
    expect(parseThemeCode('sun:f4f0e7-1c1b18-dca544-dfe8ea-e8dfzz')).toBeNull();
    expect(parseThemeCode('red; } body { display: none')).toBeNull();
  });

  it('is put on as tokens, and taken off again', () => {
    const root = document.createElement('div');
    wear(root, 'own', { ...DEFAULT_OWN, sun: '#ff0000' });
    expect(root.dataset['theme']).toBe('own');
    expect(root.style.getPropertyValue('--sun')).toBe('#ff0000');
    wear(root, 'mars', DEFAULT_OWN);
    expect(root.dataset['theme']).toBe('mars');
    expect(root.style.getPropertyValue('--sun')).toBe('');
    // Colours that are not colours are never written into a style.
    wear(root, 'own', { ...DEFAULT_OWN, sun: 'red' });
    expect(root.style.getPropertyValue('--sun')).toBe('');
  });
});

describe('the sun a theme draws', () => {
  it('is a disc, but for three', () => {
    expect(sunShape('light')).toBe('disc');
    expect(sunShape('pocket')).toBe('pixel');
    expect(sunShape('thermal')).toBe('ring');
    expect(sunShape('risograph')).toBe('halftone');
    expect(sunShape('own')).toBe('disc');
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
      if (id === 'system' || id === 'own') continue;
      const block = blockOf(id);
      expect(block, id).not.toBeNull();
      for (const token of TOKENS) expect(block, `${id} --${token}`).toContain(`--${token}:`);
      expect(block, id).toMatch(/color-scheme: (light|dark);/);
    }
  });

  it('can each be read: ink on paper, and the sun against its sky and ground', () => {
    for (const { id } of THEMES) {
      if (id === 'system' || id === 'own') continue;
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
