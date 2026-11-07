/**
 * Every piece of UI copy that is more than a label lives here, so tone stays
 * consistent and the words can be tested without a DOM. Copy that names the
 * OS, the tray or a shortcut is written once per platform, as a function of
 * `mac`, and resolved through `isMac` for the window.
 */

import { isMac } from './platform';

/** Who made this, and where to find more. Used in About and the wordmark. */
export const AUTHOR = {
  name: 'Akshay Bajpai',
  site: 'https://akshaybajpai.com',
  siteLabel: 'akshaybajpai.com',
  repo: 'https://github.com/ax5hay/KeepMeAwake',
} as const;

/** Whispered when the wordmark is clicked. */
export const CREDIT = `Made in the small hours by ${AUTHOR.name}.`;

/**
 * Tips revealed by clicking the horizon line, in order, then around again.
 * The ones that name the tray, a shortcut or the OS read right on each.
 */
export function horizonTips(mac: boolean): readonly string[] {
  return [
    'The sun is the switch. Click it, or press Space.',
    'Pick a stop on the rule and the sun will set when the time is up.',
    mac
      ? 'Closing this window keeps the sun in the menu bar. ⌘W does the same.'
      : 'Closing this window keeps the sun in your tray. Esc does the same.',
    mac
      ? 'Right-click the sun in the menu bar for everything, no window needed.'
      : 'Right-click the sun in the tray for everything without opening this window.',
    'Below 20% battery the sun pauses and waits for the charger.',
    'Until… takes a time of day. If it has already passed, it means tomorrow.',
    mac
      ? '⌘, opens settings. Esc brings you back here.'
      : 'Ctrl+, opens settings. Esc brings you back here.',
    mac
      ? 'macOS lists this under pmset -g assertions, by name.'
      : 'Windows lists this under powercfg /requests, by name.',
    'The sun has nothing to do with daylight. It is how awake your laptop is.',
    'A schedule keeps the sun up for you. Settings → Schedule.',
  ];
}

/** The horizon tips for this platform. */
export const HORIZON_TIPS: readonly string[] = horizonTips(isMac);

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

/** The status line while a timed session is inside its sunset warning. */
export function sunsetIn(secs: number): string {
  if (secs < 60) return 'The sun sets in under a minute.';
  const m = Math.ceil(secs / 60);
  return `The sun sets in ${m} minute${m === 1 ? '' : 's'}.`;
}

/** The action under the status line, and the tray item, near sunset. */
export const EXTEND_LABEL = 'Give it another 30m';

/** Whispered after the extend link is clicked. */
export const EXTENDED = 'Another 30 minutes.';

/**
 * The status line while the overnight guard asks whether anyone is there:
 * `Nobody here for 6 hours. Resting at 3:12 am.`
 */
export function nobodyHere(hours: number, time: string): string {
  return `Nobody here for ${hours} hour${hours === 1 ? '' : 's'}. Resting at ${time}.`;
}

/** The action under the status line, and the tray item, while the guard asks. */
export const KEEP_IT_UP = 'Keep it up';

/** Whispered after Keep it up: the guard will not ask again this session. */
export const KEPT_UP = 'Kept up. It will not ask again this time.';

/**
 * The Always line. While the overnight guard watches, it says so, so the
 * session's one way of ending on its own is never a surprise.
 */
export function untilYouSay(guardHours: number | null): string {
  return guardHours === null
    ? 'Until you say otherwise.'
    : `Until you say otherwise, or ${guardHours}h untouched.`;
}

/** Settings → Waking → Overnight guard. */
export const OVERNIGHT_GUARD_HINT = 'Ends an untouched Always.';

/** A Settings section Home can open directly. */
export type SettingsSection = 'waking' | 'schedule' | 'history' | 'hotkey';

/** One way from Home straight into a Settings section. */
export interface Door {
  section: SettingsSection;
  label: string;
  /** Tooltip and accessible description: what is behind it. */
  title: string;
}

/**
 * The doorways at the foot of Home: the schedule (off until set), the
 * overnight guard (on, and quiet until it acts), the log, and the hotkey,
 * which shows its own combination so the line itself says it exists.
 * `hotkey` is already formatted for display, or `null` when it is off;
 * `guardHours` is the guard's setting, `null` when off.
 */
export function doors(hotkey: string | null, guardHours: number | null = null): readonly Door[] {
  return [
    { section: 'schedule', label: 'Schedule', title: 'Awake on its own during set hours' },
    {
      section: 'waking',
      label: 'Overnight',
      title:
        guardHours === null
          ? 'Overnight guard is off: Always never rests on its own'
          : `Always rests after ${guardHours}h with nobody around`,
    },
    { section: 'history', label: 'History', title: 'When the sun was up, and how it set' },
    hotkey === null
      ? { section: 'hotkey', label: 'Hotkey', title: 'Toggle from any app' }
      : { section: 'hotkey', label: hotkey, title: `${hotkey} toggles from any app` },
  ];
}

/** Settings → History before anything has been logged. */
export const HISTORY_EMPTY = 'Nothing yet. The first sunset will be written here.';

/** The link under the history list. */
export const CLEAR_HISTORY = 'Clear history';

/** Shown in the hotkey field while it waits for a combination. */
export const HOTKEY_RECORDING = 'Press keys…';

/** The Tips row for the global hotkey. */
export const HOTKEY_TIP = 'Toggle from anywhere';

/** One row of Settings → Tips. */
export interface Tip {
  keys: string;
  does: string;
}

/** Keyboard and tray (or menu bar) help shown in Settings → Tips. */
export function tips(mac: boolean): readonly Tip[] {
  if (mac) {
    return [
      { keys: 'Space', does: 'Toggle awake and resting' },
      { keys: 'Esc', does: 'Back, or close the window' },
      { keys: '⌘,', does: 'Settings' },
      { keys: '⌘W', does: 'Close to the menu bar' },
      { keys: '⌘Q', does: 'Quit' },
      { keys: 'Menu bar · click', does: 'Open this window' },
      { keys: 'Menu bar · right click', does: 'Every action, no window' },
      { keys: 'Horizon', does: 'Click the line for a tip' },
    ];
  }
  return [
    { keys: 'Space', does: 'Toggle awake and resting' },
    { keys: 'Esc', does: 'Back, or close to the tray' },
    { keys: 'Ctrl+,', does: 'Settings' },
    { keys: 'Tray · left click', does: 'Open this window' },
    { keys: 'Tray · right click', does: 'Every action, no window' },
    { keys: 'Horizon', does: 'Click the line for a tip' },
  ];
}

/** The Tips rows for this platform. */
export const TIPS: readonly Tip[] = tips(isMac);

/** The Tips rows with the current global hotkey (if any) after Space. */
export function tipsWith(hotkey: string | null, list: readonly Tip[] = TIPS): readonly Tip[] {
  if (hotkey === null) return list;
  const [first, ...rest] = list;
  return [...(first ? [first] : []), { keys: hotkey, does: HOTKEY_TIP }, ...rest];
}

/** The strings in the window that name the OS, the tray or a shortcut. */
export interface PlatformCopy {
  /** Title bar close button, for screen readers. */
  closeLabel: string;
  /** Title bar close button tooltip. */
  closeTitle: string;
  /** Title bar settings button tooltip. */
  settingsTitle: string;
  /** Settings → Launch at sign-in. */
  launchHint: string;
  /** Settings → Default length. */
  lengthHint: string;
  /** Settings → Hotkey. */
  hotkeyHint: string;
  /** Settings → About: what holds the machine awake and where to see it. */
  about: { before: string; code: string; after: string };
  /** Status line when the OS turns the request down. */
  refused: string;
}

/** {@link PlatformCopy} for a Mac (`true`) or Windows. */
export function platformCopy(mac: boolean): PlatformCopy {
  const mechanism = 'the same mechanism a video player uses. See it under';
  const after = '. Nothing is typed, nothing moves.';
  if (mac) {
    return {
      closeLabel: 'Close to the menu bar',
      closeTitle: 'Close to the menu bar (⌘W). The sun stays up.',
      settingsTitle: 'Settings (⌘,)',
      launchHint: 'Starts quietly in the menu bar.',
      lengthHint: 'Used by the sun, the menu bar and Space.',
      hotkeyHint: 'Works in any app. Needs at least one of ⌘, ⌃, ⌥.',
      about: {
        before: `Holds a power assertion while awake, ${mechanism}`,
        code: 'pmset -g assertions',
        after,
      },
      refused: 'macOS refused the request',
    };
  }
  return {
    closeLabel: 'Close to tray',
    closeTitle: 'Close to tray (Esc). The sun stays up.',
    settingsTitle: 'Settings (Ctrl+,)',
    launchHint: 'Starts quietly in the tray.',
    lengthHint: 'Used by the sun, the tray and Space.',
    hotkeyHint: 'Works in any app. Needs at least one of Ctrl, Alt, Win.',
    about: {
      before: `Holds a Windows power request while awake, ${mechanism}`,
      code: 'powercfg /requests',
      after,
    },
    refused: 'Windows refused the request',
  };
}

/** {@link PlatformCopy} for this window. */
export const PLATFORM_COPY: PlatformCopy = platformCopy(isMac);
