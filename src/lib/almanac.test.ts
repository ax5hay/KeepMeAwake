import { describe, expect, it } from 'vitest';

import {
  barLength,
  dayKey,
  dayStart,
  dayTurn,
  findNights,
  formatTotal,
  formatWorked,
  hoursLength,
  monthGrid,
  read,
  shiftDay,
  startOfKey,
  stretchOn,
  stripOf,
  tally,
  usualNight,
  weekOf,
  yearOf,
} from './almanac';
import type { Almanac, AlmanacSettings, Mark, Span } from './types';

/** Office hours: the day turns at 1 am. */
const office: AlmanacSettings = {
  enabled: true,
  work_days: [true, true, true, true, true, false, false],
  start_minutes: 9 * 60,
  end_minutes: 18 * 60,
  empty_days_are_leave: true,
  whisper_night: true,
  rest_after_mins: null,
};

/** 3 pm to midnight: the day turns at 7 am. */
const evening: AlmanacSettings = { ...office, start_minutes: 15 * 60, end_minutes: 0 };

/** A moment in November 2026 (the 9th is a Monday), local time. */
const at = (date: number, hour: number, minute = 0) =>
  new Date(2026, 10, date, hour, minute).getTime();

const span = (from: number, to: number): Span => ({ from_unix_ms: from, to_unix_ms: to });

const book = (
  spans: Span[],
  marks: Record<string, Mark> = {},
  fixes: Record<string, Span> = {},
): Almanac => ({ version: 1, spans, marks, fixes });

const HOUR = 3_600_000;
const MIN = 60_000;

describe('the turn of the day', () => {
  it('falls opposite the middle of the working hours, on the hour', () => {
    expect(dayTurn(office)).toBe(1);
    expect(dayTurn(evening)).toBe(7);
    expect(dayTurn({ ...office, start_minutes: 22 * 60, end_minutes: 6 * 60 })).toBe(14);
  });

  it('reads equal hours as the whole day', () => {
    expect(hoursLength({ ...office, start_minutes: 600, end_minutes: 600 })).toBe(1440);
    expect(hoursLength(evening)).toBe(540);
  });

  it('puts the small hours in the day before', () => {
    expect(dayKey(dayStart(at(11, 0, 30), 1))).toBe('2026-11-10');
    expect(dayKey(dayStart(at(11, 1, 0), 1))).toBe('2026-11-11');
    expect(dayKey(dayStart(at(11, 6, 59), 7))).toBe('2026-11-10');
  });

  it('steps a day at a time and finds a day by its key', () => {
    const start = dayStart(at(10, 12), 7);
    expect(shiftDay(start, 1)).toBe(at(11, 7));
    expect(shiftDay(start, -9)).toBe(at(1, 7));
    expect(startOfKey('2026-11-10', 7)).toBe(start);
    expect(startOfKey('2026-02-30', 7)).toBeNull();
    expect(startOfKey('someday', 7)).toBeNull();
  });
});

describe('a day', () => {
  it('adds up its stretches and names its first and last touch', () => {
    const reading = read(
      book([span(at(10, 9, 2), at(10, 12, 30)), span(at(10, 13, 15), at(10, 18, 40))]),
      office,
      at(10, 19),
    );
    const day = reading.today;
    expect(day.key).toBe('2026-11-10');
    expect(day.kind).toBe('worked');
    expect(formatWorked(day.worked_ms)).toBe('8h 53m');
    expect(day.first).toBe(at(10, 9, 2));
    expect(day.last).toBe(at(10, 18, 40));
    expect(formatWorked(day.over_ms)).toBe('40m');
  });

  it('keeps work past midnight on the day it began', () => {
    // 3 pm to midnight, started early at 2 and carried on until 2 am.
    const reading = read(book([span(at(10, 14), at(11, 2))]), evening, at(11, 12));
    const day = reading.dayAt(at(10, 20));
    expect(day.key).toBe('2026-11-10');
    expect(formatWorked(day.worked_ms)).toBe('12h 00m');
    expect(formatWorked(day.over_ms)).toBe('3h 00m');
    expect(day.hours).toEqual({ from: at(10, 15), to: at(11, 0) });
    expect(reading.today.worked_ms).toBe(0);
  });

  it('splits a span that crosses the turn into both days', () => {
    const reading = read(book([span(at(10, 23), at(11, 3))]), office, at(11, 12));
    expect(formatWorked(reading.dayAt(at(10, 12)).worked_ms)).toBe('2h 00m');
    expect(formatWorked(reading.today.worked_ms)).toBe('2h 00m');
  });

  it('counts all of a day that is not a working day as over', () => {
    const reading = read(book([span(at(14, 10), at(14, 12))]), office, at(16, 12));
    const saturday = reading.dayAt(at(14, 12));
    expect(saturday.working).toBe(false);
    expect(saturday.over_ms).toBe(2 * HOUR);
  });

  it('does not call a few minutes a day of work', () => {
    const reading = read(
      book([span(at(9, 9), at(9, 17)), span(at(10, 9), at(10, 9, 4))]),
      office,
      at(11, 12),
    );
    expect(reading.dayAt(at(10, 12)).kind).toBe('leave');
  });
});

describe('what a day reads as', () => {
  const worked = [span(at(9, 9), at(9, 17)), span(at(12, 9), at(12, 17))];
  const now = at(16, 12);

  it('reads an empty working day since it began as leave', () => {
    const reading = read(book(worked), office, now);
    expect(reading.dayAt(at(10, 12)).kind).toBe('leave');
    expect(reading.dayAt(at(11, 12)).kind).toBe('leave');
  });

  it('reads the days before it began as nothing', () => {
    const reading = read(book(worked), office, now);
    expect(reading.dayAt(at(6, 12)).kind).toBe('nothing');
  });

  it('reads an empty weekend as rest, today as nothing so far, and tomorrow as ahead', () => {
    const reading = read(book(worked), office, now);
    expect(reading.dayAt(at(14, 12)).kind).toBe('rest');
    expect(reading.today.kind).toBe('nothing');
    expect(reading.dayAt(at(17, 12)).kind).toBe('ahead');
  });

  it('lets a mark win either way, and marks a day still to come', () => {
    const reading = read(
      book(worked, { '2026-11-10': 'clear', '2026-11-12': 'leave', '2026-11-20': 'leave' }),
      office,
      now,
    );
    expect(reading.dayAt(at(10, 12)).kind).toBe('nothing');
    expect(reading.dayAt(at(12, 12)).kind).toBe('leave');
    expect(reading.dayAt(at(20, 12)).kind).toBe('leave');
  });

  it('never assumes leave with the setting off', () => {
    const reading = read(book(worked), { ...office, empty_days_are_leave: false }, now);
    expect(reading.dayAt(at(10, 12)).kind).toBe('nothing');
  });
});

describe('a day corrected by hand', () => {
  const read9 = [span(at(9, 9), at(9, 12)), span(at(9, 14), at(9, 23, 30))];
  const next = span(at(10, 7), at(10, 12));

  it('stands for what was read that day, and says so', () => {
    const fix = span(at(9, 10), at(9, 18));
    const reading = read(book([...read9, next], {}, { '2026-11-09': fix }), office, at(10, 12));
    const monday = reading.dayAt(at(9, 12));
    expect(monday.fixed).toBe(true);
    expect(formatWorked(monday.worked_ms)).toBe('8h 00m');
    expect(monday.first).toBe(at(9, 10));
    expect(monday.last).toBe(at(9, 18));
    expect(reading.today.fixed).toBe(false);
    expect(formatWorked(reading.today.worked_ms)).toBe('5h 00m');
  });

  it('moves the night with it', () => {
    const fix = span(at(9, 10), at(9, 22));
    const reading = read(book([...read9, next], {}, { '2026-11-09': fix }), office, at(10, 12));
    expect(reading.today.night).toEqual({ from: at(9, 22), to: at(10, 7), key: '2026-11-10' });
  });

  it('cuts only its own day out of a span that crosses the turn', () => {
    const late = span(at(9, 20), at(10, 3));
    const fix = span(at(9, 9), at(9, 17));
    const reading = read(book([late], {}, { '2026-11-09': fix }), office, at(10, 12));
    expect(formatWorked(reading.dayAt(at(9, 12)).worked_ms)).toBe('8h 00m');
    expect(formatWorked(reading.today.worked_ms)).toBe('2h 00m');
  });

  it('can say nobody was here at all', () => {
    const empty = span(at(9, 9), at(9, 9));
    const reading = read(book([...read9, next], {}, { '2026-11-09': empty }), office, at(10, 12));
    const monday = reading.dayAt(at(9, 12));
    expect(monday.worked_ms).toBe(0);
    expect(monday.fixed).toBe(true);
  });

  it('turns two times of day into a stretch inside the day', () => {
    const reading = read(book([]), evening, at(10, 12));
    const day = reading.dayAt(at(9, 20));
    expect(stretchOn(day, 15 * 60, 30)).toEqual(span(at(9, 15), at(10, 0, 30)));
    expect(stretchOn(day, 2 * 60, 4 * 60)).toEqual(span(at(10, 2), at(10, 4)));
    // Never past the turn into the next day.
    expect(stretchOn(day, 22 * 60, 9 * 60)).toEqual(span(at(9, 22), at(10, 7)));
  });
});

describe('the night', () => {
  it('is the quiet between the last touch and the first', () => {
    const spans = [span(at(9, 9), at(9, 23, 48)), span(at(10, 7, 2), at(10, 12))];
    const nights = findNights(spans, 1);
    expect(nights).toEqual([{ from: at(9, 23, 48), to: at(10, 7, 2), key: '2026-11-10' }]);
    expect(read(book(spans), office, at(10, 12)).today.night).toEqual(nights[0]);
  });

  it('is found the same with the hours shifted late', () => {
    // Stopped at 2:40 am, back at 11:15 am: the night of the day that turns at 7.
    const nights = findNights(
      [span(at(9, 15), at(10, 2, 40)), span(at(10, 11, 15), at(10, 23))],
      7,
    );
    expect(nights.map((n) => n.key)).toEqual(['2026-11-10']);
  });

  it('is not an afternoon out, nor a weekend away', () => {
    expect(findNights([span(at(9, 9), at(9, 13)), span(at(9, 17), at(9, 20))], 1)).toEqual([]);
    expect(findNights([span(at(13, 9), at(13, 19)), span(at(16, 9), at(16, 17))], 1)).toEqual([]);
  });

  it('is not ended by a stir', () => {
    const nights = findNights(
      [
        span(at(9, 9), at(9, 23, 30)),
        span(at(10, 3, 10), at(10, 3, 10) + 20_000),
        span(at(10, 7), at(10, 12)),
      ],
      1,
    );
    expect(nights).toEqual([{ from: at(9, 23, 30), to: at(10, 7), key: '2026-11-10' }]);
  });

  it('keeps the longest when a morning has two', () => {
    const nights = findNights(
      [span(at(9, 9), at(9, 22)), span(at(10, 1, 30), at(10, 2)), span(at(10, 8), at(10, 12))],
      1,
    );
    expect(nights).toHaveLength(1);
    expect(nights[0]?.from).toBe(at(10, 2));
  });
});

describe('the usual night', () => {
  it('waits for three nights, then gives the middle of each end across midnight', () => {
    const night = (date: number, bed: [number, number], up: [number, number]) => ({
      from: at(date, ...bed),
      to: at(date + 1, ...up),
      key: '',
    });
    const nights = [night(2, [23, 40], [7, 0]), night(3, [0, 20], [7, 30])];
    expect(usualNight(nights, 1)).toBeNull();
    nights[1] = { from: at(4, 0, 20), to: at(4, 7, 30), key: '' };
    nights.push(night(5, [23, 50], [6, 45]));
    expect(usualNight(nights, 1)).toEqual({ from_minutes: 23 * 60 + 50, to_minutes: 7 * 60 });
  });
});

describe('weeks, months and tallies', () => {
  const spans = [
    span(at(9, 9), at(9, 17)),
    span(at(10, 9), at(10, 19)),
    span(at(12, 9), at(12, 13)),
  ];
  const reading = read(book(spans, { '2026-11-27': 'leave' }), office, at(13, 12));

  it('lays a week out Monday first', () => {
    const week = weekOf(reading, reading.today);
    expect(week.map((d) => d.key)).toEqual([
      '2026-11-09',
      '2026-11-10',
      '2026-11-11',
      '2026-11-12',
      '2026-11-13',
      '2026-11-14',
      '2026-11-15',
    ]);
    expect(week.map((d) => d.kind)).toEqual([
      'worked',
      'worked',
      'leave',
      'worked',
      'nothing',
      'ahead',
      'ahead',
    ]);
  });

  it('adds a week up', () => {
    const sum = tally(weekOf(reading, reading.today));
    expect(sum.days).toBe(3);
    expect(sum.leaves).toBe(1);
    expect(formatTotal(sum.worked_ms)).toBe('22h 00m');
    expect(formatWorked(sum.over_ms)).toBe('1h 00m');
  });

  it('lays a month out as a calendar and counts leave still ahead', () => {
    const grid = monthGrid(reading, 2026, 10);
    expect(grid).toHaveLength(6);
    expect(grid[0]?.map((d) => d?.key.slice(-2) ?? null)).toEqual([
      null,
      null,
      null,
      null,
      null,
      null,
      '01',
    ]);
    expect(grid[5]?.[0]?.key).toBe('2026-11-30');
    expect(tally(yearOf(reading, 2026)).leaves).toBe(2);
    expect(yearOf(reading, 2026)).toHaveLength(365);
  });
});

describe('a strip', () => {
  it('places ink, working hours, the night and the sun along the day', () => {
    const spans = [span(at(9, 9), at(9, 23)), span(at(10, 7), at(10, 13))];
    const reading = read(book(spans), office, at(10, 13));
    const strip = stripOf(reading, reading.today);
    // The day runs 1 am to 1 am: 7 am is a quarter of the way along.
    expect(strip.runs[0]?.left).toBeCloseTo(0.25);
    expect(strip.runs[0]?.width).toBeCloseTo(0.25);
    expect(strip.hours.left).toBeCloseTo(8 / 24);
    expect(strip.hours.width).toBeCloseTo(9 / 24);
    expect(strip.quiet).toEqual([{ left: 0, width: 0.25 }]);
    expect(strip.sun).toBeCloseTo(0.5);
    expect(strip.ticks.map((t) => t.unix_ms)).toEqual([
      at(10, 1),
      at(10, 7),
      at(10, 13),
      at(10, 19),
    ]);
  });

  it('stands no band of working hours on a day that is not a working day', () => {
    const reading = read(book([span(at(14, 10), at(14, 12))]), office, at(16, 12));
    expect(stripOf(reading, reading.dayAt(at(14, 12))).hours.width).toBe(0);
    expect(stripOf(reading, reading.dayAt(at(13, 12))).hours.width).toBeGreaterThan(0);
  });

  it('draws the head of the next night at the end of the day before, and no sun there', () => {
    const spans = [span(at(9, 9), at(9, 23)), span(at(10, 7), at(10, 13))];
    const reading = read(book(spans), office, at(10, 13));
    const monday = stripOf(reading, reading.dayAt(at(9, 12)));
    expect(monday.quiet[0]?.left).toBeCloseTo(22 / 24);
    expect(monday.sun).toBeNull();
  });
});

describe('formatting', () => {
  it('says time worked to the minute', () => {
    expect(formatWorked(0)).toBe('0m');
    expect(formatWorked(42 * MIN + 59_000)).toBe('42m');
    expect(formatWorked(6 * HOUR + 2 * MIN)).toBe('6h 02m');
  });

  it('drops the minutes from a long total', () => {
    expect(formatTotal(31 * HOUR + 20 * MIN)).toBe('31h 20m');
    expect(formatTotal(142 * HOUR + 40 * MIN)).toBe('143h');
  });

  it('gives a worked day a bar that is full at ten hours and never invisible', () => {
    const reading = read(
      book([span(at(9, 9), at(9, 9, 30)), span(at(10, 8), at(10, 20))]),
      office,
      at(11, 12),
    );
    expect(barLength(reading.dayAt(at(9, 12)))).toBe(0.12);
    expect(barLength(reading.dayAt(at(10, 12)))).toBe(1);
    expect(barLength(reading.today)).toBe(0);
  });
});
