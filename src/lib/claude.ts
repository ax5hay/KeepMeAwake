/**
 * Claude in the window: which of three locks is shut, and turning what it
 * understood into something the app can do. Pure, so it is tested without a
 * DOM. The key itself never reaches the window; it only knows there is one.
 */
import { nextOccurrence } from './format';
import type { AiSettings, AiStatus, PlanRequest, Understood } from './types';

/** A feature Claude powers; each has a switch of its own in the settings. */
export type AiFeature = 'week_in_words' | 'tell_the_sun';

/**
 * Where a feature stands, outermost lock first:
 * `locked` no API key; `off` a key, but Claude is switched off;
 * `feature-off` Claude is on but this feature is not; `on` ready when asked.
 */
export type Gate = 'locked' | 'off' | 'feature-off' | 'on';

/** The gate for `feature`. With the status not yet in, it reads as locked. */
export function gateOf(
  status: AiStatus | null,
  settings: AiSettings | null,
  feature: AiFeature,
): Gate {
  if (!status?.has_key) return 'locked';
  if (!settings?.enabled) return 'off';
  return settings[feature] ? 'on' : 'feature-off';
}

/** The plan a sentence was understood as, or `null` when there is nothing to start. */
export function planOf(understood: Understood, nowMs: number): PlanRequest | null {
  switch (understood.kind) {
    case 'for':
      return understood.minutes > 0 ? { kind: 'for', seconds: understood.minutes * 60 } : null;
    case 'until': {
      const at = nextOccurrence(understood.time, nowMs);
      return at === null ? null : { kind: 'until', unix_ms: at };
    }
    case 'indefinite':
      return { kind: 'indefinite' };
    case 'off':
      return { kind: 'off' };
    default:
      return null;
  }
}

/** `Tue 10 Nov 2026, 18:42`: now, as Claude is told it, in the local zone. */
export function nowLocal(nowMs: number): string {
  const d = new Date(nowMs);
  const part = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat('en-GB', options).format(d);
  const two = (n: number) => String(n).padStart(2, '0');
  return `${part({ weekday: 'short' })} ${d.getDate()} ${part({ month: 'short' })} ${d.getFullYear()}, ${two(d.getHours())}:${two(d.getMinutes())}`;
}
