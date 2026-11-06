<script lang="ts" generics="T extends string">
  interface Props {
    options: ReadonlyArray<{ value: T; label: string }>;
    value: T;
    label: string;
    onchange: (value: T) => void;
  }

  const { options, value, label, onchange }: Props = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      class:active={option.value === value}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    padding: 2px;
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 0 1px var(--hair-strong);
  }

  button {
    padding: 3px 9px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--ink-2);
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }

  button:hover {
    color: var(--ink);
  }

  button.active {
    background: var(--ink);
    color: var(--paper);
  }
</style>
