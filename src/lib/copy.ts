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

/** Settings → Schedule → From / Until. */
export const SCHEDULE_HOURS_HINT = 'Overnight is fine. The same time twice is all day.';

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

/**
 * The status line while Set with me is about to set the sun: the keys went
 * quiet inside the resting hours. `Quiet for the night. Setting at 3:12 am.`
 */
export function quietForTheNight(time: string): string {
  return `Quiet for the night. Setting at ${time}.`;
}

/** The resting line after the sun set with the night. */
export const SET_WITH_THE_NIGHT = 'Set with the night. Up again at your first touch.';

/** Settings → Almanac → Set with me. */
export const SET_WITH_ME_HINT =
  'Always sets when you go quiet at night, and rises at your first touch.';

/** Settings → Almanac → Whisper the night. */
export const WHISPER_NIGHT_HINT = 'Once, on the first open after it.';

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

/** Whispered once per version, the first time the window opens after it is found. */
export function updateWhisper(version: string): string {
  return `A new version is up: ${version}.`;
}

/** Settings → About, while a newer release waits. */
export function updateAvailable(version: string): string {
  return `${version} is available`;
}

/** The link beside it. */
export const INSTALL_UPDATE = 'Install and restart';

/** The progress line while it downloads; `null` while the size is unknown. */
export function downloading(percent: number | null): string {
  return percent === null ? 'Downloading…' : `Downloading… ${percent}%`;
}

/** Settings → About: the link that checks now, while it does, and when nothing is newer. */
export const CHECK_NOW = 'Check now';
export const CHECKING = 'Checking…';
export const NEWEST = 'This is the newest.';

/** Settings → About → Check for updates daily. */
export const UPDATES_HINT = 'Once a day, from GitHub. Nothing else leaves this machine.';

/** A Settings section Home can open directly. */
export type SettingsSection =
  'waking' | 'schedule' | 'almanac' | 'ai' | 'history' | 'hotkey' | 'about';

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
export function doors(
  hotkey: string | null,
  guardHours: number | null = null,
  update: string | null = null,
): readonly Door[] {
  // A waiting update comes first: it is the one news item the line carries,
  // and the hotkey door makes room for it so the line stays one line.
  if (update !== null) {
    return [
      { section: 'about', label: `Update ${update}`, title: `${update} is ready to install` },
      ...doors(hotkey, guardHours).slice(0, -1),
    ];
  }
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

/** The title bar's way into the Almanac. */
export const ALMANAC_TITLE = 'Almanac: your days, hours and nights';

/**
 * Whispered once on the first open after a night. It names the quiet, not
 * what was done with it: the keys went quiet, and that is all it knows.
 */
export function quietNight(from: string, to: string): string {
  return `A quiet night: ${from} to ${to}.`;
}

/** The word a day reads as when it has no hours to show. */
export const DAY_WORDS = {
  leave: 'Leave',
  rest: 'Rest',
  nothing: 'Nothing',
  ahead: 'Ahead',
} as const;

/** What the detail line can be told about the selected day. */
export interface DayFacts {
  kind: 'worked' | 'leave' | 'rest' | 'nothing' | 'ahead';
  today: boolean;
  /** Still to come. */
  ahead: boolean;
  /** Marked by hand, either way. */
  marked: boolean;
  /** Its hours were entered by hand. */
  fixed: boolean;
  /** First and last touch, already as clock times. */
  first: string | null;
  last: string | null;
  /** Time outside working hours, already formatted, or `null` for none. */
  over: string | null;
}

/** The line under the day's figure. */
export function dayDetail(day: DayFacts): string {
  const hasHours = day.kind === 'worked' || (day.today && day.kind !== 'leave');
  if (day.first !== null && day.last !== null && hasHours) {
    // By hand, it says so first; today the last touch is now, and saying so
    // would only repeat the clock.
    const span = day.fixed
      ? `Set by you: ${day.first} – ${day.last}`
      : day.today
        ? `First light ${day.first}`
        : `First light ${day.first} · last ${day.last}`;
    return day.over === null ? span : `${span} · ${day.over} over`;
  }
  if (day.fixed && day.kind !== 'leave') return 'Nobody here, set by you.';
  switch (day.kind) {
    case 'leave':
      if (day.ahead) return 'Leave, still to come.';
      return day.marked ? 'A day off, marked by you.' : 'A working day with nothing in it.';
    case 'rest':
      return 'Not a working day.';
    case 'ahead':
      return 'Still to come.';
    default:
      if (day.today) return 'Nothing yet today.';
      return day.marked ? 'Nothing here, and not a leave.' : 'Nothing was kept for this day.';
  }
}

/** The night that led into a day: `Night 11:48 pm – 7:02 am · 7h 14m`. */
export function nightLine(from: string, to: string, length: string): string {
  return `Night ${from} – ${to} · ${length}`;
}

/** The link that marks the selected day as leave. */
export const MARK_LEAVE = 'Mark as leave';

/** The same link on a day that reads as leave. */
export const NOT_A_LEAVE = 'Not a leave';

/** The link that opens a day's hours for correcting. */
export const CORRECT_HOURS = 'Correct the hours';

/** In the editor: put back what was read from the keys. */
export const USE_WHAT_WAS_READ = 'Use what was read';

/** In the editor: say nobody was at the machine that day. */
export const NOBODY_HERE = 'Nobody here';

/** Home: the eyebrow over today's figure. */
export const TODAY_SO_FAR = 'Today';

/** Home: what the day's figure and strip open. */
export const OPEN_ALMANAC = 'Open the Almanac';

/** Whispered once, the first time a day reaches the length of its working hours. */
export function fullDay(worked: string): string {
  return `A full day: ${worked}.`;
}

/** `1 day`, `4 days`. */
export function daysWorked(count: number): string {
  return `${count} day${count === 1 ? '' : 's'}`;
}

/** `No leave`, `1 leave`, `9 leaves`. */
export function leavesTaken(count: number): string {
  if (count === 0) return 'No leave';
  return `${count} leave${count === 1 ? '' : 's'}`;
}

/** Under the almanac, what it reads and where it stays. */
export const ALMANAC_FOOT =
  'Read from the keyboard and mouse, and kept on this machine. A break under ten minutes still counts.';

/** The link from the Almanac into its settings. */
export const ALMANAC_SETTINGS = 'Working days and hours…';

/** The link that empties it. */
export const CLEAR_ALMANAC = 'Clear the almanac';

/** The pane while the almanac is switched off. */
export const ALMANAC_CLOSED = 'The almanac is closed. Nothing is being read.';

/** The pane on its first day, before there is anything to draw. */
export const ALMANAC_BEGINS = 'The first day is being written.';

/** Settings → Almanac → Keep an almanac. */
export const ALMANAC_HINT = 'Days, hours and nights, from your input.';

/** Settings → Almanac → Working hours. */
export const WORK_HOURS_HINT = 'Past midnight is fine. Time outside them counts as over.';

/** Settings → Almanac → Empty working days are leave. */
export const EMPTY_LEAVE_HINT = 'Off on a machine you rarely use.';

/** A Claude feature with no API key: said wherever the feature is. */
export const AI_LOCKED = 'Locked: no API key yet.';

/** The link beside it, into Settings → Claude. */
export const AI_ADD_KEY = 'Add a key';

/** A Claude feature with a key, while Claude is switched off. */
export const AI_OFF = 'Claude is switched off.';

/** The link beside it: the master switch, from where the feature is. */
export const AI_SWITCH_ON = 'Switch it on';

/** Settings → Claude → Use Claude. */
export const AI_MASTER_HINT = 'Off, nothing is ever sent to Anthropic.';

/** The same row with no key: the switch cannot move yet. */
export const AI_MASTER_LOCKED = 'Needs an API key first.';

/** Settings → Claude, under the key field, while there is none. */
export const AI_KEY_WHY = 'Your own key from console.anthropic.com. Asking Claude is billed to it.';

/** Settings → Claude, once a key is stored. */
export const AI_KEY_SAVED = 'A key is saved.';

/** Settings → Claude: what the app does and does not do with it. */
export const AI_PROMISE =
  'Claude is asked only when you click something that says so, and is sent only what that says.';

/** The Almanac's week, put into words. */
export const WEEK_IN_WORDS = 'This week, in words';

/** What it does, shown while its switch is off. */
export const WEEK_HINT = 'Two or three sentences on the week being read.';

/** The link that asks. */
export const WEEK_READ = 'Read this week';

/** While it is being read. */
export const WEEK_READING = 'Reading…';

/** What asking sends. */
export const WEEK_SENDS = 'Sends this week of hours and nights to Anthropic, when you ask.';

/** Home's plan in a sentence. */
export const TELL_THE_SUN = 'Tell the sun';

/** What it does, shown while its switch is off. */
export const TELL_HINT = 'Say the plan in words instead of picking a stop.';

/** Shown in the empty field. */
export const TELL_PLACEHOLDER = 'two hours, or until half six';

/** The button that asks, and what it says while waiting. */
export const TELL_ASK = 'Ask';
export const TELL_THINKING = 'Asking…';

/** The button that starts what was understood. */
export const TELL_START = 'Start';

/** What asking sends. */
export const TELL_SENDS = 'Sends this sentence to Anthropic.';

/** Settings → Almanac → Break reminder. */
export const BREAK_HINT = 'A toast after that long without five minutes away.';

/** The link that copies the open month, and what it says once it has. */
export const COPY_MONTH = 'Copy month as CSV';
export const COPIED = 'Copied';

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
  /** Settings → Claude: where the API key is kept. */
  keyKept: string;
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
      keyKept: 'Kept in the macOS Keychain, not in a file.',
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
    keyKept: 'Kept in Windows Credential Manager, not in a file.',
  };
}

/** {@link PlatformCopy} for this window. */
export const PLATFORM_COPY: PlatformCopy = platformCopy(isMac);
