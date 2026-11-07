import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import DayPills from './DayPills.svelte';

const weekdays = [true, true, true, true, true, false, false];

describe('DayPills', () => {
  it('shows seven pills, Monday first, pressed for days that are on', () => {
    render(DayPills, { days: weekdays, onchange: () => {} });
    const pills = screen.getAllByRole('button');
    expect(pills.map((p) => p.textContent?.trim()).join('')).toBe('MTWTFSS');
    expect(screen.getByRole('button', { name: 'Monday' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Sunday' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('reports the whole week with one day flipped', async () => {
    const onchange = vi.fn();
    render(DayPills, { days: weekdays, onchange });
    await fireEvent.click(screen.getByRole('button', { name: 'Saturday' }));
    expect(onchange).toHaveBeenCalledWith([true, true, true, true, true, true, false]);
    await fireEvent.click(screen.getByRole('button', { name: 'Monday' }));
    expect(onchange).toHaveBeenLastCalledWith([false, true, true, true, true, false, false]);
  });
});
