import { describe, expect, it } from 'vitest';

import { gateOf, nowLocal, planOf } from './claude';
import type { AiSettings, Understood } from './types';

const on: AiSettings = { enabled: true, week_in_words: true, tell_the_sun: true };

describe('gateOf', () => {
  it('is locked without a key, whatever the switches say', () => {
    expect(gateOf({ has_key: false }, on, 'week_in_words')).toBe('locked');
    expect(gateOf(null, on, 'week_in_words')).toBe('locked');
  });

  it('is off with a key while Claude is switched off, which is how it starts', () => {
    expect(gateOf({ has_key: true }, { ...on, enabled: false }, 'tell_the_sun')).toBe('off');
    expect(gateOf({ has_key: true }, null, 'tell_the_sun')).toBe('off');
  });

  it('lets each feature be switched off on its own', () => {
    const settings = { ...on, tell_the_sun: false };
    expect(gateOf({ has_key: true }, settings, 'tell_the_sun')).toBe('feature-off');
    expect(gateOf({ has_key: true }, settings, 'week_in_words')).toBe('on');
  });
});

describe('planOf', () => {
  const now = new Date(2026, 10, 10, 18, 42).getTime();
  const understood = (patch: Partial<Understood>): Understood => ({
    kind: 'unclear',
    minutes: 0,
    time: '',
    say: '',
    ...patch,
  });

  it('turns a length into seconds and a time of day into its next occurrence', () => {
    expect(planOf(understood({ kind: 'for', minutes: 120 }), now)).toEqual({
      kind: 'for',
      seconds: 7200,
    });
    expect(planOf(understood({ kind: 'until', time: '18:30' }), now)).toEqual({
      kind: 'until',
      unix_ms: new Date(2026, 10, 11, 18, 30).getTime(),
    });
    expect(planOf(understood({ kind: 'until', time: '23:00' }), now)).toEqual({
      kind: 'until',
      unix_ms: new Date(2026, 10, 10, 23, 0).getTime(),
    });
  });

  it('passes always and off straight through', () => {
    expect(planOf(understood({ kind: 'indefinite' }), now)).toEqual({ kind: 'indefinite' });
    expect(planOf(understood({ kind: 'off' }), now)).toEqual({ kind: 'off' });
  });

  it('starts nothing it did not understand', () => {
    expect(
      planOf(understood({ kind: 'unclear', say: 'Not sure what that means' }), now),
    ).toBeNull();
    expect(planOf(understood({ kind: 'for', minutes: 0 }), now)).toBeNull();
    expect(planOf(understood({ kind: 'until', time: 'half six' }), now)).toBeNull();
  });
});

describe('nowLocal', () => {
  it('says the day and the time, 24-hour', () => {
    expect(nowLocal(new Date(2026, 10, 10, 18, 42).getTime())).toBe('Tue 10 Nov 2026, 18:42');
    expect(nowLocal(new Date(2026, 0, 5, 7, 5).getTime())).toBe('Mon 5 Jan 2026, 07:05');
  });
});
