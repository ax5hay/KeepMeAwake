/**
 * The one reactive store. Holds the latest snapshot from the engine, a
 * once-per-second clock that only runs while a countdown is visible, and the
 * current view. Everything derived is computed from pure helpers in format.ts.
 */
import { api, type Api } from './api';
import { progress, remainingSecs, statusText } from './format';
import type { PlanRequest, Settings, Snapshot } from './types';

export type View = 'home' | 'settings';

export class AppStore {
  snapshot = $state<Snapshot | null>(null);
  now = $state(Date.now());
  view = $state<View>('home');
  error = $state<string | null>(null);

  #api: Api;
  #ticker: ReturnType<typeof setInterval> | undefined;
  #unlisten: (() => void) | undefined;

  constructor(backend: Api = api) {
    this.#api = backend;
  }

  /** Loads the first snapshot and subscribes to engine pushes. */
  async init(): Promise<void> {
    this.#unlisten = await this.#api.onSnapshot((s) => this.#accept(s));
    this.#accept(await this.#api.getSnapshot());
    document.addEventListener('visibilitychange', this.#syncTicker);
  }

  dispose(): void {
    this.#unlisten?.();
    document.removeEventListener('visibilitychange', this.#syncTicker);
    this.#stopTicker();
  }

  get ready(): boolean {
    return this.snapshot !== null;
  }

  get active(): boolean {
    return this.snapshot?.mode !== 'off';
  }

  get remaining(): number | null {
    return this.snapshot ? remainingSecs(this.snapshot, this.now) : null;
  }

  get progress(): number {
    return this.snapshot ? progress(this.snapshot, this.now) : 0;
  }

  get status() {
    return this.snapshot ? statusText(this.snapshot, this.now) : { title: '', detail: '' };
  }

  get settings(): Settings | null {
    return this.snapshot?.settings ?? null;
  }

  async setPlan(request: PlanRequest): Promise<void> {
    await this.#call(() => this.#api.setPlan(request));
  }

  async toggle(): Promise<void> {
    await this.#call(() => this.#api.toggle());
  }

  async updateSettings(patch: Partial<Settings>): Promise<void> {
    const current = this.snapshot?.settings;
    if (!current) return;
    await this.#call(() => this.#api.updateSettings({ ...current, ...patch }));
  }

  async quit(): Promise<void> {
    await this.#api.quit();
  }

  async #call(fn: () => Promise<Snapshot>): Promise<void> {
    try {
      this.#accept(await fn());
      this.error = null;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  #accept(snapshot: Snapshot): void {
    this.snapshot = snapshot;
    this.now = Date.now();
    this.#syncTicker();
  }

  #syncTicker = (): void => {
    const needsClock = this.snapshot?.mode === 'timed' && document.visibilityState === 'visible';
    if (needsClock && this.#ticker === undefined) {
      this.#ticker = setInterval(() => (this.now = Date.now()), 1000);
    } else if (!needsClock) {
      this.#stopTicker();
    }
  };

  #stopTicker(): void {
    if (this.#ticker !== undefined) {
      clearInterval(this.#ticker);
      this.#ticker = undefined;
    }
  }
}

export const store = new AppStore();
