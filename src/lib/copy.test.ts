import { describe, expect, it } from 'vitest';

import {
  ALMANAC_BEGINS,
  ALMANAC_CLOSED,
  ALMANAC_FOOT,
  ALMANAC_HINT,
  AUTHOR,
  dayDetail,
  daysWorked,
  EMPTY_LEAVE_HINT,
  leavesTaken,
  nightLine,
  quietNight,
  WORK_HOURS_HINT,
  doors,
  downloading,
  updateAvailable,
  updateWhisper,
  HORIZON_TIPS,
  STAR_WHISPERS,
  horizonTip,
  horizonTips,
  isLate,
  KEEP_IT_UP,
  KEPT_UP,
  longStop,
  nobodyHere,
  OVERNIGHT_GUARD_HINT,
  platformCopy,
  starWhisper,
  sunsetIn,
  TIPS,
  tips,
  tipsWith,
  untilYouSay,
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

describe('overnight guard copy', () => {
  it('says how long nobody has been here and when the sun rests', () => {
    expect(nobodyHere(6, '3:12 am')).toBe('Nobody here for 6 hours. Resting at 3:12 am.');
    expect(nobodyHere(1, '3:12 am')).toBe('Nobody here for 1 hour. Resting at 3:12 am.');
    expect(KEEP_IT_UP).toBe('Keep it up');
  });

  it('fits the status line with the link, and the hint on one line', () => {
    expect(`${nobodyHere(24, '12:59 am')} ${KEEP_IT_UP}`.length).toBeLessThanOrEqual(64);
    // As short as the sunset hint, which fits beside the same control.
    expect(OVERNIGHT_GUARD_HINT.length).toBeLessThanOrEqual(28);
  });
});

describe('doors', () => {
  it('opens the schedule, the guard, the history and the hotkey, which shows itself', () => {
    expect(doors('Ctrl+Alt+K', 6).map((d) => [d.section, d.label])).toEqual([
      ['schedule', 'Schedule'],
      ['waking', 'Overnight'],
      ['history', 'History'],
      ['hotkey', 'Ctrl+Alt+K'],
    ]);
    expect(doors('⌥⌘K')[3]?.title).toBe('⌥⌘K toggles from any app');
    expect(doors(null)[3]?.label).toBe('Hotkey');
  });

  it('says what the guard will do, or that it is off', () => {
    expect(doors(null, 6)[1]?.title).toBe('Always rests after 6h with nobody around');
    expect(doors(null, null)[1]?.title).toMatch(/^Overnight guard is off/);
  });

  it('fits one line at 380 px with the default hotkey', () => {
    const line = doors('Ctrl+Alt+K', 6)
      .map((d) => d.label)
      .join(' · ');
    expect(line.length).toBeLessThanOrEqual(44);
  });
});

describe('update copy', () => {
  it('names the version, the action and the progress', () => {
    expect(updateWhisper('0.3.1')).toBe('A new version is up: 0.3.1.');
    expect(updateAvailable('0.3.1')).toBe('0.3.1 is available');
    expect(downloading(42)).toBe('Downloading… 42%');
    expect(downloading(null)).toBe('Downloading…');
  });

  it('puts a waiting update first on Home', () => {
    const line = doors('Ctrl+Alt+K', 6, '0.3.1');
    expect(line[0]).toEqual({
      section: 'about',
      label: 'Update 0.3.1',
      title: '0.3.1 is ready to install',
    });
    expect(line.map((d) => d.section)).toEqual(['about', 'schedule', 'waking', 'history']);
    expect(doors('Ctrl+Alt+K', 6)).toHaveLength(4);
  });
});

describe('almanac copy', () => {
  const day = {
    kind: 'worked',
    today: false,
    ahead: false,
    marked: false,
    first: '9:02 am',
    last: '6:40 pm',
    over: null,
  } as const;

  it('names the first and last touch, and what ran over', () => {
    expect(dayDetail(day)).toBe('First light 9:02 am · last 6:40 pm');
    expect(dayDetail({ ...day, over: '40m' })).toBe(
      'First light 9:02 am · last 6:40 pm · 40m over',
    );
    expect(dayDetail({ ...day, today: true, over: '40m' })).toBe('First light 9:02 am · 40m over');
  });

  it('says why a day has no hours', () => {
    const empty = { ...day, first: null, last: null };
    expect(dayDetail({ ...empty, kind: 'leave' })).toBe('A working day with nothing in it.');
    expect(dayDetail({ ...empty, kind: 'leave', marked: true })).toBe('A day off, marked by you.');
    expect(dayDetail({ ...empty, kind: 'leave', marked: true, ahead: true })).toBe(
      'Leave, still to come.',
    );
    expect(dayDetail({ ...empty, kind: 'rest' })).toBe('Not a working day.');
    expect(dayDetail({ ...empty, kind: 'ahead', ahead: true })).toBe('Still to come.');
    expect(dayDetail({ ...empty, kind: 'nothing', today: true })).toBe('Nothing yet today.');
    expect(dayDetail({ ...empty, kind: 'nothing', marked: true })).toBe(
      'Nothing here, and not a leave.',
    );
  });

  it('names the quiet and never says anyone slept', () => {
    expect(quietNight('11:48 pm', '7:02 am')).toBe('A quiet night: 11:48 pm to 7:02 am.');
    expect(nightLine('11:48 pm', '7:02 am', '7h 14m')).toBe('Night 11:48 pm – 7:02 am · 7h 14m');
    for (const text of [quietNight('a', 'b'), nightLine('a', 'b', 'c'), ALMANAC_FOOT]) {
      expect(text).not.toMatch(/sle[e]?pt?|asleep/i);
    }
  });

  it('counts days and leaves in words', () => {
    expect(daysWorked(1)).toBe('1 day');
    expect(daysWorked(4)).toBe('4 days');
    expect(leavesTaken(0)).toBe('No leave');
    expect(leavesTaken(1)).toBe('1 leave');
    expect(leavesTaken(9)).toBe('9 leaves');
  });

  it('keeps the lines short enough for where they sit', () => {
    const longest = dayDetail({ ...day, first: '12:59 am', last: '12:59 pm', over: '12h 59m' });
    expect(longest.length).toBeLessThanOrEqual(52);
    expect(nightLine('12:59 pm', '12:59 am', '12h 59m').length).toBeLessThanOrEqual(52);
    expect(quietNight('12:59 pm', '12:59 am').length).toBeLessThanOrEqual(84);
    for (const hint of [ALMANAC_HINT, EMPTY_LEAVE_HINT])
      expect(hint.length).toBeLessThanOrEqual(42);
    expect(WORK_HOURS_HINT.length).toBeLessThanOrEqual(58);
    for (const line of [ALMANAC_CLOSED, ALMANAC_BEGINS]) expect(line).not.toMatch(/!/);
  });

  it('names no platform', () => {
    for (const text of [ALMANAC_FOOT, ALMANAC_CLOSED, ALMANAC_HINT, WORK_HOURS_HINT]) {
      expect(text).not.toMatch(/tray|taskbar|Windows|menu bar|macOS/i);
    }
  });
});

describe('the Always line', () => {
  it('names the guard while it watches', () => {
    expect(untilYouSay(6)).toBe('Until you say otherwise, or 6h untouched.');
    expect(untilYouSay(null)).toBe('Until you say otherwise.');
    expect(KEPT_UP.length).toBeLessThanOrEqual(48);
  });
});

describe('AUTHOR', () => {
  it('points at the website', () => {
    expect(AUTHOR.site).toBe('https://akshaybajpai.com');
    expect(AUTHOR.siteLabel).toBe('akshaybajpai.com');
  });
});
