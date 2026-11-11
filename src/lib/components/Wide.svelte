<script lang="ts">
  /**
   * The wide window: the day drawn as the horizon, and under it a quiet
   * ledger of three columns. Now (what it is doing and what to do), Watching
   * (stay awake while…, live), and This week. Settings and the Almanac open
   * in a column under the same sky.
   */
  import Almanac from './Almanac.svelte';
  import DaySky from './DaySky.svelte';
  import DurationRule from './DurationRule.svelte';
  import Segmented from './Segmented.svelte';
  import Settings from './Settings.svelte';
  import Switch from './Switch.svelte';
  import {
    AFTER_END,
    EXTENDED,
    EXTEND_LABEL,
    KEEP_IT_UP,
    KEPT_UP,
    WATCH_ROWS,
    WATCH_SHORT,
    WIDE_KEEP,
    WIDE_SUGGEST,
    WIDE_TO_STANDARD,
    afterEndOptions,
    doors,
    rate,
    timeLeft,
  } from '$lib/copy';
  import { formatDuration } from '$lib/format';
  import { displayAccelerator } from '$lib/hotkey';
  import { isMac } from '$lib/platform';
  import { weekOfDomes } from '$lib/sky';
  import { store } from '$lib/state.svelte';
  import type { WatchSettings } from '$lib/types';

  const snapshot = $derived(store.snapshot);
  const settings = $derived(store.settings);
  const battery = $derived(snapshot?.battery ?? null);
  const watch = $derived(snapshot?.watch ?? null);
  const offerExtend = $derived(store.sunsetSoon && snapshot?.paused === null);

  type Condition = 'transfers' | 'busy' | 'full_screen' | 'display' | 'plugged_in';
  // Opened wide for someone who never chose: suggest the standard window.
  const unchosen = $derived(settings !== null && settings.wide === null);
  const conditions = $derived(
    (['transfers', 'busy', 'full_screen', 'display', 'plugged_in'] as const).filter(
      (key) => battery?.present || (key !== 'display' && key !== 'plugged_in'),
    ),
  );
  const holding = $derived(watch?.reason?.kind ?? null);
  const kindOf: Record<Condition, string> = {
    transfers: 'transfer',
    busy: 'busy',
    full_screen: 'full_screen',
    display: 'display',
    plugged_in: 'plugged_in',
  };
  function reading(key: Condition): string {
    if (!settings?.watch[key] || !watch) return '';
    if (key === 'transfers') return watch.transfer_bps === null ? '' : rate(watch.transfer_bps);
    if (key === 'busy') return watch.cpu_percent === null ? '' : `${watch.cpu_percent}%`;
    return holding === kindOf[key] ? 'now' : '';
  }
  function setWatch(patch: Partial<WatchSettings>) {
    if (!settings) return;
    void store.updateSettings({ watch: { ...settings.watch, ...patch } });
  }
  const nothingWatched = $derived(
    settings !== null &&
      settings.watch.apps.length === 0 &&
      conditions.every((key) => !settings.watch[key]),
  );

  const week = $derived(weekOfDomes(store.history, store.now, store.reading?.turn ?? 0));
  const pct = (n: number) => `${(n * 100).toFixed(2)}%`;

  const ways = $derived(
    doors(
      settings?.hotkey ? displayAccelerator(settings.hotkey, isMac) : null,
      settings?.overnight_guard_hours ?? null,
      store.update?.version ?? null,
    ),
  );
</script>

<div class="wide">
  {#if unchosen}
    <div class="suggest" role="region" aria-label="Window size">
      <p>{WIDE_SUGGEST}</p>
      <button type="button" class="primary" onclick={() => void store.setWide(false)}>
        {WIDE_TO_STANDARD}
      </button>
      <button type="button" class="keep" onclick={() => void store.updateSettings({ wide: true })}
        >{WIDE_KEEP}</button
      >
    </div>
  {/if}
  <DaySky height={unchosen ? 200 : 236} />

  {#if store.view === 'home'}
    <div class="ledger">
      <section class="now" aria-live="polite">
        <h1>{store.status.title}</h1>
        <p class="detail" class:whisper={store.whisper !== null}>
          {store.whisper ?? store.status.detail}
          {#if store.guardAsking && store.whisper === null}
            <button
              type="button"
              class="link"
              onclick={() => {
                void store.keepItUp();
                store.say(KEPT_UP);
              }}>{KEEP_IT_UP}</button
            >
          {:else if offerExtend && store.whisper === null}
            <button
              type="button"
              class="link"
              onclick={() => {
                void store.extend();
                store.say(EXTENDED);
              }}>{EXTEND_LABEL}</button
            >
          {/if}
        </p>
        <div class="rule"><DurationRule /></div>
        {#if settings}
          <div class="ending">
            <span class="label">{AFTER_END}</span>
            <Segmented
              options={afterEndOptions(isMac)}
              value={isMac && settings.after_end === 'lock' ? 'screen_off' : settings.after_end}
              label={AFTER_END}
              onchange={(v) => void store.updateSettings({ after_end: v })}
            />
          </div>
        {/if}
      </section>

      <section class="watching" aria-label="Stay awake while">
        <h2 class="eyebrow">Stay awake while</h2>
        {#if nothingWatched}
          <p class="hint empty">
            Nothing is being watched. Switch a row on to stay awake while it happens.
          </p>
        {/if}
        {#if settings}
          <ul>
            {#each settings.watch.apps as app (app)}
              {@const live = watch?.running.includes(app) ?? false}
              <li>
                <span class="dot" class:holding={live && holding === 'app'} class:on={true}></span>
                <span class="name">{app}</span>
                <span class="value">{live ? 'running' : 'not running'}</span>
              </li>
            {/each}
            {#each conditions as key (key)}
              {@const on = settings.watch[key]}
              <li class:off={!on}>
                <span class="dot" class:on class:holding={on && holding === kindOf[key]}></span>
                <span class="name" title={WATCH_ROWS[key].hint}>{WATCH_SHORT[key]}</span>
                <span class="value">{reading(key)}</span>
                <Switch
                  checked={on}
                  label={WATCH_ROWS[key].label}
                  onchange={(v) => setWatch({ [key]: v })}
                />
              </li>
            {/each}
          </ul>
          <button type="button" class="link quiet" onclick={() => store.openSettings('watch')}>
            {settings.watch.apps.length === 0 ? 'Add an app' : 'Change the apps'}
          </button>
        {/if}
      </section>

      <section class="week" aria-label="This week">
        <h2 class="eyebrow">This week</h2>
        <ol>
          {#each week as d (d.key)}
            <li class:today={d.today} class:ahead={d.ahead}>
              <span class="day">{d.label}</span>
              <span class="line">
                {#each d.domes as dome (dome.start_ms)}
                  <span
                    class="kept"
                    style="left: {pct(dome.from)}; width: {pct(
                      Math.max(dome.to - dome.from, 0.004),
                    )}"
                  ></span>
                {/each}
              </span>
              <span class="total">
                {d.ahead ? '' : d.awake_ms < 60_000 ? '–' : formatDuration(d.awake_ms / 1000)}
              </span>
            </li>
          {/each}
        </ol>
        {#if battery?.present}
          <div class="battery">
            <span class="label">{battery.on_ac ? 'Plugged in' : 'On battery'}</span>
            <span class="track"
              ><span class="fill" style="width: {battery.percent ?? 0}%"></span></span
            >
            <span class="value">
              {battery.percent === null ? '' : `${battery.percent}%`}
              {#if !battery.on_ac && battery.minutes_left !== null}
                · {timeLeft(battery.minutes_left)}
              {/if}
            </span>
          </div>
        {:else if battery}
          <p class="hint mains">On mains power: no battery rules apply.</p>
        {/if}
      </section>
    </div>

    <nav class="doors" aria-label="More in Settings">
      {#each ways as door (door.section)}
        <button type="button" title={door.title} onclick={() => store.openSettings(door.section)}>
          {door.label}
        </button>
      {/each}
    </nav>
  {:else}
    <!-- Settings and the Almanac read best as a column: one under the sky. -->
    <div class="column">
      {#if store.view === 'settings'}<Settings />{:else}<Almanac />{/if}
    </div>
  {/if}
</div>

<style>
  .wide {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 40px);
    min-height: 0;
  }

  /* The one loud thing outside the sky: ink across the top, until chosen. */
  .suggest {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 16px 10px 28px;
    background: var(--ink);
    color: var(--paper);
    flex: none;
  }

  .suggest p {
    flex: 1;
    margin: 0;
  }

  .suggest .primary {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    background: var(--paper);
    color: var(--ink);
    font-weight: 600;
  }

  .suggest .primary:focus-visible,
  .suggest .keep:focus-visible {
    outline: 1.5px solid var(--paper);
    outline-offset: 2px;
  }

  .suggest .keep {
    color: var(--paper);
    opacity: 0.75;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .ledger {
    display: grid;
    grid-template-columns: minmax(300px, 1.15fr) minmax(250px, 1fr) minmax(230px, 0.9fr);
    flex: 1;
    min-height: 0;
  }

  .ledger > section {
    padding: 20px 28px 10px;
    min-width: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .ledger > section::-webkit-scrollbar {
    display: none;
  }

  .ledger > section + section {
    border-left: 1px solid var(--hair);
  }

  /* Now: the word, what it means, and the two things to set. */
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 54px;
    font-weight: 400;
    font-variation-settings:
      'opsz' 72,
      'SOFT' 50;
    line-height: 1;
    letter-spacing: -0.01em;
  }

  .detail {
    margin: 10px 0 0;
    color: var(--ink-2);
    min-height: 2.6em;
  }

  .detail.whisper {
    font-family: var(--font-display);
    font-style: italic;
  }

  .link {
    margin-left: 6px;
    color: var(--ink);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 3px;
  }

  .link:hover {
    text-decoration-color: var(--ink);
  }

  .rule {
    margin: 8px -24px 0;
  }

  .ending {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--hair);
  }

  .label {
    color: var(--ink);
  }

  /* Watching: each condition, its state as a dot, its reading, its switch. */
  .eyebrow {
    margin: 0 0 10px;
  }

  ul,
  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .watching li {
    display: grid;
    grid-template-columns: 10px 1fr auto auto;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    border-top: 1px solid var(--hair);
  }

  .watching li:first-child {
    border-top: 0;
  }

  .watching li.off .name {
    color: var(--ink-2);
  }

  /* A hairline ring off, an ink ring on and quiet, the sun's colour when
     it is what keeps the laptop awake. */
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1px solid var(--hair-strong);
  }

  .dot.on {
    border-color: var(--ink);
  }

  .dot.holding {
    border-color: var(--sun);
    background: var(--sun);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .value {
    color: var(--ink-2);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .watching li:not(:has(:global(.switch))) .value {
    grid-column: 3 / 5;
  }

  .empty {
    margin: 0 0 10px;
  }

  .quiet {
    margin: 10px 0 0;
    color: var(--ink-2);
    font-size: 12px;
  }

  /* The week: a small horizon a day, with the stretches as ink. */
  .week li {
    position: relative;
    display: grid;
    grid-template-columns: 34px 1fr 56px;
    align-items: center;
    gap: 10px;
    height: 26px;
    color: var(--ink-2);
  }

  .week li.today {
    color: var(--ink);
  }

  .week li.ahead {
    color: var(--ink-3);
  }

  .day {
    font-size: 12px;
  }

  .line {
    position: relative;
    height: 8px;
    border-bottom: 1px solid var(--hair-strong);
  }

  .today .line {
    border-bottom-color: var(--ink-3);
  }

  .kept {
    position: absolute;
    bottom: -1px;
    height: 3px;
    background: var(--ink);
  }

  .total {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .battery {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 12px;
    align-items: center;
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--hair);
  }

  .battery .value {
    grid-column: 1 / 3;
    text-align: left;
  }

  .track {
    height: 4px;
    border-radius: 2px;
    background: var(--paper-2);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--ink);
  }

  .mains {
    margin: 16px 0 0;
    padding-top: 14px;
    border-top: 1px solid var(--hair);
  }

  .doors {
    display: flex;
    gap: 20px;
    padding: 10px 28px 14px;
    border-top: 1px solid var(--hair);
    flex: none;
  }

  .doors button {
    color: var(--ink-2);
    font-size: 12px;
  }

  .doors button:hover {
    color: var(--ink);
  }

  .column {
    flex: 1;
    min-height: 0;
    width: min(100%, 520px);
    margin: 0 auto;
    overflow-y: auto;
    border-left: 1px solid var(--hair);
    border-right: 1px solid var(--hair);
  }

  @media (max-width: 940px) {
    .ledger > section {
      padding: 20px 20px 10px;
    }

    h1 {
      font-size: 46px;
    }
  }
</style>
