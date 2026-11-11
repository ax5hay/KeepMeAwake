import { describe, expect, it } from 'vitest';

import { windowAction, type KeyPress } from './keys';

function key(k: string, mods: Partial<Omit<KeyPress, 'key'>> = {}): KeyPress {
  return { key: k, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods };
}

describe('windowAction', () => {
  it('toggles on Space, except while typing or on a button', () => {
    expect(windowAction(key(' '), 'BODY', false)).toBe('toggle');
    expect(windowAction(key(' '), 'INPUT', false)).toBeNull();
    expect(windowAction(key(' '), 'BUTTON', false)).toBeNull();
  });

  it('goes back or closes on Esc', () => {
    expect(windowAction(key('Escape'), 'BODY', false)).toBe('back-or-close');
    expect(windowAction(key('Escape'), 'BODY', true)).toBe('back-or-close');
  });

  it('opens settings on Ctrl+, and ⌘,', () => {
    expect(windowAction(key(',', { ctrlKey: true }), 'BODY', false)).toBe('settings');
    expect(windowAction(key(',', { metaKey: true }), 'BODY', true)).toBe('settings');
  });

  it('closes on ⌘W and minimises on ⌘M, on a Mac only', () => {
    expect(windowAction(key('w', { metaKey: true }), 'BODY', true)).toBe('close');
    expect(windowAction(key('m', { metaKey: true }), 'BODY', true)).toBe('minimise');
    expect(windowAction(key('W', { metaKey: true, shiftKey: true }), 'BODY', true)).toBeNull();
    expect(windowAction(key('w', { metaKey: true }), 'BODY', false)).toBeNull();
    expect(windowAction(key('w', { ctrlKey: true }), 'BODY', true)).toBeNull();
  });

  it('closes on ⌘W even while typing, as every Mac window does', () => {
    expect(windowAction(key('w', { metaKey: true }), 'INPUT', true)).toBe('close');
  });

  it('ignores everything else', () => {
    expect(windowAction(key('k'), 'BODY', true)).toBeNull();
  });

  it('opens the key sheet on ?, but not while typing', () => {
    const press = { key: '?', ctrlKey: false, metaKey: false, altKey: false, shiftKey: true };
    expect(windowAction(press, 'BODY', false)).toBe('keys');
    expect(windowAction(press, 'BODY', true)).toBe('keys');
    expect(windowAction(press, 'INPUT', false)).toBeNull();
    expect(windowAction({ ...press, ctrlKey: true }, 'BODY', false)).toBeNull();
  });
});
