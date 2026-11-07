import { describe, expect, it } from 'vitest';

import { formatDay, formatLength, formatSpan, historyRows } from './history';
import type { History, HistoryEntry } from './types';

/** Local wall time on a day in November 2026 (the 3rd is a Tuesday). */
function at(day: number, h: number, m: number): number {
  return new Date(2026, 10, day, h, m).getTime();
}

const NOW = at(9, 15, 0);

function entry(partial: Partial<HistoryEntry>): HistoryEntry {
  return {
    started_unix_ms: at(9, 9, 2),
    ended_unix_ms: at(9, 11, 30),
    mode: 'timed',
    ended: 'timer',
    pauses: [],
    ...partial,
  };
}

describe('formatDay', () => {
  it('says Today, Yesterday, then the weekday, day and month', () => {
    expect(formatDay(at(9, 9, 0), NOW, 'en-US')).toBe('Today');
    expect(formatDay(at(8, 23, 59), NOW, 'en-US')).toBe('Yesterday');
    expect(formatDay(at(3, 10, 15), NOW, 'en-GB')).toBe('Tue 3 Nov');
  });

  it('keeps the day-month order in a locale that writes it the other way', () => {
    expect(formatDay(at(3, 10, 15), NOW, 'en-US')).toBe('Tue 3 Nov');
  });
});

describe('formatSpan', () => {
  it('shows both ends in local time, and a dash for an end it never had', () => {
    expect(formatSpan(at(9, 9, 2), at(9, 11, 30), 'en-US')).toBe('9:02 am – 11:30 am');
    expect(formatSpan(at(9, 22, 15), null, 'en-US')).toBe('10:15 pm – –');
  });
});

describe('formatLength', () => {
  it('reads like the rule: hours and minutes, no zero minutes', () => {
    expect(formatLength(at(9, 9, 2), at(9, 11, 30))).toBe('2h 28m');
    expect(formatLength(at(9, 7, 41), at(9, 7, 46))).toBe('5m');
    expect(formatLength(at(9, 9, 0), at(9, 18, 0))).toBe('9h');
    expect(formatLength(at(9, 9, 0), at(9, 9, 0) + 40_000)).toBe('40s');
    expect(formatLength(at(9, 9, 0), null)).toBe('–');
  });
});

describe('historyRows', () => {
  it('lists newest first with how each ended, at most the limit', () => {
    const history: History = {
      version: 1,
      entries: [
        entry({ started_unix_ms: at(3, 22, 15), ended_unix_ms: at(4, 1, 5), ended: 'battery' }),
        entry({ started_unix_ms: at(8, 9, 0), ended_unix_ms: at(8, 18, 0), ended: 'schedule' }),
        entry({ started_unix_ms: at(9, 7, 41), ended_unix_ms: at(9, 7, 46), ended: 'user' }),
        entry({}),
      ],
    };
    const rows = historyRows(history, NOW, 'en-US', 3);
    expect(rows.map((r) => [r.day, r.span, r.length, r.ended])).toEqual([
      ['Today', '9:02 am – 11:30 am', '2h 28m', 'timer'],
      ['Today', '7:41 am – 7:46 am', '5m', 'you'],
      ['Yesterday', '9:00 am – 6:00 pm', '9h', 'schedule'],
    ]);
  });

  it('describes pauses under their row, and a stretch still running', () => {
    const history: History = {
      version: 1,
      entries: [
        entry({ started_unix_ms: at(8, 22, 15), ended_unix_ms: null, ended: 'sleep' }),
        entry({
          ended_unix_ms: null,
          ended: null,
          pauses: [
            { from_unix_ms: at(9, 12, 10), to_unix_ms: at(9, 12, 40), reason: 'on_battery' },
            { from_unix_ms: at(9, 13, 0), to_unix_ms: null, reason: 'power_request_failed' },
          ],
        }),
      ],
    };
    const [running, slept] = historyRows(history, NOW, 'en-US');
    expect(running?.ended).toBe('now');
    expect(running?.length).toBe('–');
    expect(running?.pauses).toEqual([
      'paused 12:10 pm – 12:40 pm · battery',
      'paused 1:00 pm – – · refused',
    ]);
    expect(slept?.ended).toBe('sleep');
    expect(slept?.span).toBe('10:15 pm – –');
    expect(slept?.day).toBe('Yesterday');
  });

  it('says idle for a stretch the overnight guard let go', () => {
    const history: History = {
      version: 1,
      entries: [entry({ mode: 'indefinite', ended: 'idle' })],
    };
    expect(historyRows(history, NOW, 'en-US')[0]?.ended).toBe('idle');
  });
});
