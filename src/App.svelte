<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';

  import Home from '$lib/components/Home.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
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

  function onKey(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const typing = target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA';
    if (event.key === 'Escape') {
      if (store.view === 'settings') store.view = 'home';
      else void getCurrentWindow().close();
    } else if (event.key === ' ' && !typing && target?.tagName !== 'BUTTON') {
      event.preventDefault();
      void store.toggle();
    } else if (event.key === ',' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      store.view = store.view === 'settings' ? 'home' : 'settings';
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<TitleBar />

<main class:ready={store.ready}>
  {#if store.ready}
    <div class="pane" class:offscreen={store.view !== 'home'} aria-hidden={store.view !== 'home'}>
      <Home />
    </div>
    <div
      class="pane settings"
      class:offscreen={store.view !== 'settings'}
      aria-hidden={store.view !== 'settings'}
    >
      <Settings />
    </div>
  {/if}
</main>

{#if store.error}
  <div class="toast" role="alert">{store.error}</div>
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

  .pane.settings.offscreen {
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
  }
</style>
