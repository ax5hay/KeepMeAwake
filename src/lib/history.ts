/**
 * Turning the session log into the rows Settings → History shows. Pure, so
 * it is tested without a DOM.
 */
import { formatClock } from './format';
import type { Ended, History, PauseReason } from './types';

/** One line of the list, already in words. */
export interface HistoryRow {
  key: string;
  day: string;
  span: string;
  length: string;
  /** How it ended, or `now` while it runs. */
  ended: string;
  /** One line per pause: `paused 12:10 pm – 12:40 pm · battery`. */
  pauses: string[];
}

/** How the list says each ending: who or what set the sun. */
const ENDED: Readonly<Record<Ended, string>> = {
  user: 'you',
  timer: 'timer',
  schedule: 'schedule',
  battery: 'battery',
  sleep: 'sleep',
  crash: 'crash',
};

const PAUSED_BY: Readonly<Record<PauseReason, string>> = {
  low_battery: 'battery',
  on_battery: 'battery',
  power_request_failed: 'refused',
};

/** Shown for an end the stretch never had. */
const NO_END = '–';

function dayNumber(ms: number): number {
  const d = new Date(ms);
  return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86_400_000;
}

/** `Today`, `Yesterday`, else `Tue 3 Nov` in that order whatever the locale. */
export function formatDay(unixMs: number, nowMs: number, locale?: string): string {
  const ago = dayNumber(nowMs) - dayNumber(unixMs);
  if (ago === 0) return 'Today';
  if (ago === 1) return 'Yesterday';
  const date = new Date(unixMs);
  const part = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(locale, options).format(date);
  return `${part({ weekday: 'short' })} ${date.getDate()} ${part({ month: 'short' })}`;
}

/** `9:02 am – 11:30 am`, with a dash for a missing end. */
export function formatSpan(start: number, end: number | null, locale?: string): string {
  const to = end === null ? NO_END : formatClock(end, locale);
  return `${formatClock(start, locale)} – ${to}`;
}

/** `2h 28m`, `9h`, `5m`, `40s`; a dash without an end. */
export function formatLength(start: number, end: number | null): string {
  if (end === null) return NO_END;
  const secs = Math.max(0, Math.round((end - start) / 1000));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h === 0 && m === 0) return `${secs}s`;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** The newest `limit` entries, newest first, as rows. */
export function historyRows(
  history: History,
  nowMs: number,
  locale?: string,
  limit = 30,
): HistoryRow[] {
  return history.entries
    .slice(-limit)
    .reverse()
    .map((entry) => ({
      key: String(entry.started_unix_ms),
      day: formatDay(entry.started_unix_ms, nowMs, locale),
      span: formatSpan(entry.started_unix_ms, entry.ended_unix_ms, locale),
      length: formatLength(entry.started_unix_ms, entry.ended_unix_ms),
      ended: entry.ended === null ? 'now' : ENDED[entry.ended],
      pauses: entry.pauses.map(
        (pause) =>
          `paused ${formatSpan(pause.from_unix_ms, pause.to_unix_ms, locale)} · ${PAUSED_BY[pause.reason]}`,
      ),
    }));
}
