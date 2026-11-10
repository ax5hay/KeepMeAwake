/**
 * Typed wrappers over the Tauri commands. This is the only file that imports
 * `invoke`, so the IPC surface is visible in one place.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  AiStatus,
  Almanac,
  History,
  Mark,
  PlanRequest,
  Settings,
  Snapshot,
  Span,
  Understood,
  UpdateInfo,
  UpdateProgress,
} from './types';

export const SNAPSHOT_EVENT = 'snapshot';
export const UPDATE_EVENT = 'update';
export const PROGRESS_EVENT = 'update-progress';

export const api = {
  getSnapshot: () => invoke<Snapshot>('get_snapshot'),
  setPlan: (request: PlanRequest) => invoke<Snapshot>('set_plan', { request }),
  toggle: () => invoke<Snapshot>('toggle_session'),
  extendSession: () => invoke<Snapshot>('extend_session'),
  keepItUp: () => invoke<Snapshot>('keep_it_up'),
  skipSchedule: () => invoke<Snapshot>('skip_schedule'),
  resumeSchedule: () => invoke<Snapshot>('resume_schedule'),
  updateSettings: (settings: Settings) => invoke<Snapshot>('update_settings', { settings }),
  quit: () => invoke('quit_app'),
  getHistory: () => invoke<History>('get_history'),
  clearHistory: () => invoke<History>('clear_history'),
  getAlmanac: () => invoke<Almanac>('get_almanac'),
  markDay: (day: string, mark: Mark | null) => invoke<Almanac>('mark_day', { day, mark }),
  fixDay: (day: string, stretch: Span | null) => invoke<Almanac>('fix_day', { day, stretch }),
  clearAlmanac: () => invoke<Almanac>('clear_almanac'),
  aiStatus: () => invoke<AiStatus>('ai_status'),
  setAiKey: (key: string | null) => invoke<AiStatus>('set_ai_key', { key }),
  aiReadWeek: (digest: string) => invoke<string>('ai_read_week', { digest }),
  aiPlan: (sentence: string, now: string) => invoke<Understood>('ai_plan', { sentence, now }),
  getUpdate: () => invoke<UpdateInfo | null>('get_update'),
  checkForUpdate: () => invoke<UpdateInfo | null>('check_for_update'),
  installUpdate: () => invoke('install_update'),
  onSnapshot: (handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>(SNAPSHOT_EVENT, (event) => handler(event.payload)),
  onUpdate: (handler: (info: UpdateInfo) => void): Promise<UnlistenFn> =>
    listen<UpdateInfo>(UPDATE_EVENT, (event) => handler(event.payload)),
  onUpdateProgress: (handler: (progress: UpdateProgress) => void): Promise<UnlistenFn> =>
    listen<UpdateProgress>(PROGRESS_EVENT, (event) => handler(event.payload)),
};

export type Api = typeof api;
