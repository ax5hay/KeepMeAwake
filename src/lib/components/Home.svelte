<script lang="ts">
  import DayStrip from './DayStrip.svelte';
  import DurationRule from './DurationRule.svelte';
  import Row from './Row.svelte';
  import Scene from './Scene.svelte';
  import Switch from './Switch.svelte';
  import { formatWorked, stripOf } from '$lib/almanac';
  import {
    DAY_WORDS,
    EXTEND_LABEL,
    EXTENDED,
    KEEP_IT_UP,
    KEPT_UP,
    OPEN_ALMANAC,
    TODAY_SO_FAR,
    doors,
  } from '$lib/copy';
  import { batteryText } from '$lib/format';
  import { displayAccelerator } from '$lib/hotkey';
  import { isMac } from '$lib/platform';
  import { store } from '$lib/state.svelte';

  const settings = $derived(store.settings);
  const snapshot = $derived(store.snapshot);
  const lowBatteryOn = $derived(settings?.pause_below_battery_percent !== null);
  const lowBatteryHint = $derived.by(() => {
    if (!snapshot || !settings) return '';
    const threshold = settings.pause_below_battery_percent;
    const rule = threshold === null ? 'Off' : `Below ${threshold}%`;
    return `${rule} · ${batteryText(snapshot)}`;
  });
  const ways = $derived(
    doors(
      settings?.hotkey ? displayAccelerator(settings.hotkey, isMac) : null,
      settings?.overnight_guard_hours ?? null,
      store.update?.version ?? null,
    ),
  );
  // Your day beside the laptop's: today's hours, and the day as a horizon.
  const reading = $derived(store.reading);
  const today = $derived(reading?.today ?? null);
  const figure = $derived(
    today === null ? '' : today.kind === 'leave' ? DAY_WORDS.leave : formatWorked(today.worked_ms),
  );
  // Paused, the line says why it holds off; offering more time there reads oddly.
  const offerExtend = $derived(store.sunsetSoon && snapshot?.paused === null);
</script>

<Scene />

<!-- The day as a small horizon, under the big one. -->
{#if reading && today}
  <button
    type="button"
    class="day"
    title={OPEN_ALMANAC}
    aria-label={OPEN_ALMANAC}
    onclick={() => store.openAlmanac()}
  >
    <DayStrip strip={stripOf(reading, today)} size="row" label="" />
  </button>
{/if}

<section class="status" aria-live="polite">
  <div class="head">
    <h1>{store.status.title}</h1>
    {#if today}
      <button
        type="button"
        class="today"
        title={OPEN_ALMANAC}
        aria-label="{TODAY_SO_FAR}: {figure}. {OPEN_ALMANAC}"
        onclick={() => store.openAlmanac()}
      >
        <span class="eyebrow">{TODAY_SO_FAR}</span>
        <span class="figure">{figure}</span>
      </button>
    {/if}
  </div>
  {#key store.whisper}
    <p class:whisper={store.whisper !== null}>
      {store.whisper ?? store.status.detail}
      {#if store.guardAsking && store.whisper === null}
        <button
          type="button"
          class="extend"
          onclick={() => {
            void store.keepItUp();
            store.say(KEPT_UP);
          }}>{KEEP_IT_UP}</button
        >
      {:else if offerExtend && store.whisper === null}
        <button
          type="button"
          class="extend"
          onclick={() => {
            void store.extend();
            store.say(EXTENDED);
          }}>{EXTEND_LABEL}</button
        >
      {/if}
    </p>
  {/key}
</section>

<DurationRule />

<section class="rows">
  <Row label="Keep the screen on" hint="Off lets the display dim while the laptop stays awake.">
    <Switch
      checked={settings?.keep_display_on ?? true}
      label="Keep the screen on"
      onchange={(v) => void store.updateSettings({ keep_display_on: v })}
    />
  </Row>
  <Row label="Pause on low battery" hint={lowBatteryHint}>
    <Switch
      checked={lowBatteryOn}
      label="Pause on low battery"
      onchange={(v) => void store.updateSettings({ pause_below_battery_percent: v ? 20 : null })}
    />
  </Row>
</section>

<nav class="doors" aria-label="More in Settings">
  {#each ways as door, i (door.section)}
    {#if i > 0}<span class="dot" aria-hidden="true">·</span>{/if}
    <button
      type="button"
      class:news={door.section === 'about'}
      title={door.title}
      onclick={() => store.openSettings(door.section)}
    >
      {door.label}
    </button>
  {/each}
</nav>

<style>
  .status {
    padding: 14px 24px 6px;
    min-height: 84px;
  }

  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
  }

  /* Your day, beside the laptop's: quiet until looked for, and a way in. */
  .today {
    display: grid;
    justify-items: end;
    gap: 1px;
    padding: 2px 6px 1px;
    margin-right: -6px;
    border-radius: var(--radius-sm);
    transition: background-color var(--t-fast) var(--ease);
  }

  .today .eyebrow {
    font-size: 10px;
  }

  .figure {
    font-family: var(--font-display);
    font-size: 20px;
    line-height: 1.15;
    color: var(--ink);
    font-variant-numeric: tabular-nums;
    font-variation-settings:
      'opsz' 72,
      'SOFT' 50;
  }

  .today:hover,
  .day:hover {
    background: var(--hair);
  }

  /* The day as a horizon, under the big one: the picture, then its ledger. */
  .day {
    display: block;
    margin: 8px 18px 0;
    padding: 5px 6px 3px;
    border-radius: var(--radius-sm);
    transition: background-color var(--t-fast) var(--ease);
  }

  .day + .status {
    padding-top: 6px;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-weight: 400;
    font-size: 36px;
    line-height: 1.05;
    letter-spacing: -0.01em;
    font-variation-settings:
      'opsz' 72,
      'SOFT' 50;
  }

  p {
    margin: 6px 0 0;
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
    animation: settle var(--t-mid) var(--ease-out);
  }

  .extend {
    margin-left: 2px;
    padding: 0;
    font: inherit;
    color: var(--ink);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
    transition: text-decoration-color var(--t-fast) var(--ease);
  }

  .extend:hover {
    text-decoration-color: var(--ink);
  }

  p.whisper {
    color: var(--ink);
    font-family: var(--font-display);
    font-style: italic;
    font-size: 14px;
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

  .rows {
    margin-top: auto;
    padding: 0 24px 4px;
  }

  /* The way in to what Home has no room for: a colophon, not a toolbar. */
  .doors {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: baseline;
    gap: 0 2px;
    margin: 0 24px;
    padding: 6px 0 12px;
    border-top: 1px solid var(--hair);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .doors button {
    padding: 2px 6px;
    color: var(--ink-2);
    border-radius: var(--radius-sm);
    text-decoration: underline;
    text-decoration-color: transparent;
    text-underline-offset: 3px;
    transition:
      color var(--t-fast) var(--ease),
      text-decoration-color var(--t-fast) var(--ease);
  }

  .doors button:hover,
  .doors button:focus-visible {
    color: var(--ink);
    text-decoration-color: var(--hair-strong);
  }

  .dot {
    color: var(--ink-3);
  }

  /* A waiting update reads first and in full ink, still no colour. */
  .doors button.news {
    color: var(--ink);
    text-decoration-color: var(--hair-strong);
  }

  /* At the minimum height the two rows need the room. */
  @media (max-height: 560px) {
    .doors,
    .day {
      display: none;
    }
  }
</style>
