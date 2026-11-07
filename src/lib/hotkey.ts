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
 * Super, then the key. `null` while only modifiers are down, for keys the
 * hotkey cannot use, and without at least one of Ctrl, Alt or Win (Shift
 * alone would steal ordinary typing).
 */
export function toAccelerator(event: KeyLike): string | null {
  const key = keyName(event.code || codeFromKey(event.key ?? ''));
  if (key === null) return null;
  if (!(event.ctrlKey || event.altKey || event.metaKey)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');
  parts.push(key);
  return parts.join('+');
}

/** How an accelerator reads on screen: Windows calls Super "Win". */
export function displayAccelerator(accelerator: string): string {
  return accelerator
    .split('+')
    .map((part) => (part === 'Super' ? 'Win' : part))
    .join('+');
}
