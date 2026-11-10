import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import MonthGrid from './MonthGrid.svelte';
import { monthGrid, read, type Day } from '$lib/almanac';
import type { AlmanacSettings } from '$lib/types';

const settings: AlmanacSettings = {
  enabled: true,
  work_days: [true, true, true, true, true, false, false],
  start_minutes: 540,
  end_minutes: 1080,
  empty_days_are_leave: true,
  whisper_night: true,
  rest_after_mins: null,
};

const at = (date: number, hour: number) => new Date(2026, 10, date, hour).getTime();

const reading = read(
  {
    version: 1,
    spans: [{ from_unix_ms: at(9, 9), to_unix_ms: at(9, 17) }],
    marks: { '2026-11-20': 'leave' },
    fixes: {},
  },
  settings,
  at(10, 12),
);

const describeDay = (day: Day) => `day ${day.key}: ${day.kind}`;

describe('MonthGrid', () => {
  it('gives every date of the month a button, the one being read pressed', () => {
    render(MonthGrid, {
      rows: monthGrid(reading, 2026, 10),
      selected: '2026-11-10',
      describe: describeDay,
      onselect: () => {},
    });
    expect(screen.getAllByRole('button')).toHaveLength(30);
    expect(screen.getByRole('button', { name: 'day 2026-11-10: nothing' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByRole('button', { name: 'day 2026-11-09: worked' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });

  it('marks a worked day with a bar, leave with a ring and today with the sun', () => {
    const { container } = render(MonthGrid, {
      rows: monthGrid(reading, 2026, 10),
      selected: '2026-11-10',
      describe: describeDay,
      onselect: () => {},
    });
    expect(container.querySelectorAll('.bar')).toHaveLength(1);
    expect(container.querySelectorAll('.ring')).toHaveLength(1);
    expect(container.querySelectorAll('.day.today')).toHaveLength(1);
    const leave = screen.getByRole('button', { name: 'day 2026-11-20: leave' });
    expect(leave.querySelector('.ring')).not.toBeNull();
  });

  it('hands back the day that was clicked', async () => {
    const onselect = vi.fn();
    render(MonthGrid, {
      rows: monthGrid(reading, 2026, 10),
      selected: '2026-11-10',
      describe: describeDay,
      onselect,
    });
    await fireEvent.click(screen.getByRole('button', { name: 'day 2026-11-09: worked' }));
    expect(onselect).toHaveBeenCalledOnce();
    expect(onselect.mock.calls[0]?.[0].key).toBe('2026-11-09');
  });
});
