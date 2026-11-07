<script lang="ts">
  /**
   * Seven weekday toggles, Monday first. Each is a real button with
   * `aria-pressed`, so the row reads as seven switches to a screen reader.
   */
  import { WEEKDAYS } from '$lib/format';

  interface Props {
    days: readonly boolean[];
    onchange: (days: boolean[]) => void;
  }

  const { days, onchange }: Props = $props();

  function flip(index: number) {
    onchange(WEEKDAYS.map((_, i) => (i === index ? !days[i] : (days[i] ?? false))));
  }
</script>

<div class="pills" role="group" aria-label="Days">
  {#each WEEKDAYS as day, i (day.name)}
    <button
      type="button"
      aria-pressed={days[i] ?? false}
      aria-label={day.name}
      title={day.name}
      onclick={() => flip(i)}
    >
      {day.letter}
    </button>
  {/each}
</div>

<style>
  .pills {
    display: flex;
    gap: 3px;
  }

  button {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--ink-2);
    box-shadow: inset 0 0 0 1px var(--hair-strong);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease),
      box-shadow var(--t-fast) var(--ease);
  }

  button:hover {
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--ink-2);
  }

  button[aria-pressed='true'] {
    background: var(--ink);
    color: var(--paper);
    box-shadow: inset 0 0 0 1px var(--ink);
  }

  button:focus-visible {
    border-radius: 50%;
  }
</style>
