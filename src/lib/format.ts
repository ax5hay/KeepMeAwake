/**
 * Pure text helpers. No DOM, no Tauri, fully unit tested.
 */
import {
  BACK_ON_SCHEDULE,
  LATE_NIGHT,
  awakeAgain,
  isLate,
  onSchedule,
  platformCopy,
  skippedUntil,
  skipping,
  sunsetIn,
} from './copy';
import { isMac } from './platform';
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

/**
 * A moment as `9:00 am`, or `9:00 am on Monday` when it is further off than
 * tomorrow, so a Friday evening does not read as "tomorrow morning".
 */
export function formatWhen(unixMs: number, nowMs: number, locale?: string): string {
  const clock = formatClock(unixMs, locale);
  const day = (ms: number) => {
    const d = new Date(ms);
    return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86_400_000;
  };
  if (day(unixMs) - day(nowMs) <= 1) return clock;
  const weekday = new Intl.DateTimeFormat(locale, { weekday: 'long' }).format(new Date(unixMs));
  return `${clock} on ${weekday}`;
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

/**
 * `true` while a timed session is inside its sunset warning: no more than
 * the warning left, and a plan longer than the warning. The same rule the
 * engine uses to send its toast.
 */
export function sunsetSoon(snapshot: Snapshot, nowMs: number): boolean {
  const mins = snapshot.settings.sunset_warning_mins;
  const left = remainingSecs(snapshot, nowMs);
  if (snapshot.mode !== 'timed' || mins === null || left === null) return false;
  const warning = mins * 60;
  return left > 0 && left <= warning && (snapshot.plan_secs ?? 0) > warning;
}

/** Why the sun is paused, as the end of a sentence. */
export function pauseText(reason: PauseReason, mac: boolean = isMac): string {
  switch (reason) {
    case 'low_battery':
      return 'battery is low';
    case 'on_battery':
      return 'running on battery';
    case 'power_request_failed':
      return platformCopy(mac).refused;
  }
}

export interface StatusText {
  title: string;
  detail: string;
}

/** The two lines under the scene. */
export function statusText(snapshot: Snapshot, nowMs: number, locale?: string): StatusText {
  if (snapshot.mode === 'off') {
    const { schedule } = snapshot;
    if (schedule.skipped && schedule.window_end_unix_ms !== null) {
      return {
        title: 'Resting',
        detail: skipping(formatClock(schedule.window_end_unix_ms, locale)),
      };
    }
    const resting = 'Your laptop sleeps as usual.';
    if (schedule.enabled && schedule.next_start_unix_ms !== null) {
      const when = formatWhen(schedule.next_start_unix_ms, nowMs, locale);
      return { title: 'Resting', detail: `${resting} ${awakeAgain(when)}` };
    }
    return { title: 'Resting', detail: resting };
  }
  if (snapshot.paused) {
    return { title: 'Paused', detail: `Holding off while ${pauseText(snapshot.paused)}.` };
  }
  const late = isLate(new Date(nowMs).getHours()) ? ` ${LATE_NIGHT}` : '';
  if (snapshot.mode === 'indefinite') {
    return { title: 'Awake', detail: `Until you say otherwise.${late}` };
  }
  const ends = snapshot.ends_at_unix_ms ?? nowMs;
  if (snapshot.mode === 'scheduled') {
    return { title: 'Awake', detail: `${onSchedule(formatClock(ends, locale))}${late}` };
  }
  const left = remainingSecs(snapshot, nowMs) ?? 0;
  if (sunsetSoon(snapshot, nowMs)) return { title: 'Awake', detail: sunsetIn(left) };
  return {
    title: 'Awake',
    detail: `Until ${formatClock(ends, locale)} · ${formatDuration(left)} left${late}`,
  };
}

/**
 * What to whisper after the sun was clicked, when the click skipped or
 * resumed the schedule rather than starting or stopping a session.
 */
export function toggleWhisper(before: Snapshot, after: Snapshot, locale?: string): string | null {
  const end = after.schedule.window_end_unix_ms;
  if (!before.schedule.skipped && after.schedule.skipped && end !== null) {
    return skippedUntil(formatClock(end, locale));
  }
  if (before.schedule.skipped && after.mode === 'scheduled') return BACK_ON_SCHEDULE;
  return null;
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

/** Weekdays in the order the schedule stores them, Monday first. */
export const WEEKDAYS: ReadonlyArray<{ letter: string; name: string }> = [
  { letter: 'M', name: 'Monday' },
  { letter: 'T', name: 'Tuesday' },
  { letter: 'W', name: 'Wednesday' },
  { letter: 'T', name: 'Thursday' },
  { letter: 'F', name: 'Friday' },
  { letter: 'S', name: 'Saturday' },
  { letter: 'S', name: 'Sunday' },
];

/** Minutes after midnight as `HH:MM`, the value an `<input type="time">` takes. */
export function minutesToTime(minutes: number): string {
  const m = clamp(Math.round(minutes), 0, 24 * 60 - 1);
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
}

/** `HH:MM` back to minutes after midnight, or `null` if it is not a time. */
export function timeToMinutes(hhmm: string): number | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(hhmm);
  if (!match) return null;
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) return null;
  return hours * 60 + minutes;
}
