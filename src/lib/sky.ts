/**
 * The wide window's day, as geometry: when the laptop was kept awake, laid
 * across a 24-hour horizon as domes of light, and the week as seven small
 * days. Pure, so it is tested without a DOM.
 */
import type { Ended, History } from './types';

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

/** A stretch the sun was up, as fractions of the day. */
export interface Dome {
  /** Where it starts and ends across the day, 0–1. */
  from: number;
  to: number;
  /** Still up: it ends at now, and the sun is on it. */
  running: boolean;
  /** The real instants, for the caption. */
  start_ms: number;
  end_ms: number;
  /** How it ended, when it has. */
  ended: Ended | null;
}

/** The day `now` falls in, turning over at `turn` o'clock local time. */
export function dayAround(now: number, turn = 0): { start: number; end: number } {
  const start = new Date(now);
  start.setHours(turn, 0, 0, 0);
  if (start.getTime() > now) start.setDate(start.getDate() - 1);
  const end = new Date(start);
  end.setDate(end.getDate() + 1);
  return { start: start.getTime(), end: end.getTime() };
}

/**
 * Every stretch the sun was up between `start` and `end`, clipped to them,
 * oldest first. One still running ends at `now`. Under a minute across the
 * day's width is too small to see, and is left out.
 */
export function domesBetween(
  history: History | null,
  start: number,
  end: number,
  now: number,
): Dome[] {
  if (history === null || end <= start) return [];
  const length = end - start;
  const domes: Dome[] = [];
  for (const entry of history.entries) {
    // Still up only while it has no ending; one that never ended cleanly
    // (killed, crashed) has no end to draw to, and is left out.
    const running = entry.ended === null && entry.ended_unix_ms === null;
    if (!running && entry.ended_unix_ms === null) continue;
    const to = Math.min(running ? now : (entry.ended_unix_ms ?? now), end);
    const from = Math.max(entry.started_unix_ms, start);
    if (to - from < 60_000) continue;
    domes.push({
      from: (from - start) / length,
      to: (to - start) / length,
      running: running && now < end,
      start_ms: entry.started_unix_ms,
      end_ms: running ? now : (entry.ended_unix_ms ?? now),
      ended: entry.ended,
    });
  }
  return domes.sort((a, b) => a.from - b.from);
}

/** Total time the sun was up in `domes`, across `length` ms of day. */
export function awakeMs(domes: readonly Dome[], length: number): number {
  return domes.reduce((sum, d) => sum + (d.to - d.from) * length, 0);
}

/**
 * The dome over one stretch: half an ellipse standing on the horizon, as
 * wide as the stretch and as tall as it is long, up to `tallest`. A running
 * stretch is drawn rising to the sun: a quarter, ending at its top.
 */
export function domePath(
  dome: Dome,
  width: number,
  horizon: number,
  tallest: number,
  scale = 1,
): { d: string; peak: number } {
  const x1 = dome.from * width;
  const x2 = Math.max(dome.to * width, x1 + 1);
  if (dome.running) {
    const rx = Math.max(x2 - x1, 1);
    // The sun climbs as the stretch goes on: low at first, a sunrise,
    // and as high as any dome after a working day.
    const ry = heightFor((dome.to - dome.from) * DAY, tallest, 0.38) * scale;
    // A quarter ellipse from the start on the line up to the sun at now.
    return {
      d: `M ${f(x1)} ${f(horizon)} A ${f(rx)} ${f(ry)} 0 0 1 ${f(x2)} ${f(horizon - ry)}`,
      peak: horizon - ry,
    };
  }
  const rx = (x2 - x1) / 2;
  const ry = heightFor((dome.to - dome.from) * DAY, tallest, 0.2) * scale;
  return {
    d: `M ${f(x1)} ${f(horizon)} A ${f(rx)} ${f(ry)} 0 0 1 ${f(x2)} ${f(horizon)} Z`,
    peak: horizon - ry,
  };
}

/**
 * A dome as three stepped bands of light, outermost first, the way the
 * scene's afterglow stands over the horizon: each band is the same dome at
 * a third less height, and their overlap makes the light deepen toward the
 * line. A running dome's bands are closed down to the line at now.
 */
export function domeBands(dome: Dome, width: number, horizon: number, tallest: number): string[] {
  return [1, 2 / 3, 1 / 3].map((scale) => {
    const { d } = domePath(dome, width, horizon, tallest, scale);
    return dome.running ? `${d} L ${f(dome.to * width)} ${f(horizon)} Z` : d;
  });
}

/** As tall as a stretch is long: `floor` of the tallest at once, a tenth
 * more an hour, the whole of it after a long day. */
function heightFor(ms: number, tallest: number, floor: number): number {
  const hours = ms / HOUR;
  return Math.max(6, Math.min(tallest, tallest * (floor + 0.1 * hours)));
}

const f = (n: number) => (Math.round(n * 100) / 100).toString();

/** One day of the week column. */
export interface WeekDay {
  key: string;
  /** `Mon`. */
  label: string;
  start: number;
  domes: Dome[];
  awake_ms: number;
  today: boolean;
  ahead: boolean;
}

/** Monday to Sunday of the week `now` is in, each with its domes. */
export function weekOfDomes(
  history: History | null,
  now: number,
  turn = 0,
  locale?: string,
): WeekDay[] {
  const today = dayAround(now, turn);
  const weekday = (new Date(today.start).getDay() + 6) % 7; // Monday 0
  const monday = new Date(today.start);
  monday.setDate(monday.getDate() - weekday);
  const days: WeekDay[] = [];
  for (let i = 0; i < 7; i += 1) {
    const s = new Date(monday);
    s.setDate(s.getDate() + i);
    const e = new Date(s);
    e.setDate(e.getDate() + 1);
    const start = s.getTime();
    const end = e.getTime();
    const domes = domesBetween(history, start, end, now);
    days.push({
      key: `${s.getFullYear()}-${s.getMonth() + 1}-${s.getDate()}`,
      label: new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(s),
      start,
      domes,
      awake_ms: awakeMs(domes, end - start),
      today: start === today.start,
      ahead: start > now,
    });
  }
  return days;
}

/** Hour marks across the day at the clock's own quarters (midnight, 6, noon,
 * 6), wherever the day turns. */
export function hourMarks(start: number, end: number): { at: number; unix_ms: number }[] {
  const length = end - start;
  const marks: { at: number; unix_ms: number }[] = [];
  const t = new Date(start);
  t.setMinutes(0, 0, 0);
  while (t.getTime() < end) {
    if (t.getTime() >= start && t.getHours() % 6 === 0) {
      marks.push({ at: (t.getTime() - start) / length, unix_ms: t.getTime() });
    }
    t.setHours(t.getHours() + 1);
  }
  return marks;
}

/** Where now is across the day, 0–1. */
export function nowAt(now: number, start: number, end: number): number {
  return Math.min(1, Math.max(0, (now - start) / (end - start)));
}
