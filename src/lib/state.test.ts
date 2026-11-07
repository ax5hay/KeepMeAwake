import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Api } from './api';
import { AppStore } from './state.svelte';
import type { History, Snapshot } from './types';

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

function fakeApi(initial: Snapshot) {
  let current = initial;
  let push: ((s: Snapshot) => void) | undefined;
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
    extendSession: vi.fn(async () => current),
    getHistory: vi.fn(async () => logged),
    clearHistory: vi.fn(async () => ({ version: 1, entries: [] })),
    onSnapshot: vi.fn(async (handler) => {
      push = handler;
      return () => {
        push = undefined;
      };
    }),
  };
  return { api, push: (s: Snapshot) => push?.(s) };
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
});
