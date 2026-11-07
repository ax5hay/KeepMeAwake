/**
 * Turning a key press into the accelerator syntax the Rust side registers
 * (`Ctrl+Alt+K`, `Super+Shift+F9`). Pure, so it is tested without a DOM.
 */

/** The parts of a `KeyboardEvent` the recorder reads. */
export interface KeyLike {
  code: string;
  /** Used only when `code` is empty, as it is for some synthesised input. */
  key?: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const ARROWS: Readonly<Record<string, string>> = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
};

/**
 * `KeyboardEvent.key` spelled as a `code`, for events that carry no code.
 * Layout-dependent, which is why `code` is preferred whenever it is there.
 */
export function codeFromKey(key: string): string {
  if (/^[a-z]$/i.test(key)) return `Key${key.toUpperCase()}`;
  if (/^\d$/.test(key)) return `Digit${key}`;
  if (key === ' ') return 'Space';
  return key;
}

/** The main key of an accelerator, from `KeyboardEvent.code`, or `null`. */
export function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^Digit(\d)$/.exec(code);
  if (digit) return digit[1] ?? null;
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  if (code === 'Space' || code === 'Enter') return code;
  return ARROWS[code] ?? null;
}

/**
 * The accelerator for a key press, modifiers in the order Ctrl, Alt, Shift,
 * Super (Cmd on a Mac), then the key. `null` while only modifiers are down,
 * for keys the hotkey cannot use, and without at least one of Ctrl, Alt or
 * Win (Shift alone would steal ordinary typing).
 */
export function toAccelerator(event: KeyLike, mac = false): string | null {
  const key = keyName(event.code || codeFromKey(event.key ?? ''));
  if (key === null) return null;
  if (!(event.ctrlKey || event.altKey || event.metaKey)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push(mac ? 'Cmd' : 'Super');
  parts.push(key);
  return parts.join('+');
}

/** Mac modifier glyphs, in the order macOS menus list them. */
const MAC_MODIFIERS: ReadonlyArray<readonly [glyph: string, names: readonly string[]]> = [
  ['⌃', ['Ctrl', 'Control']],
  ['⌥', ['Alt', 'Option']],
  ['⇧', ['Shift']],
  ['⌘', ['Cmd', 'Command', 'Super', 'Meta']],
];

/** Keys macOS menus draw as symbols. */
const MAC_KEYS: Readonly<Record<string, string>> = {
  Up: '↑',
  Down: '↓',
  Left: '←',
  Right: '→',
  Enter: '↩',
};

/**
 * How an accelerator reads on screen. Windows calls Super "Win"; a Mac shows
 * glyphs without separators (`⌥⌘K`), modifiers in menu order whatever order
 * they were stored in.
 */
export function displayAccelerator(accelerator: string, mac = false): string {
  const parts = accelerator.split('+');
  if (!mac) return parts.map((part) => (part === 'Super' ? 'Win' : part)).join('+');
  const key = parts.at(-1) ?? '';
  const mods = new Set(parts.slice(0, -1));
  const glyphs = MAC_MODIFIERS.filter(([, names]) => names.some((n) => mods.has(n))).map(
    ([glyph]) => glyph,
  );
  return glyphs.join('') + (MAC_KEYS[key] ?? key);
}
