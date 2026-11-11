<script lang="ts">
  /**
   * What can be laid over the window, one at a time: every key on one sheet
   * (`?`), the credits and how the app got here, and a year looked back on.
   * A sheet covers the pane under it; Esc or Back lifts it.
   */
  import { formatTotal, formatWorked, usualNight, yearReview, type Day } from '$lib/almanac';
  import {
    AUTHOR,
    CREDITS,
    CREDITS_TITLE,
    KEYS_FOOT,
    KEYS_TITLE,
    VERSIONS,
    YEAR_EMPTY,
    daysInARow,
    daysWorked,
    leavesTaken,
    tipsWith,
    yearTitle,
  } from '$lib/copy';
  import { formatClock } from '$lib/format';
  import { displayAccelerator } from '$lib/hotkey';
  import { isMac } from '$lib/platform';
  import { store } from '$lib/state.svelte';

  const keys = $derived(
    tipsWith(store.settings?.hotkey ? displayAccelerator(store.settings.hotkey, isMac) : null),
  );

  const review = $derived(
    store.sheet === 'year' && store.reading ? yearReview(store.reading, store.sheetYear) : null,
  );
  const usual = $derived(
    store.reading ? usualNight(store.reading.nights, store.reading.turn) : null,
  );
  const most = $derived(review ? Math.max(1, ...review.months) : 1);

  const part = (ms: number, options: Intl.DateTimeFormatOptions) =>
    new Date(ms).toLocaleDateString(undefined, options);
  const dateOf = (day: Day) =>
    `${part(day.start, { weekday: 'short' })} ${new Date(day.start).getDate()} ${part(day.start, { month: 'short' })}`;
  const monthName = (month: number) =>
    part(new Date(store.sheetYear, month, 1).getTime(), { month: 'long' });
  const clockOf = (minutes: number) => formatClock(new Date(2000, 0, 1, 0, minutes).getTime());

  const title = $derived(
    store.sheet === 'keys'
      ? KEYS_TITLE
      : store.sheet === 'credits'
        ? CREDITS_TITLE
        : yearTitle(store.sheetYear),
  );
</script>

<div class="sheet" role="dialog" aria-label={title}>
  <header>
    <h2 class="eyebrow">{title}</h2>
    <button type="button" class="link" onclick={() => (store.sheet = null)}>Back</button>
  </header>

  <div class="scroll">
    {#if store.sheet === 'keys'}
      <dl class="keys">
        {#each keys as tip (tip.keys)}
          <dt><kbd>{tip.keys}</kbd></dt>
          <dd>{tip.does}</dd>
        {/each}
      </dl>
      <p class="hint">{KEYS_FOOT}</p>
    {:else if store.sheet === 'credits'}
      <p class="lede">
        <span class="name">Keep me awake</span>
        <span class="version">{store.snapshot?.app_version}</span>
      </p>
      <p class="hint">Made by {AUTHOR.name} · {AUTHOR.siteLabel}</p>

      <h3 class="eyebrow">Version history</h3>
      <ol class="versions">
        {#each VERSIONS as version (version.name)}
          <li>
            <span class="dot" aria-hidden="true"></span>
            <span class="when">{version.name} <span class="date">{version.when}</span></span>
            <span class="what">{version.what}</span>
          </li>
        {/each}
      </ol>

      <h3 class="eyebrow">Made with</h3>
      <dl class="credits">
        {#each CREDITS as credit (credit.what)}
          <dt>{credit.what}</dt>
          <dd>{credit.by}</dd>
        {/each}
      </dl>
    {:else if review}
      {#if review.days === 0}
        <p class="hint">{YEAR_EMPTY}</p>
      {:else}
        <p class="figure">{formatTotal(review.worked_ms)}</p>
        <p class="hint">
          {daysWorked(review.days)} worked · {leavesTaken(review.leaves).toLowerCase()}
        </p>

        <div class="months" aria-hidden="true">
          {#each review.months as worked, month (month)}
            <span class="month">
              <span
                class="bar"
                class:busiest={month === review.busiest}
                style="height: {Math.round((worked / most) * 100)}%"
              ></span>
              <span class="letter">{monthName(month).slice(0, 1)}</span>
            </span>
          {/each}
        </div>

        <dl class="facts">
          {#if review.busiest !== null}
            <dt>Fullest month</dt>
            <dd>
              {monthName(review.busiest)} · {formatTotal(review.months[review.busiest] ?? 0)}
            </dd>
          {/if}
          {#if review.longest}
            <dt>Longest day</dt>
            <dd>{dateOf(review.longest)} · {formatWorked(review.longest.worked_ms)}</dd>
          {/if}
          {#if review.earliest?.first}
            <dt>Earliest start</dt>
            <dd>{dateOf(review.earliest)} · {formatClock(review.earliest.first)}</dd>
          {/if}
          {#if review.latest?.last}
            <dt>Latest finish</dt>
            <dd>{dateOf(review.latest)} · {formatClock(review.latest.last)}</dd>
          {/if}
          <dt>Most days in a row</dt>
          <dd>{daysInARow(review.run)}</dd>
          {#if review.over_ms >= 60_000}
            <dt>Outside your hours</dt>
            <dd>{formatTotal(review.over_ms)}</dd>
          {/if}
          {#if usual}
            <dt>Usually away</dt>
            <dd>{clockOf(usual.from_minutes)} – {clockOf(usual.to_minutes)}</dd>
          {/if}
        </dl>
      {/if}
    {/if}
  </div>
</div>

<style>
  .sheet {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    background: var(--paper);
    animation: lay var(--t-mid) var(--ease-out);
  }

  @keyframes lay {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 14px 24px 10px;
    border-bottom: 1px solid var(--hair);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 24px 24px;
  }

  .link {
    padding: 3px 0;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--ink);
    text-decoration-color: var(--ink);
  }

  h3 {
    margin: 22px 0 8px;
  }

  .keys,
  .credits,
  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 9px 16px;
    align-items: baseline;
    font-size: 13px;
  }

  .facts {
    margin-top: 18px;
  }

  .keys dt,
  .credits dt,
  .facts dt {
    color: var(--ink-2);
  }

  .keys dd,
  .credits dd,
  .facts dd {
    color: var(--ink);
  }

  kbd {
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--paper-2);
    box-shadow: inset 0 0 0 1px var(--hair);
    font-family: var(--font-ui);
    font-size: 12px;
    color: var(--ink);
    white-space: nowrap;
  }

  .keys + .hint {
    margin-top: 18px;
  }

  .lede {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .name,
  .figure {
    font-family: var(--font-display);
    color: var(--ink);
  }

  .name {
    font-size: 22px;
    font-style: italic;
  }

  .figure {
    font-size: 40px;
    line-height: 1.1;
  }

  .version {
    font-size: 12px;
    color: var(--ink-3);
  }

  /* The versions hang off one line, newest at the top: the horizon, stood up. */
  .versions {
    position: relative;
    display: grid;
    gap: 12px;
    padding-left: 16px;
    list-style: none;
  }

  .versions::before {
    content: '';
    position: absolute;
    left: 3px;
    top: 6px;
    bottom: 6px;
    width: 1px;
    background: var(--hair-strong);
  }

  .versions li {
    position: relative;
    display: grid;
    gap: 1px;
    font-size: 13px;
  }

  .dot {
    position: absolute;
    left: -16px;
    top: 6px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ink-3);
    box-shadow: 0 0 0 2px var(--paper);
  }

  .versions li:first-child .dot {
    background: var(--sun);
  }

  .when {
    color: var(--ink);
  }

  .date {
    margin-left: 4px;
    font-size: 11px;
    color: var(--ink-3);
  }

  .what {
    color: var(--ink-2);
  }

  /* Twelve months as twelve bars on one line; the fullest is the sun's. */
  .months {
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 5px;
    height: 74px;
    margin-top: 18px;
    border-bottom: 1px solid var(--hair-strong);
  }

  .month {
    position: relative;
    display: flex;
    align-items: flex-end;
  }

  .bar {
    width: 100%;
    min-height: 0;
    border-radius: 2px 2px 0 0;
    background: var(--ink);
  }

  .bar.busiest {
    background: var(--sun);
  }

  .letter {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -17px;
    font-size: 10px;
    text-align: center;
    color: var(--ink-3);
  }

  .months + .facts {
    margin-top: 34px;
  }
</style>
