/**
 * Mirrors of the Rust types in `src-tauri/src/core`. Field names are
 * snake_case on purpose: they cross the IPC boundary untouched, so a mismatch
 * would be a silent bug. Keep these in step with the Rust definitions.
 */

/** `scheduled`: no manual session, but a schedule window is open and not skipped. */
export type Mode = 'off' | 'indefinite' | 'timed' | 'scheduled';

export type PauseReason = 'low_battery' | 'on_battery' | 'power_request_failed';

export type Theme = 'system' | 'light' | 'dark';

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
  settings: Settings;
  app_version: string;
}

export type PlanRequest =
  | { kind: 'off' }
  | { kind: 'indefinite' }
  | { kind: 'for'; seconds: number }
  | { kind: 'until'; unix_ms: number };

/** How an awake stretch ended. Mirrors `core::history::Ended`. */
export type Ended = 'user' | 'timer' | 'schedule' | 'battery' | 'sleep' | 'crash';

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
