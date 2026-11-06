import { describe, expect, it } from 'vitest';

import {
  batteryText,
  formatClock,
  formatDuration,
  formatStop,
  nextOccurrence,
  progress,
  remainingSecs,
  statusText,
  suggestedUntil,
} from './format';
import type { Snapshot } from './types';

const base: Snapshot = {
  mode: 'off',
  started_at_unix_ms: null,
  ends_at_unix_ms: null,
  remaining_secs: null,
  plan_secs: null,
  guards: { system: false, display: false },
  paused: null,
  battery: { on_ac: true, percent: 80 },
  settings: {
    version: 1,
    keep_display_on: true,
    default_duration_secs: null,
    startup_plan: 'off',
    pause_below_battery_percent: 20,
    pause_on_battery: false,
    start_hidden: true,
    close_to_tray: true,
    free_memory_when_hidden: true,
    theme: 'system',
  },
  app_version: '0.1.0',
};

const NOW = Date.UTC(2026, 0, 1, 12, 0, 0);

describe('formatDuration', () => {
  it('matches the Rust formatter', () => {
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(60)).toBe('1m');
    expect(formatDuration(12 * 60 + 30)).toBe('12m');
    expect(formatDuration(3600)).toBe('1h 00m');
    expect(formatDuration(2 * 3600 + 5 * 60)).toBe('2h 05m');
  });

  it('never goes negative', () => {
    expect(formatDuration(-5)).toBe('0s');
  });
});

describe('formatStop', () => {
  it('uses minutes under an hour and hours above', () => {
    expect(formatStop(1800)).toBe('30m');
    expect(formatStop(3600)).toBe('1h');
    expect(formatStop(5400)).toBe('1.5h');
    expect(formatStop(8 * 3600)).toBe('8h');
  });
});

describe('formatClock', () => {
  it('renders hours and minutes in the given locale', () => {
    const noon = new Date(2026, 0, 1, 17, 5).getTime();
    expect(formatClock(noon, 'en-US')).toBe('5:05 pm');
    expect(formatClock(noon, 'en-GB')).toBe('17:05');
  });
});

describe('remainingSecs / progress', () => {
  it('is null when there is no deadline', () => {
    expect(remainingSecs(base, NOW)).toBeNull();
    expect(progress(base, NOW)).toBe(0);
  });

  it('rounds up and clamps at zero', () => {
    const s = { ...base, mode: 'timed' as const, ends_at_unix_ms: NOW + 1500, plan_secs: 60 };
    expect(remainingSecs(s, NOW)).toBe(2);
    expect(remainingSecs(s, NOW + 10_000)).toBe(0);
  });

  it('progress runs from 0 to 1 over the plan', () => {
    const s = { ...base, mode: 'timed' as const, ends_at_unix_ms: NOW + 60_000, plan_secs: 60 };
    expect(progress(s, NOW)).toBe(0);
    expect(progress(s, NOW + 30_000)).toBeCloseTo(0.5);
    expect(progress(s, NOW + 90_000)).toBe(1);
  });
});

describe('statusText', () => {
  it('describes every state', () => {
    expect(statusText(base, NOW).title).toBe('Resting');
    expect(statusText({ ...base, mode: 'indefinite' }, NOW)).toEqual({
      title: 'Awake',
      detail: 'Until you say otherwise.',
    });
    expect(statusText({ ...base, mode: 'indefinite', paused: 'low_battery' }, NOW)).toEqual({
      title: 'Paused',
      detail: 'Holding off while battery is low.',
    });
    const ends = new Date(2026, 0, 1, 17, 30).getTime();
    const timed = { ...base, mode: 'timed' as const, ends_at_unix_ms: ends, plan_secs: 3600 };
    expect(statusText(timed, ends - 90 * 60 * 1000, 'en-US').detail).toBe(
      'Until 5:30 pm · 1h 30m left',
    );
  });
});

describe('batteryText', () => {
  it('names the source and charge', () => {
    expect(batteryText(base)).toBe('Plugged in · 80%');
    expect(batteryText({ ...base, battery: { on_ac: false, percent: 42 } })).toBe(
      'On battery · 42%',
    );
    expect(batteryText({ ...base, battery: { on_ac: true, percent: null } })).toBe('Plugged in');
  });
});

describe('nextOccurrence', () => {
  const now = new Date(2026, 0, 1, 12, 0).getTime();

  it('picks today when the time is still ahead', () => {
    expect(nextOccurrence('17:30', now)).toBe(new Date(2026, 0, 1, 17, 30).getTime());
  });

  it('rolls to tomorrow when the time has passed', () => {
    expect(nextOccurrence('09:00', now)).toBe(new Date(2026, 0, 2, 9, 0).getTime());
  });

  it('rejects garbage', () => {
    expect(nextOccurrence('25:00', now)).toBeNull();
    expect(nextOccurrence('soon', now)).toBeNull();
  });
});

describe('suggestedUntil', () => {
  it('is an hour ahead rounded up to a quarter hour', () => {
    expect(suggestedUntil(new Date(2026, 0, 1, 12, 1).getTime())).toBe('13:15');
    expect(suggestedUntil(new Date(2026, 0, 1, 12, 0).getTime())).toBe('13:00');
  });
});
