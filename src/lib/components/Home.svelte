<script lang="ts">
  import DurationRule from './DurationRule.svelte';
  import Row from './Row.svelte';
  import Scene from './Scene.svelte';
  import Switch from './Switch.svelte';
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
  <p>{store.status.detail}</p>
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
  }

  .rows {
    margin-top: auto;
    padding: 0 24px 14px;
  }
</style>
