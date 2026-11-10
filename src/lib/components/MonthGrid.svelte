<script lang="ts">
  /**
   * A month as a calendar, Monday first. Under each date a mark: a bar for a
   * day worked, longer for a longer day; a ring for leave, taken or still to
   * come. Today carries the sun. Each date is a real button, so the month is
   * walked with Tab and read out day by day.
   */
  import { barLength, type Day } from '$lib/almanac';
  import { WEEKDAYS } from '$lib/format';

  interface Props {
    rows: readonly (readonly (Day | null)[])[];
    /** The key of the day being read. */
    selected: string;
    /** What a day says to a screen reader and on hover. */
    describe: (day: Day) => string;
    onselect: (day: Day) => void;
  }

  const { rows, selected, describe, onselect }: Props = $props();
</script>

<div class="month" role="group" aria-label="Days of the month">
  <div class="week names" aria-hidden="true">
    {#each WEEKDAYS as weekday (weekday.name)}
      <span>{weekday.letter}</span>
    {/each}
  </div>
  {#each rows as row, r (r)}
    <div class="week">
      {#each row as day, c (day?.key ?? `blank-${c}`)}
        {#if day}
          <button
            type="button"
            class="day"
            class:today={day.today}
            class:off={!day.working}
            class:ahead={day.kind === 'ahead'}
            aria-pressed={day.key === selected}
            aria-label={describe(day)}
            title={describe(day)}
            onclick={() => onselect(day)}
          >
            <span class="sun" aria-hidden="true"></span>
            <span class="date">{Number(day.key.slice(-2))}</span>
            <span class="mark" aria-hidden="true">
              {#if day.kind === 'leave'}
                <span class="ring"></span>
              {:else if day.kind === 'worked'}
                <span class="bar" style="width: {Math.round(barLength(day) * 20)}px"></span>
              {/if}
            </span>
          </button>
        {:else}
          <span></span>
        {/if}
      {/each}
    </div>
  {/each}
</div>

<style>
  .week {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }

  .names {
    padding-bottom: 2px;
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--ink-3);
    text-align: center;
  }

  .day {
    display: grid;
    grid-template-rows: 6px 17px 9px;
    justify-items: center;
    align-items: center;
    padding: 2px 0 3px;
    border-radius: var(--radius-sm);
    color: var(--ink-2);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    transition:
      background-color var(--t-fast) var(--ease),
      color var(--t-fast) var(--ease);
  }

  .day:hover {
    color: var(--ink);
    background: var(--hair);
  }

  .day[aria-pressed='true'] {
    color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--hair-strong);
  }

  .day.off,
  .day.ahead {
    color: var(--ink-3);
  }

  .day.today {
    color: var(--ink);
    font-weight: 600;
  }

  .sun {
    width: 4px;
    height: 4px;
    border-radius: 50%;
  }

  .today .sun {
    background: var(--sun);
  }

  .mark {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }

  .bar {
    height: 2px;
    border-radius: 1px;
    background: var(--ink);
  }

  .ring {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.2px var(--ink-2);
  }
</style>
