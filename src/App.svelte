<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';

  import Almanac from '$lib/components/Almanac.svelte';
  import Home from '$lib/components/Home.svelte';
  import Mini from '$lib/components/Mini.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import Sheets from '$lib/components/Sheets.svelte';
  import Timesheet from '$lib/components/Timesheet.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import { windowAction } from '$lib/keys';
  import { isMac } from '$lib/platform';
  import { store } from '$lib/state.svelte';
  import { msToTurn, resolveTheme, SYSTEM_DARK, themeAt, wear } from '$lib/theme';

  /** Whether the OS is in its dark mode: what `system` follows. */
  let systemDark = $state(false);
  /** Now, as far as the theme cares: moved on only when an evening starts or ends. */
  let themeClock = $state(Date.now());

  onMount(() => {
    const media = window.matchMedia(SYSTEM_DARK);
    const follow = () => (systemDark = media.matches);
    follow();
    media.addEventListener('change', follow);
    void (async () => {
      await store.init();
      // First paint is done; let the native window appear.
      await getCurrentWindow().show();
      await getCurrentWindow().setFocus();
    })();
    return () => {
      media.removeEventListener('change', follow);
      store.dispose();
    };
  });

  // What the window wears: a theme being looked at in the picker, else the
  // evening's inside the evening, else the one chosen.
  $effect(() => {
    const settings = store.settings;
    if (!settings) {
      wear(document.documentElement, resolveTheme('system', systemDark), {
        paper: '',
        ink: '',
        sun: '',
        sky: '',
        ground: '',
      });
      return;
    }
    const at = new Date(themeClock);
    const theme =
      store.preview ??
      themeAt(settings.theme, settings.after_dark, at.getHours() * 60 + at.getMinutes());
    const palette = resolveTheme(theme, systemDark);
    wear(document.documentElement, palette, settings.own_theme);
    store.palette = palette;
  });

  // With an evening theme, one timeout to the moment it next starts or ends.
  // No evening, no timer.
  $effect(() => {
    const afterDark = store.settings?.after_dark;
    if (!afterDark) return;
    const wait = msToTurn(afterDark, new Date(themeClock));
    if (wait === null) return;
    const timer = setTimeout(() => (themeClock = Date.now()), wait + 500);
    return () => clearTimeout(timer);
  });

  // A theme is only looked at from its swatch: leaving Settings ends it.
  $effect(() => {
    if (store.view !== 'settings') store.preview = null;
  });

  // An error says its piece and goes; a click sends it sooner. One timeout,
  // armed only while an error is on screen.
  const ERROR_MS = 6000;
  $effect(() => {
    if (store.error === null) return;
    const shown = store.error;
    const timer = setTimeout(() => {
      if (store.error === shown) store.error = null;
    }, ERROR_MS);
    return () => clearTimeout(timer);
  });

  function onKey(event: KeyboardEvent) {
    const target = (event.target as HTMLElement | null)?.tagName ?? '';
    const action = windowAction(event, target, isMac);
    if (action === null) return;
    if (action !== 'back-or-close') event.preventDefault();
    switch (action) {
      case 'back-or-close':
        // Esc undoes the nearest thing: a sheet, the mini window, a pane.
        if (store.sheet !== null) store.sheet = null;
        else if (store.mini) void store.setMini(false);
        else if (store.view !== 'home') store.view = 'home';
        else void getCurrentWindow().close();
        break;
      case 'toggle':
        void store.toggle();
        break;
      case 'settings':
        if (store.mini) break;
        store.sheet = null;
        if (store.view === 'settings') store.view = 'home';
        else store.openSettings();
        break;
      case 'keys':
        if (!store.mini) store.sheet = store.sheet === 'keys' ? null : 'keys';
        break;
      // Same path as the title bar's close: to the menu bar, the sun stays up.
      case 'close':
        void getCurrentWindow().close();
        break;
      case 'minimise':
        void getCurrentWindow().minimize();
        break;
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if store.mini}
  {#if store.ready}<Mini />{/if}
{:else}
  <TitleBar />

  <main class:ready={store.ready}>
    {#if store.ready}
      <div
        class="pane"
        class:offscreen={store.view !== 'home'}
        inert={store.view !== 'home' || store.sheet !== null}
      >
        <Home />
      </div>
      <div
        class="pane aside"
        class:offscreen={store.view !== 'almanac'}
        inert={store.view !== 'almanac' || store.sheet !== null}
      >
        <Almanac />
      </div>
      <div
        class="pane aside"
        class:offscreen={store.view !== 'settings'}
        inert={store.view !== 'settings' || store.sheet !== null}
      >
        <Settings />
      </div>
      {#if store.sheet !== null}
        <Sheets />
      {/if}
    {/if}
  </main>
{/if}

{#if store.printing}
  <Timesheet year={store.printing.year} month={store.printing.month} />
{/if}

{#if store.error}
  <div class="toast" role="alert">
    <button type="button" title="Dismiss" onclick={() => (store.error = null)}>{store.error}</button
    >
  </div>
{/if}

<style>
  main {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .pane {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    transition:
      transform var(--t-mid) var(--ease-out),
      opacity var(--t-mid) var(--ease);
  }

  .pane.offscreen {
    pointer-events: none;
    opacity: 0;
    transform: translateX(-24px);
  }

  .pane.aside.offscreen {
    transform: translateX(24px);
  }

  .toast {
    position: fixed;
    left: 16px;
    right: 16px;
    bottom: 16px;
    padding: 10px 12px;
    background: var(--ink);
    color: var(--paper);
    border-radius: var(--radius-sm);
    font-size: 12px;
    box-shadow: var(--shadow);
    animation: rise var(--t-mid) var(--ease-out);
  }

  .toast button {
    display: block;
    width: 100%;
    text-align: left;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
