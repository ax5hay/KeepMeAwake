import { describe, expect, it } from 'vitest';

import {
  AUTHOR,
  HORIZON_TIPS,
  STAR_WHISPERS,
  horizonTip,
  horizonTips,
  isLate,
  longStop,
  platformCopy,
  starWhisper,
  sunsetIn,
  TIPS,
  tips,
  tipsWith,
} from './copy';

/** Every string that differs by platform, flattened. */
function everything(mac: boolean): string[] {
  const { about, ...rest } = platformCopy(mac);
  return [
    ...horizonTips(mac),
    ...tips(mac).flatMap((tip) => [tip.keys, tip.does]),
    ...Object.values(rest),
    ...Object.values(about),
  ];
}

describe('horizonTip', () => {
  it('cycles through the tips and wraps in both directions', () => {
    expect(horizonTip(0)).toBe(HORIZON_TIPS[0]);
    expect(horizonTip(HORIZON_TIPS.length)).toBe(HORIZON_TIPS[0]);
    expect(horizonTip(-1)).toBe(HORIZON_TIPS[HORIZON_TIPS.length - 1]);
  });

  it('has no tip longer than a line of the status area', () => {
    for (const mac of [false, true]) {
      for (const tip of horizonTips(mac)) expect(tip.length).toBeLessThanOrEqual(84);
    }
  });

  it('has as many tips on a Mac as on Windows', () => {
    expect(horizonTips(true)).toHaveLength(horizonTips(false).length);
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

describe('platform copy', () => {
  it('never says tray, taskbar or Windows on a Mac', () => {
    for (const text of everything(true)) expect(text).not.toMatch(/tray|taskbar|Windows/i);
  });

  it('never says menu bar or uses Mac glyphs on Windows', () => {
    for (const text of everything(false)) expect(text).not.toMatch(/menu bar|[⌘⌃⌥⇧]/);
  });

  it('names the place the sun lives and how to prove it', () => {
    expect(platformCopy(true).closeLabel).toBe('Close to the menu bar');
    expect(platformCopy(false).closeLabel).toBe('Close to tray');
    expect(platformCopy(true).about.code).toBe('pmset -g assertions');
    expect(platformCopy(false).about.code).toBe('powercfg /requests');
  });

  it('lists the Mac shortcuts in Tips', () => {
    const keys = tips(true).map((tip) => tip.keys);
    expect(keys).toEqual(expect.arrayContaining(['⌘,', '⌘W', '⌘Q', 'Menu bar · click']));
    expect(tips(false).map((tip) => tip.keys)).toContain('Ctrl+,');
  });

  it('keeps every Tips row short enough for one line', () => {
    for (const mac of [false, true]) {
      for (const tip of tips(mac)) expect(tip.does.length).toBeLessThanOrEqual(28);
    }
  });
});

describe('sunsetIn', () => {
  it('counts whole minutes up, then says under a minute', () => {
    expect(sunsetIn(300)).toBe('The sun sets in 5 minutes.');
    expect(sunsetIn(299)).toBe('The sun sets in 5 minutes.');
    expect(sunsetIn(60)).toBe('The sun sets in 1 minute.');
    expect(sunsetIn(59)).toBe('The sun sets in under a minute.');
  });
});

describe('AUTHOR', () => {
  it('points at the website', () => {
    expect(AUTHOR.site).toBe('https://akshaybajpai.com');
    expect(AUTHOR.siteLabel).toBe('akshaybajpai.com');
  });
});
