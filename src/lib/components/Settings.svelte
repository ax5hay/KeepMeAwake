<script lang="ts">
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';

  import DayPills from './DayPills.svelte';
  import History from './History.svelte';
  import HotkeyField from './HotkeyField.svelte';
  import Row from './Row.svelte';
  import Segmented from './Segmented.svelte';
  import Stepper from './Stepper.svelte';
  import Switch from './Switch.svelte';
  import { AUTHOR, OVERNIGHT_GUARD_HINT, PLATFORM_COPY, tipsWith } from '$lib/copy';
  import { displayAccelerator } from '$lib/hotkey';
  import { formatStop, minutesToTime, timeToMinutes } from '$lib/format';
  import { isMac } from '$lib/platform';
  import { store } from '$lib/state.svelte';
  import type { Notifications, Schedule, StartupPlan, Theme } from '$lib/types';

  const settings = $derived(store.settings);

  function setSchedule(patch: Partial<Schedule>) {
    if (!settings) return;
    void store.updateSettings({ schedule: { ...settings.schedule, ...patch } });
  }

  function setNotifications(patch: Partial<Notifications>) {
    if (!settings) return;
    void store.updateSettings({ notifications: { ...settings.notifications, ...patch } });
  }

  function setTime(field: 'start_minutes' | 'end_minutes') {
    return (event: Event) => {
      const minutes = timeToMinutes((event.currentTarget as HTMLInputElement).value);
      if (minutes !== null) setSchedule({ [field]: minutes });
    };
  }

  // The log is fetched each time Settings opens, never at launch.
  $effect(() => {
    if (store.view === 'settings') void store.loadHistory();
  });

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

  function open(url: string) {
    return (event: Event) => {
      event.preventDefault();
      void openUrl(url);
    };
  }

  const durationOptions = [
    { value: 'inf', label: 'Always' },
    ...[1800, 3600, 7200, 14_400, 28_800].map((s) => ({ value: String(s), label: formatStop(s) })),
  ];

  const batteryOptions = [
    { value: 'off', label: 'Off' },
    ...[10, 15, 20, 25, 30, 40, 50].map((p) => ({ value: String(p), label: `${p}%` })),
  ];

  const sunsetOptions: ReadonlyArray<{ value: 'off' | '2' | '5' | '10'; label: string }> = [
    { value: 'off', label: 'Off' },
    { value: '2', label: '2m' },
    { value: '5', label: '5m' },
    { value: '10', label: '10m' },
  ];

  const guardOptions: ReadonlyArray<{ value: 'off' | '4' | '6' | '8'; label: string }> = [
    { value: 'off', label: 'Off' },
    { value: '4', label: '4h' },
    { value: '6', label: '6h' },
    { value: '8', label: '8h' },
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
        hint={autostart === null ? 'Unavailable' : PLATFORM_COPY.launchHint}
      >
        <Switch
          checked={autostart === true}
          disabled={autostart === null}
          label="Launch at sign-in"
          onchange={(v) => void setAutostart(v)}
        />
      </Row>
      <Row label="Open the window at launch" hint="Only when you start it yourself.">
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
      <Row label="Default length" hint={PLATFORM_COPY.lengthHint}>
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
      <Row label="Warn before sunset" hint="A nudge before a timer ends.">
        <Segmented
          options={sunsetOptions}
          value={settings.sunset_warning_mins === null
            ? 'off'
            : (String(settings.sunset_warning_mins) as '2' | '5' | '10')}
          label="Warn before sunset"
          onchange={(v) =>
            void store.updateSettings({ sunset_warning_mins: v === 'off' ? null : Number(v) })}
        />
      </Row>
      <Row label="Overnight guard" hint={OVERNIGHT_GUARD_HINT}>
        <Segmented
          options={guardOptions}
          value={settings.overnight_guard_hours === null
            ? 'off'
            : (String(settings.overnight_guard_hours) as '4' | '6' | '8')}
          label="Overnight guard"
          onchange={(v) =>
            void store.updateSettings({ overnight_guard_hours: v === 'off' ? null : Number(v) })}
        />
      </Row>
      <Row label="Keep the screen on" hint="Off lets the display dim while the laptop stays awake.">
        <Switch
          checked={settings.keep_display_on}
          label="Keep the screen on"
          onchange={(v) => void store.updateSettings({ keep_display_on: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">Schedule</h2>
      <Row label="Keep a schedule" hint="Awake during these hours, resting outside them.">
        <Switch
          checked={settings.schedule.enabled}
          label="Keep a schedule"
          onchange={(v) => setSchedule({ enabled: v })}
        />
      </Row>
      <div
        class="dependent"
        class:off={!settings.schedule.enabled}
        inert={!settings.schedule.enabled}
      >
        <Row label="Days">
          <DayPills days={settings.schedule.days} onchange={(days) => setSchedule({ days })} />
        </Row>
        <Row label="From / Until" hint="Overnight is fine: 10:00 pm until 6:00 am.">
          <div class="times">
            <input
              type="time"
              aria-label="From"
              value={minutesToTime(settings.schedule.start_minutes)}
              onchange={setTime('start_minutes')}
            />
            <input
              type="time"
              aria-label="Until"
              value={minutesToTime(settings.schedule.end_minutes)}
              onchange={setTime('end_minutes')}
            />
          </div>
        </Row>
      </div>
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
      <Row label="Pause whenever unplugged" hint="Strictest option: battery means resting.">
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

    <section>
      <h2 class="eyebrow">Notifications</h2>
      <Row label="When a timer ends">
        <Switch
          checked={settings.notifications.timer}
          label="When a timer ends"
          onchange={(v) => setNotifications({ timer: v })}
        />
      </Row>
      <Row label="Battery pauses and resumes">
        <Switch
          checked={settings.notifications.battery}
          label="Battery pauses and resumes"
          onchange={(v) => setNotifications({ battery: v })}
        />
      </Row>
      <Row label="Schedule starts and ends">
        <Switch
          checked={settings.notifications.schedule}
          label="Schedule starts and ends"
          onchange={(v) => setNotifications({ schedule: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">Hotkey</h2>
      <Row label="Hotkey" hint={PLATFORM_COPY.hotkeyHint}>
        <HotkeyField
          value={settings.hotkey}
          onchange={(v) => void store.updateSettings({ hotkey: v })}
        />
      </Row>
    </section>

    <section>
      <h2 class="eyebrow">History</h2>
      <History history={store.history} now={store.now} onclear={() => void store.clearHistory()} />
    </section>

    <section>
      <h2 class="eyebrow">Tips</h2>
      <dl class="tips">
        {#each tipsWith(settings.hotkey ? displayAccelerator(settings.hotkey, isMac) : null) as tip (tip.keys)}
          <dt>{tip.keys}</dt>
          <dd>{tip.does}</dd>
        {/each}
      </dl>
      <p class="hint whisper">There are a few more things to find in the sky.</p>
    </section>

    <section class="about">
      <h2 class="eyebrow">About</h2>
      <p>
        <span class="name">Keep me awake</span>
        <span class="version">{store.snapshot?.app_version}</span>
      </p>
      <p class="hint">
        {PLATFORM_COPY.about.before}
        <code>{PLATFORM_COPY.about.code}</code>{PLATFORM_COPY.about.after}
      </p>
      <p class="hint">
        Made by <a href={AUTHOR.site} onclick={open(AUTHOR.site)}>{AUTHOR.name}</a>
        <span class="sep">·</span>
        <a href={AUTHOR.site} onclick={open(AUTHOR.site)}>{AUTHOR.siteLabel}</a>
      </p>
      <p class="links">
        <a href={AUTHOR.repo} onclick={open(AUTHOR.repo)}>Source</a>
        <button type="button" class="quit" onclick={() => void store.quit()}>
          Quit Keep me awake
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

  .dependent {
    transition: opacity var(--t-fast) var(--ease);
  }

  .dependent.off {
    opacity: 0.4;
  }

  .times {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tips {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 14px;
    row-gap: 6px;
    margin: 8px 0 0;
  }

  .tips dt {
    color: var(--ink);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .tips dd {
    margin: 0;
    color: var(--ink-2);
  }

  .whisper {
    margin: 10px 0 0;
    font-family: var(--font-display);
    font-style: italic;
    font-size: 13px;
    color: var(--ink-3);
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

  .sep {
    color: var(--ink-3);
    margin: 0 4px;
  }

  code {
    font-family: 'Cascadia Mono', Consolas, monospace;
    white-space: nowrap;
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
