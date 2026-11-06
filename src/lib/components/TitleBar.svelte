<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { store } from '$lib/state.svelte';

  const win = getCurrentWindow();
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg class="mark" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="6.5" r="3.6" fill="currentColor" />
      <rect x="1.5" y="11.5" width="13" height="1.4" rx="0.7" fill="currentColor" />
    </svg>
    <span class="name" data-tauri-drag-region>Keep me awake</span>
  </div>

  <nav aria-label="Window">
    <button
      type="button"
      class="ctl"
      aria-label={store.view === 'settings' ? 'Back' : 'Settings'}
      aria-pressed={store.view === 'settings'}
      title={store.view === 'settings' ? 'Back (Esc)' : 'Settings (Ctrl+,)'}
      onclick={() => (store.view = store.view === 'settings' ? 'home' : 'settings')}
    >
      {#if store.view === 'settings'}
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3.5 5.5 8l4.5 4.5" /></svg>
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="2.2" />
          <path
            d="M8 1.8v2.1M8 12.1v2.1M1.8 8h2.1M12.1 8h2.1M3.6 3.6l1.5 1.5M10.9 10.9l1.5 1.5M3.6 12.4l1.5-1.5M10.9 5.1l1.5-1.5"
          />
        </svg>
      {/if}
    </button>
    <button type="button" class="ctl" aria-label="Minimise" onclick={() => void win.minimize()}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5h10" /></svg>
    </button>
    <button
      type="button"
      class="ctl close"
      aria-label="Close to tray"
      title="Close to tray (Esc)"
      onclick={() => void win.close()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
    </button>
  </nav>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 6px 0 14px;
    flex: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-2);
  }

  .mark {
    width: 14px;
    height: 14px;
  }

  .name {
    font-family: var(--font-display);
    font-style: italic;
    font-weight: 400;
    font-size: 14px;
    letter-spacing: 0.005em;
  }

  nav {
    display: flex;
  }

  .ctl {
    width: 34px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    color: var(--ink-2);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }

  .ctl svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .ctl:hover,
  .ctl[aria-pressed='true'] {
    background: var(--hair);
    color: var(--ink);
  }

  .ctl.close:hover {
    background: #c84b3c;
    color: #fff;
  }
</style>
