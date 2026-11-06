<script lang="ts">
  /**
   * Steps through a fixed list of choices with two arrows. Compact, keyboard
   * friendly, and it never needs a dropdown.
   */
  interface Props {
    options: ReadonlyArray<{ value: string; label: string }>;
    value: string;
    label: string;
    onchange: (value: string) => void;
  }

  const { options, value, label, onchange }: Props = $props();

  const index = $derived(
    Math.max(
      0,
      options.findIndex((o) => o.value === value),
    ),
  );
  const current = $derived(options[index]?.label ?? '');

  function step(delta: number) {
    const next = options[Math.min(options.length - 1, Math.max(0, index + delta))];
    if (next && next.value !== value) onchange(next.value);
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') step(-1);
    else if (event.key === 'ArrowRight' || event.key === 'ArrowUp') step(1);
    else return;
    event.preventDefault();
  }
</script>

<div
  class="stepper"
  role="spinbutton"
  aria-label={label}
  aria-valuetext={current}
  tabindex="0"
  onkeydown={onKey}
>
  <button
    type="button"
    tabindex="-1"
    aria-label="Previous"
    disabled={index === 0}
    onclick={() => step(-1)}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M7.5 2.5 4 6l3.5 3.5" /></svg>
  </button>
  <span class="value">{current}</span>
  <button
    type="button"
    tabindex="-1"
    aria-label="Next"
    disabled={index === options.length - 1}
    onclick={() => step(1)}
  >
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M4.5 2.5 8 6l-3.5 3.5" /></svg>
  </button>
</div>

<style>
  .stepper {
    display: inline-flex;
    align-items: center;
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 0 1px var(--hair-strong);
  }

  .value {
    min-width: 64px;
    text-align: center;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  button {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    color: var(--ink-2);
    border-radius: 4px;
  }

  button:hover:not(:disabled) {
    color: var(--ink);
    background: var(--hair);
  }

  button:disabled {
    opacity: 0.3;
  }

  svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
