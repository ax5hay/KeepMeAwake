/**
 * Mirrors of the Rust types in `src-tauri/src/core`. Field names are
 * snake_case on purpose: they cross the IPC boundary untouched, so a mismatch
 * would be a silent bug. Keep these in step with the Rust definitions.
 */

/** `scheduled`: no manual session, but a schedule window is open and not skipped. */
export type Mode = 'off' | 'indefinite' | 'timed' | 'scheduled';

export type PauseReason = 'low_battery' | 'on_battery' | 'power_request_failed';

/**
 * The colours the window wears: the app's own three, then palettes to
 * choose. Each but `system` names a block of tokens in the stylesheet.
 */
export type Theme =
  | 'system'
  | 'light'
  | 'dark'
  | 'solarized_light'
  | 'solarized_dark'
  | 'gruvbox'
  | 'nord'
  | 'dracula'
  | 'catppuccin'
  | 'tokyo_night'
  | 'monokai'
  | 'terminal'
  | 'blueprint'
  | 'pocket'
  | 'newsprint'
  | 'risograph'
  | 'dial_up'
  | 'mars'
  | 'deep_sea'
  | 'vaporwave'
  | 'thermal'
  | 'negative'
  | 'miami'
  | 'bubblegum'
  | 'highlighter'
  | 'grape_soda'
  | 'hi_vis'
  | 'own';

/** A theme of one's own: five colours, each `#rrggbb`. */
export interface OwnTheme {
  paper: string;
  ink: string;
  sun: string;
  sky: string;
  ground: string;
}

/** Another theme from `from_minutes` until `until_minutes`, past midnight or not. */
export interface AfterDark {
  enabled: boolean;
  theme: Theme;
  from_minutes: number;
  until_minutes: number;
}

export type StartupPlan = 'off' | 'indefinite' | 'default_duration';

export interface Settings {
  version: number;
  keep_display_on: boolean;
  default_duration_secs: number | null;
  startup_plan: StartupPlan;
  pause_below_battery_percent: number | null;
  pause_on_battery: boolean;
  start_hidden: boolean;
  close_to_tray: boolean;
  free_memory_when_hidden: boolean;
  theme: Theme;
  schedule: Schedule;
  notifications: Notifications;
  /** Accelerator such as `Ctrl+Alt+K`; `null` turns the hotkey off. */
  hotkey: string | null;
  /** Minutes before a timed session ends to warn; `null` is off. */
  sunset_warning_mins: number | null;
  /** Hours without input after which an Always session asks, then rests; `null` is off. */
  overnight_guard_hours: number | null;
  almanac: AlmanacSettings;
  ai: AiSettings;
  /** The five colours of the `own` theme. */
  own_theme: OwnTheme;
  /** A second theme for the evening. */
  after_dark: AfterDark;
  /** A soft note when the sun rises and sets, while the window is open. */
  sounds: boolean;
  /** Ask GitHub once a day for a newer release. */
  check_for_updates: boolean;
  /** When it last asked, Unix ms; 0 for never. Written by the app, not the UI. */
  last_update_check_unix_ms: number;
}

/**
 * Claude's switches. `enabled` is the master and is off until someone turns
 * it on; each feature has its own. The API key is not a setting: it lives
 * in the OS credential store, and the window only learns whether there is one.
 */
export interface AiSettings {
  enabled: boolean;
  week_in_words: boolean;
  tell_the_sun: boolean;
  day_in_a_line: boolean;
  ask_the_almanac: boolean;
  /** Everything of AI's is out of sight, but for one button in Settings. */
  hidden: boolean;
  /** The service asked. */
  provider: Provider;
  /** The model asked; empty for the service's smallest. */
  model: string;
  /** For a service of one's own: its address, `https://…/v1`. */
  base_url: string;
}

/**
 * Which AI service is asked: Anthropic's Claude, OpenAI, or any other that
 * speaks the same chat format, at an address of one's own.
 */
export type Provider = 'anthropic' | 'openai' | 'custom';

/** Whether an API key is stored. Mirrors `app::ai::AiStatus`. */
export interface AiStatus {
  has_key: boolean;
}

/** A sentence, understood as a plan. Mirrors `core::ai::Understood`. */
export interface Understood {
  kind: 'for' | 'until' | 'indefinite' | 'off' | 'unclear';
  /** For `for`: minutes. Zero otherwise. */
  minutes: number;
  /** For `until`: local `HH:MM`. Empty otherwise. */
  time: string;
  /** The plan in a few plain words, or what was not understood. */
  say: string;
}

/**
 * What the almanac keeps and how it reads a day. The hours are when work is
 * meant to happen; `end_minutes` at or before `start_minutes` runs past
 * midnight, and equal ones mean the whole day.
 */
export interface AlmanacSettings {
  enabled: boolean;
  /** Monday first, seven entries. */
  work_days: boolean[];
  start_minutes: number;
  end_minutes: number;
  /** A past working day with nothing in it reads as leave. */
  empty_days_are_leave: boolean;
  /** Morning whisper once, on the first open after it. */
  whisper_night: boolean;
  /**
   * Minutes without input, inside the resting hours, after which an Always
   * session sets for the night and rises at the next touch; `null` is off.
   */
  rest_after_mins: number | null;
  /** Minutes without a five-minute break before a toast says so; `null` never. */
  break_after_mins: number | null;
}

/** A newer release found by the daily check. Mirrors `app::update::UpdateInfo`. */
export interface UpdateInfo {
  version: string;
  notes: string | null;
}

/** How far an update's download has got. Mirrors `app::update::Progress`. */
export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

/** One switch per kind of desktop toast. */
export interface Notifications {
  timer: boolean;
  battery: boolean;
  schedule: boolean;
}

/** One window of time on chosen weekdays; `end_minutes < start_minutes` runs overnight. */
export interface Schedule {
  enabled: boolean;
  /** Monday first, seven entries. */
  days: boolean[];
  start_minutes: number;
  end_minutes: number;
}

export interface GuardState {
  system: boolean;
  display: boolean;
}

export interface BatteryState {
  on_ac: boolean;
  percent: number | null;
}

export interface ScheduleState {
  enabled: boolean;
  in_window: boolean;
  skipped: boolean;
  window_start_unix_ms: number | null;
  window_end_unix_ms: number | null;
  next_start_unix_ms: number | null;
}

export interface Snapshot {
  mode: Mode;
  started_at_unix_ms: number | null;
  ends_at_unix_ms: number | null;
  remaining_secs: number | null;
  plan_secs: number | null;
  guards: GuardState;
  paused: PauseReason | null;
  battery: BatteryState;
  schedule: ScheduleState;
  /** While the overnight guard asks: when the session ends unless answered. */
  guard_rests_at_unix_ms: number | null;
  /** The question is the night's (Sleep when I'm away), not the guard's hours. */
  guard_night: boolean;
  /** The sun set with the night and rises again at the next touch. */
  rises_on_touch: boolean;
  /** The overnight guard watches this session: Always, guard on, not kept up. */
  guard_watching: boolean;
  settings: Settings;
  app_version: string;
}

export type PlanRequest =
  | { kind: 'off' }
  | { kind: 'indefinite' }
  | { kind: 'for'; seconds: number }
  | { kind: 'until'; unix_ms: number };

/** How an awake stretch ended. Mirrors `core::history::Ended`. */
export type Ended = 'user' | 'timer' | 'schedule' | 'battery' | 'idle' | 'sleep' | 'crash';

/** A stretch inside an entry during which the sun was up but not holding. */
export interface Pause {
  from_unix_ms: number;
  /** `null` if it never resumed. */
  to_unix_ms: number | null;
  reason: PauseReason;
}

/** One awake stretch. Mirrors `core::history::Entry`. */
export interface HistoryEntry {
  started_unix_ms: number;
  /** `null` while running, or when it never ended cleanly. */
  ended_unix_ms: number | null;
  /** The mode it started in. */
  mode: Mode;
  /** `null` while running. */
  ended: Ended | null;
  pauses: Pause[];
}

/** The session log, oldest entry first. Mirrors `core::history::History`. */
export interface History {
  version: number;
  entries: HistoryEntry[];
}

/** A stretch someone was at the keyboard, mouse or trackpad. Mirrors `core::almanac::Span`. */
export interface Span {
  from_unix_ms: number;
  to_unix_ms: number;
}

/** What someone said about a day: leave, or not a leave. Mirrors `core::almanac::Mark`. */
export type Mark = 'leave' | 'clear';

/** Presence and marked days. Mirrors `core::almanac::Almanac`. */
export interface Almanac {
  version: number;
  /** Oldest first, never overlapping. */
  spans: Span[];
  /** By local date, `2026-11-10`. */
  marks: Record<string, Mark>;
  /** Days corrected by hand: the one stretch that stands for what was read. */
  fixes: Record<string, Span>;
}
