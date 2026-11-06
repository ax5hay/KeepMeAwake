/**
 * Typed wrappers over the Tauri commands. This is the only file that imports
 * `invoke`, so the IPC surface is visible in one place.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { PlanRequest, Settings, Snapshot } from './types';

export const SNAPSHOT_EVENT = 'snapshot';

export const api = {
  getSnapshot: () => invoke<Snapshot>('get_snapshot'),
  setPlan: (request: PlanRequest) => invoke<Snapshot>('set_plan', { request }),
  toggle: () => invoke<Snapshot>('toggle_session'),
  updateSettings: (settings: Settings) => invoke<Snapshot>('update_settings', { settings }),
  quit: () => invoke('quit_app'),
  onSnapshot: (handler: (snapshot: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>(SNAPSHOT_EVENT, (event) => handler(event.payload)),
};

export type Api = typeof api;
