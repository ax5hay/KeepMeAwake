<script lang="ts">
  import TellTheSun from './TellTheSun.svelte';
  /**
   * A slide-rule of durations. Each stop is a tick on a hairline; the active
   * stop carries a filled dot. "Until" opens an inline time field.
   */
  import { longStop } from '$lib/copy';
  import { formatClock, formatStop, nextOccurrence, suggestedUntil } from '$lib/format';
  import { store } from '$lib/state.svelte';

  export const STOPS = [1800, 3600, 7200, 14_400, 28_800] as const;

  let untilOpen = $state(false);
  let untilValue = $state('');
  let untilInput: HTMLInputElement | undefined = $state();
  let untilStop: HTMLButtonElement | undefined = $state();

  /** Esc in the field closes the field, not the window, and focus goes home. */
  function onFormKey(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.stopPropagation();
    untilOpen = false;
    untilStop?.focus();
  }

  const snapshot = $derived(store.snapshot);
  const activeStop = $derived.by<string | null>(() => {
    // While the schedule holds the sun no stop is active; picking one overrides it.
    if (!snapshot) return null;
    // Nothing on the rule was chosen: the schedule or a condition holds it.
    if (['off', 'scheduled', 'watching'].includes(snapshot.mode)) return null;
    if (snapshot.mode === 'indefinite') return 'inf';
    const match = STOPS.find((s) => s === snapshot.plan_secs);
    return match ? String(match) : 'until';
  });
  const untilLabel = $derived(
    activeStop === 'until' && snapshot?.ends_at_unix_ms
      ? `Until ${formatClock(snapshot.ends_at_unix_ms)}`
      : 'Until…',
  );

  function pick(seconds: number | null) {
    untilOpen = false;
    void store.setPlan(seconds === null ? { kind: 'indefinite' } : { kind: 'for', seconds });
  }

  function openUntil() {
    untilOpen = !untilOpen;
    if (untilOpen) {
      untilValue = suggestedUntil(Date.now());
      queueMicrotask(() => untilInput?.focus());
    }
  }

  function submitUntil(event: Event) {
    event.preventDefault();
    const at = nextOccurrence(untilValue, Date.now());
    if (at === null) return;
    untilOpen = false;
    void store.setPlan({ kind: 'until', unix_ms: at });
  }

  // While a plan is being typed, Home gives it the room its rows take.
  $effect(() => {
    store.planning = untilOpen;
    return () => (store.planning = false);
  });
</script>

<div class="rule" role="group" aria-label="How long to stay awake">
  <div class="line" aria-hidden="true"></div>
  <button
    type="button"
    class="stop"
    class:active={activeStop === 'inf'}
    aria-pressed={activeStop === 'inf'}
    title="Keep awake until you say otherwise"
    onclick={() => pick(null)}
  >
    <span class="tick"></span>
    <span class="label">Always</span>
  </button>
  {#each STOPS as seconds (seconds)}
    <button
      type="button"
      class="stop"
      class:active={activeStop === String(seconds)}
      aria-pressed={activeStop === String(seconds)}
      title="Keep awake for {longStop(seconds)}"
      onclick={() => pick(seconds)}
    >
      <span class="tick"></span>
      <span class="label">{formatStop(seconds)}</span>
    </button>
  {/each}
  <button
    type="button"
    class="stop until"
    class:active={activeStop === 'until'}
    aria-pressed={activeStop === 'until'}
    aria-expanded={untilOpen}
    title="Keep awake until a time of day"
    bind:this={untilStop}
    onclick={openUntil}
  >
    <span class="tick"></span>
    <span class="label">{untilLabel}</span>
  </button>
</div>

{#if untilOpen}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form class="until-form" onsubmit={submitUntil} onkeydown={onFormKey}>
    <label>
      <span class="sr-only">Stay awake until</span>
      <input type="time" bind:value={untilValue} bind:this={untilInput} required />
    </label>
    <button type="submit" class="set">Set</button>
    <span class="hint">Today, or tomorrow if that time has passed.</span>
  </form>
  <TellTheSun ondone={() => (untilOpen = false)} />
{/if}

<style>
  .rule {
    position: relative;
    display: flex;
    justify-content: space-between;
    padding: 10px 24px 0;
  }

  .line {
    position: absolute;
    left: 24px;
    right: 24px;
    top: 10px;
    height: 1px;
    background: var(--hair-strong);
  }

  .stop {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0 4px 8px;
    color: var(--ink-3);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    transition: color var(--t-fast) var(--ease);
  }

  .stop:hover {
    color: var(--ink-2);
  }

  .tick {
    width: 1px;
    height: 9px;
    background: var(--hair-strong);
    transition:
      background-color var(--t-fast) var(--ease),
      transform var(--t-fast) var(--ease);
  }

  .stop.active {
    color: var(--ink);
  }

  .stop.active .tick {
    background: var(--ink);
    transform: scaleX(2);
  }

  .stop.active::before {
    content: '';
    position: absolute;
    top: -4px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ink);
  }

  .label {
    white-space: nowrap;
  }

  .until-form {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 24px 8px;
  }

  .set {
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--paper);
    font-size: 12px;
  }

  .hint {
    font-size: 11px;
  }
</style>
