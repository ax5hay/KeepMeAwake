<script lang="ts">
  /**
   * A plan, said in words: "two hours", "until half six", "till I say".
   * The model reads the sentence; the sun does nothing until what it understood
   * has been shown and agreed to.
   */
  import AiFeature from './AiFeature.svelte';
  import AiGroup from './AiGroup.svelte';
  import { nowLocal, planOf, serviceOf } from '$lib/ai';
  import {
    TELL_ASK,
    TELL_HINT,
    TELL_PLACEHOLDER,
    TELL_START,
    TELL_THE_SUN,
    TELL_THINKING,
    sendsSentence,
  } from '$lib/copy';
  import { store } from '$lib/state.svelte';
  import type { Understood } from '$lib/types';

  interface Props {
    /** Called once a plan has been started, so the form around it can close. */
    ondone: () => void;
  }

  const { ondone }: Props = $props();

  let sentence = $state('');
  let asking = $state(false);
  let understood = $state<Understood | null>(null);

  const plan = $derived(understood ? planOf(understood, Date.now()) : null);

  async function ask(event: Event) {
    event.preventDefault();
    if (asking || sentence.trim() === '') return;
    asking = true;
    understood = await store.askPlan(sentence.trim(), nowLocal(Date.now()));
    asking = false;
  }

  async function start() {
    if (!plan) return;
    await store.setPlan(plan);
    sentence = '';
    understood = null;
    ondone();
  }

  // Esc clears what was understood before it leaves the form.
  function onKey(event: KeyboardEvent) {
    if (event.key !== 'Escape' || understood === null) return;
    event.stopPropagation();
    understood = null;
  }
</script>

{#if store.aiState !== 'hidden'}
  <div class="tell">
    <AiGroup>
      <AiFeature feature="tell_the_sun" title={TELL_THE_SUN} hint={TELL_HINT}>
        {#if understood}
          <p class="said" role="status">
            <span class="words">{understood.say}</span>
            {#if plan}
              <button type="button" class="go" onclick={() => void start()}>{TELL_START}</button>
            {/if}
            <button type="button" class="link" onclick={() => (understood = null)}>
              {plan ? 'No' : 'Again'}
            </button>
          </p>
        {:else}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <form onsubmit={ask} onkeydown={onKey}>
            <input
              type="text"
              aria-label={TELL_THE_SUN}
              placeholder={TELL_PLACEHOLDER}
              maxlength="200"
              bind:value={sentence}
              disabled={asking}
            />
            <button type="submit" class="go" disabled={asking || sentence.trim() === ''}>
              {asking ? TELL_THINKING : TELL_ASK}
            </button>
          </form>
          <p class="sends">{sendsSentence(serviceOf(store.settings?.ai ?? null).sentTo)}</p>
        {/if}
      </AiFeature>
    </AiGroup>
  </div>
{/if}

<style>
  .tell {
    padding: 2px 24px 8px;
  }

  form,
  .said {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 0;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--hair-strong);
    border-radius: var(--radius-sm);
    background: var(--paper);
    font-size: 12px;
  }

  input::placeholder {
    color: var(--ink-3);
  }

  .go {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--paper);
    font-size: 12px;
    white-space: nowrap;
    transition: opacity var(--t-fast) var(--ease);
  }

  .go:disabled {
    opacity: 0.4;
  }

  /* What it understood, in the whisper's voice: it is being said back. */
  .words {
    flex: 1;
    min-width: 0;
    font-family: var(--font-display);
    font-style: italic;
    font-size: 14px;
    color: var(--ink);
  }

  .sends {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--ink-3);
  }

  .link {
    padding: 0;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--ink);
  }
</style>
