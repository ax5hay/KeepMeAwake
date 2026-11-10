<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';

  import Almanac from '$lib/components/Almanac.svelte';
  import Home from '$lib/components/Home.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import { windowAction } from '$lib/keys';
  import { isMac } from '$lib/platform';
  import { store } from '$lib/state.svelte';
  import { watchTheme } from '$lib/theme';

  onMount(() => {
    let stopTheme = () => {};
    void (async () => {
      await store.init();
      stopTheme = watchTheme(() => store.settings?.theme ?? 'system');
      // First paint is done; let the native window appear.
      await getCurrentWindow().show();
      await getCurrentWindow().setFocus();
    })();
    return () => {
      stopTheme();
      store.dispose();
    };
  });

  $effect(() => {
    // Re-resolve when the user changes the theme setting.
    const theme = store.settings?.theme;
    if (theme) {
      const stop = watchTheme(() => theme);
      return stop;
    }
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
        if (store.view !== 'home') store.view = 'home';
        else void getCurrentWindow().close();
        break;
      case 'toggle':
        void store.toggle();
        break;
      case 'settings':
        if (store.view === 'settings') store.view = 'home';
        else store.openSettings();
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

<TitleBar />

<main class:ready={store.ready}>
  {#if store.ready}
    <div class="pane" class:offscreen={store.view !== 'home'} inert={store.view !== 'home'}>
      <Home />
    </div>
    <div
      class="pane aside"
      class:offscreen={store.view !== 'almanac'}
      inert={store.view !== 'almanac'}
    >
      <Almanac />
    </div>
    <div
      class="pane aside"
      class:offscreen={store.view !== 'settings'}
      inert={store.view !== 'settings'}
    >
      <Settings />
    </div>
  {/if}
</main>

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
