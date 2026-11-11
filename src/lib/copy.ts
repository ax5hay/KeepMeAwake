/**
 * Every piece of UI copy that is more than a label lives here, so tone stays
 * consistent and the words can be tested without a DOM. Copy that names the
 * OS, the tray or a shortcut is written once per platform, as a function of
 * `mac`, and resolved through `isMac` for the window.
 */

import { isMac } from './platform';
import type { AfterEnd, WatchReason, WatchState } from './types';

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
export function horizonTips(mac: boolean, battery = true): readonly string[] {
  const tips = [
    'The sun is the switch. Click it, or press Space.',
    'Pick a length under the sun to stay awake for just that long.',
    mac
      ? 'Closing this window keeps the sun in the menu bar. ⌘W does the same.'
      : 'Closing this window keeps the sun in your tray. Esc does the same.',
    mac
      ? 'Right-click the sun in the menu bar for everything, no window needed.'
      : 'Right-click the sun in the tray for everything without opening this window.',
    BATTERY_TIP,
    'Until… takes a time of day. A time already gone today means tomorrow.',
    mac
      ? '⌘, opens settings. Esc brings you back here.'
      : 'Ctrl+, opens settings. Esc brings you back here.',
    mac
      ? 'To check it is working, run pmset -g assertions and look for its name.'
      : 'To check it is working, run powercfg /requests and look for its name.',
    'The sun is not about daylight. Up means your laptop stays awake.',
    'To stay awake at set hours every day, use Settings → Schedule.',
  ];
  return battery ? tips : tips.filter((tip) => tip !== BATTERY_TIP);
}

/** The one tip about the battery, left out on a machine without one. */
const BATTERY_TIP = 'Below 20% battery it pauses, and picks up again when you plug in.';

/** The horizon tips for this platform, with a battery. */
export const HORIZON_TIPS: readonly string[] = horizonTips(isMac);
const NO_BATTERY_TIPS: readonly string[] = horizonTips(isMac, false);

/** Tip `n` (0-based), wrapping; without a battery, the battery tip is left out. */
export function horizonTip(n: number, battery = true): string {
  const tips = battery ? HORIZON_TIPS : NO_BATTERY_TIPS;
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
export const LATE_NIGHT = 'It is late. Your laptop will stay awake.';

/** Status detail while a schedule window holds the sun up. */
export function onSchedule(time: string): string {
  return `Until ${time} · on schedule`;
}

/** `about 3h 10m left`, `about 45m left`: the OS's estimate of the charge left. */
export function timeLeft(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `about ${m}m left`;
  return m === 0 ? `about ${h}h left` : `about ${h}h ${m}m left`;
}

/** A network rate, the way a download dialog shows it: `640 KB/s`, `2.4 MB/s`. */
export function rate(bytesPerSec: number): string {
  if (bytesPerSec >= 1_000_000) return `${(bytesPerSec / 1_000_000).toFixed(1)} MB/s`;
  return `${Math.max(1, Math.round(bytesPerSec / 1000))} KB/s`;
}

/**
 * The status line while a watched condition keeps the laptop awake:
 * `While Xcode runs`, `While the network is busy · 2.4 MB/s`. The tray says
 * the same in `tray.rs`.
 */
export function awakeWhile(reason: WatchReason, watch: WatchState): string {
  switch (reason.kind) {
    case 'app':
      return `While ${reason.name} runs`;
    case 'process':
      return `While process ${reason.pid} runs`;
    case 'transfer':
      return watch.transfer_bps === null
        ? 'While the network is busy'
        : `While the network is busy · ${rate(watch.transfer_bps)}`;
    case 'busy':
      return watch.cpu_percent === null
        ? 'While the computer is busy'
        : `While the computer is busy · ${watch.cpu_percent}%`;
    case 'full_screen':
      return 'While something is full screen';
    case 'display':
      return 'While a display is connected';
    case 'plugged_in':
      return 'While plugged in';
  }
}

/** The resting line after the sun was clicked while a condition held. */
export function skippingUntil(reason: WatchReason): string {
  switch (reason.kind) {
    case 'app':
      return `Skipping until ${reason.name} quits.`;
    case 'process':
      return `Skipping until process ${reason.pid} ends.`;
    case 'transfer':
      return 'Skipping until the network is quiet.';
    case 'busy':
      return 'Skipping until the computer is calm.';
    case 'full_screen':
      return 'Skipping until full screen ends.';
    case 'display':
      return 'Skipping until the display is unplugged.';
    case 'plugged_in':
      return 'Skipping until it is unplugged.';
  }
}

/** Settings → Stay awake while…, one row each. */
export const WATCH_ROWS = {
  apps: {
    label: 'These apps are running',
    hint: 'Pick from what is running now, or type a name.',
  },
  transfers: {
    label: 'A download or upload is going',
    hint: 'Over 64 KB/s, until it has been quiet for 2 minutes.',
  },
  busy: {
    label: 'The computer is busy',
    hint: 'Processor over 30%: a build, an export, a render.',
  },
  full_screen: { label: 'Something is full screen', hint: 'A video, a game or a presentation.' },
  display: { label: 'An external display is connected', hint: 'At a desk with a monitor.' },
  plugged_in: { label: 'Plugged in', hint: 'On battery it sleeps as usual.' },
} as const;

/** Settings → Staying awake → When it ends. */
export const AFTER_END = 'When it ends';
export function afterEndHint(mac: boolean, action: AfterEnd): string {
  if (mac && action === 'screen_off') {
    return 'Locks too, if your Mac asks for a password on wake.';
  }
  return "After a timer, the schedule or a condition ends, if you've been away a minute.";
}

/** The choices: a Mac has no public way to lock. */
export function afterEndOptions(mac: boolean): ReadonlyArray<{ value: AfterEnd; label: string }> {
  return [
    { value: 'nothing', label: 'Nothing' },
    ...(mac ? [] : [{ value: 'lock' as const, label: 'Lock' }]),
    { value: 'screen_off', label: 'Screen off' },
    { value: 'sleep', label: 'Sleep' },
  ];
}

/** The same rows, named shortly for the wide window's Watching column. */
export const WATCH_SHORT = {
  transfers: 'Downloads and uploads',
  busy: 'A busy processor',
  full_screen: 'Full screen',
  display: 'An external display',
  plugged_in: 'Plugged in',
} as const;

/** The field that adds an app, and its button. */
export const ADD_APP = 'Add an app';
export const ADD_APP_PLACEHOLDER = 'Xcode, cargo, Blender…';

/** Settings → Schedule → From / Until. */
export const SCHEDULE_HOURS_HINT = 'Can run past midnight. The same start and end means all day.';

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
  if (secs < 60) return 'Ends in under a minute.';
  const m = Math.ceil(secs / 60);
  return `Ends in ${m} minute${m === 1 ? '' : 's'}.`;
}

/** The action under the status line, and the tray item, near sunset. */
export const EXTEND_LABEL = 'Add 30 minutes';

/** Whispered after the extend link is clicked. */
export const EXTENDED = 'Added 30 minutes.';

/**
 * The status line while the overnight guard asks whether anyone is there:
 * `No input for 6h. Sleeps at 3:12 am.`
 */
export function nobodyHere(hours: number, time: string): string {
  return `No input for ${hours}h. Sleeps at ${time}.`;
}

/**
 * The status line while Sleep when I'm away is about to set the sun: the keys went
 * quiet inside the resting hours. `Away for the night? Sleeps at 3:12 am.`
 */
export function quietForTheNight(time: string): string {
  return `Away for the night? Sleeps at ${time}.`;
}

/** The resting line after the sun set with the night. */
export const SET_WITH_THE_NIGHT =
  'Sleeping while you are away. Always comes back on when you return.';

/** Settings → Almanac → Sleep when I'm away. */
export const SET_WITH_ME_HINT =
  "At night, lets the laptop sleep once you've been away this long, and turns Always back on when you return.";

/** Settings → Almanac → Morning whisper. */
export const WHISPER_NIGHT_HINT = 'When you were away last night, shown once in the morning.';

/** The action under the status line, and the tray item, while the guard asks. */
export const KEEP_IT_UP = "I'm here";

/** Whispered after I'm here: the guard will not ask again this session. */
export const KEPT_UP = 'Staying awake. It will not ask again this time.';

/**
 * The Always line. While the overnight guard watches, it says so, so the
 * session's one way of ending on its own is never a surprise.
 */
export function untilYouSay(guardHours: number | null): string {
  return guardHours === null
    ? 'Until you turn it off.'
    : `Until you turn it off, or ${guardHours}h with no input.`;
}

/** Settings → Waking → Overnight guard. */
export const OVERNIGHT_GUARD_HINT = "Stops Always if you're away.";

/** Whispered once per version, the first time the window opens after it is found. */
export function updateWhisper(version: string): string {
  return `A new version is up: ${version}.`;
}

/** Settings → About, while a newer release waits. */
export function updateAvailable(version: string): string {
  return `Update available: ${version}`;
}

/** The link beside it. */
export const INSTALL_UPDATE = 'Install and restart';

/** The progress line while it downloads; `null` while the size is unknown. */
export function downloading(percent: number | null): string {
  return percent === null ? 'Downloading…' : `Downloading… ${percent}%`;
}

/** Settings → About: the link that checks now, while it does, and when nothing is newer. */
export const CHECK_NOW = 'Check for updates';
export const CHECKING = 'Checking…';
export const NEWEST = "You're up to date.";

/** Settings → About → Check for updates automatically. */
export const UPDATES_HINT = 'Looks on GitHub once a day, and when you open this window.';

/** A Settings section Home can open directly. */
export type SettingsSection =
  'waking' | 'watch' | 'schedule' | 'almanac' | 'ai' | 'history' | 'hotkey' | 'about';

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
    { section: 'schedule', label: 'Schedule', title: 'Stay awake at set hours, on its own' },
    {
      section: 'waking',
      label: 'Overnight',
      title:
        guardHours === null
          ? 'Overnight guard is off: Always runs until you turn it off'
          : `Always turns off after ${guardHours}h with no input`,
    },
    {
      section: 'history',
      label: 'History',
      title: 'When it kept your laptop awake, and why it stopped',
    },
    hotkey === null
      ? { section: 'hotkey', label: 'Hotkey', title: 'A shortcut that works in any app' }
      : { section: 'hotkey', label: hotkey, title: `${hotkey} turns it on or off from any app` },
  ];
}

/** The title bar's way into the Almanac. */
export const ALMANAC_TITLE = 'Almanac: the hours you worked, day by day';

/**
 * Whispered once on the first open after a night. It names the quiet, not
 * what was done with it: the keys went quiet, and that is all it knows.
 */
export function quietNight(from: string, to: string): string {
  return `Last night you were away from ${from} to ${to}.`;
}

/** The word a day reads as when it has no hours to show. */
export const DAY_WORDS = {
  leave: 'Leave',
  rest: 'Day off',
  nothing: 'No activity',
  ahead: 'Upcoming',
} as const;

/** What the detail line can be told about the selected day. */
export interface DayFacts {
  kind: 'worked' | 'leave' | 'rest' | 'nothing' | 'ahead';
  today: boolean;
  /** In the future. */
  ahead: boolean;
  /** Marked by hand, either way. */
  marked: boolean;
  /** Its hours were entered by hand. */
  fixed: boolean;
  /** A working day by the setting that the record says is not worked. */
  usual_off?: boolean;
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
        ? `Started ${day.first}`
        : `Started ${day.first} · ended ${day.last}`;
    return day.over === null ? span : `${span} · ${day.over} extra`;
  }
  if (day.fixed && day.kind !== 'leave') return 'Marked as away, by you.';
  switch (day.kind) {
    case 'leave':
      if (day.ahead) return 'Planned leave.';
      return day.marked
        ? 'Leave, marked by you.'
        : 'No activity on a working day: counted as leave.';
    case 'rest':
      return day.usual_off
        ? 'A day you do not usually work: a day off, not leave.'
        : 'Not a working day.';
    case 'ahead':
      return 'In the future.';
    default:
      if (day.today) return 'No activity yet today.';
      return day.marked ? 'No activity, and not leave.' : 'No activity recorded.';
  }
}

/** The night that led into a day: `Night 11:48 pm – 7:02 am · 7h 14m`. */
export function nightLine(from: string, to: string, length: string): string {
  return `Away overnight ${from} – ${to} · ${length}`;
}

/** The link that marks the selected day as leave. */
export const MARK_LEAVE = 'Mark as leave';

/** The same link on a day that reads as leave. */
export const NOT_A_LEAVE = 'Not leave';

/** The link that opens a day's hours for correcting. */
export const CORRECT_HOURS = 'Edit hours';

/** In the editor: put back what was read from the keys. */
export const USE_WHAT_WAS_READ = 'Undo my edit';

/** In the editor: say nobody was at the machine that day. */
export const NOBODY_HERE = "I wasn't here";

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
  'Based on keyboard and mouse activity, stored only on this machine. Breaks under ten minutes still count.';

/** The link from the Almanac into its settings. */
export const ALMANAC_SETTINGS = 'Working days and hours…';

/** The link that empties it. */
export const CLEAR_ALMANAC = 'Clear the almanac';

/** The pane while the almanac is switched off. */
export const ALMANAC_CLOSED = 'The almanac is off. Nothing is being recorded.';

/** The pane on its first day, before there is anything to draw. */
export const ALMANAC_BEGINS =
  'Your first day is being recorded. There will be more to see tomorrow.';

/** Settings → Almanac → Keep an almanac. */
export const ALMANAC_HINT = 'Logs your hours from keyboard and mouse.';

/** The sheet `?` lays over the window. */
export const KEYS_TITLE = 'Keyboard shortcuts';
export const KEYS_FOOT = 'In the theme picker, the arrow keys move between themes.';

/** The sheet behind Settings → About → Credits. */
export const CREDITS_TITLE = 'Credits';
export const CREDITS_LINK = 'Credits and history';

/** How the app got here, newest first. */
export const VERSIONS: ReadonlyArray<{ name: string; when: string; what: string }> = [
  {
    name: '0.7',
    when: 'Nov 2026',
    what: 'The small things: a theme of your own, the moon, a mini window, a year looked back on.',
  },
  { name: '0.6', when: 'Nov 2026', what: 'Themes, and an installer that looks like the app.' },
  { name: '0.5', when: 'Nov 2026', what: 'AI, as a choice. Checking for updates when you ask.' },
  { name: '0.4', when: 'Nov 2026', what: 'The Almanac: days, hours, leave and nights.' },
  { name: '0.3', when: 'Nov 2026', what: 'History, the overnight guard, updates of its own.' },
  { name: '0.2', when: 'Nov 2026', what: 'A schedule, a hotkey, notifications.' },
  { name: '0.1', when: '2026', what: 'One sun, a tray icon, timers and battery rules.' },
  {
    name: 'Before',
    when: '',
    what: 'A script that pressed a key every few minutes, which this replaces.',
  },
];

/** What it is made with, and whose palettes it borrows. */
export const CREDITS: ReadonlyArray<{ what: string; by: string }> = [
  { what: 'Fraunces', by: 'Undercase Type, under the Open Font License' },
  { what: 'Tauri · Rust · Svelte', by: 'their makers and communities' },
  { what: 'Solarized', by: 'Ethan Schoonover' },
  { what: 'Gruvbox', by: 'Pavel Pertsev' },
  { what: 'Nord', by: 'Sven Greb and Arctic Ice Studio' },
  { what: 'Dracula', by: 'Zeno Rocha' },
  { what: 'Catppuccin', by: 'the Catppuccin community' },
  { what: 'Tokyo Night', by: 'enkia' },
  { what: 'Monokai', by: 'Wimer Hazenberg' },
];

/** A year, looked back on: the sheet's title and its link in the Almanac. */
export function yearTitle(year: number): string {
  return `${year} in review`;
}
export const YEAR_LINK = 'Year in review';
export const YEAR_EMPTY = 'Nothing was recorded this year.';

/** `1 day`, `9 days in a row`. */
export function daysInARow(count: number): string {
  return count === 1 ? '1 day' : `${count} days in a row`;
}

/** The Almanac's foot: the open month, to the printer. */
export const PRINT_MONTH = 'Print month';

/** The title bar's way into the mini window, and the mini window's way back. */
export const MINI_TITLE = 'Mini window: just the sun, above other windows';
export const MINI_BACK = 'Back to the whole window';

/** The title bar's way into the wide window. */
export const WIDE_TITLE = 'Wide window: the whole day, and everything under it';

/** Settings → Theme → After dark. */
export const AFTER_DARK = 'After dark';
export const AFTER_DARK_HINT = 'Switch to a different theme in the evening.';

/** Settings → Sounds. */
export const SOUNDS = 'Sounds';
export const SOUNDS_HINT = 'A soft note when it turns on and off, while this window is open.';

/** A theme of one's own: its five colours, and the line that shares it. */
export const OWN_PART_LABELS = {
  paper: 'Paper',
  ink: 'Ink',
  sun: 'Sun',
  sky: 'Sky',
  ground: 'Ground',
} as const;
export const OWN_CODE_HINT = "Your theme as a line of text. Paste someone else's here to use it.";
export const OWN_CODE_BAD = 'That is not a theme code. It starts with sun: and has five colours.';
export const OWN_COPY = 'Copy';
export const OWN_COPIED = 'Copied';

/** Settings → Theme, under the name of the one worn. */
export const THEME_HINT = 'Changes the colours, nothing else.';

/** The way back to the app's own two. */
export const THEME_RESET = 'Use the system theme';

/** Settings → Almanac → Working hours. */
export const WORK_HOURS_HINT = 'Can run past midnight. Time outside them counts as extra.';

/** Settings → Almanac → Count missed working days as leave. */
export const EMPTY_LEAVE = 'Count missed working days as leave';
export const EMPTY_LEAVE_HINT = 'Weekends and days off never count.';

/** AI's features with no API key: said wherever they are. */
export const AI_LOCKED = 'Locked: needs an API key.';

/** The link beside it, into Settings → AI. */
export const AI_ADD_KEY = 'Add a key';

/** AI's features with a key, while AI is switched off. */
export const AI_OFF = 'AI is switched off.';

/** The link beside it: the master switch, from where the feature is. */
export const AI_SWITCH_ON = 'Switch it on';

/** Settings → AI → Use AI. */
export const AI_MASTER_HINT = 'While off, nothing is ever sent anywhere.';

/** The same row with no key: the switch cannot move yet. */
export const AI_MASTER_LOCKED = 'Needs an API key first.';

/** Settings → AI → Service. */
export const AI_SERVICE_HINT = 'Which AI answers. Each one has its own key.';

/** Settings → AI → Model, for a service with a smallest model of its own. */
export function modelHint(smallest: string): string {
  return smallest === ''
    ? "The model's name, as the service lists it."
    : `Leave empty for the smallest, ${smallest}.`;
}

/** Settings → AI → Address, for a service of one's own. */
export const AI_ADDRESS_HINT =
  'Any service that speaks the OpenAI chat format, or a model running on this machine.';

/** Settings → AI, under the key field, while there is none. */
export function keyWhy(keysFrom: string): string {
  return `Your own key, from ${keysFrom}. Asking is billed to it.`;
}

/** Settings → AI, once a key is stored. */
export const AI_KEY_SAVED = 'A key is saved.';

/** Settings → AI: what the app does and does not do with it. */
export const AI_PROMISE =
  'AI is asked only when you click something that says so, and is sent only what that says.';

/** Settings → AI: what it costs to ask. */
export const AI_CHEAP =
  'It asks the smallest model for a line or two at a time, so asking costs a small fraction of a cent.';

/** Settings → AI → Hide AI. */
export const AI_HIDE_HINT =
  'Takes every AI feature out of sight. One button here brings them back.';

/** Settings → AI, while hidden: all that is left of it. */
export const AI_HIDDEN = 'AI is hidden';
export const AI_HIDDEN_HINT = 'Nothing of it is shown, and nothing can be asked.';
export const AI_ENABLE = 'Enable AI';

/** The Almanac's day, put into a line. */
export const DAY_IN_A_LINE = 'The day, in a line';
export const DAY_HINT = 'One sentence about the selected day.';
export const DAY_READ = 'Read this day';

/** A question, answered from the month in view. */
export const ASK_THE_ALMANAC = 'Ask the almanac';
export const ASK_HINT = 'Ask a question about the month shown.';
export const ASK_PLACEHOLDER = 'how many hours last week?';

/** The Almanac's week, put into words. */
export const WEEK_IN_WORDS = 'This week, in words';
export const WEEK_HINT = 'Two or three sentences about the selected week.';
export const WEEK_READ = 'Read this week';

/** While an answer is on its way. */
export const WEEK_READING = 'Working…';

/** What asking sends, and to whom: said under each feature. */
export function sendsDay(to: string): string {
  return `Sends this one day to ${to}, when you ask.`;
}
export function sendsWeek(to: string): string {
  return `Sends this week of hours and nights to ${to}, when you ask.`;
}
export function sendsMonth(to: string): string {
  return `Sends your question and this month of days to ${to}.`;
}
export function sendsSentence(to: string): string {
  return `Sends this sentence to ${to}.`;
}

/** Home's plan in a sentence. */
export const TELL_THE_SUN = 'Tell the sun';

/** What it does, shown while its switch is off. */
export const TELL_HINT =
  'Type how long to stay awake, in your own words, instead of picking a time.';

/** Shown in the empty field. */
export const TELL_PLACEHOLDER = 'for 2 hours, or until 6:30 pm';

/** The button that asks, and what it says while waiting. */
export const TELL_ASK = 'Ask';
export const TELL_THINKING = 'Asking…';

/** The button that starts what was understood. */
export const TELL_START = 'Start';

/** Settings → Almanac → Break reminder. */
export const BREAK_HINT = 'A notification after this long without a 5-minute break.';

/** The link that copies the open month, and what it says once it has. */
export const COPY_MONTH = 'Copy month as CSV';
export const COPIED = 'Copied';

/** Settings → History before anything has been logged. */
export const HISTORY_EMPTY =
  'Nothing yet. Each time it keeps your laptop awake will be listed here.';

/** The link under the history list. */
export const CLEAR_HISTORY = 'Clear history';

/** Shown in the hotkey field while it waits for a combination. */
export const HOTKEY_RECORDING = 'Press keys…';

/** The Tips row for the global hotkey. */
export const HOTKEY_TIP = 'Turn on or off from any app';

/** One row of Settings → Tips. */
export interface Tip {
  keys: string;
  does: string;
}

/** Keyboard and tray (or menu bar) help shown in Settings → Tips. */
export function tips(mac: boolean): readonly Tip[] {
  if (mac) {
    return [
      { keys: 'Space', does: 'Turn staying awake on or off' },
      { keys: 'Esc', does: 'Back, or close the window' },
      { keys: '⌘,', does: 'Settings' },
      { keys: '⌘E', does: 'Wide window, and back' },
      { keys: '?', does: 'All keyboard shortcuts' },
      { keys: '⌘W', does: 'Close to the menu bar' },
      { keys: '⌘Q', does: 'Quit' },
      { keys: 'Menu bar · click', does: 'Open this window' },
      { keys: 'Menu bar · right click', does: 'Every action, as a menu' },
      { keys: 'Horizon', does: 'Click the line for a tip' },
    ];
  }
  return [
    { keys: 'Space', does: 'Turn staying awake on or off' },
    { keys: 'Esc', does: 'Back, or close to the tray' },
    { keys: 'Ctrl+,', does: 'Settings' },
    { keys: 'Ctrl+E', does: 'Wide window, and back' },
    { keys: '?', does: 'All keyboard shortcuts' },
    { keys: 'Tray · left click', does: 'Open this window' },
    { keys: 'Tray · right click', does: 'Every action, as a menu' },
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
  /** Settings → AI: where the API key is kept. */
  keyKept: string;
}

/** {@link PlatformCopy} for a Mac (`true`) or Windows. */
export function platformCopy(mac: boolean): PlatformCopy {
  const mechanism = 'the same way a video player does. You can see it with';
  const after = '. Nothing is typed and the mouse never moves.';
  if (mac) {
    return {
      closeLabel: 'Close to the menu bar',
      closeTitle: 'Close to the menu bar (⌘W). It keeps running there.',
      settingsTitle: 'Settings (⌘,)',
      launchHint: 'Starts quietly in the menu bar.',
      lengthHint: 'How long a click or Space keeps it awake.',
      hotkeyHint: 'Works in any app. Needs at least one of ⌘, ⌃, ⌥.',
      about: {
        before: `While awake, it asks macOS to stay awake, ${mechanism}`,
        code: 'pmset -g assertions',
        after,
      },
      refused: 'macOS refused the request',
      keyKept: 'Kept in the macOS Keychain, not in a file.',
    };
  }
  return {
    closeLabel: 'Close to tray',
    closeTitle: 'Close to tray (Esc). It keeps running there.',
    settingsTitle: 'Settings (Ctrl+,)',
    launchHint: 'Starts quietly in the tray.',
    lengthHint: 'How long a click or Space keeps it awake.',
    hotkeyHint: 'Works in any app. Needs at least one of Ctrl, Alt, Win.',
    about: {
      before: `While awake, it asks Windows to stay awake, ${mechanism}`,
      code: 'powercfg /requests',
      after,
    },
    refused: 'Windows refused the request',
    keyKept: 'Kept in Windows Credential Manager, not in a file.',
  };
}

/** {@link PlatformCopy} for this window. */
export const PLATFORM_COPY: PlatformCopy = platformCopy(isMac);
