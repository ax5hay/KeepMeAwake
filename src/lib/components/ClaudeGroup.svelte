<script lang="ts">
  /**
   * The frame Claude's features sit in, wherever they are. It says once, for
   * all of them, whose they are and whether they can be used: locked with no
   * key, switched off with one, or ready. The features inside are always
   * listed, so what Claude could do is seen before any of it is switched on.
   * With "Hide AI" on, the whole frame is gone.
   */
  import type { Snippet } from 'svelte';

  import { AI_ADD_KEY, AI_LOCKED, AI_OFF, AI_SWITCH_ON } from '$lib/copy';
  import { store } from '$lib/state.svelte';

  interface Props {
    children: Snippet;
  }

  const { children }: Props = $props();

  const state = $derived(store.aiState);
</script>

{#if state !== 'hidden'}
  <div class="group" data-state={state}>
    <p class="head">
      <span class="by">
        {#if state === 'locked'}
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect x="3.5" y="7" width="9" height="6.5" rx="1.5" />
            <path d="M5.5 7V5.2a2.5 2.5 0 0 1 5 0V7" />
          </svg>
        {/if}
        Claude
      </span>
      {#if state === 'locked'}
        <span class="why">{AI_LOCKED}</span>
        <button type="button" class="link" onclick={() => store.openSettings('ai')}>
          {AI_ADD_KEY}
        </button>
      {:else if state === 'off'}
        <span class="why">{AI_OFF}</span>
        <button type="button" class="link" onclick={() => void store.setAiEnabled(true)}>
          {AI_SWITCH_ON}
        </button>
      {/if}
    </p>
    {@render children()}
  </div>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 8px;
    min-height: 22px;
    margin: 0;
    font-size: 12px;
  }

  /* Whose features these are: said once, small, in the eyebrow's voice. */
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

  .why {
    color: var(--ink-2);
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
