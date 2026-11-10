<script lang="ts">
  /**
   * One day as a small horizon, a turn of the day at each end. On a working
   * day a band of sky stands where the working hours are; ink lies on the line wherever someone
   * was here, so ink outside the band is time over. The night runs dotted
   * under the line, the way the sun's path does in the scene, and on today
   * the sun sits on the line at now, at the leading edge of the ink.
   *
   * Drawn with positioned boxes rather than an SVG: the strip stretches with
   * the pane, and boxes stretch without bending their dots or their sun.
   */
  import type { Strip } from '$lib/almanac';

  interface Props {
    strip: Strip;
    /** The day's own strip, with its hours named; or a row of the week. */
    size?: 'day' | 'row';
    /** What the drawing says, for a screen reader. */
    label: string;
    /** One per quarter mark, on the day's own strip. */
    ticks?: readonly string[];
  }

  const { strip, size = 'day', label, ticks = [] }: Props = $props();

  const pct = (fraction: number) => `${(fraction * 100).toFixed(3)}%`;
</script>

<div class="strip" class:row={size === 'row'} role="img" aria-label={label}>
  <span class="hours" style="left: {pct(strip.hours.left)}; width: {pct(strip.hours.width)}"></span>
  {#each strip.quiet as quiet, i (i)}
    <span class="quiet" style="left: {pct(quiet.left)}; width: {pct(quiet.width)}"></span>
  {/each}
  <span class="line"></span>
  {#each strip.runs as run, i (i)}
    <span class="run" style="left: {pct(run.left)}; width: {pct(run.width)}"></span>
  {/each}
  {#if strip.sun !== null}
    <span class="sun" style="left: {pct(strip.sun)}"></span>
  {/if}
  {#if size === 'day'}
    {#each strip.ticks as tick, i (tick.at)}
      <span class="tick" style="left: {pct(tick.at)}"></span>
      <span class="at" class:first={i === 0} style="left: {pct(tick.at)}">{ticks[i] ?? ''}</span>
    {/each}
  {/if}
</div>

<style>
  .strip {
    --sky: 22px;
    --ink-height: 9px;
    --sun-size: 11px;
    position: relative;
    height: calc(var(--sky) + 30px);
  }

  .strip.row {
    --sky: 9px;
    --ink-height: 5px;
    --sun-size: 7px;
    height: calc(var(--sky) + 6px);
  }

  .strip > span {
    position: absolute;
  }

  /* Working hours: a band of the awake sky, standing on the line. */
  .hours {
    top: 0;
    height: var(--sky);
    background: var(--sky-on);
    border-radius: 2px 2px 0 0;
  }

  .line {
    left: 0;
    right: 0;
    top: var(--sky);
    height: 1px;
    background: var(--hair-strong);
  }

  .strip:not(.row) .line {
    height: 2px;
    margin-top: -1px;
    border-radius: 1px;
    background: var(--ink);
  }

  .run {
    top: calc(var(--sky) - var(--ink-height));
    height: var(--ink-height);
    min-width: 1px;
    background: var(--ink);
    border-radius: 1px 1px 0 0;
  }

  /* The night: dotted, under the line, like the path the sun has yet to go. */
  .quiet {
    top: calc(var(--sky) + 7px);
    border-top: 2px dotted var(--ink-3);
  }

  .row .quiet {
    top: calc(var(--sky) + 3px);
    border-top-width: 1px;
  }

  /* Now. The one accent, used where the sun is. */
  .sun {
    top: calc(var(--sky) - var(--sun-size));
    width: var(--sun-size);
    height: var(--sun-size);
    margin-left: calc(var(--sun-size) / -2);
    border-radius: 50%;
    background: var(--sun);
    box-shadow: 0 0 0 2px var(--paper);
  }

  .row .sun {
    box-shadow: 0 0 0 1.5px var(--paper);
  }

  .tick {
    top: calc(var(--sky) + 1px);
    width: 1px;
    height: 4px;
    background: var(--hair-strong);
  }

  .at {
    top: calc(var(--sky) + 14px);
    padding-left: 0;
    transform: translateX(-50%);
    font-size: 10px;
    line-height: 1.3;
    letter-spacing: 0.02em;
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The first label starts at the edge the line does. */
  .at.first {
    transform: none;
  }
</style>
