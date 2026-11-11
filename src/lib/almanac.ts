/**
 * Reading the almanac: turning spans of presence into days, hours worked,
 * leave and nights, in local time. Pure, so it is tested without a DOM.
 *
 * A day here is not midnight to midnight. It turns over in the middle of
 * the hours nobody works, so someone on 3 pm to midnight who carries on
 * until two still has one day, not a day and a stub of the next.
 */
import { clamp } from './format';
import type { Almanac, AlmanacSettings, Mark, Span } from './types';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY_MINUTES = 24 * 60;

/** Less than this in a day and it was not a day's work: a lid opened, a message read. */
export const WORKED_FROM_MS = 5 * MINUTE;

/** Presence shorter than this does not end a night: a nudged mouse is not a morning. */
export const STIR_MS = 2 * MINUTE;

/** A night is a quiet at least this long. */
export const NIGHT_MIN_MS = 3 * HOUR;

/** Longer than this and it was not a night: a weekend away, a holiday. */
export const NIGHT_MAX_MS = 16 * HOUR;

/** How much of a quiet must fall in the resting hours to be a night. */
export const NIGHT_OVERLAP_MS = 2 * HOUR;

/** The resting hours reach this far either side of the turn of the day. */
export const REST_REACH_MS = 5 * HOUR;

/** Nights the usual night is taken from. */
const USUAL_FROM = 14;

/** Nights needed before a usual night means anything. */
const USUAL_NEEDS = 3;

/** A stretch of presence inside one day. */
export interface Stretch {
  from: number;
  to: number;
}

/** The quiet between the last touch of one day and the first of the next. */
export interface Night {
  from: number;
  to: number;
  /** The day it leads into: the one whose morning it ends on. */
  key: string;
}

/** How a day reads. `ahead` is a day still to come with nothing said about it. */
export type Kind = 'worked' | 'leave' | 'rest' | 'nothing' | 'ahead';

/** One day of the almanac. */
export interface Day {
  /** Local date the day starts on, `2026-11-10`. */
  key: string;
  /** When it turns over into this day, and into the next, Unix ms. */
  start: number;
  end: number;
  /** Presence inside it, oldest first. */
  stretches: Stretch[];
  worked_ms: number;
  /** The part of it outside working hours; all of it on a day that is not a working day. */
  over_ms: number;
  /** First and last touch, or `null` with nothing in it. */
  first: number | null;
  last: number | null;
  /** The working hours on this day, as instants. */
  hours: Stretch;
  /** A working weekday, by the setting. */
  working: boolean;
  /**
   * A working weekday by the setting, but one the record says is not
   * worked: empty week after week while the rest of the week is not.
   */
  usual_off: boolean;
  kind: Kind;
  /** What was said about it, if anything. */
  mark: Mark | null;
  /** Its hours were set by hand, in place of what was read. */
  fixed: boolean;
  /** The night that led into it. */
  night: Night | null;
  today: boolean;
}

/** The almanac, read. */
export interface Reading {
  /** The hour the day turns over, 0–23. */
  turn: number;
  now: number;
  today: Day;
  nights: Night[];
  /** The day at `ms`. */
  dayAt: (ms: number) => Day;
}

/** Length of the working hours in minutes; equal ends mean the whole day. */
export function hoursLength(settings: AlmanacSettings): number {
  const length = (settings.end_minutes - settings.start_minutes + DAY_MINUTES) % DAY_MINUTES;
  return length === 0 ? DAY_MINUTES : length;
}

/**
 * The hour at which a day turns over: opposite the middle of the working
 * hours, on the hour. 9 am to 6 pm turns at 1 am; 3 pm to midnight at 7 am.
 */
export function dayTurn(settings: AlmanacSettings): number {
  const middle = settings.start_minutes + hoursLength(settings) / 2;
  return Math.floor(((middle + DAY_MINUTES / 2) % DAY_MINUTES) / 60) % 24;
}

/** `2026-11-10` for the local date of `date`. */
function keyOf(date: Date): string {
  const two = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())}`;
}

/** When the day containing `ms` turned over. */
export function dayStart(ms: number, turn: number): number {
  const d = new Date(ms);
  const start = new Date(d.getFullYear(), d.getMonth(), d.getDate(), turn).getTime();
  if (start <= ms) return start;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() - 1, turn).getTime();
}

/** The turn `days` days after (or before) the one at `start`. Honest across DST. */
export function shiftDay(start: number, days: number): number {
  const d = new Date(start);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + days, d.getHours()).getTime();
}

/** The key of the day that turns over at `start`. */
export function dayKey(start: number): string {
  return keyOf(new Date(start));
}

/** When the day `key` turns over, or `null` if `key` is not a date. */
export function startOfKey(key: string, turn: number): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key);
  if (!match) return null;
  const start = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]), turn);
  return keyOf(start) === key ? start.getTime() : null;
}

/** The working hours of the day that turns over at `start`. */
function hoursOn(start: number, settings: AlmanacSettings): Stretch {
  const d = new Date(start);
  const at = (minutes: number) =>
    new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, minutes).getTime();
  // The first start of work at or after the turn: later that date, or the next.
  const today = at(settings.start_minutes);
  const offset = today >= start ? 0 : DAY_MINUTES;
  return {
    from: at(settings.start_minutes + offset),
    to: at(settings.start_minutes + offset + hoursLength(settings)),
  };
}

function overlap(a: Stretch, b: Stretch): number {
  return Math.max(0, Math.min(a.to, b.to) - Math.max(a.from, b.from));
}

/**
 * The nights among `spans`: each quiet of 3 to 16 hours with at least two
 * of them in the resting hours (five either side of a turn of the day).
 * One per day, the longest; stirs are looked past.
 */
export function findNights(spans: readonly Span[], turn: number): Night[] {
  const solid = spans.filter((span) => span.to_unix_ms - span.from_unix_ms >= STIR_MS);
  const byKey = new Map<string, Night>();
  for (let i = 1; i < solid.length; i += 1) {
    const before = solid[i - 1];
    const after = solid[i];
    if (!before || !after) continue;
    const quiet = { from: before.to_unix_ms, to: after.from_unix_ms };
    const length = quiet.to - quiet.from;
    if (length < NIGHT_MIN_MS || length > NIGHT_MAX_MS) continue;
    // The turn it rests across: whichever its resting hours hold most of.
    const first = dayStart(quiet.from, turn);
    let best = { turnAt: first, held: 0 };
    for (const turnAt of [first, shiftDay(first, 1), shiftDay(first, 2)]) {
      const held = overlap(quiet, { from: turnAt - REST_REACH_MS, to: turnAt + REST_REACH_MS });
      if (held > best.held) best = { turnAt, held };
    }
    if (best.held < NIGHT_OVERLAP_MS) continue;
    const key = dayKey(best.turnAt);
    const known = byKey.get(key);
    if (!known || known.to - known.from < length) byKey.set(key, { ...quiet, key });
  }
  return [...byKey.values()].sort((a, b) => a.from - b.from);
}

/**
 * The first moment at or after `notBefore` that the clock reads `minutes`
 * after midnight.
 */
function nextAt(minutes: number, notBefore: number): number {
  const d = new Date(notBefore);
  const at = (days: number) =>
    new Date(d.getFullYear(), d.getMonth(), d.getDate() + days, 0, minutes).getTime();
  return at(0) >= notBefore ? at(0) : at(1);
}

/**
 * The stretch a day is corrected to when someone says they were here from
 * one time of day until another: the first such time in the day, and the
 * first `until` after it, never past the turn into the next day.
 */
export function stretchOn(
  day: { start: number; end: number },
  fromMinutes: number,
  untilMinutes: number,
): Span {
  const from = nextAt(fromMinutes, day.start);
  const to = Math.min(nextAt(untilMinutes, from), day.end);
  return { from_unix_ms: from, to_unix_ms: Math.max(from, to) };
}

/**
 * The spans as they stand once corrections are in: on a day set by hand,
 * what was read gives way to the one stretch that was entered.
 */
export function corrected(almanac: Almanac, turn: number): Span[] {
  const fixes = Object.entries(almanac.fixes ?? {});
  if (fixes.length === 0) return almanac.spans;
  const days = fixes
    .map(([key, span]) => ({ start: startOfKey(key, turn), span }))
    .filter((fix): fix is { start: number; span: Span } => fix.start !== null);
  let spans = almanac.spans;
  for (const { start } of days) {
    const end = shiftDay(start, 1);
    spans = spans.flatMap((span) => {
      if (span.to_unix_ms <= start || span.from_unix_ms >= end) return [span];
      const kept: Span[] = [];
      if (span.from_unix_ms < start) kept.push({ ...span, to_unix_ms: start });
      if (span.to_unix_ms > end) kept.push({ ...span, from_unix_ms: end });
      return kept;
    });
  }
  // A correction of no length says nobody was here: the day is left empty.
  const entered = days.map((fix) => fix.span).filter((s) => s.to_unix_ms > s.from_unix_ms);
  return [...spans, ...entered].sort((a, b) => a.from_unix_ms - b.from_unix_ms);
}

/** How many weeks each side of a day are asked whether its weekday is worked. */
export const OFF_WEEKS = 4;

/** How many of them must say it is not, with none saying it is. */
export const OFF_FROM = 3;

/** Reads `almanac` as it stands at `now`. */
export function read(almanac: Almanac, settings: AlmanacSettings, now: number): Reading {
  const turn = dayTurn(settings);
  const spans = corrected(almanac, turn);
  const stretches = new Map<string, Stretch[]>();
  for (const span of spans) {
    // A span across a turn of the day is two stretches, one in each day.
    let from = span.from_unix_ms;
    const to = Math.max(from, span.to_unix_ms);
    for (;;) {
      const start = dayStart(from, turn);
      const end = shiftDay(start, 1);
      const key = dayKey(start);
      const list = stretches.get(key) ?? [];
      list.push({ from, to: Math.min(to, end) });
      stretches.set(key, list);
      if (to <= end) break;
      from = end;
    }
  }

  const nights = findNights(spans, turn);
  const nightByKey = new Map(nights.map((night) => [night.key, night]));
  const first = spans[0];
  const began = first ? dayStart(first.from_unix_ms, turn) : null;
  const todayStart = dayStart(now, turn);
  const cache = new Map<string, Day>();

  const workedOn = (start: number): boolean =>
    (stretches.get(dayKey(start)) ?? []).reduce((sum, s) => sum + (s.to - s.from), 0) >=
    WORKED_FROM_MS;

  /** Somebody worked in the week holding `start`, on some other day of it. */
  function weekWorked(start: number): boolean {
    const weekday = (new Date(start).getDay() + 6) % 7;
    for (let i = 0; i < 7; i += 1) {
      if (i !== weekday && workedOn(shiftDay(start, i - weekday))) return true;
    }
    return false;
  }

  /**
   * Whether the weekday of the empty day at `start` is one that is not
   * worked, whatever the setting says: the same weekday was empty in at
   * least `OFF_FROM` of the weeks around it, in weeks that were worked,
   * and in none of them was it worked or taken as leave. A week nobody
   * worked at all says nothing either way, so a holiday stays leave.
   */
  function usuallyOff(start: number): boolean {
    if (began === null || start < began || !weekWorked(start)) return false;
    let empty = 0;
    for (let week = -OFF_WEEKS; week <= OFF_WEEKS; week += 1) {
      const other = shiftDay(start, week * 7);
      if (week === 0 || other < began || other >= todayStart) continue;
      if (workedOn(other) || almanac.marks[dayKey(other)] === 'leave') return false;
      if (weekWorked(other)) empty += 1;
    }
    return empty >= OFF_FROM;
  }

  function dayAt(ms: number): Day {
    const start = dayStart(ms, turn);
    const key = dayKey(start);
    const cached = cache.get(key);
    if (cached) return cached;
    const end = shiftDay(start, 1);
    const inside = stretches.get(key) ?? [];
    const hours = hoursOn(start, settings);
    const working = settings.work_days[(new Date(start).getDay() + 6) % 7] ?? false;
    const worked_ms = inside.reduce((sum, s) => sum + (s.to - s.from), 0);
    const within = inside.reduce((sum, s) => sum + overlap(s, hours), 0);
    const mark = almanac.marks[key] ?? null;
    const today = start === todayStart;
    const empty = worked_ms < WORKED_FROM_MS;
    const usual_off = working && empty && start < todayStart && usuallyOff(start);
    let kind: Kind;
    if (mark === 'leave') kind = 'leave';
    else if (!empty) kind = 'worked';
    else if (start > todayStart) kind = 'ahead';
    else if (today) kind = 'nothing';
    else if (!working || usual_off) kind = 'rest';
    else if (
      settings.empty_days_are_leave &&
      mark !== 'clear' &&
      began !== null &&
      start >= began
    ) {
      kind = 'leave';
    } else kind = 'nothing';
    const lastStretch = inside[inside.length - 1];
    const day: Day = {
      key,
      start,
      end,
      stretches: inside,
      worked_ms,
      over_ms: working ? worked_ms - within : worked_ms,
      first: inside[0]?.from ?? null,
      last: lastStretch?.to ?? null,
      hours,
      working,
      usual_off,
      kind,
      mark,
      fixed: key in (almanac.fixes ?? {}),
      night: nightByKey.get(key) ?? null,
      today,
    };
    cache.set(key, day);
    return day;
  }

  return { turn, now, today: dayAt(now), nights, dayAt };
}

/** The seven days of the week holding `day`, Monday first. */
export function weekOf(reading: Reading, day: Day): Day[] {
  const weekday = (new Date(day.start).getDay() + 6) % 7;
  const monday = shiftDay(day.start, -weekday);
  return [0, 1, 2, 3, 4, 5, 6].map((i) => reading.dayAt(shiftDay(monday, i)));
}

/** The days of a month, in order. `month` is 0-based. */
export function monthOf(reading: Reading, year: number, month: number): Day[] {
  const days: Day[] = [];
  const length = new Date(year, month + 1, 0).getDate();
  for (let date = 1; date <= length; date += 1) {
    days.push(reading.dayAt(new Date(year, month, date, reading.turn).getTime()));
  }
  return days;
}

/** A month as a calendar: rows of seven, Monday first, `null` outside the month. */
export function monthGrid(reading: Reading, year: number, month: number): (Day | null)[][] {
  const days = monthOf(reading, year, month);
  const lead = (new Date(year, month, 1).getDay() + 6) % 7;
  const cells: (Day | null)[] = [...Array<null>(lead).fill(null), ...days];
  while (cells.length % 7 !== 0) cells.push(null);
  const rows: (Day | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) rows.push(cells.slice(i, i + 7));
  return rows;
}

/** Every day of a year, in order. */
export function yearOf(reading: Reading, year: number): Day[] {
  return [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].flatMap((month) => monthOf(reading, year, month));
}

/** What a run of days adds up to. */
export interface Tally {
  worked_ms: number;
  over_ms: number;
  /** Days worked. */
  days: number;
  /** Days of leave, taken or still ahead. */
  leaves: number;
}

/** Adds `days` up. */
export function tally(days: readonly Day[]): Tally {
  const sum: Tally = { worked_ms: 0, over_ms: 0, days: 0, leaves: 0 };
  for (const day of days) {
    sum.worked_ms += day.worked_ms;
    sum.over_ms += day.over_ms;
    if (day.kind === 'worked') sum.days += 1;
    if (day.kind === 'leave') sum.leaves += 1;
  }
  return sum;
}

/** When the keys usually go quiet and come back, minutes after midnight. */
export interface UsualNight {
  from_minutes: number;
  to_minutes: number;
}

/** The middle one of `minutes` as times of day, counted from `origin` so a run across midnight stays together. */
function middle(minutes: readonly number[], origin: number): number {
  const turned = minutes.map((m) => (m - origin + DAY_MINUTES) % DAY_MINUTES).sort((a, b) => a - b);
  const mid = turned[Math.floor(turned.length / 2)] ?? 0;
  return (mid + origin) % DAY_MINUTES;
}

/** The usual night over the last fourteen, or `null` before three are known. */
export function usualNight(nights: readonly Night[], turn: number): UsualNight | null {
  const recent = nights.slice(-USUAL_FROM);
  if (recent.length < USUAL_NEEDS) return null;
  const minutesOf = (ms: number) => {
    const d = new Date(ms);
    return d.getHours() * 60 + d.getMinutes();
  };
  // Counted from half a day before the turn, nights do not wrap.
  const origin = (turn * 60 + DAY_MINUTES / 2) % DAY_MINUTES;
  return {
    from_minutes: middle(
      recent.map((n) => minutesOf(n.from)),
      origin,
    ),
    to_minutes: middle(
      recent.map((n) => minutesOf(n.to)),
      origin,
    ),
  };
}

/** A piece of a strip, as fractions of the day's width. */
export interface Piece {
  left: number;
  width: number;
}

/** A day drawn on its 24-hour horizon. */
export interface Strip {
  /** Ink: where someone was here. */
  runs: Piece[];
  /** The dotted run under the line: the night, at either end of the day. */
  quiet: Piece[];
  /** The band of sky: working hours, on a working day; empty on any other. */
  hours: Piece;
  /** Where now is, on today only. */
  sun: number | null;
  /** Quarter marks, with the instant each stands for. */
  ticks: { at: number; unix_ms: number }[];
}

/** Lays `day` out on its strip. */
export function stripOf(reading: Reading, day: Day): Strip {
  const length = day.end - day.start;
  const piece = (s: Stretch): Piece => {
    const from = clamp((s.from - day.start) / length, 0, 1);
    const to = clamp((s.to - day.start) / length, 0, 1);
    return { left: from, width: Math.max(0, to - from) };
  };
  const whole = { from: day.start, to: day.end };
  const next = reading.dayAt(day.end).night;
  const quiet = [day.night, next]
    .filter((night): night is Night => night !== null && overlap(night, whole) > 0)
    .map(piece);
  return {
    runs: day.stretches.map(piece),
    quiet,
    hours: day.working ? piece(day.hours) : { left: 0, width: 0 },
    sun: day.today ? clamp((reading.now - day.start) / length, 0, 1) : null,
    ticks: [0, 1, 2, 3].map((quarter) => ({
      at: quarter / 4,
      unix_ms: new Date(day.start).setHours(reading.turn + quarter * 6),
    })),
  };
}

const clockOf = (ms: number) =>
  new Intl.DateTimeFormat('en-GB', { hour: '2-digit', minute: '2-digit', hour12: false }).format(
    new Date(ms),
  );

const dateOf = (ms: number, locale = 'en-GB') =>
  new Intl.DateTimeFormat(locale, { weekday: 'short', day: 'numeric', month: 'short' }).format(
    new Date(ms),
  );

/**
 * A week as plain lines, one day each: what Claude is sent when asked to
 * put the week into words, and nothing more than the pane already shows.
 */
export function weekDigest(reading: Reading, week: readonly Day[]): string {
  return digestOf(reading, week);
}

/** One day as Claude is sent it: the same line, under the same heading. */
export function dayDigest(reading: Reading, day: Day): string {
  return digestOf(reading, [day]);
}

/**
 * A month as Claude is sent it. Days still to come say nothing unless they
 * are leave, so they are left out: fewer lines, fewer tokens.
 */
export function monthDigest(reading: Reading, month: readonly Day[]): string {
  return digestOf(
    reading,
    month.filter((day) => day.start <= reading.today.start || day.kind === 'leave'),
  );
}

function digestOf(reading: Reading, week: readonly Day[]): string {
  const hours = week.find((day) => day.working)?.hours ?? week[0]?.hours;
  const head = hours
    ? `Working hours: ${clockOf(hours.from)} to ${clockOf(hours.to)}. Times are local, 24-hour.`
    : 'Times are local, 24-hour.';
  const lines = week.map((day) => {
    const name = dateOf(day.start);
    if (day.start > reading.today.start) {
      return `${name}: still to come${day.kind === 'leave' ? ', leave planned' : ''}.`;
    }
    if (day.kind !== 'worked') {
      const what = day.kind === 'nothing' ? 'nothing recorded' : day.kind;
      return `${name}: ${what}${day.today ? ' so far' : ''}.`;
    }
    const parts = [`${formatWorked(day.worked_ms)} at the keyboard`];
    if (day.first !== null && day.last !== null) {
      parts.push(`first touch ${clockOf(day.first)}, last ${clockOf(day.last)}`);
    }
    if (day.over_ms >= MINUTE) parts.push(`${formatWorked(day.over_ms)} outside working hours`);
    if (day.night) {
      const { from, to } = day.night;
      parts.push(
        `the night before ran ${clockOf(from)} to ${clockOf(to)} (${formatWorked(to - from)})`,
      );
    }
    if (day.fixed) parts.push('hours entered by hand');
    if (day.today) parts.push('today, not over yet');
    return `${name}: ${parts.join('; ')}.`;
  });
  return [head, ...lines].join('\n');
}

/** One CSV field: quoted when it has a comma, a quote or a line break in it. */
function field(value: string | number): string {
  const text = String(value);
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

/**
 * Days as CSV, one row each, for a timesheet or a spreadsheet: the date,
 * what the day read as, minutes worked and over, the first and last touch,
 * the night that led into it, and whether its hours were entered by hand.
 */
export function daysCsv(days: readonly Day[]): string {
  const time = (ms: number | null) => (ms === null ? '' : clockOf(ms));
  const rows = days.map((day) =>
    [
      day.key,
      new Intl.DateTimeFormat('en-GB', { weekday: 'short' }).format(new Date(day.start)),
      day.kind,
      Math.floor(day.worked_ms / MINUTE),
      Math.floor(day.over_ms / MINUTE),
      time(day.first),
      time(day.last),
      time(day.night?.from ?? null),
      time(day.night?.to ?? null),
      day.fixed ? 'yes' : 'no',
    ]
      .map(field)
      .join(','),
  );
  const head =
    'date,weekday,day,worked_minutes,over_minutes,first_touch,last_touch,night_from,night_to,by_hand';
  return [head, ...rows].join('\n');
}

/** `6h 42m`, `42m`, `0m`: time worked, to the minute. */
export function formatWorked(ms: number): string {
  const minutes = Math.max(0, Math.floor(ms / MINUTE));
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return h === 0 ? `${m}m` : `${h}h ${String(m).padStart(2, '0')}m`;
}

/** A year, looked back on. */
export interface YearReview {
  year: number;
  /** Days worked, their hours, the part outside working hours, and leave. */
  days: number;
  worked_ms: number;
  over_ms: number;
  leaves: number;
  /** Hours worked in each month, January first. */
  months: number[];
  /** The month with the most in it, 0 to 11, or `null` in an empty year. */
  busiest: number | null;
  /** The longest day worked. */
  longest: Day | null;
  /** The day that started earliest, and the one that ended latest, by the clock of its own day. */
  earliest: Day | null;
  latest: Day | null;
  /** The most days worked in a row. */
  run: number;
}

/** What `year` came to, from the days of it the almanac holds. */
export function yearReview(reading: Reading, year: number): YearReview {
  const days = yearOf(reading, year);
  const sum = tally(days);
  const months = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map(
    (month) => tally(monthOf(reading, year, month)).worked_ms,
  );
  const most = Math.max(...months);
  const worked = days.filter((day) => day.kind === 'worked');
  const best = (score: (day: Day) => number): Day | null =>
    worked.reduce<Day | null>(
      (top, day) => (top === null || score(day) > score(top) ? day : top),
      null,
    );
  let run = 0;
  let current = 0;
  for (const day of days) {
    current = day.kind === 'worked' ? current + 1 : 0;
    run = Math.max(run, current);
  }
  return {
    year,
    days: sum.days,
    worked_ms: sum.worked_ms,
    over_ms: sum.over_ms,
    leaves: sum.leaves,
    months,
    busiest: most > 0 ? months.indexOf(most) : null,
    longest: best((day) => day.worked_ms),
    earliest: best((day) => -((day.first ?? day.end) - day.start)),
    latest: best((day) => (day.last ?? day.start) - day.start),
    run,
  };
}

/** `142h`, `31h 20m`: a total over many days; minutes only while they still matter. */
export function formatTotal(ms: number): string {
  const minutes = Math.max(0, Math.floor(ms / MINUTE));
  return minutes >= 100 * 60 ? `${Math.round(minutes / 60)}h` : formatWorked(ms);
}

/** How long a worked day's bar is in the month, 0–1: full at ten hours. */
export function barLength(day: Day): number {
  if (day.kind !== 'worked') return 0;
  return clamp(day.worked_ms / (10 * HOUR), 0.12, 1);
}
