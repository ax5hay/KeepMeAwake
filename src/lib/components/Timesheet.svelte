<script lang="ts">
  /**
   * A month laid out for the printer: a row a day, and what it came to.
   * Never seen in the window; the print stylesheet in base.css shows this
   * and nothing else. Black on white whatever theme is worn: it is paper.
   */
  import { formatTotal, formatWorked, monthOf, tally, type Day } from '$lib/almanac';
  import { DAY_WORDS, daysWorked, leavesTaken } from '$lib/copy';
  import { formatClock } from '$lib/format';
  import { store } from '$lib/state.svelte';

  interface Props {
    year: number;
    month: number;
  }

  const { year, month }: Props = $props();

  const days = $derived(store.reading ? monthOf(store.reading, year, month) : []);
  const sum = $derived(tally(days));
  const title = $derived(
    new Date(year, month, 1).toLocaleDateString(undefined, { month: 'long', year: 'numeric' }),
  );

  const dayOf = (day: Day) =>
    new Date(day.start).toLocaleDateString(undefined, { weekday: 'short' });
  const worked = (day: Day) => day.kind === 'worked' || (day.today && day.worked_ms > 0);
  const note = (day: Day) => {
    if (worked(day)) return day.fixed ? 'Set by hand' : '';
    return day.kind === 'leave' || day.kind === 'rest' ? DAY_WORDS[day.kind] : '';
  };
</script>

<div class="timesheet">
  <h1>{title}</h1>
  <p class="sub">
    {formatTotal(sum.worked_ms)} · {daysWorked(sum.days)} worked · {leavesTaken(
      sum.leaves,
    ).toLowerCase()}
  </p>
  <table>
    <thead>
      <tr>
        <th>Date</th>
        <th>Day</th>
        <th>Started</th>
        <th>Ended</th>
        <th class="n">Hours</th>
        <th class="n">Extra</th>
        <th>Note</th>
      </tr>
    </thead>
    <tbody>
      {#each days as day (day.key)}
        <tr class:off={!day.working}>
          <td>{new Date(day.start).getDate()}</td>
          <td>{dayOf(day)}</td>
          <td>{worked(day) && day.first !== null ? formatClock(day.first) : ''}</td>
          <td>{worked(day) && day.last !== null && !day.today ? formatClock(day.last) : ''}</td>
          <td class="n">{worked(day) ? formatWorked(day.worked_ms) : ''}</td>
          <td class="n">{worked(day) && day.over_ms >= 60_000 ? formatWorked(day.over_ms) : ''}</td>
          <td>{note(day)}</td>
        </tr>
      {/each}
    </tbody>
    <tfoot>
      <tr>
        <td colspan="4">Total</td>
        <td class="n">{formatTotal(sum.worked_ms)}</td>
        <td class="n">{sum.over_ms >= 60_000 ? formatTotal(sum.over_ms) : ''}</td>
        <td></td>
      </tr>
    </tfoot>
  </table>
  <p class="foot">Keep me awake · from the almanac kept on this machine</p>
</div>

<style>
  .timesheet {
    display: none;
  }

  @media print {
    .timesheet {
      display: block;
      padding: 0;
      font-family: var(--font-ui);
      font-size: 10.5pt;
      color: #000;
    }

    h1 {
      font-family: var(--font-display);
      font-size: 22pt;
      font-weight: 400;
    }

    .sub {
      margin: 2pt 0 14pt;
      color: #444;
    }

    table {
      width: 100%;
      border-collapse: collapse;
    }

    th,
    td {
      padding: 4pt 6pt;
      border-bottom: 0.5pt solid #bbb;
      text-align: left;
    }

    th {
      border-bottom: 1pt solid #000;
      font-size: 8.5pt;
      font-weight: 600;
      letter-spacing: 0.06em;
      text-transform: uppercase;
    }

    .n {
      text-align: right;
      font-variant-numeric: tabular-nums;
    }

    tr.off td {
      color: #777;
    }

    tfoot td {
      border-top: 1pt solid #000;
      border-bottom: 0;
      font-weight: 600;
    }

    .foot {
      margin-top: 14pt;
      font-size: 8.5pt;
      color: #666;
    }
  }
</style>
