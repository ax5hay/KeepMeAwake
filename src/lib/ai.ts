/**
 * AI in the window: which of three locks is shut, and turning what it
 * understood into something the app can do. Pure, so it is tested without a
 * DOM. The key itself never reaches the window; it only knows there is one.
 */
import { nextOccurrence } from './format';
import type { AiSettings, AiStatus, PlanRequest, Provider, Understood } from './types';

/** A feature AI powers; each has a switch of its own in the settings. */
export type AiFeature = 'week_in_words' | 'tell_the_sun' | 'day_in_a_line' | 'ask_the_almanac';

/**
 * Where a feature stands, outermost lock first:
 * `hidden` Hide AI is on; `locked` no API key; `off` a key, but AI is switched off;
 * `feature-off` AI is on but this feature is not; `on` ready when asked.
 */
export type Gate = 'hidden' | 'locked' | 'off' | 'feature-off' | 'on';

/** Where AI as a whole stands, before any one feature's own switch. */
export type AiState = 'hidden' | 'locked' | 'off' | 'on';

/**
 * AI as a whole. `hidden` comes first: with Hide AI on, nothing of it
 * is shown at all, key or no key.
 */
export function stateOf(status: AiStatus | null, settings: AiSettings | null): AiState {
  if (settings?.hidden) return 'hidden';
  if (!status?.has_key) return 'locked';
  return settings?.enabled ? 'on' : 'off';
}

/** The gate for `feature`. With the status not yet in, it reads as locked. */
export function gateOf(
  status: AiStatus | null,
  settings: AiSettings | null,
  feature: AiFeature,
): Gate {
  const state = stateOf(status, settings);
  if (state !== 'on') return state;
  return settings?.[feature] ? 'on' : 'feature-off';
}

/** What the window needs to say about a service. */
export interface Service {
  /** On the picker. */
  label: string;
  /** Who is sent what is sent: `Sends this week to Anthropic`. */
  sentTo: string;
  /** Where a key comes from. */
  keysFrom: string;
  /** Shown in the empty key field. */
  keyLooksLike: string;
  /** The model asked when none is named; empty when one must be. */
  smallest: string;
}

/** The services, in the order the picker shows them. */
export const SERVICES: Readonly<Record<Provider, Service>> = {
  anthropic: {
    label: 'Claude',
    sentTo: 'Anthropic',
    keysFrom: 'console.anthropic.com',
    keyLooksLike: 'sk-ant-…',
    smallest: 'claude-haiku-5-5',
  },
  openai: {
    label: 'OpenAI',
    sentTo: 'OpenAI',
    keysFrom: 'platform.openai.com',
    keyLooksLike: 'sk-…',
    smallest: 'gpt-5.4-nano',
  },
  custom: {
    label: 'Other',
    sentTo: 'the service you chose',
    keysFrom: 'the service you chose',
    keyLooksLike: 'its key',
    smallest: '',
  },
};

/** The service the settings choose; Claude until something else is chosen. */
export function serviceOf(settings: AiSettings | null): Service {
  return SERVICES[settings?.provider ?? 'anthropic'];
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

/** `Tue 10 Nov 2026, 18:42`: now, as the model is told it, in the local zone. */
export function nowLocal(nowMs: number): string {
  const d = new Date(nowMs);
  const part = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat('en-GB', options).format(d);
  const two = (n: number) => String(n).padStart(2, '0');
  return `${part({ weekday: 'short' })} ${d.getDate()} ${part({ month: 'short' })} ${d.getFullYear()}, ${two(d.getHours())}:${two(d.getMinutes())}`;
}
