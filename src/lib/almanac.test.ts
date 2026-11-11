import { describe, expect, it } from 'vitest';

import {
  barLength,
  dayDigest,
  monthDigest,
  monthOf,
  daysCsv,
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
  weekDigest,
  weekOf,
  yearOf,
  yearReview,
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
  break_after_mins: null,
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
const NEWLINE = String.fromCharCode(10);
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

describe('a weekday that is not worked', () => {
  // Five weeks from Monday 5 October 2026, worked Monday to Thursday.
  const october = (date: number, hour: number) => new Date(2026, 9, date, hour).getTime();
  const weeks = [5, 12, 19, 26, 33];
  const monToThu = weeks.flatMap((monday) =>
    [0, 1, 2, 3].map((i) => span(october(monday + i, 9), october(monday + i, 17))),
  );
  const now = october(40, 12);

  it('reads its empty days as days off, not a leave every week', () => {
    const reading = read(book(monToThu), office, now);
    for (const monday of weeks) {
      const friday = reading.dayAt(october(monday + 4, 12));
      expect(friday.working).toBe(true);
      expect(friday.usual_off).toBe(true);
      expect(friday.kind).toBe('rest');
    }
    expect(tally(monthOf(reading, 2026, 9)).leaves).toBe(0);
  });

  it('still reads one missed day as leave, and a week away as leave', () => {
    const spans = monToThu.filter(
      (s) => s.from_unix_ms !== october(13, 9) && s.from_unix_ms < october(26, 0),
    );
    const worksFridays = [
      ...spans,
      ...[9, 16, 23].map((date) => span(october(date, 9), october(date, 17))),
    ];
    const reading = read(book(worksFridays), office, now);
    expect(reading.dayAt(october(13, 12)).kind).toBe('leave');
    // The week of the 26th: nobody here at all.
    for (const date of [26, 27, 28, 29, 30]) {
      expect(reading.dayAt(october(date, 12)).kind).toBe('leave');
    }
  });

  it('does not decide from fewer than three weeks, or against a leave you marked', () => {
    const two = monToThu.filter((s) => s.from_unix_ms < october(19, 0));
    const early = read(book(two), office, october(19, 12));
    expect(early.dayAt(october(9, 12)).kind).toBe('leave');
    const marked = read(book(monToThu, { '2026-10-16': 'leave' }), office, now);
    expect(marked.dayAt(october(16, 12)).kind).toBe('leave');
    expect(marked.dayAt(october(9, 12)).kind).toBe('leave');
  });
});

describe('a year, looked back on', () => {
  it('finds its longest day, its edges, its fullest month and its longest run', () => {
    const spans = [
      span(at(9, 9), at(9, 17)),
      span(at(10, 7, 30), at(10, 13)),
      span(at(11, 10), at(11, 22, 15)),
      span(at(13, 9), at(13, 12)),
    ];
    const reading = read(book(spans, { '2026-11-12': 'leave' }), office, at(16, 12));
    const review = yearReview(reading, 2026);
    expect(review.days).toBe(4);
    expect(review.leaves).toBe(1);
    expect(review.worked_ms).toBe((8 + 5.5 + 12.25 + 3) * HOUR);
    expect(review.busiest).toBe(10);
    expect(review.months[10]).toBe(review.worked_ms);
    expect(review.longest?.key).toBe('2026-11-11');
    expect(review.earliest?.key).toBe('2026-11-10');
    expect(review.latest?.key).toBe('2026-11-11');
    expect(review.run).toBe(3);
  });

  it('is empty for a year with nothing in it', () => {
    const review = yearReview(read(book([]), office, at(16, 12)), 2025);
    expect(review).toMatchObject({ days: 0, busiest: null, longest: null, run: 0 });
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

describe('a week, as Claude is sent it', () => {
  const spans = [span(at(9, 9, 2), at(9, 23, 48)), span(at(10, 7, 2), at(10, 12))];
  const reading = read(book(spans, { '2026-11-13': 'leave' }), office, at(10, 12));
  const digest = weekDigest(reading, weekOf(reading, reading.today));

  it('is one line a day under the working hours, with nothing the pane does not show', () => {
    const lines = digest.split('\n');
    expect(lines).toHaveLength(8);
    expect(lines[0]).toBe('Working hours: 09:00 to 18:00. Times are local, 24-hour.');
    expect(lines[1]).toBe(
      'Mon 9 Nov: 14h 46m at the keyboard; first touch 09:02, last 23:48; 5h 48m outside working hours.',
    );
    expect(lines[2]).toBe(
      'Tue 10 Nov: 4h 58m at the keyboard; first touch 07:02, last 12:00; 1h 58m outside working hours; the night before ran 23:48 to 07:02 (7h 14m); today, not over yet.',
    );
    expect(lines[3]).toBe('Wed 11 Nov: still to come.');
    expect(lines[5]).toBe('Fri 13 Nov: still to come, leave planned.');
  });

  it('sends one day as its own line, and a month without the days still to come', () => {
    const day = dayDigest(reading, reading.dayAt(at(9, 12))).split(NEWLINE);
    expect(day).toHaveLength(2);
    expect(day[1]).toMatch(/^Mon 9 Nov: 14h 46m at the keyboard/);
    const month = monthDigest(reading, monthOf(reading, 2026, 10)).split(NEWLINE);
    // The heading, the ten days so far, and the one leave still ahead.
    expect(month).toHaveLength(12);
    expect(month[month.length - 1]).toBe('Fri 13 Nov: still to come, leave planned.');
    expect(monthDigest(reading, monthOf(reading, 2026, 10)).length).toBeLessThan(6000);
  });

  it('is short: a week is a few hundred characters', () => {
    expect(digest.length).toBeLessThan(900);
  });
});

describe('days as CSV', () => {
  const spans = [span(at(9, 9, 2), at(9, 23, 48)), span(at(10, 7, 2), at(10, 12))];
  const reading = read(book(spans, { '2026-11-12': 'leave' }), office, at(13, 12));
  const csv = daysCsv(weekOf(reading, reading.dayAt(at(10, 12)))).split('\n');

  it('has a header and a row a day', () => {
    expect(csv).toHaveLength(8);
    expect(csv[0]).toBe(
      'date,weekday,day,worked_minutes,over_minutes,first_touch,last_touch,night_from,night_to,by_hand',
    );
    expect(csv[1]).toBe('2026-11-09,Mon,worked,886,348,09:02,23:48,,,no');
    expect(csv[2]).toBe('2026-11-10,Tue,worked,298,118,07:02,12:00,23:48,07:02,no');
    expect(csv[4]).toBe('2026-11-12,Thu,leave,0,0,,,,,no');
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
