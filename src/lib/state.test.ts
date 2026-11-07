import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Api } from './api';
import { AppStore } from './state.svelte';
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
    schedule: {
      enabled: false,
      days: [true, true, true, true, true, false, false],
      start_minutes: 540,
      end_minutes: 1080,
    },
  },
  app_version: '0.1.0',
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
    updateSettings: vi.fn(async (settings) => {
      current = { ...current, settings };
      return current;
    }),
    quit: vi.fn(async () => undefined),
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
});
