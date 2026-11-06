<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    hint?: string;
    children: Snippet;
  }

  const { label, hint, children }: Props = $props();
</script>

<div class="row">
  <div class="text">
    <div class="label">{label}</div>
    {#if hint}
      <div class="hint">{hint}</div>
    {/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 11px 0;
    border-top: 1px solid var(--hair);
  }

  .text {
    min-width: 0;
  }

  .label {
    color: var(--ink);
  }

  .hint {
    margin-top: 1px;
    font-variant-numeric: tabular-nums;
  }

  .control {
    display: flex;
    align-items: center;
    flex: none;
  }
</style>
