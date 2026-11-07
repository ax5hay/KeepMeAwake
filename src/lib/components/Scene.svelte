<script lang="ts">
  /**
   * The horizon. Sky and ground are flat bands; the sun is the toggle. A grain
   * overlay keeps the flat colour from feeling like a screenshot of a slide,
   * and a band of afterglow above the line gives the flat sky its depth.
   */
  import { store } from '$lib/state.svelte';
  import { SCENE, remainingArcPath, sunPosition } from '$lib/sun';

  const mode = $derived(store.snapshot?.mode ?? 'off');
  const paused = $derived(store.snapshot?.paused !== null && store.snapshot !== null);
  const sun = $derived(sunPosition(mode, store.progress));
  const timed = $derived(mode === 'timed');
  const awake = $derived(mode !== 'off' && !paused);
  const label = $derived(mode === 'off' ? 'Keep awake' : 'Let it sleep');

  const { width, height, horizonY, sunRadius } = SCENE;

  /** A few fixed stars; they fade in while resting. Hand-placed, not random. */
  const stars = [
    { x: 46, y: 38, r: 1.1 },
    { x: 92, y: 74, r: 0.8 },
    { x: 138, y: 30, r: 0.9 },
    { x: 246, y: 52, r: 1.1 },
    { x: 298, y: 26, r: 0.8 },
    { x: 338, y: 86, r: 1 },
    { x: 212, y: 104, r: 0.7 },
  ];
</script>

<div class="scene" data-mode={mode} data-paused={paused}>
  <svg viewBox="0 0 {width} {height}" role="img" aria-label="Horizon showing the current state">
    <defs>
      <filter id="grain" x="0" y="0" width="100%" height="100%">
        <feTurbulence
          type="fractalNoise"
          baseFrequency="0.95"
          numOctaves="2"
          stitchTiles="stitch"
        />
        <feColorMatrix type="saturate" values="0" />
      </filter>
    </defs>

    <rect class="sky" {width} height={horizonY} />
    <!-- Afterglow as three printed steps, not a gradient. -->
    <rect class="afterglow" y={horizonY - 54} {width} height="54" />
    <rect class="afterglow" y={horizonY - 32} {width} height="32" />
    <rect class="afterglow" y={horizonY - 14} {width} height="14" />

    <g class="stars" data-visible={!awake}>
      {#each stars as star (star.x)}
        <circle cx={star.x} cy={star.y} r={star.r} />
      {/each}
    </g>

    {#if timed}
      <path class="arc" d={remainingArcPath(sun)} />
    {/if}

    <g class="sun" style="transform: translate({sun.x}px, {sun.y}px)">
      <circle class="halo" r={sunRadius + 10} />
      <circle class="disc" r={sunRadius} />
      {#if paused}
        <rect class="cloud" x={-sunRadius - 4} y="-3" width={sunRadius * 2 + 8} height="6" rx="3" />
      {/if}
    </g>

    <rect class="ground" y={horizonY} {width} height={height - horizonY} />
    <rect class="horizon" x="22" y={horizonY - 1} width={width - 44} height="2" rx="1" />

    <g class="reflection" opacity={awake ? 1 : 0}>
      <rect x={sun.x - 30} y={horizonY + 22} width="60" height="4" rx="2" />
      <rect x={sun.x - 16} y={horizonY + 38} width="32" height="4" rx="2" />
    </g>

    <rect class="grain" {width} {height} filter="url(#grain)" />
  </svg>

  <button
    type="button"
    class="hit"
    style="left: {(sun.x / width) * 100}%; top: {(sun.y / height) * 100}%"
    aria-label={label}
    title="{label} (Space)"
    onclick={() => void store.toggle()}
  ></button>
</div>

<style>
  .scene {
    position: relative;
    width: 100%;
    max-width: 440px;
    margin: 0 auto;
    aspect-ratio: 380 / 240;
    flex: none;
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .sky {
    fill: var(--sky-off);
    transition: fill var(--t-slow) var(--ease);
  }

  .scene[data-mode='indefinite'] .sky,
  .scene[data-mode='timed'] .sky {
    fill: var(--sky-on);
  }

  .scene[data-paused='true'] .sky {
    fill: var(--sky-paused);
  }

  /* Dusk light along the horizon: the sun's colour at low opacity. Stronger
     while resting (the sun has just set), fainter at midday. */
  .afterglow {
    fill: var(--sun);
    opacity: var(--afterglow-off);
    transition: opacity var(--t-slow) var(--ease);
  }

  .scene[data-mode='indefinite'] .afterglow,
  .scene[data-mode='timed'] .afterglow {
    opacity: var(--afterglow-on);
  }

  .scene[data-paused='true'] .afterglow {
    opacity: 0.02;
  }

  .stars circle {
    fill: var(--ink);
  }

  .stars {
    opacity: 0;
    transition: opacity var(--t-slow) var(--ease);
  }

  .stars[data-visible='true'] {
    opacity: var(--stars);
  }

  .ground {
    fill: var(--ground-off);
    transition: fill var(--t-slow) var(--ease);
  }

  .scene[data-mode='indefinite'] .ground,
  .scene[data-mode='timed'] .ground {
    fill: var(--ground-on);
  }

  .horizon {
    fill: var(--ink);
  }

  .arc {
    fill: none;
    stroke: var(--hair-strong);
    stroke-width: 1;
    stroke-dasharray: 1.5 4;
    stroke-linecap: round;
  }

  .sun {
    transition: transform var(--t-slow) var(--ease);
    will-change: transform;
  }

  .disc {
    fill: var(--sun);
    transition: fill var(--t-slow) var(--ease);
  }

  .scene[data-paused='true'] .disc {
    fill: var(--sun-dim);
  }

  .halo {
    fill: var(--sun);
    opacity: 0;
    transition: opacity var(--t-fast) var(--ease);
  }

  .scene:has(.hit:hover) .halo,
  .scene:has(.hit:focus-visible) .halo {
    opacity: 0.18;
  }

  .cloud {
    fill: var(--paper);
  }

  .reflection rect {
    fill: var(--sun);
  }

  .reflection rect:first-child {
    opacity: 0.5;
  }

  .reflection rect:last-child {
    opacity: 0.28;
  }

  .reflection {
    transition: opacity var(--t-slow) var(--ease);
  }

  .grain {
    opacity: var(--grain-opacity);
    mix-blend-mode: var(--grain-blend);
    pointer-events: none;
  }

  /* Invisible circular hit target that tracks the sun. Keeps the SVG free of
     interactivity and gives keyboard users a real button. */
  .hit {
    position: absolute;
    width: 17%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    transition:
      left var(--t-slow) var(--ease),
      top var(--t-slow) var(--ease);
  }

  .hit:focus-visible {
    outline: 1.5px solid var(--ink);
    outline-offset: 4px;
  }
</style>
