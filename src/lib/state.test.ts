import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Api } from './api';
import { AppStore } from './state.svelte';
import type { AiStatus, Almanac, History, Snapshot, UpdateInfo, UpdateProgress } from './types';

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
  guard_night: false,
  rises_on_touch: false,
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
    almanac: {
      enabled: true,
      work_days: [true, true, true, true, true, false, false],
      start_minutes: 540,
      end_minutes: 1080,
      empty_days_are_leave: true,
      whisper_night: true,
      rest_after_mins: null,
      break_after_mins: null,
    },
    ai: {
      enabled: false,
      week_in_words: true,
      tell_the_sun: true,
      day_in_a_line: true,
      ask_the_almanac: true,
      hidden: false,
      provider: 'anthropic',
      model: '',
      base_url: '',
    },
    own_theme: {
      paper: '#f4f0e7',
      ink: '#1c1b18',
      sun: '#dca544',
      sky: '#dfe8ea',
      ground: '#e8dfca',
    },
    after_dark: { enabled: false, theme: 'dark', from_minutes: 1140, until_minutes: 420 },
    sounds: false,
    check_for_updates: true,
    last_update_check_unix_ms: 0,
  },
  app_version: '0.1.0',
};

const logged: History = {
  version: 1,
  entries: [
    {
      started_unix_ms: 1,
      ended_unix_ms: 2,
      mode: 'timed',
      ended: 'timer',
      pauses: [],
    },
  ],
};

const blank: Almanac = { version: 1, spans: [], marks: {}, fixes: {} };

function fakeApi(initial: Snapshot, waiting: UpdateInfo | null = null, kept: Almanac = blank) {
  let current = initial;
  let almanac = kept;
  let key: AiStatus = { has_key: false };
  let push: ((s: Snapshot) => void) | undefined;
  let announce: ((u: UpdateInfo) => void) | undefined;
  let report: ((p: UpdateProgress) => void) | undefined;
  const api: Api = {
    getSnapshot: vi.fn(async () => current),
    setPlan: vi.fn(async (request) => {
      current =
        request.kind === 'off'
          ? { ...current, mode: 'off', ends_at_unix_ms: null, plan_secs: null }
          : request.kind === 'for'
            ? {
                ...current,
                mode: 'timed',
                ends_at_unix_ms: Date.now() + request.seconds * 1000,
                plan_secs: request.seconds,
              }
            : { ...current, mode: 'indefinite', ends_at_unix_ms: null, plan_secs: null };
      return current;
    }),
    toggle: vi.fn(async () => {
      current = { ...current, mode: current.mode === 'off' ? 'indefinite' : 'off' };
      return current;
    }),
    skipSchedule: vi.fn(async () => current),
    resumeSchedule: vi.fn(async () => current),
    updateSettings: vi.fn(async (settings) => {
      current = { ...current, settings };
      return current;
    }),
    quit: vi.fn(async () => undefined),
    setMini: vi.fn(async () => undefined),
    printWindow: vi.fn(async () => undefined),
    extendSession: vi.fn(async () => current),
    keepItUp: vi.fn(async () => {
      current = { ...current, guard_rests_at_unix_ms: null };
      return current;
    }),
    getHistory: vi.fn(async () => logged),
    clearHistory: vi.fn(async () => ({ version: 1, entries: [] })),
    getAlmanac: vi.fn(async () => almanac),
    markDay: vi.fn(async (day, mark) => {
      const others = Object.entries(almanac.marks).filter(([key]) => key !== day);
      const marks = Object.fromEntries(mark === null ? others : [...others, [day, mark]]);
      almanac = { ...almanac, marks };
      return almanac;
    }),
    fixDay: vi.fn(async (day, stretch) => {
      const others = Object.entries(almanac.fixes).filter(([key]) => key !== day);
      const fixes = Object.fromEntries(stretch === null ? others : [...others, [day, stretch]]);
      almanac = { ...almanac, fixes };
      return almanac;
    }),
    clearAlmanac: vi.fn(async () => {
      almanac = blank;
      return almanac;
    }),
    onSnapshot: vi.fn(async (handler) => {
      push = handler;
      return () => {
        push = undefined;
      };
    }),
    aiStatus: vi.fn(async () => key),
    setAiKey: vi.fn(async (value) => {
      key = { has_key: value !== null && value.trim() !== '' };
      return key;
    }),
    aiReadWeek: vi.fn(async () => 'A long Monday, and a short night after it.'),
    aiReadDay: vi.fn(async () => 'A long day that ran past midnight.'),
    aiAsk: vi.fn(async () => 'About 22 hours across three days.'),
    aiPlan: vi.fn(async () => ({
      kind: 'for' as const,
      minutes: 120,
      time: '',
      say: 'Awake for 2 hours',
    })),
    getUpdate: vi.fn(async () => waiting),
    checkForUpdate: vi.fn(async () => waiting),
    installUpdate: vi.fn(async () => {
      throw new Error('The update could not be installed: offline');
    }),
    onUpdate: vi.fn(async (handler) => {
      announce = handler;
      return () => (announce = undefined);
    }),
    onUpdateProgress: vi.fn(async (handler) => {
      report = handler;
      return () => (report = undefined);
    }),
  };
  return {
    api,
    push: (s: Snapshot) => push?.(s),
    announce: (u: UpdateInfo) => announce?.(u),
    report: (p: UpdateProgress) => report?.(p),
  };
}

describe('AppStore', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 1, 12, 0, 0));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('loads the first snapshot and subscribes', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    expect(store.ready).toBe(false);
    await store.init();
    expect(store.ready).toBe(true);
    expect(store.active).toBe(false);
    expect(store.status.title).toBe('Resting');
    expect(api.onSnapshot).toHaveBeenCalledOnce();
    store.dispose();
  });

  it('accepts pushed snapshots from the engine', async () => {
    const { api, push } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    push({ ...base, mode: 'indefinite' });
    expect(store.active).toBe(true);
    expect(store.status.title).toBe('Awake');
    store.dispose();
  });

  it('ticks once a second only while a timed session is visible', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    await store.setPlan({ kind: 'for', seconds: 120 });
    expect(store.remaining).toBe(120);
    vi.advanceTimersByTime(30_000);
    expect(store.remaining).toBe(90);
    expect(store.progress).toBeCloseTo(0.25);

    await store.setPlan({ kind: 'off' });
    expect(store.remaining).toBeNull();
    expect(vi.getTimerCount()).toBe(0);
    store.dispose();
  });

  it('merges partial settings over the current ones', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    await store.updateSettings({ keep_display_on: false });
    expect(api.updateSettings).toHaveBeenCalledWith({ ...base.settings, keep_display_on: false });
    expect(store.settings?.keep_display_on).toBe(false);
    store.dispose();
  });

  it('whispers when the sun skips or resumes the schedule', async () => {
    const end = new Date(2026, 0, 1, 18, 0).getTime();
    const window = {
      ...base.schedule,
      enabled: true,
      in_window: true,
      window_start_unix_ms: new Date(2026, 0, 1, 9, 0).getTime(),
      window_end_unix_ms: end,
    };
    const scheduled: Snapshot = {
      ...base,
      mode: 'scheduled',
      ends_at_unix_ms: end,
      schedule: window,
    };
    const skipped: Snapshot = { ...base, schedule: { ...window, skipped: true } };
    const { api } = fakeApi(scheduled);
    let next = skipped;
    api.toggle = vi.fn(async () => {
      const out = next;
      next = next === skipped ? scheduled : skipped;
      return out;
    });
    const store = new AppStore(api);
    await store.init();
    expect(store.status.detail).toMatch(/^Until 6:00\s?pm · on schedule$/i);
    await store.toggle();
    expect(store.whisper).toMatch(/^Skipped until 6:00\s?pm\.$/i);
    expect(store.status.detail).toMatch(/^Skipping today's schedule until 6:00\s?pm\.$/i);
    await store.toggle();
    expect(store.whisper).toBe('Back on schedule.');
    store.dispose();
  });

  it('ticks while a schedule window is on screen', async () => {
    const end = Date.now() + 3600_000;
    const { api } = fakeApi({
      ...base,
      mode: 'scheduled',
      ends_at_unix_ms: end,
      plan_secs: 7200,
      schedule: { ...base.schedule, enabled: true, in_window: true, window_end_unix_ms: end },
    });
    const store = new AppStore(api);
    await store.init();
    expect(store.progress).toBeCloseTo(0.5);
    vi.advanceTimersByTime(36 * 60_000);
    expect(store.progress).toBeCloseTo(0.8);
    store.dispose();
  });

  it('surfaces backend errors instead of swallowing them', async () => {
    const { api } = fakeApi(base);
    api.toggle = vi.fn(async () => {
      throw new Error('nope');
    });
    const store = new AppStore(api);
    await store.init();
    await store.toggle();
    expect(store.error).toBe('nope');
    store.dispose();
  });

  it('loads history only when asked, and clears it', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(api.getHistory).not.toHaveBeenCalled();
    expect(store.history).toBeNull();
    await store.loadHistory();
    expect(store.history?.entries).toHaveLength(1);
    await store.clearHistory();
    expect(store.history?.entries).toEqual([]);
    store.dispose();
  });

  it('gives a timed session another 30 minutes through the backend', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    await store.extend();
    expect(api.extendSession).toHaveBeenCalledOnce();
    store.dispose();
  });

  it('hears of an update, whispers about it once per version', async () => {
    localStorage.clear();
    const { api, announce } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(store.update).toBeNull();
    announce({ version: '0.3.1', notes: null });
    expect(store.update?.version).toBe('0.3.1');
    expect(store.whisper).toBe('A new version is up: 0.3.1.');
    store.dispose();

    // The next window, same version: no second whisper.
    const again = new AppStore(fakeApi(base, { version: '0.3.1', notes: null }).api);
    await again.init();
    expect(again.update?.version).toBe('0.3.1');
    expect(again.whisper).toBeNull();
    again.dispose();
  });

  it('shows download progress, and says why an install failed', async () => {
    const { api, report } = fakeApi(base, { version: '0.3.1', notes: null });
    const store = new AppStore(api);
    await store.init();
    const installing = store.installUpdate();
    expect(store.installing).toBe(-1);
    report({ downloaded: 42, total: 100 });
    expect(store.installing).toBe(42);
    await installing;
    expect(store.installing).toBeNull();
    expect(store.error).toBe('The update could not be installed: offline');
    store.dispose();
  });

  it('opens Settings at a section, or where it was left', () => {
    const store = new AppStore(fakeApi(base).api);
    store.openSettings('history');
    expect(store.view).toBe('settings');
    expect(store.section).toBe('history');
    store.view = 'home';
    store.openSettings();
    expect(store.section).toBeNull();
  });

  it('answers the overnight guard and stops asking', async () => {
    const asking: Snapshot = {
      ...base,
      mode: 'indefinite',
      guard_rests_at_unix_ms: Date.now() + 600_000,
    };
    const { api } = fakeApi(asking);
    const store = new AppStore(api);
    await store.init();
    expect(store.guardAsking).toBe(true);
    expect(store.status.detail).toMatch(/^No input for 6h\./);
    await store.keepItUp();
    expect(api.keepItUp).toHaveBeenCalledOnce();
    expect(store.guardAsking).toBe(false);
    store.dispose();
  });
});

describe('AppStore and a check by hand', () => {
  it('says it is the newest when nothing is found, and shows what is', async () => {
    const none = new AppStore(fakeApi(base).api);
    await none.init();
    await none.checkForUpdate();
    expect(none.checked).toBe('newest');
    expect(none.update).toBeNull();
    none.dispose();

    const { api } = fakeApi(base);
    api.checkForUpdate = vi.fn(async () => ({ version: '0.4.1', notes: null }));
    const found = new AppStore(api);
    await found.init();
    await found.checkForUpdate();
    expect(found.update?.version).toBe('0.4.1');
    expect(found.checked).toBe('idle');
    found.dispose();
  });
});

describe('AppStore and AI', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 0, 1, 12, 0, 0));
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts locked, then off with a key, then on by choice', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(store.gate('week_in_words')).toBe('locked');

    expect(await store.setAiKey('sk-ant-test')).toBe(true);
    expect(store.gate('week_in_words')).toBe('off');
    expect(api.setAiKey).toHaveBeenCalledWith('sk-ant-test');

    await store.setAiEnabled(true);
    expect(store.gate('week_in_words')).toBe('on');
    expect(store.gate('tell_the_sun')).toBe('on');

    await store.setAiFeature('tell_the_sun', false);
    expect(store.gate('tell_the_sun')).toBe('feature-off');
    expect(store.gate('week_in_words')).toBe('on');

    await store.setAiKey(null);
    expect(store.gate('week_in_words')).toBe('locked');
    store.dispose();
  });

  it('never holds the key: only whether there is one', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    await store.setAiKey('sk-ant-secret');
    expect(JSON.stringify(store.ai)).toBe('{"has_key":true}');
    expect(JSON.stringify(store.snapshot)).not.toContain('sk-ant-secret');
    store.dispose();
  });

  it('asks only when asked, and says why when it cannot', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(api.aiReadWeek).not.toHaveBeenCalled();
    expect(api.aiPlan).not.toHaveBeenCalled();

    expect(await store.readWeek('Mon 9 Nov: 8h')).toBe(
      'A long Monday, and a short night after it.',
    );
    expect((await store.askPlan('two hours', 'now'))?.say).toBe('Awake for 2 hours');

    api.aiReadWeek = vi.fn(async () => {
      throw new Error('AI is switched off.');
    });
    // A different week: the one above is remembered, and is not asked again.
    expect(await store.readWeek('Mon 16 Nov: 8h')).toBeNull();
    expect(store.error).toBe('AI is switched off.');
    store.dispose();
  });
});

describe('AppStore and the AI service', () => {
  it('starts a new service from its smallest model and asks afresh for its key', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    await store.setAiKey('sk-ant-test');
    await store.setAiModel('  claude-opus-5-5 ');
    expect(store.settings?.ai.model).toBe('claude-opus-5-5');
    expect(api.aiStatus).toHaveBeenCalledTimes(1);

    await store.setAiProvider('openai');
    expect(store.settings?.ai.provider).toBe('openai');
    expect(store.settings?.ai.model).toBe('');
    expect(api.aiStatus).toHaveBeenCalledTimes(2);

    await store.setAiProvider('openai');
    expect(api.aiStatus).toHaveBeenCalledTimes(2);
    await store.setAiAddress(' https://example.test/v1 ');
    expect(store.settings?.ai.base_url).toBe('https://example.test/v1');
    store.dispose();
  });
});

describe('AppStore, AI and cost', () => {
  it('answers the same question about the same days from memory, not from the API', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(await store.readDay('Mon 9 Nov: 8h')).toBe('A long day that ran past midnight.');
    expect(await store.readDay('Mon 9 Nov: 8h')).toBe('A long day that ran past midnight.');
    expect(api.aiReadDay).toHaveBeenCalledTimes(1);
    await store.readDay('Tue 10 Nov: 7h');
    expect(api.aiReadDay).toHaveBeenCalledTimes(2);

    await store.askAlmanac('How many hours?', 'Mon 9 Nov: 8h');
    await store.askAlmanac('How many hours?', 'Mon 9 Nov: 8h');
    expect(api.aiAsk).toHaveBeenCalledTimes(1);
    await store.readWeek('Mon 9 Nov: 8h');
    await store.readWeek('Mon 9 Nov: 8h');
    expect(api.aiReadWeek).toHaveBeenCalledTimes(1);
    store.dispose();
  });

  it('does not remember a failure, so it can be asked again', async () => {
    const { api } = fakeApi(base);
    let calls = 0;
    api.aiReadDay = vi.fn(async () => {
      calls += 1;
      if (calls === 1) throw new Error('Claude is busy just now. Try again in a minute.');
      return 'A quiet day.';
    });
    const store = new AppStore(api);
    await store.init();
    expect(await store.readDay('Mon: 8h')).toBeNull();
    expect(await store.readDay('Mon: 8h')).toBe('A quiet day.');
    store.dispose();
  });

  it('hides everything of Claude with one setting, and brings it back with one', async () => {
    const { api } = fakeApi(base);
    const store = new AppStore(api);
    await store.init();
    expect(store.aiState).toBe('locked');
    await store.setAiHidden(true);
    expect(store.aiState).toBe('hidden');
    expect(store.gate('tell_the_sun')).toBe('hidden');
    await store.setAiHidden(false);
    expect(store.aiState).toBe('locked');
    store.dispose();
  });
});

describe('AppStore and the almanac', () => {
  const morning = new Date(2026, 10, 10, 9, 30).getTime();
  /** Stopped at 11:48 pm, back at 7:02 am. */
  const slept: Almanac = {
    version: 1,
    spans: [
      {
        from_unix_ms: new Date(2026, 10, 9, 9, 0).getTime(),
        to_unix_ms: new Date(2026, 10, 9, 23, 48).getTime(),
      },
      {
        from_unix_ms: new Date(2026, 10, 10, 7, 2).getTime(),
        to_unix_ms: morning,
      },
    ],
    marks: {},
    fixes: {},
  };

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(morning);
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('whispers the night once, on the first open after it', async () => {
    const first = new AppStore(fakeApi(base, null, slept).api);
    await first.init();
    expect(first.whisper).toBe('Last night you were away from 11:48 pm to 7:02 am.');
    first.dispose();

    const again = new AppStore(fakeApi(base, null, slept).api);
    await again.init();
    expect(again.whisper).toBeNull();
    again.dispose();
  });

  it('says nothing without a night, and does not read the almanac while it is off', async () => {
    const quiet = new AppStore(fakeApi(base).api);
    await quiet.init();
    expect(quiet.whisper).toBeNull();
    quiet.dispose();

    const off = { ...base, settings: { ...base.settings } };
    off.settings.almanac = { ...base.settings.almanac, enabled: false };
    const { api } = fakeApi(off, null, slept);
    const store = new AppStore(api);
    await store.init();
    expect(api.getAlmanac).not.toHaveBeenCalled();
    expect(store.whisper).toBeNull();
    store.dispose();
  });

  it('keeps the night to itself with the whisper switched off', async () => {
    const hushed = { ...base, settings: { ...base.settings } };
    hushed.settings.almanac = { ...base.settings.almanac, whisper_night: false };
    const { api } = fakeApi(hushed, null, slept);
    const store = new AppStore(api);
    await store.init();
    expect(store.whisper).toBeNull();
    expect(store.almanac).toEqual(slept);
    store.dispose();
  });

  it('keeps today fresh from the engine, no more than once a minute', async () => {
    const { api, push } = fakeApi(base, null, slept);
    const store = new AppStore(api);
    await store.init();
    expect(store.reading?.today.key).toBe('2026-11-10');
    expect(api.getAlmanac).toHaveBeenCalledTimes(1);
    push({ ...base, mode: 'indefinite' });
    expect(api.getAlmanac).toHaveBeenCalledTimes(1);
    vi.setSystemTime(morning + 60_000);
    push({ ...base, mode: 'indefinite' });
    expect(api.getAlmanac).toHaveBeenCalledTimes(2);
    store.dispose();
  });

  it('calls a day full once, when it reaches the length of its working hours', async () => {
    const long: Almanac = {
      ...slept,
      spans: [
        {
          from_unix_ms: new Date(2026, 10, 10, 9, 0).getTime(),
          to_unix_ms: new Date(2026, 10, 10, 18, 2).getTime(),
        },
      ],
    };
    vi.setSystemTime(new Date(2026, 10, 10, 18, 2));
    const { api } = fakeApi(base, null, long);
    const store = new AppStore(api);
    await store.init();
    expect(store.whisper).toBeNull();
    await store.loadAlmanac();
    expect(store.whisper).toBe('A full day: 9h 02m.');
    store.whisper = null;
    await store.loadAlmanac();
    expect(store.whisper).toBeNull();
    store.dispose();
  });

  it('opens the Almanac, marks a day, takes the mark away and clears', async () => {
    const { api } = fakeApi(base, null, slept);
    const store = new AppStore(api);
    await store.init();
    store.openAlmanac();
    expect(store.view).toBe('almanac');
    await vi.waitFor(() => expect(api.getAlmanac).toHaveBeenCalledTimes(2));

    await store.markDay('2026-11-12', 'leave');
    expect(store.almanac?.marks).toEqual({ '2026-11-12': 'leave' });
    await store.markDay('2026-11-12', null);
    expect(store.almanac?.marks).toEqual({});

    const stretch = { from_unix_ms: 1, to_unix_ms: 2 };
    await store.fixDay('2026-11-12', stretch);
    expect(store.almanac?.fixes).toEqual({ '2026-11-12': stretch });
    await store.fixDay('2026-11-12', null);
    expect(store.almanac?.fixes).toEqual({});

    await store.clearAlmanac();
    expect(store.almanac?.spans).toEqual([]);
    store.dispose();
  });

  it('says why when a day cannot be marked', async () => {
    const { api } = fakeApi(base);
    api.markDay = vi.fn(async () => {
      throw new Error('someday is not a day');
    });
    const store = new AppStore(api);
    await store.init();
    await store.markDay('someday', 'leave');
    expect(store.error).toBe('someday is not a day');
    store.dispose();
  });
});
