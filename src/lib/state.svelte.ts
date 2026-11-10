/**
 * The one reactive store. Holds the latest snapshot from the engine, a
 * once-per-second clock that only runs while a countdown is visible, the
 * current view, the almanac while its pane is open, and the "whisper": a line of text that briefly replaces the
 * status detail (tips, easter eggs). Everything derived is computed from pure
 * helpers in format.ts.
 */
import { read } from './almanac';
import { api, type Api } from './api';
import { quietNight, updateWhisper, type SettingsSection } from './copy';
import {
  downloadPercent,
  formatClock,
  progress,
  remainingSecs,
  statusText,
  sunsetSoon,
  toggleWhisper,
} from './format';
import type {
  Almanac,
  History,
  Mark,
  PlanRequest,
  Settings,
  Snapshot,
  Span,
  UpdateInfo,
} from './types';

/** Where the version last whispered about is remembered, per webview. */
const WHISPERED_KEY = 'keepmeawake.update-whispered';

/** Where the night last whispered about is remembered: the moment it ended. */
const NIGHT_KEY = 'keepmeawake.night-whispered';

function remembered(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function remember(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage off: the whisper may come again next time. Harmless.
  }
}

export type View = 'home' | 'settings' | 'almanac';

/** How long the morning's whisper stays: it has two times in it to read. */
export const NIGHT_WHISPER_MS = 7000;

/** How long a whisper stays on screen. */
export const WHISPER_MS = 4200;

export class AppStore {
  snapshot = $state<Snapshot | null>(null);
  now = $state(Date.now());
  view = $state<View>('home');
  error = $state<string | null>(null);
  whisper = $state<string | null>(null);
  /** The session log; loaded when Settings opens, not at launch. */
  history = $state<History | null>(null);
  /** The almanac; loaded when its pane opens and once at launch for the night. */
  almanac = $state<Almanac | null>(null);
  /** When the almanac was last read from the app, for what "now" means in it. */
  almanacAt = $state(Date.now());
  /** The Settings section to bring into view as Settings opens, if any. */
  section = $state<SettingsSection | null>(null);
  /** A newer release, as the daily check found it. */
  update = $state<UpdateInfo | null>(null);
  /** While installing: `null` until it starts, then the percent (or -1 while the size is unknown). */
  installing = $state<number | null>(null);

  #api: Api;
  #ticker: ReturnType<typeof setInterval> | undefined;
  #whisperTimer: ReturnType<typeof setTimeout> | undefined;
  #unlisten: Array<() => void> = [];

  constructor(backend: Api = api) {
    this.#api = backend;
  }

  /** Loads the first snapshot and subscribes to engine pushes. */
  async init(): Promise<void> {
    this.#unlisten = await Promise.all([
      this.#api.onSnapshot((s) => this.#accept(s)),
      this.#api.onUpdate((info) => this.#foundUpdate(info)),
      this.#api.onUpdateProgress((p) => (this.installing = downloadPercent(p) ?? -1)),
    ]);
    this.#accept(await this.#api.getSnapshot());
    document.addEventListener('visibilitychange', this.#syncTicker);
    try {
      const info = await this.#api.getUpdate();
      if (info) this.#foundUpdate(info);
    } catch {
      // No update news is fine; the window works the same.
    }
    await this.#greet();
  }

  /**
   * The morning's whisper: the night that led into today, said once. The
   * almanac is read for it only while it is being kept.
   */
  async #greet(): Promise<void> {
    const settings = this.settings;
    if (!settings?.almanac.enabled) return;
    try {
      this.almanac = await this.#api.getAlmanac();
      this.almanacAt = Date.now();
    } catch {
      return;
    }
    const night = read(this.almanac, settings.almanac, this.almanacAt).today.night;
    if (night === null || remembered(NIGHT_KEY) === String(night.to)) return;
    remember(NIGHT_KEY, String(night.to));
    this.say(quietNight(formatClock(night.from), formatClock(night.to)), NIGHT_WHISPER_MS);
  }

  /** Downloads, installs and relaunches; on failure says why and stays. */
  async installUpdate(): Promise<void> {
    if (this.installing !== null) return;
    this.installing = -1;
    try {
      await this.#api.installUpdate();
    } catch (err) {
      this.installing = null;
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  #foundUpdate(info: UpdateInfo): void {
    this.update = info;
    if (remembered(WHISPERED_KEY) !== info.version) {
      remember(WHISPERED_KEY, info.version);
      this.say(updateWhisper(info.version));
    }
  }

  dispose(): void {
    for (const stop of this.#unlisten) stop();
    this.#unlisten = [];
    document.removeEventListener('visibilitychange', this.#syncTicker);
    this.#stopTicker();
    if (this.#whisperTimer !== undefined) clearTimeout(this.#whisperTimer);
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

  /** A timed session is inside its sunset warning. */
  get sunsetSoon(): boolean {
    return this.snapshot ? sunsetSoon(this.snapshot, this.now) : false;
  }

  get settings(): Settings | null {
    return this.snapshot?.settings ?? null;
  }

  /** Opens Settings, at `section` if given, else where it was left. */
  openSettings(section: SettingsSection | null = null): void {
    this.section = section;
    this.view = 'settings';
  }

  /** Opens the Almanac and reads it afresh. */
  openAlmanac(): void {
    this.view = 'almanac';
    void this.loadAlmanac();
  }

  /** Fetches the almanac; the app takes one fresh look first. */
  async loadAlmanac(): Promise<void> {
    try {
      this.almanac = await this.#api.getAlmanac();
      this.almanacAt = Date.now();
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Marks a day as leave or as not a leave; `null` takes the mark away. */
  async markDay(day: string, mark: Mark | null): Promise<void> {
    try {
      this.almanac = await this.#api.markDay(day, mark);
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Corrects a day to one stretch by hand; `null` goes back to what was read. */
  async fixDay(day: string, stretch: Span | null): Promise<void> {
    try {
      this.almanac = await this.#api.fixDay(day, stretch);
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Empties the almanac. It starts again from now. */
  async clearAlmanac(): Promise<void> {
    try {
      this.almanac = await this.#api.clearAlmanac();
      this.almanacAt = Date.now();
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** The overnight guard is asking whether anyone is there. */
  get guardAsking(): boolean {
    return (this.snapshot?.guard_rests_at_unix_ms ?? null) !== null;
  }

  /** Answers the overnight guard: someone is here. */
  async keepItUp(): Promise<void> {
    await this.#call(() => this.#api.keepItUp());
  }

  /** Gives a timed session another 30 minutes. */
  async extend(): Promise<void> {
    try {
      this.#accept(await this.#api.extendSession());
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Fetches the session log. */
  async loadHistory(): Promise<void> {
    try {
      this.history = await this.#api.getHistory();
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Empties the session log. No confirmation: it is a log, not data. */
  async clearHistory(): Promise<void> {
    try {
      this.history = await this.#api.clearHistory();
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Shows `text` under the title for a few seconds, then the detail returns. */
  say(text: string, ms: number = WHISPER_MS): void {
    this.whisper = text;
    if (this.#whisperTimer !== undefined) clearTimeout(this.#whisperTimer);
    this.#whisperTimer = setTimeout(() => {
      this.whisper = null;
      this.#whisperTimer = undefined;
    }, ms);
  }

  async setPlan(request: PlanRequest): Promise<void> {
    await this.#call(() => this.#api.setPlan(request));
  }

  /** What the sun does; the engine decides, the whisper says what it chose. */
  async toggle(): Promise<void> {
    const before = this.snapshot;
    await this.#call(() => this.#api.toggle());
    const after = this.snapshot;
    const whisper = before && after ? toggleWhisper(before, after) : null;
    if (whisper) this.say(whisper);
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
    const mode = this.snapshot?.mode;
    const counting = mode === 'timed' || mode === 'scheduled';
    const needsClock = counting && document.visibilityState === 'visible';
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
