import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import History from './History.svelte';
import type { History as Log } from '$lib/types';

const NOW = new Date(2026, 10, 9, 15, 0).getTime();

describe('History', () => {
  it('says so when nothing has been logged, with nothing to clear', () => {
    render(History, { history: { version: 1, entries: [] }, now: NOW, onclear: () => {} });
    expect(
      screen.getByText('Nothing yet. The first sunset will be written here.'),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Clear history' })).toBeNull();
  });

  it('lists entries with their pauses and clears on request', async () => {
    const log: Log = {
      version: 1,
      entries: [
        {
          started_unix_ms: new Date(2026, 10, 9, 9, 2).getTime(),
          ended_unix_ms: new Date(2026, 10, 9, 11, 30).getTime(),
          mode: 'timed',
          ended: 'timer',
          pauses: [
            {
              from_unix_ms: new Date(2026, 10, 9, 10, 0).getTime(),
              to_unix_ms: new Date(2026, 10, 9, 10, 20).getTime(),
              reason: 'low_battery',
            },
          ],
        },
      ],
    };
    const onclear = vi.fn();
    render(History, { history: log, now: NOW, onclear });
    expect(screen.getByText('Today')).toBeInTheDocument();
    expect(screen.getByText('2h 28m')).toBeInTheDocument();
    expect(screen.getByText('timer')).toBeInTheDocument();
    expect(screen.getByText(/^paused .* · battery$/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Clear history' }));
    expect(onclear).toHaveBeenCalledOnce();
  });

  it('shows nothing until the log has loaded', () => {
    const { container } = render(History, { history: null, now: NOW, onclear: () => {} });
    expect(container.textContent?.trim()).toBe('');
  });
});
