import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import DayStrip from './DayStrip.svelte';
import type { Strip } from '$lib/almanac';

const strip: Strip = {
  runs: [
    { left: 0.25, width: 0.1 },
    { left: 0.4, width: 0.2 },
  ],
  quiet: [{ left: 0, width: 0.25 }],
  hours: { left: 0.33, width: 0.375 },
  sun: 0.6,
  ticks: [0, 0.25, 0.5, 0.75].map((at) => ({ at, unix_ms: 0 })),
};

describe('DayStrip', () => {
  it('is one image that says what it draws', () => {
    render(DayStrip, { strip, label: 'Tue 10 Nov: 6h 42m' });
    expect(screen.getByRole('img', { name: 'Tue 10 Nov: 6h 42m' })).toBeInTheDocument();
  });

  it('lays ink, the night and the sun along the day, and names its hours', () => {
    const { container } = render(DayStrip, {
      strip,
      label: 'a day',
      ticks: ['7 am', '1 pm', '7 pm', '1 am'],
    });
    expect(container.querySelectorAll('.run')).toHaveLength(2);
    expect(container.querySelectorAll('.quiet')).toHaveLength(1);
    expect(container.querySelector('.sun')).toHaveStyle({ left: '60.000%' });
    expect(container.querySelector('.hours')).toHaveStyle({ left: '33.000%', width: '37.500%' });
    expect(screen.getByText('1 pm')).toBeInTheDocument();
  });

  it('has no sun on a day that is not today, and no hours named in a row of the week', () => {
    const { container } = render(DayStrip, {
      strip: { ...strip, sun: null },
      size: 'row',
      label: 'a day',
      ticks: ['7 am', '1 pm', '7 pm', '1 am'],
    });
    expect(container.querySelector('.sun')).toBeNull();
    expect(screen.queryByText('1 pm')).toBeNull();
  });
});
