import { describe, expect, it } from 'vitest';

import {
  AUTHOR,
  HORIZON_TIPS,
  STAR_WHISPERS,
  horizonTip,
  isLate,
  longStop,
  starWhisper,
  TIPS,
  tipsWith,
} from './copy';

describe('horizonTip', () => {
  it('cycles through the tips and wraps in both directions', () => {
    expect(horizonTip(0)).toBe(HORIZON_TIPS[0]);
    expect(horizonTip(HORIZON_TIPS.length)).toBe(HORIZON_TIPS[0]);
    expect(horizonTip(-1)).toBe(HORIZON_TIPS[HORIZON_TIPS.length - 1]);
  });

  it('has no tip longer than a line of the status area', () => {
    for (const tip of HORIZON_TIPS) expect(tip.length).toBeLessThanOrEqual(84);
  });
});

describe('starWhisper', () => {
  it('counts up and ends on the seventh', () => {
    expect(starWhisper(1)).toBe(STAR_WHISPERS[0]);
    expect(starWhisper(7)).toBe(STAR_WHISPERS[6]);
    expect(starWhisper(99)).toBe(STAR_WHISPERS[6]);
    expect(starWhisper(0)).toBe(STAR_WHISPERS[0]);
  });
});

describe('isLate', () => {
  it('is the small hours only', () => {
    expect(isLate(0)).toBe(true);
    expect(isLate(4)).toBe(true);
    expect(isLate(5)).toBe(false);
    expect(isLate(23)).toBe(false);
  });
});

describe('longStop', () => {
  it('spells durations out', () => {
    expect(longStop(60)).toBe('1 minute');
    expect(longStop(1800)).toBe('30 minutes');
    expect(longStop(3600)).toBe('1 hour');
    expect(longStop(7200)).toBe('2 hours');
    expect(longStop(5400)).toBe('1.5 hours');
  });
});

describe('tipsWith', () => {
  it('lists the hotkey after Space, and nothing when it is off', () => {
    expect(tipsWith('Ctrl+Alt+K')[1]).toEqual({ keys: 'Ctrl+Alt+K', does: 'Toggle from anywhere' });
    expect(tipsWith('Ctrl+Alt+K')).toHaveLength(TIPS.length + 1);
    expect(tipsWith(null)).toBe(TIPS);
  });
});

describe('AUTHOR', () => {
  it('points at the website', () => {
    expect(AUTHOR.site).toBe('https://akshaybajpai.com');
    expect(AUTHOR.siteLabel).toBe('akshaybajpai.com');
  });
});
