<script lang="ts">
  import DurationRule from './DurationRule.svelte';
  import Row from './Row.svelte';
  import Scene from './Scene.svelte';
  import Switch from './Switch.svelte';
  import { EXTEND_LABEL, EXTENDED } from '$lib/copy';
  import { batteryText } from '$lib/format';
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
</script>

<Scene />

<section class="status" aria-live="polite">
  <h1>{store.status.title}</h1>
  {#key store.whisper}
    <p class:whisper={store.whisper !== null}>
      {store.whisper ?? store.status.detail}
      {#if store.sunsetSoon && store.whisper === null}
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
    padding: 0 24px 14px;
  }
</style>
