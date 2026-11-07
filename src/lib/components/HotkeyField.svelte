<script lang="ts">
  /**
   * Shows the global hotkey and records a new one. Change turns the field
   * into a recorder: the next key press with Ctrl, Alt or Win (on a Mac,
   * ⌘, ⌃ or ⌥) is shown as it is held and saved when it is released. Esc cancels. While recording, keys
   * are kept from the rest of the window so Space or Esc do not act.
   */
  import { onDestroy } from 'svelte';

  import { HOTKEY_RECORDING } from '$lib/copy';
  import { displayAccelerator, toAccelerator } from '$lib/hotkey';
  import { isMac } from '$lib/platform';

  interface Props {
    value: string | null;
    onchange: (value: string | null) => void;
  }

  const { value, onchange }: Props = $props();

  let recording = $state(false);
  let pending = $state<string | null>(null);
  let field: HTMLSpanElement | undefined = $state();

  const shown = $derived.by(() => {
    if (recording) return pending ? displayAccelerator(pending, isMac) : HOTKEY_RECORDING;
    return value ? displayAccelerator(value, isMac) : 'Off';
  });

  function start() {
    recording = true;
    pending = null;
    window.addEventListener('keydown', onKeyDown, true);
    window.addEventListener('keyup', onKeyUp, true);
    queueMicrotask(() => field?.focus());
  }

  function stop() {
    recording = false;
    pending = null;
    window.removeEventListener('keydown', onKeyDown, true);
    window.removeEventListener('keyup', onKeyUp, true);
  }

  function swallow(event: KeyboardEvent) {
    event.preventDefault();
    event.stopPropagation();
  }

  function onKeyDown(event: KeyboardEvent) {
    swallow(event);
    if (event.key === 'Escape') {
      stop();
      return;
    }
    const accelerator = toAccelerator(event, isMac);
    if (accelerator) pending = accelerator;
  }

  function onKeyUp(event: KeyboardEvent) {
    swallow(event);
    if (!pending) return;
    const chosen = pending;
    stop();
    if (chosen !== value) onchange(chosen);
  }

  onDestroy(stop);
</script>

<div class="hotkey">
  <span
    class="combo"
    class:recording
    class:off={!recording && value === null}
    bind:this={field}
    tabindex="-1"
    aria-live="polite"
    aria-label={recording ? HOTKEY_RECORDING : `Hotkey: ${shown}`}>{shown}</span
  >
  {#if recording}
    <button type="button" class="action" onclick={stop}>Cancel</button>
  {:else}
    <button type="button" class="action" onclick={start}>Change</button>
    <button type="button" class="action" disabled={value === null} onclick={() => onchange(null)}>
      Off
    </button>
  {/if}
</div>

<style>
  .hotkey {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .combo {
    min-width: 84px;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    background: var(--paper-2);
    color: var(--ink);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: center;
    white-space: nowrap;
  }

  .combo.off {
    color: var(--ink-3);
  }

  .combo.recording {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--ink);
  }

  .action {
    padding: 3px 4px;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
    transition: color var(--t-fast) var(--ease);
  }

  .action:hover:not(:disabled) {
    color: var(--ink);
    text-decoration-color: var(--ink);
  }

  .action:disabled {
    color: var(--ink-3);
    text-decoration: none;
  }
</style>
