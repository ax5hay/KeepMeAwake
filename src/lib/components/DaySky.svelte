<script lang="ts">
  /**
   * Today, drawn as the horizon: the wide window's sky. Its width is the
   * day. Each stretch the laptop was kept awake rises from the line as a
   * dome of the sun's light, as tall as it was long; the one running now
   * climbs to the sun, which stands at now. The working hours are the lit
   * sky, the rest is the resting sky with its stars. Ink on the line is
   * where someone was here, and the night runs dotted under it, as the
   * Almanac draws them.
   *
   * One SVG, rebuilt only when the history, the almanac, the size or the
   * minute changes. The domes rise once, when the window opens.
   */
  import { onMount } from 'svelte';

  import { stripOf } from '$lib/almanac';
  import { endedWord } from '$lib/history';
  import { formatClock, formatDuration } from '$lib/format';
  import {
    awakeMs,
    dayAround,
    domeBands,
    domePath,
    domesBetween,
    hourMarks,
    nowAt,
    type Dome,
  } from '$lib/sky';
  import { store } from '$lib/state.svelte';

  interface Props {
    height?: number;
  }

  const { height = 300 }: Props = $props();

  let width = $state(0);
  // The sun walks with the clock: once a minute is a few pixels at most.
  let now = $state(Date.now());
  onMount(() => {
    const tick = () => {
      if (document.visibilityState === 'visible') now = Date.now();
    };
    const id = window.setInterval(tick, 60_000);
    document.addEventListener('visibilitychange', tick);
    return () => {
      window.clearInterval(id);
      document.removeEventListener('visibilitychange', tick);
    };
  });

  const reading = $derived(store.reading);
  const snapshot = $derived(store.snapshot);
  const day = $derived(
    reading ? { start: reading.today.start, end: reading.today.end } : dayAround(now),
  );
  const strip = $derived(reading ? stripOf(reading, reading.today) : null);
  const domes = $derived(domesBetween(store.history, day.start, day.end, now));
  const awake = $derived(snapshot !== null && snapshot.mode !== 'off');
  const paused = $derived(snapshot?.paused != null);

  const horizon = $derived(Math.round(height * 0.8));
  const tallest = $derived(Math.round(horizon * 0.66));
  const radius = 22;
  const at = $derived(nowAt(now, day.start, day.end));
  const sunX = $derived(Math.min(Math.max(at * width, radius + 6), width - radius - 6));
  const running = $derived(domes.find((d) => d.running) ?? null);
  const sunY = $derived.by(() => {
    if (!awake) return horizon + radius * 0.5;
    // On the dome it is climbing, or as high as a dome goes.
    return running ? domePath(running, width, horizon, tallest).peak : horizon - tallest;
  });

  // Fixed stars, as in the scene: placed by hand, never random.
  const STARS = [
    [0.03, 0.18],
    [0.07, 0.42],
    [0.11, 0.12],
    [0.16, 0.31],
    [0.21, 0.08],
    [0.26, 0.24],
    [0.71, 0.14],
    [0.76, 0.33],
    [0.81, 0.09],
    [0.86, 0.26],
    [0.91, 0.15],
    [0.95, 0.38],
  ] as const;
  // Stars only in the resting sky, not over the working hours.
  const lit = $derived(
    STARS.filter(([x]) => {
      if (!strip || strip.hours.width === 0) return true;
      return x < strip.hours.left || x > strip.hours.left + strip.hours.width;
    }),
  );

  const marks = $derived(hourMarks(day.start, day.end));
  const hourLabel = (ms: number) =>
    new Intl.DateTimeFormat(undefined, { hour: 'numeric' }).format(new Date(ms)).toLowerCase();

  let hovered = $state<Dome | null>(null);
  const caption = $derived.by(() => {
    if (hovered) {
      const span = `${formatClock(hovered.start_ms)} – ${hovered.running ? 'now' : formatClock(hovered.end_ms)}`;
      const length = formatDuration(Math.round((hovered.end_ms - hovered.start_ms) / 1000));
      const why = hovered.ended ? ` · ended by ${endedWord(hovered.ended)}` : ' · still awake';
      return `${span} · ${length}${why}`;
    }
    const total = awakeMs(domes, day.end - day.start);
    if (total < 60_000) return 'Not kept awake yet today.';
    return `Kept awake ${formatDuration(Math.round(total / 1000))} today`;
  });

  const label = $derived.by(() => {
    if (snapshot?.mode === 'scheduled') return "Skip today's schedule";
    if (snapshot?.mode === 'watching') return 'Let it sleep until this ends';
    return awake ? 'Let it sleep' : 'Keep awake';
  });
  const px = (n: number) => `${n.toFixed(1)}px`;
</script>

<figure class="sky" bind:clientWidth={width} style="height: {height}px" data-awake={awake}>
  {#if width > 0}
    <svg {width} {height} viewBox="0 0 {width} {height}" aria-hidden="true">
      <defs>
        <filter id="day-grain" x="0" y="0" width="100%" height="100%">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="0.95"
            numOctaves="2"
            stitchTiles="stitch"
          />
          <feColorMatrix type="saturate" values="0" />
        </filter>
      </defs>

      <!-- The resting sky, and the working hours lit. -->
      <rect class="sky-off" x="0" y="0" {width} height={horizon} />
      {#if strip && strip.hours.width > 0}
        <rect
          class="sky-on"
          x={strip.hours.left * width}
          y="0"
          width={strip.hours.width * width}
          height={horizon}
        />
      {:else if awake}
        <rect class="sky-on" x="0" y="0" {width} height={horizon} />
      {/if}
      {#each lit as [x, y], i (i)}
        <circle class="star" cx={x * width} cy={y * horizon} r="1.1" />
      {/each}

      <!-- Each stretch kept awake, as a dome of light. -->
      <g class="domes">
        {#each domes as dome, i (dome.start_ms)}
          {@const bands = domeBands(dome, width, horizon, tallest)}
          {@const outline = domePath(dome, width, horizon, tallest)}
          <g class="dome" class:running={dome.running} style="--i: {i}">
            {#each bands as band, b (b)}
              <path class="band" d={band} />
            {/each}
            <path class="edge" class:paused={dome.running && paused} d={outline.d} />
          </g>
          <!-- A wider, invisible target, so a thin dome is easy to point at. -->
          <path
            class="hit"
            role="presentation"
            d={bands[0]}
            onmouseenter={() => (hovered = dome)}
            onmouseleave={() => (hovered = null)}
          />
        {/each}
      </g>

      <!-- Now. -->
      <line class="now" x1={sunX} y1="0" x2={sunX} y2={height} />
      <circle class="sun" class:paused cx={sunX} cy={sunY} r={radius} />
      {#if paused}
        <rect
          class="cloud"
          x={sunX - radius * 1.25}
          y={sunY - 3}
          width={radius * 2.5}
          height="6"
          rx="3"
        />
      {/if}

      <!-- The ground, the line, who was here, and the night. -->
      <rect class="ground" x="0" y={horizon} {width} height={height - horizon} />
      {#if strip}
        {#each strip.runs as run, i (i)}
          <rect
            class="here"
            x={run.left * width}
            y={horizon - 1.5}
            width={Math.max(run.width * width, 1)}
            height="3"
          />
        {/each}
        {#each strip.quiet as night, i (i)}
          <line
            class="night"
            x1={night.left * width}
            y1={horizon + 12}
            x2={(night.left + night.width) * width}
            y2={horizon + 12}
          />
        {/each}
      {/if}
      <line class="horizon" x1="0" y1={horizon} x2={width} y2={horizon} />
      {#each marks as mark (mark.at)}
        <line
          class="tick"
          x1={mark.at * width}
          y1={horizon}
          x2={mark.at * width}
          y2={horizon + 6}
        />
        <text class="hour" x={mark.at * width + 6} y={horizon + 22}>{hourLabel(mark.unix_ms)}</text>
      {/each}

      <rect class="grain" x="0" y="0" {width} {height} filter="url(#day-grain)" />
    </svg>

    <!-- The sun is the switch here too: a real button over it. -->
    <button
      type="button"
      class="hit-sun"
      aria-label={label}
      title="{label} (Space)"
      style="left: {px(sunX - radius - 6)}; top: {px(sunY - radius - 6)}"
      onclick={() => void store.toggle()}
    ></button>
    <figcaption>{caption}</figcaption>
  {/if}
</figure>

<style>
  .sky {
    position: relative;
    margin: 0;
    overflow: hidden;
    flex: none;
  }

  svg {
    display: block;
  }

  .sky-off {
    fill: var(--sky-off);
  }

  .sky-on {
    fill: var(--sky-on);
  }

  .star {
    fill: var(--stars);
    opacity: 0.55;
  }

  .sky[data-awake='true'] .star {
    opacity: 0.25;
  }

  /* Light, not paint: the sun's colour in three stepped bands, as the
     scene's afterglow stands over its horizon. */
  .band {
    fill: var(--sun);
    fill-opacity: 0.11;
  }

  .edge {
    fill: none;
    stroke: var(--sun);
    stroke-opacity: 0.6;
    stroke-width: 1.3;
  }

  /* The stretch running now is drawn in ink up to the sun. */
  .running .edge {
    stroke: var(--ink);
    stroke-opacity: 1;
    stroke-linecap: round;
  }

  .running .edge.paused {
    stroke-dasharray: 2 5;
  }

  .hit {
    fill: transparent;
    stroke: transparent;
    stroke-width: 14;
    pointer-events: all;
    cursor: default;
  }

  /* The one orchestrated moment: the day's light rises as the window opens. */
  .dome {
    transform-box: fill-box;
    transform-origin: bottom;
    animation: rise var(--t-slow) var(--ease-out) both;
    animation-delay: calc(var(--i, 0) * 60ms);
  }

  @keyframes rise {
    from {
      transform: scaleY(0);
    }
  }

  .now {
    stroke: var(--ink-3);
    stroke-width: 1;
    stroke-dasharray: 1 4;
    opacity: 0.6;
  }

  .sun {
    fill: var(--sun);
    transition:
      cx var(--t-slow) var(--ease),
      cy var(--t-slow) var(--ease);
  }

  .sun.paused {
    fill: var(--sun-dim);
  }

  .cloud {
    fill: var(--sky-paused);
  }

  .ground {
    fill: var(--ground-off);
  }

  .sky[data-awake='true'] .ground {
    fill: var(--ground-on);
  }

  .horizon {
    stroke: var(--ink);
    stroke-width: 1.5;
  }

  .here {
    fill: var(--ink);
  }

  .night {
    stroke: var(--ink-3);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-dasharray: 0.1 6;
  }

  .tick {
    stroke: var(--ink-3);
    stroke-width: 1;
  }

  .hour {
    fill: var(--ink-3);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .grain {
    opacity: var(--grain-opacity);
    mix-blend-mode: var(--grain-blend);
    pointer-events: none;
  }

  .hit-sun {
    position: absolute;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    transition:
      left var(--t-slow) var(--ease),
      top var(--t-slow) var(--ease);
  }

  .hit-sun:focus-visible {
    outline: 1.5px solid var(--ink);
    outline-offset: 2px;
  }

  figcaption {
    position: absolute;
    left: 28px;
    top: 18px;
    color: var(--ink-2);
    font-family: var(--font-display);
    font-style: italic;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .dome {
      animation: none;
    }

    .sun,
    .hit-sun {
      transition: none;
    }
  }
</style>
