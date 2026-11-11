<script lang="ts">
  /**
   * The themes, each shown as what it is: a small horizon in its own
   * colours, with its sun on the line. A swatch wears its theme through the
   * same attribute the page does, so what it shows cannot drift from what
   * choosing it gives. System is the two it moves between, side by side.
   */
  import { SHELVES } from '$lib/theme';
  import type { Theme } from '$lib/types';

  interface Props {
    value: Theme;
    onchange: (theme: Theme) => void;
  }

  const { value, onchange }: Props = $props();

  const halves = (id: Theme): ReadonlyArray<string> => (id === 'system' ? ['light', 'dark'] : [id]);
</script>

<div class="picker">
  {#each SHELVES as shelf (shelf.title)}
    <div class="shelf" role="radiogroup" aria-label={`${shelf.title} themes`}>
      <div class="title"><span class="eyebrow">{shelf.title}</span> <span>{shelf.hint}</span></div>
      <div class="swatches">
        {#each shelf.themes as theme (theme.id)}
          <button
            type="button"
            role="radio"
            aria-checked={theme.id === value}
            class="swatch"
            class:chosen={theme.id === value}
            onclick={() => onchange(theme.id)}
          >
            <span class="view" aria-hidden="true">
              {#each halves(theme.id) as half (half)}
                <span class="half" data-theme={half}>
                  <span class="ground"></span>
                  <span class="sun"></span>
                </span>
              {/each}
            </span>
            <span class="name">{theme.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .picker {
    display: grid;
    gap: 14px;
    padding: 4px 0 12px;
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 7px;
    font-size: 12px;
    color: var(--ink-3);
  }

  .swatches {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px 8px;
  }

  .swatch {
    display: grid;
    gap: 4px;
    min-width: 0;
    text-align: left;
  }

  .view {
    display: flex;
    height: 38px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    box-shadow: 0 0 0 1px var(--hair-strong);
    transition: box-shadow var(--t-fast) var(--ease);
  }

  .swatch:hover .view {
    box-shadow: 0 0 0 1px var(--ink-2);
  }

  .swatch.chosen .view {
    box-shadow:
      0 0 0 2px var(--paper),
      0 0 0 3.5px var(--ink);
  }

  .half {
    position: relative;
    flex: 1;
    background: var(--sky-on);
  }

  .ground {
    position: absolute;
    inset: 62% 0 0;
    background: var(--ground-on);
    border-top: 1px solid var(--hair-strong);
  }

  /* The sun sits on the line, half risen, as it does on Home. */
  .sun {
    position: absolute;
    left: 50%;
    top: 62%;
    width: 13px;
    height: 13px;
    margin: -7px 0 0 -6.5px;
    border-radius: 50%;
    background: var(--sun);
  }

  .name {
    overflow: hidden;
    font-size: 11px;
    line-height: 1.25;
    color: var(--ink-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .swatch.chosen .name {
    color: var(--ink);
  }
</style>
