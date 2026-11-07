import { describe, expect, it } from 'vitest';

import { displayAccelerator, keyName, toAccelerator, type KeyLike } from './hotkey';

function press(code: string, mods: Partial<Omit<KeyLike, 'code'>> = {}): KeyLike {
  return { code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods };
}

describe('toAccelerator', () => {
  it('records letters upper-case', () => {
    expect(toAccelerator(press('KeyK', { ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+K');
  });

  it('records digits and function keys', () => {
    expect(toAccelerator(press('Digit1', { ctrlKey: true }))).toBe('Ctrl+1');
    expect(toAccelerator(press('F9', { metaKey: true, shiftKey: true }))).toBe('Shift+Super+F9');
    expect(toAccelerator(press('F12', { altKey: true }))).toBe('Alt+F12');
  });

  it('records Space, Enter and arrows', () => {
    expect(toAccelerator(press('Space', { altKey: true }))).toBe('Alt+Space');
    expect(toAccelerator(press('Enter', { ctrlKey: true }))).toBe('Ctrl+Enter');
    expect(toAccelerator(press('ArrowUp', { ctrlKey: true, shiftKey: true }))).toBe(
      'Ctrl+Shift+Up',
    );
  });

  it('orders modifiers Ctrl, Alt, Shift, Super', () => {
    const all = { ctrlKey: true, altKey: true, shiftKey: true, metaKey: true };
    expect(toAccelerator(press('KeyA', all))).toBe('Ctrl+Alt+Shift+Super+A');
  });

  it('rejects combinations without Ctrl, Alt or Win', () => {
    expect(toAccelerator(press('KeyK'))).toBeNull();
    expect(toAccelerator(press('KeyK', { shiftKey: true }))).toBeNull();
  });

  it('waits while only modifiers are down, and ignores keys it cannot use', () => {
    expect(toAccelerator(press('ControlLeft', { ctrlKey: true }))).toBeNull();
    expect(toAccelerator(press('Semicolon', { ctrlKey: true }))).toBeNull();
  });
});

describe('toAccelerator without a code', () => {
  it('falls back to the key', () => {
    const synthetic = { ...press('', { ctrlKey: true, shiftKey: true }), key: 'K' };
    expect(toAccelerator(synthetic)).toBe('Ctrl+Shift+K');
    expect(toAccelerator({ ...press('', { altKey: true }), key: ' ' })).toBe('Alt+Space');
    expect(toAccelerator({ ...press('', { altKey: true }), key: 'F9' })).toBe('Alt+F9');
    expect(toAccelerator({ ...press('', { ctrlKey: true }), key: '!' })).toBeNull();
  });
});

describe('keyName', () => {
  it('maps codes to accelerator keys', () => {
    expect(keyName('KeyZ')).toBe('Z');
    expect(keyName('Digit0')).toBe('0');
    expect(keyName('F24')).toBe('F24');
    expect(keyName('F25')).toBeNull();
    expect(keyName('ArrowLeft')).toBe('Left');
  });
});

describe('displayAccelerator', () => {
  it('calls Super Win', () => {
    expect(displayAccelerator('Super+Shift+F9')).toBe('Win+Shift+F9');
    expect(displayAccelerator('Ctrl+Alt+K')).toBe('Ctrl+Alt+K');
  });
});
