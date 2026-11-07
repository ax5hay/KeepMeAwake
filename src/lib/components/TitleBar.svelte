<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { CREDIT, PLATFORM_COPY } from '$lib/copy';
  import { store } from '$lib/state.svelte';

  const win = getCurrentWindow();
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg class="mark" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="6.5" r="3.6" fill="currentColor" />
      <rect x="1.5" y="11.5" width="13" height="1.4" rx="0.7" fill="currentColor" />
    </svg>
    <button type="button" class="name" title="Keep me awake" onclick={() => store.say(CREDIT)}>
      Keep me awake
    </button>
  </div>

  <nav aria-label="Window">
    <button
      type="button"
      class="ctl"
      aria-label={store.view === 'settings' ? 'Back' : 'Settings'}
      aria-pressed={store.view === 'settings'}
      title={store.view === 'settings' ? 'Back (Esc)' : PLATFORM_COPY.settingsTitle}
      onclick={() => (store.view === 'settings' ? (store.view = 'home') : store.openSettings())}
    >
      {#if store.view === 'settings'}
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3.5 5.5 8l4.5 4.5" /></svg>
      {:else}
        <!-- Sliders, not a gear: a gear at 16 px is one more sun in an app
             whose only accent is the sun. -->
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 4h6.1M11.9 4h1.6M2.5 8h1.6M7.4 8h6.1M2.5 12h7.6M13.4 12h.1" />
          <circle cx="10.25" cy="4" r="1.65" />
          <circle cx="5.75" cy="8" r="1.65" />
          <circle cx="11.75" cy="12" r="1.65" />
        </svg>
      {/if}
    </button>
    <button
      type="button"
      class="ctl"
      aria-label="Minimise"
      title="Minimise"
      onclick={() => void win.minimize()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5h10" /></svg>
    </button>
    <button
      type="button"
      class="ctl close"
      aria-label={PLATFORM_COPY.closeLabel}
      title={PLATFORM_COPY.closeTitle}
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
    color: inherit;
    padding: 2px 4px;
    margin-left: -4px;
    border-radius: var(--radius-sm);
    transition: color var(--t-fast) var(--ease);
  }

  .name:hover {
    color: var(--ink);
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
