<script lang="ts">
  /**
   * One of AI's features, inside an AiGroup: its name, its own
   * switch, and under them either what it does (while it cannot be used) or
   * the feature itself.
   */
  import type { Snippet } from 'svelte';

  import Switch from './Switch.svelte';
  import type { AiFeature } from '$lib/ai';
  import { store } from '$lib/state.svelte';

  interface Props {
    feature: AiFeature;
    title: string;
    /** What it does, shown while it is locked, off, or switched off itself. */
    hint: string;
    children: Snippet;
  }

  const { feature, title, hint, children }: Props = $props();

  const gate = $derived(store.gate(feature));
  const settings = $derived(store.settings?.ai ?? null);
</script>

<div class="feature" data-gate={gate}>
  <div class="row">
    <span class="title">{title}</span>
    <Switch
      checked={settings?.[feature] ?? false}
      disabled={gate === 'locked' || gate === 'off'}
      label={title}
      onchange={(v) => void store.setAiFeature(feature, v)}
    />
  </div>
  {#if gate === 'on'}
    {@render children()}
  {:else}
    <p class="hint">{hint}</p>
  {/if}
</div>

<style>
  .feature {
    padding: 8px 0 0;
  }

  .feature + :global(.feature) {
    margin-top: 8px;
    border-top: 1px solid var(--hair);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .title {
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Not usable yet: named, but quietly. */
  .feature:not([data-gate='on']) .title {
    color: var(--ink-2);
  }

  .hint {
    margin: 1px 0 0;
    font-size: 12px;
    color: var(--ink-3);
    text-wrap: pretty;
  }
</style>
