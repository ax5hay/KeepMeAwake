/**
 * Every piece of UI copy that is more than a label lives here, so tone stays
 * consistent and the words can be tested without a DOM.
 */

/** Who made this, and where to find more. Used in About and the wordmark. */
export const AUTHOR = {
  name: 'Akshay Bajpai',
  site: 'https://akshaybajpai.com',
  siteLabel: 'akshaybajpai.com',
  repo: 'https://github.com/BuiltByBajpai/KeepMeAwake',
} as const;

/** Whispered when the wordmark is clicked. */
export const CREDIT = `Made in the small hours by ${AUTHOR.name}.`;

/** Tips revealed by clicking the horizon line, in order, then around again. */
export const HORIZON_TIPS: readonly string[] = [
  'The sun is the switch. Click it, or press Space.',
  'Pick a stop on the rule and the sun will set when the time is up.',
  'Closing this window keeps the sun in your tray. Esc does the same.',
  'Right-click the sun in the tray for everything without opening this window.',
  'Below 20% battery the sun pauses and waits for the charger.',
  'Until… takes a time of day. If it has already passed, it means tomorrow.',
  'Ctrl+, opens settings. Esc brings you back here.',
  'Windows lists this under powercfg /requests, by name.',
  'The sun has nothing to do with daylight. It is how awake your laptop is.',
  'A schedule keeps the sun up for you. Settings → Schedule.',
];

/** Tip `n` (0-based), wrapping. */
export function horizonTip(n: number): string {
  const tips = HORIZON_TIPS;
  const index = ((n % tips.length) + tips.length) % tips.length;
  return tips[index] ?? '';
}

/** Said as stars are found. The last one plays when all seven are lit. */
export const STAR_WHISPERS: readonly string[] = [
  'A star.',
  'Two.',
  'Three. They are fixed; nothing in this sky is random.',
  'Four.',
  'Five.',
  'Six. One more.',
  'All seven. The night is yours.',
];

/** Whisper for the `count`-th star found (1-based). */
export function starWhisper(count: number): string {
  return STAR_WHISPERS[Math.min(Math.max(count, 1), STAR_WHISPERS.length) - 1] ?? '';
}

/** Said when the reflection on the water is touched. */
export const RIPPLE = 'Still water.';

/** Hours of the night during which the awake line gets a second sentence. */
const LATE_FROM = 0;
const LATE_UNTIL = 5;

/** `true` between midnight and five in the morning, local time. */
export function isLate(hour: number): boolean {
  return hour >= LATE_FROM && hour < LATE_UNTIL;
}

/** Appended to the awake detail line late at night. */
export const LATE_NIGHT = 'It is late. The sun will hold the line.';

/** Status detail while a schedule window holds the sun up. */
export function onSchedule(time: string): string {
  return `Until ${time} · on schedule`;
}

/** Appended to the resting line while a schedule waits for its next window. */
export function awakeAgain(when: string): string {
  return `Awake again at ${when}.`;
}

/** Status detail while today's window is skipped. */
export function skipping(time: string): string {
  return `Skipping today's schedule until ${time}.`;
}

/** Whispered when the sun skips the rest of a window. */
export function skippedUntil(time: string): string {
  return `Skipped until ${time}.`;
}

/** Whispered when a skipped window is resumed. */
export const BACK_ON_SCHEDULE = 'Back on schedule.';

/** Long form of a duration stop for tooltips: "30 minutes", "2 hours". */
export function longStop(secs: number): string {
  if (secs < 3600) {
    const m = Math.round(secs / 60);
    return `${m} minute${m === 1 ? '' : 's'}`;
  }
  const h = secs / 3600;
  const shown = Number.isInteger(h) ? String(h) : h.toFixed(1);
  return `${shown} hour${h === 1 ? '' : 's'}`;
}

/** Keyboard and tray help shown in Settings → Tips. */
export const TIPS: ReadonlyArray<{ keys: string; does: string }> = [
  { keys: 'Space', does: 'Toggle awake and resting' },
  { keys: 'Esc', does: 'Back, or close to the tray' },
  { keys: 'Ctrl+,', does: 'Settings' },
  { keys: 'Tray · left click', does: 'Open this window' },
  { keys: 'Tray · right click', does: 'Every action, no window' },
  { keys: 'Horizon', does: 'Click the line for a tip' },
];
