/**
 * The window's own keyboard shortcuts, as a pure mapping from a key press to
 * what the window should do, so the platform differences are tested without
 * a DOM. The global hotkey is separate: see `hotkey.ts`.
 */

/** The parts of a `KeyboardEvent` the window reads. */
export interface KeyPress {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

/** What a shortcut asks the window to do. */
export type WindowAction =
  'toggle' | 'back-or-close' | 'settings' | 'close' | 'minimise' | 'keys' | 'wide';

/**
 * The action for `event`, or `null`. `target` is the focused element's tag
 * name: Space types into fields and presses buttons rather than toggling.
 * With no system title bar, ⌘W and ⌘M are the window's to handle on a Mac.
 */
export function windowAction(event: KeyPress, target: string, mac: boolean): WindowAction | null {
  const plainCmd = mac && event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
  const typing = target === 'INPUT' || target === 'TEXTAREA';
  const key = event.key.toLowerCase();
  if (event.key === 'Escape') return 'back-or-close';
  if (plainCmd && key === 'w') return 'close';
  if (plainCmd && key === 'm') return 'minimise';
  if (event.key === ',' && (event.ctrlKey || event.metaKey)) return 'settings';
  // ⌘E on a Mac, Ctrl+E elsewhere: the wide window, and back.
  const plainCtrl = !mac && event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey;
  if (key === 'e' && (plainCmd || plainCtrl)) return 'wide';
  if (event.key === ' ' && !typing && target !== 'BUTTON') return 'toggle';
  // `?` is Shift+/ on most layouts; either way it is the key sheet.
  if (event.key === '?' && !typing && !event.ctrlKey && !event.metaKey && !event.altKey)
    return 'keys';
  return null;
}
