<script lang="ts">
  /**
   * The almanac: a book of days. The day being read comes first, as a word
   * or a figure and its strip; then its week, its month, and what they add
   * up to. Everything here is derived from the spans the app keeps; the pane
   * holds only which day is being read and which month is open.
   */
  import ClaudeGate from './ClaudeGate.svelte';
  import DayStrip from './DayStrip.svelte';
  import MonthGrid from './MonthGrid.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import {
    WORKED_FROM_MS,
    daysCsv,
    formatTotal,
    formatWorked,
    monthGrid,
    monthOf,
    read,
    startOfKey,
    stretchOn,
    stripOf,
    tally,
    usualNight,
    weekDigest,
    weekOf,
    yearOf,
    type Day,
    type Tally,
  } from '$lib/almanac';
  import {
    ALMANAC_BEGINS,
    ALMANAC_CLOSED,
    ALMANAC_FOOT,
    ALMANAC_HINT,
    ALMANAC_SETTINGS,
    CLEAR_ALMANAC,
    COPIED,
    COPY_MONTH,
    CORRECT_HOURS,
    DAY_WORDS,
    MARK_LEAVE,
    NOBODY_HERE,
    NOT_A_LEAVE,
    USE_WHAT_WAS_READ,
    WEEK_HINT,
    WEEK_IN_WORDS,
    WEEK_READ,
    WEEK_READING,
    WEEK_SENDS,
    dayDetail,
    daysWorked,
    leavesTaken,
    nightLine,
  } from '$lib/copy';
  import { formatClock, minutesToTime, timeToMinutes } from '$lib/format';
  import { store } from '$lib/state.svelte';

  const settings = $derived(store.settings?.almanac ?? null);
  const reading = $derived(
    store.almanac && settings ? read(store.almanac, settings, store.almanacAt) : null,
  );

  /** The key of the day being read; `null` is today, whichever day that is. */
  let chosen = $state<string | null>(null);
  /** A month opened with the arrows; `null` follows the day being read. */
  let stepped = $state<{ year: number; month: number } | null>(null);

  const day = $derived.by(() => {
    if (!reading) return null;
    const start = chosen === null ? null : startOfKey(chosen, reading.turn);
    return start === null ? reading.today : reading.dayAt(start);
  });
  const shown = $derived.by(() => {
    if (stepped) return stepped;
    const start = new Date(day?.start ?? store.almanacAt);
    return { year: start.getFullYear(), month: start.getMonth() };
  });

  const week = $derived(reading && day ? weekOf(reading, day) : []);
  const month = $derived(reading ? monthOf(reading, shown.year, shown.month) : []);
  const rows = $derived(reading ? monthGrid(reading, shown.year, shown.month) : []);
  const year = $derived(reading ? tally(yearOf(reading, shown.year)) : null);
  const usual = $derived(reading ? usualNight(reading.nights, reading.turn) : null);

  /** The week put into words, and the week it is of, so another week starts clean. */
  let words = $state<{ of: string; text: string } | null>(null);
  let reading_week = $state(false);
  const weekKey = $derived(week[0]?.key ?? '');

  async function readWeek() {
    if (!reading || reading_week) return;
    reading_week = true;
    const of = weekKey;
    const text = await store.readWeek(weekDigest(reading, week));
    if (text !== null) words = { of, text };
    reading_week = false;
  }

  /** The open month, copied as CSV for a timesheet. */
  let copied = $state(false);
  async function copyMonth() {
    try {
      await navigator.clipboard.writeText(daysCsv(month));
      copied = true;
      setTimeout(() => (copied = false), 2400);
    } catch {
      store.error = 'The month could not be copied.';
    }
  }

  /** The hours being entered for the day being read; `null` while not correcting. */
  let editing = $state<{ from: string; until: string } | null>(null);

  // Left and come back: it opens on today again.
  $effect(() => {
    if (store.view !== 'almanac') {
      chosen = null;
      stepped = null;
      editing = null;
    }
  });

  const minutesOf = (ms: number) => {
    const d = new Date(ms);
    return d.getHours() * 60 + d.getMinutes();
  };

  /** Opens the editor on what the day has, or on the working hours. */
  function correct(d: Day) {
    if (!settings) return;
    editing = {
      from: minutesToTime(d.first === null ? settings.start_minutes : minutesOf(d.first)),
      until: minutesToTime(d.last === null ? settings.end_minutes : minutesOf(d.last)),
    };
  }

  function saveHours(event: Event, d: Day) {
    event.preventDefault();
    if (!editing) return;
    const from = timeToMinutes(editing.from);
    const until = timeToMinutes(editing.until);
    if (from === null || until === null) return;
    void store.fixDay(d.key, stretchOn(d, from, until));
    editing = null;
  }

  /** Nobody was here: a correction of no length. */
  function emptyDay(d: Day) {
    void store.fixDay(d.key, { from_unix_ms: d.start, to_unix_ms: d.start });
    editing = null;
  }

  function uncorrect(d: Day) {
    void store.fixDay(d.key, null);
    editing = null;
  }

  // Esc leaves the editor, not the pane.
  function onEditorKey(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.stopPropagation();
    editing = null;
  }

  // Today's figure grows while the pane is on screen: one read a minute,
  // and none once it is closed or the window is hidden.
  $effect(() => {
    if (store.view !== 'almanac') return;
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible') void store.loadAlmanac();
    }, 60_000);
    return () => clearInterval(timer);
  });

  const part = (ms: number, options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(undefined, options).format(new Date(ms));

  /** `Tue 10 Nov`, in that order whatever the locale. */
  function dateOf(d: Day): string {
    return `${part(d.start, { weekday: 'short' })} ${new Date(d.start).getDate()} ${part(d.start, { month: 'short' })}`;
  }

  /** The eyebrow over the figure: today and yesterday say so first. */
  function nameOf(d: Day): string {
    if (!reading) return '';
    if (d.today) return `Today · ${dateOf(d)}`;
    return d.end === reading.today.start ? `Yesterday · ${dateOf(d)}` : dateOf(d);
  }

  /** A day in a word or a figure: its hours, or what it was instead. */
  function figureOf(d: Day): string {
    if (d.kind === 'worked' || (d.today && d.kind !== 'leave')) return formatWorked(d.worked_ms);
    return DAY_WORDS[d.kind];
  }

  function detailOf(d: Day): string {
    return dayDetail({
      kind: d.kind,
      today: d.today,
      ahead: reading !== null && d.start > reading.today.start,
      marked: d.mark !== null,
      fixed: d.fixed,
      first: d.first === null ? null : formatClock(d.first),
      last: d.last === null ? null : formatClock(d.last),
      over: d.over_ms >= 60_000 ? formatWorked(d.over_ms) : null,
    });
  }

  function nightOf(d: Day): string | null {
    if (!d.night) return null;
    const { from, to } = d.night;
    return nightLine(formatClock(from), formatClock(to), formatWorked(to - from));
  }

  /** The whole day in one sentence, for a screen reader and a tooltip. */
  function describe(d: Day): string {
    const what = d.kind === 'worked' ? formatWorked(d.worked_ms) : DAY_WORDS[d.kind].toLowerCase();
    return `${dateOf(d)}: ${what}`;
  }

  /** A week's row says less: the hours, or one quiet word. */
  function totalOf(d: Day): string {
    if (d.kind === 'worked') return formatWorked(d.worked_ms);
    if (d.kind === 'leave') return 'leave';
    return d.kind === 'rest' ? 'rest' : '–';
  }

  /** What a week or a month adds up to; nothing at all before there is anything. */
  function sumOf(t: Tally, withLeaves: boolean): string {
    const parts = t.days === 0 ? [] : [formatTotal(t.worked_ms), daysWorked(t.days)];
    if (withLeaves && t.leaves > 0) parts.push(leavesTaken(t.leaves).toLowerCase());
    return parts.join(' · ');
  }

  const clockOf = (minutes: number) => formatClock(new Date(2000, 0, 1, 0, minutes).getTime());

  function select(d: Day) {
    chosen = d.today ? null : d.key;
    stepped = null;
    editing = null;
  }

  function step(by: number) {
    const next = new Date(shown.year, shown.month + by, 1);
    stepped = { year: next.getFullYear(), month: next.getMonth() };
  }

  /**
   * Leave, or not. Taking leave off an empty working day has to say so
   * outright, or it would read as leave again on its own.
   */
  function toggleLeave(d: Day) {
    if (!reading) return;
    if (d.kind !== 'leave') return void store.markDay(d.key, 'leave');
    const wouldReadAsLeave =
      d.working && d.start < reading.today.start && d.worked_ms < WORKED_FROM_MS;
    void store.markDay(d.key, wouldReadAsLeave ? 'clear' : null);
  }
</script>

<div class="scroll">
  {#if settings && !settings.enabled}
    <p class="aside">{ALMANAC_CLOSED}</p>
    <Row label="Keep an almanac" hint={ALMANAC_HINT}>
      <Switch
        checked={false}
        label="Keep an almanac"
        onchange={(v) => {
          if (!settings) return;
          void store
            .updateSettings({ almanac: { ...settings, enabled: v } })
            .then(() => store.loadAlmanac());
        }}
      />
    </Row>
  {:else if reading && day}
    <section class="day">
      <div class="over">
        <h2 class="eyebrow">{nameOf(day)}</h2>
        {#if !day.today}
          <button type="button" class="link" onclick={() => select(reading.today)}>Today</button>
        {/if}
      </div>
      {#key day.key}
        <h1>{figureOf(day)}</h1>
        <p class="detail">{detailOf(day)}</p>
        {#if nightOf(day)}
          <p class="detail night">{nightOf(day)}</p>
        {/if}
      {/key}
      <div class="strip">
        <DayStrip
          strip={stripOf(reading, day)}
          label={`${describe(day)}. ${detailOf(day)}`}
          ticks={stripOf(reading, day).ticks.map((tick) =>
            part(tick.unix_ms, { hour: 'numeric' }).toLowerCase(),
          )}
        />
      </div>
      {#if editing}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <form class="hours" onsubmit={(e) => saveHours(e, day)} onkeydown={onEditorKey}>
          <label>
            <span>From</span>
            <input type="time" required bind:value={editing.from} />
          </label>
          <label>
            <span>Until</span>
            <input type="time" required bind:value={editing.until} />
          </label>
          <button type="submit" class="save">Save</button>
        </form>
        <p class="links">
          <button type="button" class="link" onclick={() => emptyDay(day)}>{NOBODY_HERE}</button>
          {#if day.fixed}
            <button type="button" class="link" onclick={() => uncorrect(day)}>
              {USE_WHAT_WAS_READ}
            </button>
          {/if}
          <button type="button" class="link" onclick={() => (editing = null)}>Cancel</button>
        </p>
      {:else}
        <p class="links">
          <button type="button" class="link" onclick={() => toggleLeave(day)}>
            {day.kind === 'leave' ? NOT_A_LEAVE : MARK_LEAVE}
          </button>
          <button type="button" class="link" onclick={() => correct(day)}>{CORRECT_HOURS}</button>
        </p>
      {/if}
      {#if store.almanac?.spans.length === 0}
        <p class="aside">{ALMANAC_BEGINS}</p>
      {/if}
    </section>

    <section>
      <div class="over">
        <h2 class="eyebrow">Week</h2>
        <span class="sum">{sumOf(tally(week), false)}</span>
      </div>
      <ol class="week">
        {#each week as d (d.key)}
          <li>
            <button
              type="button"
              class="weekday"
              class:today={d.today}
              aria-pressed={d.key === day.key}
              aria-label={describe(d)}
              onclick={() => select(d)}
            >
              <span class="name">
                {part(d.start, { weekday: 'short' })}
                <span class="date">{new Date(d.start).getDate()}</span>
              </span>
              <DayStrip strip={stripOf(reading, d)} size="row" label="" />
              <span class="total" class:quiet={d.kind !== 'worked'}>{totalOf(d)}</span>
            </button>
          </li>
        {/each}
      </ol>
    </section>

    <section>
      <ClaudeGate feature="week_in_words" title={WEEK_IN_WORDS} hint={WEEK_HINT}>
        {#if words && words.of === weekKey}
          <p class="words">{words.text}</p>
        {/if}
        <p class="links ask">
          <button
            type="button"
            class="link"
            disabled={reading_week}
            onclick={() => void readWeek()}
          >
            {reading_week ? WEEK_READING : WEEK_READ}
          </button>
          <span class="sends">{WEEK_SENDS}</span>
        </p>
      </ClaudeGate>
    </section>

    <section>
      <div class="over">
        <div class="pager">
          <button type="button" aria-label="Previous month" onclick={() => step(-1)}>
            <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M7.5 2.5 4 6l3.5 3.5" /></svg>
          </button>
          <h2 class="eyebrow">
            {part(new Date(shown.year, shown.month, 1).getTime(), {
              month: 'long',
              year: 'numeric',
            })}
          </h2>
          <button type="button" aria-label="Next month" onclick={() => step(1)}>
            <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M4.5 2.5 8 6l-3.5 3.5" /></svg>
          </button>
        </div>
        <span class="sum">{sumOf(tally(month), true)}</span>
      </div>
      <MonthGrid {rows} selected={day.key} {describe} onselect={select} />
    </section>

    <section>
      <dl class="tally">
        {#if year}
          <dt>{shown.year}</dt>
          <dd>{leavesTaken(year.leaves)} · {daysWorked(year.days)} worked</dd>
        {/if}
        {#if usual}
          <dt>Usual night</dt>
          <dd>{clockOf(usual.from_minutes)} – {clockOf(usual.to_minutes)}</dd>
        {/if}
      </dl>
      <p class="hint foot">{ALMANAC_FOOT}</p>
      <p class="links">
        <button type="button" class="link" onclick={() => store.openSettings('almanac')}>
          {ALMANAC_SETTINGS}
        </button>
        <button type="button" class="link" onclick={() => void store.clearAlmanac()}>
          {CLEAR_ALMANAC}
        </button>
      </p>
      <p class="links">
        <button type="button" class="link" onclick={() => void copyMonth()}>
          {copied ? COPIED : COPY_MONTH}
        </button>
      </p>
    </section>
  {/if}
</div>

<style>
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 24px 16px;
    scrollbar-width: thin;
    scrollbar-color: var(--hair-strong) transparent;
  }

  section + section {
    margin-top: 20px;
    padding-top: 12px;
    border-top: 1px solid var(--hair);
  }

  /* A heading on the left, what it adds up to on the right. */
  .over {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    min-height: 22px;
  }

  h2 {
    margin: 0;
    font-weight: 500;
  }

  .sum {
    font-size: 12px;
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The day's figure speaks in the voice of Home's status word. */
  h1 {
    margin: 6px 0 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: 36px;
    line-height: 1.05;
    letter-spacing: -0.01em;
    font-variant-numeric: tabular-nums;
    font-variation-settings:
      'opsz' 72,
      'SOFT' 50;
    animation: settle var(--t-mid) var(--ease-out);
  }

  .detail {
    margin: 6px 0 0;
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
    animation: settle var(--t-mid) var(--ease-out);
  }

  .detail.night {
    margin-top: 1px;
    color: var(--ink-3);
  }

  @keyframes settle {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .strip {
    margin: 18px 0 8px;
  }

  .link {
    padding: 3px 0;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
    transition:
      color var(--t-fast) var(--ease),
      text-decoration-color var(--t-fast) var(--ease);
  }

  .link:hover {
    color: var(--ink);
    text-decoration-color: var(--ink);
  }

  /* Correcting a day: two times and a word, where the links were. */
  .hours {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .hours label {
    display: grid;
    gap: 2px;
    font-size: 11px;
    color: var(--ink-3);
  }

  .hours input {
    font-size: 12px;
    color: var(--ink);
  }

  .save {
    margin-left: auto;
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--paper);
    font-size: 12px;
  }

  /* What Claude said of the week: said, so in the whisper's voice. */
  .words {
    margin: 8px 0 0;
    font-family: var(--font-display);
    font-style: italic;
    font-size: 14px;
    line-height: 1.45;
    color: var(--ink);
    text-wrap: pretty;
    user-select: text;
    cursor: text;
  }

  .ask {
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .sends {
    font-size: 11px;
    color: var(--ink-3);
  }

  .link:disabled {
    color: var(--ink-3);
    text-decoration-color: transparent;
  }

  .aside {
    margin: 10px 0 0;
    font-family: var(--font-display);
    font-style: italic;
    font-size: 13px;
    color: var(--ink-3);
  }

  .week {
    margin: 6px -8px 0;
    padding: 0;
    list-style: none;
  }

  .weekday {
    display: grid;
    grid-template-columns: 46px 1fr 54px;
    align-items: center;
    column-gap: 10px;
    width: 100%;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    color: var(--ink-2);
    text-align: left;
    font-variant-numeric: tabular-nums;
    transition: background-color var(--t-fast) var(--ease);
  }

  .weekday:hover {
    background: var(--hair);
  }

  .weekday[aria-pressed='true'] {
    background: var(--paper-2);
    color: var(--ink);
  }

  .name {
    display: flex;
    justify-content: space-between;
    white-space: nowrap;
  }

  .weekday.today .name {
    color: var(--ink);
    font-weight: 600;
  }

  .date {
    color: var(--ink-3);
    font-weight: 400;
  }

  .total {
    color: var(--ink);
    text-align: right;
    white-space: nowrap;
  }

  .total.quiet {
    color: var(--ink-3);
  }

  .pager {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: -6px;
  }

  .pager h2 {
    min-width: 108px;
    text-align: center;
  }

  .pager button {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    color: var(--ink-2);
    border-radius: 4px;
  }

  .pager button:hover {
    color: var(--ink);
    background: var(--hair);
  }

  .pager svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  section :global(.month) {
    margin-top: 8px;
  }

  .tally {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 14px;
    row-gap: 6px;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .tally dt {
    color: var(--ink);
  }

  .tally dd {
    margin: 0;
    color: var(--ink-2);
  }

  .foot {
    margin: 12px 0 0;
  }

  .links {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin: 6px 0 0;
  }

  .day .links {
    margin-top: 0;
  }

  .day .hours + .links {
    margin-top: 6px;
  }
</style>
