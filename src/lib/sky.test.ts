import { describe, expect, it } from 'vitest';

import { awakeMs, dayAround, domePath, domesBetween, hourMarks, nowAt, weekOfDomes } from './sky';
import type { History } from './types';

/** The value, or a failed test: no `!` needed. */
function present<T>(value: T | undefined): T {
  if (value === undefined) throw new Error('expected a value');
  return value;
}

const at = (d: number, h: number, m = 0) => new Date(2026, 10, d, h, m).getTime();

const history = (...entries: [number, number | null][]): History => ({
  version: 1,
  entries: entries.map(([started, ended]) => ({
    started_unix_ms: started,
    ended_unix_ms: ended,
    mode: 'indefinite',
    ended: ended === null ? null : 'user',
    pauses: [],
  })),
});

describe('the day', () => {
  it('turns at midnight, or at the almanac turn', () => {
    const now = at(12, 3);
    expect(dayAround(now)).toEqual({ start: at(12, 0), end: at(13, 0) });
    // At 3 am with a 4 am turn, the day began yesterday.
    expect(dayAround(now, 4)).toEqual({ start: at(11, 4), end: at(12, 4) });
  });

  it('clips stretches to the day and ends a running one at now', () => {
    const { start, end } = dayAround(at(12, 15));
    const domes = domesBetween(
      history([at(11, 22), at(12, 2)], [at(12, 9), at(12, 11)], [at(12, 14), null]),
      start,
      end,
      at(12, 15),
    );
    expect(domes).toHaveLength(3);
    expect(domes[0]?.from).toBe(0);
    expect(domes[0]?.to).toBeCloseTo(2 / 24);
    expect(domes[2]?.running).toBe(true);
    expect(domes[2]?.to).toBeCloseTo(15 / 24);
    expect(awakeMs(domes, end - start)).toBeCloseTo(5 * 3_600_000);
  });

  it('never draws a stretch that ended uncleanly as still running', () => {
    const { start, end } = dayAround(at(12, 15));
    const crashed: History = {
      version: 1,
      entries: [
        {
          started_unix_ms: at(12, 9),
          ended_unix_ms: null,
          mode: 'indefinite',
          ended: 'sleep',
          pauses: [],
        },
      ],
    };
    expect(domesBetween(crashed, start, end, at(12, 15))).toEqual([]);
  });

  it('leaves out what is too short to see, and nothing without a log', () => {
    const { start, end } = dayAround(at(12, 15));
    expect(domesBetween(history([at(12, 9), at(12, 9) + 30_000]), start, end, at(12, 15))).toEqual(
      [],
    );
    expect(domesBetween(null, start, end, at(12, 15))).toEqual([]);
  });

  it('draws a dome as tall as its stretch is long, up to the tallest', () => {
    const { start, end } = dayAround(at(12, 15));
    const [short, long] = domesBetween(
      history([at(12, 1), at(12, 2)], [at(12, 3), at(12, 13)]),
      start,
      end,
      at(12, 15),
    );
    const a = domePath(present(short), 960, 200, 120);
    const b = domePath(present(long), 960, 200, 120);
    expect(a.peak).toBeGreaterThan(b.peak);
    expect(b.peak).toBe(80);
    expect(a.d.startsWith('M 40 200 A')).toBe(true);
  });

  it('draws a running stretch rising to the sun', () => {
    const { start, end } = dayAround(at(12, 15));
    const [running] = domesBetween(history([at(12, 12), null]), start, end, at(12, 15));
    const path = domePath(present(running), 960, 200, 120);
    // Three hours up: well above the line, not yet the top.
    expect(path.peak).toBeCloseTo(200 - 120 * 0.68);
    expect(path.d.startsWith('M 480 200 A')).toBe(true);
  });

  it('marks every six hours and finds now', () => {
    const { start, end } = dayAround(at(12, 15));
    expect(hourMarks(start, end).map((m) => m.at)).toEqual([0, 0.25, 0.5, 0.75]);
    // A day turning at 1 am still marks the clock's 6, noon, 6 and midnight.
    const turned = dayAround(at(12, 15), 1);
    expect(hourMarks(turned.start, turned.end).map((m) => new Date(m.unix_ms).getHours())).toEqual([
      6, 12, 18, 0,
    ]);
    expect(nowAt(at(12, 18), start, end)).toBe(0.75);
    expect(nowAt(at(14, 0), start, end)).toBe(1);
  });
});

describe('the week', () => {
  it('runs Monday to Sunday with today marked and totals each day', () => {
    // 12 Nov 2026 is a Thursday.
    const week = weekOfDomes(history([at(10, 9), at(10, 12)]), at(12, 15), 0, 'en-GB');
    expect(week.map((d) => d.label)).toEqual(['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']);
    expect(week[3]?.today).toBe(true);
    expect(week[4]?.ahead).toBe(true);
    expect(week[1]?.awake_ms).toBeCloseTo(3 * 3_600_000);
  });
});
