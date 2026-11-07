/**
 * Mirrors of the Rust types in `src-tauri/src/core`. Field names are
 * snake_case on purpose: they cross the IPC boundary untouched, so a mismatch
 * would be a silent bug. Keep these in step with the Rust definitions.
 */

export type Mode = 'off' | 'indefinite' | 'timed';

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

export interface Snapshot {
  mode: Mode;
  started_at_unix_ms: number | null;
  ends_at_unix_ms: number | null;
  remaining_secs: number | null;
  plan_secs: number | null;
  guards: GuardState;
  paused: PauseReason | null;
  battery: BatteryState;
  settings: Settings;
  app_version: string;
}

export type PlanRequest =
  | { kind: 'off' }
  | { kind: 'indefinite' }
  | { kind: 'for'; seconds: number }
  | { kind: 'until'; unix_ms: number };
