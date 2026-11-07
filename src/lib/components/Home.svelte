<script lang="ts">
  import DurationRule from './DurationRule.svelte';
  import Row from './Row.svelte';
  import Scene from './Scene.svelte';
  import Switch from './Switch.svelte';
  import { EXTEND_LABEL, EXTENDED, KEEP_IT_UP, KEPT_UP, doors } from '$lib/copy';
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
    ),
  );
  // Paused, the line says why it holds off; offering more time there reads oddly.
  const offerExtend = $derived(store.sunsetSoon && snapshot?.paused === null);
</script>

<Scene />

<section class="status" aria-live="polite">
  <h1>{store.status.title}</h1>
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
    <button type="button" title={door.title} onclick={() => store.openSettings(door.section)}>
      {door.label}
    </button>
  {/each}
</nav>

<style>
  .status {
    padding: 14px 24px 6px;
    min-height: 84px;
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

  /* At the minimum height the two rows need the room. */
  @media (max-height: 560px) {
    .doors {
      display: none;
    }
  }
</style>
