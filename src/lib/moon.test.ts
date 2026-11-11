import { describe, expect, it } from 'vitest';

import { moonLine, moonLit, moonName, moonPath, moonPhase } from './moon';

describe('the moon', () => {
  it('is new at the moon it counts from, and full half a month on', () => {
    const start = Date.UTC(2000, 0, 6, 18, 14);
    expect(moonPhase(start)).toBeCloseTo(0, 5);
    expect(moonPhase(start + 14.765 * 86_400_000)).toBeCloseTo(0.5, 2);
  });

  it('agrees with the almanacs, to within a day', () => {
    // Full moons: 24 November 2026, 14:53 UTC; and 3 January 2026, 10:03 UTC.
    for (const full of [Date.UTC(2026, 10, 24, 14, 53), Date.UTC(2026, 0, 3, 10, 3)]) {
      expect(Math.abs(moonPhase(full) - 0.5)).toBeLessThan(0.035);
    }
    // New moon: 9 November 2026, 07:02 UTC.
    const fresh = moonPhase(Date.UTC(2026, 10, 9, 7, 2));
    expect(Math.min(fresh, 1 - fresh)).toBeLessThan(0.035);
  });

  it('names its phases and says how much is lit', () => {
    expect(moonName(0)).toBe('New moon');
    expect(moonName(0.12)).toBe('Waxing crescent');
    expect(moonName(0.25)).toBe('First quarter');
    expect(moonName(0.38)).toBe('Waxing gibbous');
    expect(moonName(0.5)).toBe('Full moon');
    expect(moonName(0.62)).toBe('Waning gibbous');
    expect(moonName(0.75)).toBe('Last quarter');
    expect(moonName(0.9)).toBe('Waning crescent');
    expect(moonLit(0.25)).toBeCloseTo(0.5, 5);
    expect(moonLine(0.38)).toBe('Waxing gibbous, 86% lit.');
  });

  it('draws the lit side: right while waxing, left while waning', () => {
    expect(moonPath(0.12, 10)).toMatch(/^M 0 -10 A 10 10 0 0 1 0 10 A [\d.]+ 10 0 0 0 0 -10 Z$/);
    expect(moonPath(0.38, 10)).toMatch(/A 10 10 0 0 1 0 10 A [\d.]+ 10 0 0 1 0 -10 Z$/);
    expect(moonPath(0.62, 10)).toMatch(/A 10 10 0 0 0 0 10 A [\d.]+ 10 0 0 0 0 -10 Z$/);
    expect(moonPath(0.9, 10)).toMatch(/A 10 10 0 0 0 0 10 A [\d.]+ 10 0 0 1 0 -10 Z$/);
    // At the quarter the terminator is a straight line.
    expect(moonPath(0.25, 10)).toContain('A 0 10');
  });
});
