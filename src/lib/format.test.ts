import { describe, expect, it } from 'vitest';

import {
  batteryText,
  formatClock,
  formatDuration,
  formatStop,
  formatWhen,
  minutesToTime,
  nextOccurrence,
  pauseText,
  sunsetSoon,
  progress,
  remainingSecs,
  statusText,
  suggestedUntil,
  timeToMinutes,
  toggleWhisper,
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
  schedule: {
    enabled: false,
    in_window: false,
    skipped: false,
    window_start_unix_ms: null,
    window_end_unix_ms: null,
    next_start_unix_ms: null,
  },
  guard_rests_at_unix_ms: null,
  guard_watching: false,
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
    schedule: {
      enabled: false,
      days: [true, true, true, true, true, false, false],
      start_minutes: 540,
      end_minutes: 1080,
    },
    notifications: { timer: true, battery: false, schedule: false },
    hotkey: 'Ctrl+Alt+K',
    sunset_warning_mins: 5,
    overnight_guard_hours: 6,
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

describe('statusText with a schedule', () => {
  const start = new Date(2026, 0, 1, 9, 0).getTime();
  const end = new Date(2026, 0, 1, 18, 0).getTime();
  const open = {
    ...base.schedule,
    enabled: true,
    in_window: true,
    window_start_unix_ms: start,
    window_end_unix_ms: end,
    next_start_unix_ms: new Date(2026, 0, 2, 9, 0).getTime(),
  };
  const noon = new Date(2026, 0, 1, 12, 0).getTime();

  it('names the window while scheduled', () => {
    const s = { ...base, mode: 'scheduled' as const, ends_at_unix_ms: end, schedule: open };
    expect(statusText(s, noon, 'en-US')).toEqual({
      title: 'Awake',
      detail: 'Until 6:00 pm · on schedule',
    });
  });

  it('says until when a skip lasts', () => {
    const s = { ...base, schedule: { ...open, skipped: true } };
    expect(statusText(s, noon, 'en-US')).toEqual({
      title: 'Resting',
      detail: "Skipping today's schedule until 6:00 pm.",
    });
  });

  it('says when the sun rises again outside a window', () => {
    const s = {
      ...base,
      schedule: { ...open, in_window: false, window_start_unix_ms: null, window_end_unix_ms: null },
    };
    const evening = new Date(2026, 0, 1, 19, 0).getTime();
    expect(statusText(s, evening, 'en-US').detail).toBe(
      'Your laptop sleeps as usual. Awake again at 9:00 am.',
    );
  });

  it('pauses still win over the schedule', () => {
    const s = {
      ...base,
      mode: 'scheduled' as const,
      paused: 'on_battery' as const,
      schedule: open,
    };
    expect(statusText(s, noon, 'en-US').title).toBe('Paused');
  });
});

describe('formatWhen', () => {
  // 2 January 2026 is a Friday.
  const fridayEvening = new Date(2026, 0, 2, 19, 0).getTime();

  it('is just the clock for today and tomorrow', () => {
    expect(formatWhen(new Date(2026, 0, 2, 21, 0).getTime(), fridayEvening, 'en-US')).toBe(
      '9:00 pm',
    );
    expect(formatWhen(new Date(2026, 0, 3, 9, 0).getTime(), fridayEvening, 'en-US')).toBe(
      '9:00 am',
    );
  });

  it('names the day when it is further off', () => {
    expect(formatWhen(new Date(2026, 0, 5, 9, 0).getTime(), fridayEvening, 'en-US')).toBe(
      '9:00 am on Monday',
    );
  });
});

describe('toggleWhisper', () => {
  const end = new Date(2026, 0, 1, 18, 0).getTime();
  const open = { ...base.schedule, enabled: true, in_window: true, window_end_unix_ms: end };
  const scheduled = { ...base, mode: 'scheduled' as const, schedule: open };
  const skipped = { ...base, schedule: { ...open, skipped: true } };

  it('names the skip and the return', () => {
    expect(toggleWhisper(scheduled, skipped, 'en-US')).toBe('Skipped until 6:00 pm.');
    expect(toggleWhisper(skipped, scheduled, 'en-US')).toBe('Back on schedule.');
  });

  it('stays quiet for plain sessions', () => {
    expect(toggleWhisper(base, { ...base, mode: 'indefinite' }, 'en-US')).toBeNull();
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

describe('minutesToTime / timeToMinutes', () => {
  it('round-trips the schedule defaults and the day edges', () => {
    expect(minutesToTime(540)).toBe('09:00');
    expect(minutesToTime(1080)).toBe('18:00');
    expect(minutesToTime(0)).toBe('00:00');
    expect(minutesToTime(1439)).toBe('23:59');
    for (const m of [0, 75, 540, 1439]) expect(timeToMinutes(minutesToTime(m))).toBe(m);
  });

  it('clamps and rejects', () => {
    expect(minutesToTime(5000)).toBe('23:59');
    expect(timeToMinutes('')).toBeNull();
    expect(timeToMinutes('24:00')).toBeNull();
    expect(timeToMinutes('9:5')).toBeNull();
  });
});

describe('pauseText', () => {
  it('names the OS that refused the request', () => {
    expect(pauseText('power_request_failed', false)).toBe('Windows refused the request');
    expect(pauseText('power_request_failed', true)).toBe('macOS refused the request');
    expect(pauseText('on_battery', true)).toBe('running on battery');
  });
});

describe('statusText while the overnight guard asks', () => {
  const rests = new Date(2026, 0, 2, 3, 12).getTime();
  const asking = { ...base, mode: 'indefinite' as const, guard_rests_at_unix_ms: rests };

  it('asks in place of the usual line, awake or paused', () => {
    expect(statusText(asking, rests - 600_000, 'en-US')).toEqual({
      title: 'Awake',
      detail: 'Nobody here for 6 hours. Resting at 3:12 am.',
    });
    expect(statusText({ ...asking, paused: 'low_battery' }, rests, 'en-US')).toEqual({
      title: 'Paused',
      detail: 'Nobody here for 6 hours. Resting at 3:12 am.',
    });
  });

  it('goes back to the usual line once withdrawn', () => {
    const withdrawn = { ...asking, guard_rests_at_unix_ms: null };
    expect(statusText(withdrawn, rests, 'en-US').detail).toMatch(/^Until you say otherwise\./);
  });
});

describe('sunsetSoon', () => {
  const ends = new Date(2026, 0, 1, 17, 30).getTime();
  const timed = { ...base, mode: 'timed' as const, ends_at_unix_ms: ends, plan_secs: 600 };
  const at = (secsLeft: number) => ends - secsLeft * 1000;

  it('holds from the warning to the end, by the engine’s rule', () => {
    expect(sunsetSoon(timed, at(301))).toBe(false);
    expect(sunsetSoon(timed, at(300))).toBe(true);
    expect(sunsetSoon(timed, at(1))).toBe(true);
    expect(sunsetSoon(timed, at(0))).toBe(false);
  });

  it('never for a short plan, with the warning off, or without an end', () => {
    expect(sunsetSoon({ ...timed, plan_secs: 300 }, at(60))).toBe(false);
    const off = { ...timed, settings: { ...timed.settings, sunset_warning_mins: null } };
    expect(sunsetSoon(off, at(60))).toBe(false);
    expect(sunsetSoon({ ...base, mode: 'indefinite' }, at(60))).toBe(false);
    expect(sunsetSoon({ ...timed, mode: 'scheduled' }, at(60))).toBe(false);
  });

  it('never while an open window outlasts the timer', () => {
    const held = {
      ...timed,
      schedule: {
        ...timed.schedule,
        enabled: true,
        in_window: true,
        window_end_unix_ms: ends + 3_600_000,
      },
    };
    expect(sunsetSoon(held, at(60))).toBe(false);
    expect(sunsetSoon({ ...held, schedule: { ...held.schedule, skipped: true } }, at(60))).toBe(
      true,
    );
    const shortWindow = { ...held, schedule: { ...held.schedule, window_end_unix_ms: ends - 1 } };
    expect(sunsetSoon(shortWindow, at(60))).toBe(true);
  });

  it('turns the status line to the sunset', () => {
    expect(statusText(timed, at(299), 'en-US')).toEqual({
      title: 'Awake',
      detail: 'The sun sets in 5 minutes.',
    });
    expect(statusText(timed, at(400), 'en-US').detail).toBe('Until 5:30 pm · 6m left');
  });
});
