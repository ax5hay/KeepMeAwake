<script lang="ts">
  /**
   * The window as the sun alone: the scene, and one line under it. Small and
   * above other windows, for keeping an eye on. The sun still switches; the
   * line drags the window, and its button brings the rest back.
   */
  import { MINI_BACK } from '$lib/copy';
  import { store } from '$lib/state.svelte';

  import Scene from './Scene.svelte';
</script>

<div class="mini">
  <Scene />
  <div class="line" data-tauri-drag-region>
    <span class="word" data-tauri-drag-region>{store.status.title}</span>
    <button
      type="button"
      aria-label={MINI_BACK}
      title="{MINI_BACK} (Esc)"
      onclick={() => void store.setMini(false)}
    >
      <!-- Corners pulled outward: the window, grown back. -->
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M9.5 3.5h3v3M12.5 3.5 9 7M6.5 12.5h-3v-3M3.5 12.5 7 9" />
      </svg>
    </button>
  </div>
</div>

<style>
  .mini {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--paper);
  }

  .line {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 0;
    padding: 0 6px 0 12px;
    cursor: default;
  }

  .word {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 400;
    color: var(--ink);
  }

  button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    color: var(--ink-2);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }

  button:hover {
    background: var(--paper-2);
    color: var(--ink);
  }

  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
