/**
 * Pure text helpers. No DOM, no Tauri, fully unit tested.
 */
import { LATE_NIGHT, isLate } from './copy';
import type { PauseReason, Snapshot } from './types';

/** `1h 05m`, `12m`, `45s`. Mirrors the Rust tray formatter. */
export function formatDuration(totalSecs: number): string {
  const secs = Math.max(0, Math.floor(totalSecs));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h === 0 && m === 0) return `${s}s`;
  if (h === 0) return `${m}m`;
  return `${h}h ${String(m).padStart(2, '0')}m`;
}

/** Short label for duration stops: `30m`, `1h`, `8h`. */
export function formatStop(secs: number): string {
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  const h = secs / 3600;
  return Number.isInteger(h) ? `${h}h` : `${h.toFixed(1)}h`;
}

/** Wall clock like `5:42 pm`, in the user's locale. */
export function formatClock(unixMs: number, locale?: string): string {
  return new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' })
    .format(new Date(unixMs))
    .toLowerCase();
}

/** Seconds left, rounded up, never negative. */
export function remainingSecs(snapshot: Snapshot, nowMs: number): number | null {
  if (snapshot.ends_at_unix_ms === null) return null;
  return Math.max(0, Math.ceil((snapshot.ends_at_unix_ms - nowMs) / 1000));
}

/** Fraction of the current timed plan that has elapsed, 0..1. */
export function progress(snapshot: Snapshot, nowMs: number): number {
  const left = remainingSecs(snapshot, nowMs);
  if (left === null || !snapshot.plan_secs) return 0;
  return clamp(1 - left / snapshot.plan_secs, 0, 1);
}

export function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

export function pauseText(reason: PauseReason): string {
  switch (reason) {
    case 'low_battery':
      return 'battery is low';
    case 'on_battery':
      return 'running on battery';
    case 'power_request_failed':
      return 'Windows refused the request';
  }
}

export interface StatusText {
  title: string;
  detail: string;
}

/** The two lines under the scene. */
export function statusText(snapshot: Snapshot, nowMs: number, locale?: string): StatusText {
  if (snapshot.mode === 'off') {
    return { title: 'Resting', detail: 'Your laptop sleeps as usual.' };
  }
  if (snapshot.paused) {
    return { title: 'Paused', detail: `Holding off while ${pauseText(snapshot.paused)}.` };
  }
  const late = isLate(new Date(nowMs).getHours()) ? ` ${LATE_NIGHT}` : '';
  if (snapshot.mode === 'indefinite') {
    return { title: 'Awake', detail: `Until you say otherwise.${late}` };
  }
  const left = remainingSecs(snapshot, nowMs) ?? 0;
  const ends = snapshot.ends_at_unix_ms ?? nowMs;
  return {
    title: 'Awake',
    detail: `Until ${formatClock(ends, locale)} · ${formatDuration(left)} left${late}`,
  };
}

/** Battery line for the low-battery row. */
export function batteryText(snapshot: Snapshot): string {
  const { on_ac, percent } = snapshot.battery;
  const source = on_ac ? 'Plugged in' : 'On battery';
  return percent === null ? source : `${source} · ${percent}%`;
}

/**
 * Next occurrence of `HH:MM` after `nowMs`: today if still ahead, else
 * tomorrow. Returns Unix ms.
 */
export function nextOccurrence(hhmm: string, nowMs: number): number | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(hhmm);
  if (!match) return null;
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) return null;
  const target = new Date(nowMs);
  target.setHours(hours, minutes, 0, 0);
  if (target.getTime() <= nowMs) target.setDate(target.getDate() + 1);
  return target.getTime();
}

/** `HH:MM` for an `<input type="time">`, rounded up to the next quarter hour. */
export function suggestedUntil(nowMs: number, aheadMs = 60 * 60 * 1000): string {
  const t = new Date(nowMs + aheadMs);
  const q = 15 * 60 * 1000;
  const rounded = new Date(Math.ceil(t.getTime() / q) * q);
  return `${String(rounded.getHours()).padStart(2, '0')}:${String(rounded.getMinutes()).padStart(2, '0')}`;
}
