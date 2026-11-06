<script lang="ts">
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';

  import Row from './Row.svelte';
  import Segmented from './Segmented.svelte';
  import Stepper from './Stepper.svelte';
  import Switch from './Switch.svelte';
  import { formatStop } from '$lib/format';
  import { store } from '$lib/state.svelte';
  import type { StartupPlan, Theme } from '$lib/types';

  const REPO = 'https://github.com/BuiltByBajpai/KeepMeAwake';

  const settings = $derived(store.settings);

  let autostart = $state<boolean | null>(null);
  onMount(() => {
    void isEnabled()
      .then((v) => (autostart = v))
      .catch(() => (autostart = null));
  });

  async function setAutostart(on: boolean) {
    try {
      if (on) await enable();
      else await disable();
      autostart = await isEnabled();
    } catch (err) {
      store.error = err instanceof Error ? err.message : String(err);
    }
  }

  const durationOptions = [
    { value: 'inf', label: 'Always' },
    ...[1800, 3600, 7200, 14_400, 28_800].map((s) => ({ value: String(s), label: formatStop(s) })),
  ];

  const batteryOptions = [
    { value: 'off', label: 'Off' },
    ...[10, 15, 20, 25, 30, 40, 50].map((p) => ({ value: String(p), label: `${p}%` })),
  ];

  const themeOptions: ReadonlyArray<{ value: Theme; label: string }> = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Paper' },
    { value: 'dark', label: 'Night' },
  ];

  const startupOptions: ReadonlyArray<{ value: StartupPlan; label: string }> = [
    { value: 'off', label: 'Resting' },
    { value: 'indefinite', label: 'Awake' },
    { value: 'default_duration', label: 'Default length' },
  ];
</script>

<div class="scroll">
  {#if settings}
    <section>
      <h2 class="eyebrow">Startup</h2>
      <Row
        label="Launch at sign-in"
        hint={autostart === null ? 'Unavailable' : 'Starts quietly in the tray.'}
      >
        <Switch
          checked={autostart === true}
          disabled={autostart === null}
          label="Launch at sign-in"
          onchange={(v) => void setAutostart(v)}
        />
      </Row>
      <Row label="Open the window at launch">
        <Switch
          checked={!settings.start_hidden}
          label="Open the window at launch"
          onchange={(v) => void store.updateSettings({ start_hidden: !v })}
        />
      </Row>
      <Row label="Begin as">
        <Segmented
          options={startupOptions}
          value={settings.startup_plan}
          label="State at launch"
          onchange={(v) => void store.updateSettings({ startup_plan: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">Waking</h2>
      <Row label="Default length" hint="Used by the sun, the tray and Space.">
        <Stepper
          options={durationOptions}
          value={settings.default_duration_secs === null
            ? 'inf'
            : String(settings.default_duration_secs)}
          label="Default length"
          onchange={(v) =>
            void store.updateSettings({ default_duration_secs: v === 'inf' ? null : Number(v) })}
        />
      </Row>
      <Row label="Keep the screen on">
        <Switch
          checked={settings.keep_display_on}
          label="Keep the screen on"
          onchange={(v) => void store.updateSettings({ keep_display_on: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">Battery</h2>
      <Row label="Pause below" hint="Resumes as soon as you plug in.">
        <Stepper
          options={batteryOptions}
          value={settings.pause_below_battery_percent === null
            ? 'off'
            : String(settings.pause_below_battery_percent)}
          label="Pause below battery level"
          onchange={(v) =>
            void store.updateSettings({
              pause_below_battery_percent: v === 'off' ? null : Number(v),
            })}
        />
      </Row>
      <Row label="Pause whenever unplugged">
        <Switch
          checked={settings.pause_on_battery}
          label="Pause whenever unplugged"
          onchange={(v) => void store.updateSettings({ pause_on_battery: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">Window</h2>
      <Row label="Closing keeps it running" hint="Off quits the app when you close the window.">
        <Switch
          checked={settings.close_to_tray}
          label="Closing keeps it running"
          onchange={(v) => void store.updateSettings({ close_to_tray: v })}
        />
      </Row>
      <Row label="Free memory when closed" hint="Reopening takes a moment longer.">
        <Switch
          checked={settings.free_memory_when_hidden}
          disabled={!settings.close_to_tray}
          label="Free memory when closed"
          onchange={(v) => void store.updateSettings({ free_memory_when_hidden: v })}
        />
      </Row>
      <Row label="Appearance">
        <Segmented
          options={themeOptions}
          value={settings.theme}
          label="Appearance"
          onchange={(v) => void store.updateSettings({ theme: v })}
        />
      </Row>
    </section>

    <section class="about">
      <h2 class="eyebrow">About</h2>
      <p>
        <span class="name">Keep me awake</span>
        <span class="version">{store.snapshot?.app_version}</span>
      </p>
      <p class="hint">
        Holds a Windows power request while awake, the same mechanism a video player uses. See it
        under <code>powercfg /requests</code>.
      </p>
      <p class="links">
        <a
          href={REPO}
          onclick={(e) => {
            e.preventDefault();
            void openUrl(REPO);
          }}>Source</a
        >
        <button type="button" class="quit" onclick={() => void store.quit()}
          >Quit KeepMeAwake</button
        >
      </p>
    </section>
  {/if}
</div>

<style>
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 24px 16px;
    scrollbar-width: thin;
    scrollbar-color: var(--hair-strong) transparent;
  }

  section + section {
    margin-top: 18px;
  }

  h2 {
    margin: 0 0 2px;
    font-weight: 500;
  }

  .about p {
    margin: 8px 0 0;
  }

  .name {
    font-family: var(--font-display);
    font-style: italic;
    font-size: 16px;
  }

  .version {
    margin-left: 8px;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }

  code {
    font-family: 'Cascadia Mono', Consolas, monospace;
    font-size: 11px;
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--paper-2);
  }

  .links {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .quit {
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
  }

  .quit:hover {
    color: var(--ink);
    text-decoration-color: var(--ink);
  }
</style>
