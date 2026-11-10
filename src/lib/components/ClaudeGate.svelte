<script lang="ts">
  /**
   * The frame every Claude feature sits in, so each one says the same thing
   * the same way: what it is, whether it can be used, and why not. The
   * feature is always shown, never hidden: locked with no key, switched off
   * with one, or ready, with its own switch beside its name.
   */
  import type { Snippet } from 'svelte';

  import Switch from './Switch.svelte';
  import type { AiFeature } from '$lib/claude';
  import { AI_ADD_KEY, AI_LOCKED, AI_OFF, AI_SWITCH_ON } from '$lib/copy';
  import { store } from '$lib/state.svelte';

  interface Props {
    feature: AiFeature;
    title: string;
    /** What it does and what it sends, shown while it is off. */
    hint: string;
    children: Snippet;
  }

  const { feature, title, hint, children }: Props = $props();

  const gate = $derived(store.gate(feature));
</script>

<div class="gate" data-gate={gate}>
  <div class="head">
    <span class="by">
      {#if gate === 'locked'}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <rect x="3.5" y="7" width="9" height="6.5" rx="1.5" />
          <path d="M5.5 7V5.2a2.5 2.5 0 0 1 5 0V7" />
        </svg>
      {/if}
      Claude
    </span>
    <span class="title">{title}</span>
    <Switch
      checked={gate === 'on'}
      disabled={gate === 'locked' || gate === 'off'}
      label={title}
      onchange={(v) => void store.setAiFeature(feature, v)}
    />
  </div>
  {#if gate === 'locked'}
    <p class="why">
      {AI_LOCKED}
      <button type="button" class="link" onclick={() => store.openSettings('ai')}>
        {AI_ADD_KEY}
      </button>
    </p>
  {:else if gate === 'off'}
    <p class="why">
      {AI_OFF}
      <button type="button" class="link" onclick={() => void store.setAiEnabled(true)}>
        {AI_SWITCH_ON}
      </button>
    </p>
  {:else if gate === 'feature-off'}
    <p class="why">{hint}</p>
  {:else}
    {@render children()}
  {/if}
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 22px;
  }

  /* Whose feature it is: said once, small, in the eyebrow's voice. */
  .by {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border-radius: 999px;
    box-shadow: inset 0 0 0 1px var(--hair-strong);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-2);
    white-space: nowrap;
  }

  .by svg {
    width: 11px;
    height: 11px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .title {
    flex: 1;
    min-width: 0;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .gate:not([data-gate='on']) .title {
    color: var(--ink-2);
  }

  .why {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--ink-2);
    text-wrap: pretty;
  }

  .link {
    padding: 0;
    font-size: 12px;
    color: var(--ink);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
    transition: text-decoration-color var(--t-fast) var(--ease);
  }

  .link:hover {
    text-decoration-color: var(--ink);
  }
</style>
