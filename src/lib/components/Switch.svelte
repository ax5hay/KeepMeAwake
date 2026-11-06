<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (value: boolean) => void;
  }

  const { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  class="switch"
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    --w: 34px;
    --h: 18px;
    position: relative;
    width: var(--w);
    height: var(--h);
    border-radius: 999px;
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--hair-strong);
    transition:
      background-color var(--t-fast) var(--ease),
      box-shadow var(--t-fast) var(--ease);
    flex: none;
  }

  .switch[aria-checked='true'] {
    background: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--ink);
  }

  .switch:disabled {
    opacity: 0.4;
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ink-2);
    transition:
      transform var(--t-fast) var(--ease-out),
      background-color var(--t-fast) var(--ease);
  }

  .switch[aria-checked='true'] .knob {
    background: var(--paper);
    transform: translateX(16px);
  }

  .switch:hover:not(:disabled) .knob {
    background: var(--ink);
  }

  .switch[aria-checked='true']:hover:not(:disabled) .knob {
    background: var(--paper);
  }
</style>
