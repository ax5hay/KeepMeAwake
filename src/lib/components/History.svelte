<script lang="ts">
  /**
   * The session log: the newest stretches first, each with its day, span,
   * length and how it ended, and its pauses as thin inset lines underneath.
   * `history` is null until it has loaded, and then nothing is drawn.
   */
  import { CLEAR_HISTORY, HISTORY_EMPTY } from '$lib/copy';
  import { historyRows } from '$lib/history';
  import type { History } from '$lib/types';

  interface Props {
    history: History | null;
    now: number;
    onclear: () => void;
  }

  const { history, now, onclear }: Props = $props();

  const rows = $derived(history ? historyRows(history, now) : []);
</script>

{#if history}
  {#if rows.length === 0}
    <p class="empty">{HISTORY_EMPTY}</p>
  {:else}
    <ol class="log">
      {#each rows as row (row.key)}
        <li>
          <span class="day">{row.day}</span>
          <span class="span">{row.span}</span>
          <span class="length">{row.length}</span>
          <span class="ended" class:now={row.ended === 'now'}>{row.ended}</span>
          {#each row.pauses as pause, i (i)}
            <span class="pause">{pause}</span>
          {/each}
        </li>
      {/each}
    </ol>
    <button type="button" class="clear" onclick={onclear}>{CLEAR_HISTORY}</button>
  {/if}
{/if}

<style>
  .empty {
    margin: 8px 0 0;
    font-family: var(--font-display);
    font-style: italic;
    font-size: 13px;
    color: var(--ink-3);
  }

  .log {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  li {
    display: grid;
    grid-template-columns: 4.6em 1fr auto 4.4em;
    column-gap: 8px;
    padding: 5px 0;
    border-top: 1px solid var(--hair);
  }

  li:first-child {
    border-top: 0;
  }

  .day,
  .span,
  .length {
    white-space: nowrap;
  }

  .day {
    color: var(--ink-2);
  }

  .span {
    color: var(--ink);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .length {
    color: var(--ink);
    text-align: right;
  }

  .ended {
    color: var(--ink-3);
    text-align: right;
  }

  /* Running: ink, not amber; the sun is the one accent. */
  .ended.now {
    color: var(--ink);
  }

  .pause {
    grid-column: 2 / -1;
    margin-top: 2px;
    padding-left: 8px;
    border-left: 1px solid var(--hair-strong);
    color: var(--ink-3);
    font-size: 11px;
  }

  .clear {
    margin-top: 8px;
    padding: 3px 0;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
    transition: color var(--t-fast) var(--ease);
  }

  .clear:hover {
    color: var(--ink);
    text-decoration-color: var(--ink);
  }
</style>
