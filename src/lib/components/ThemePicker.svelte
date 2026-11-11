<script lang="ts">
  /**
   * The themes, each shown as what it is: a small horizon in its own
   * colours, with its sun on the line. A swatch wears its theme through the
   * same attribute the page does, so what it shows cannot drift from what
   * choosing it gives. System is the two it moves between, side by side.
   *
   * Pointing at a swatch wears it on the whole window until the pointer
   * leaves; a click keeps it. The last shelf is a theme of one's own: five
   * colours, and one line of text to give it to someone else.
   */
  import { OWN_CODE_BAD, OWN_CODE_HINT, OWN_COPIED, OWN_COPY, OWN_PART_LABELS } from '$lib/copy';
  import { OWN_PARTS, SHELVES, THEMES, ownStyle, parseThemeCode, themeCode } from '$lib/theme';
  import type { OwnTheme, Theme } from '$lib/types';

  interface Props {
    value: Theme;
    own: OwnTheme;
    onchange: (theme: Theme) => void;
    /** A theme to wear for a look, or `null` to stop looking. */
    onpreview: (theme: Theme | null) => void;
    onown: (own: OwnTheme) => void;
  }

  const { value, own, onchange, onpreview, onown }: Props = $props();

  /** Arrows walk the themes, wearing each: a radio group's keys. */
  function onkeydown(event: KeyboardEvent) {
    if ((event.target as HTMLElement | null)?.tagName === 'INPUT') return;
    const step = { ArrowRight: 1, ArrowDown: 4, ArrowLeft: -1, ArrowUp: -4 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const from = THEMES.findIndex((theme) => theme.id === value);
    const to = Math.min(THEMES.length - 1, Math.max(0, from + step));
    const next = THEMES[to];
    if (!next || to === from) return;
    onchange(next.id);
    picker?.querySelector<HTMLElement>(`[data-id='${next.id}']`)?.focus();
  }

  let picker = $state<HTMLElement | null>(null);

  const halves = (id: Theme): ReadonlyArray<string> => (id === 'system' ? ['light', 'dark'] : [id]);

  /** The line as typed, while it is not yet a theme. */
  let draft = $state<string | null>(null);
  let copied = $state(false);
  const code = $derived(themeCode(own));
  const bad = $derived(draft !== null && parseThemeCode(draft) === null);

  function oncode(event: Event) {
    const text = (event.currentTarget as HTMLInputElement).value;
    const parsed = parseThemeCode(text);
    if (parsed) {
      draft = null;
      onown(parsed);
    } else {
      draft = text;
    }
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      copied = true;
      setTimeout(() => (copied = false), 2400);
    } catch {
      // No clipboard: the line is in the field, to be selected by hand.
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="picker" bind:this={picker} {onkeydown} onmouseleave={() => onpreview(null)}>
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
            data-id={theme.id}
            tabindex={theme.id === value ? 0 : -1}
            class:chosen={theme.id === value}
            onclick={() => onchange(theme.id)}
            onmouseenter={() => onpreview(theme.id)}
          >
            <span class="view" aria-hidden="true">
              {#each halves(theme.id) as half (half)}
                <span
                  class="half"
                  data-theme={half}
                  style={half === 'own' ? ownStyle(own) : undefined}
                >
                  <span class="ground"></span>
                  <span class="sun"></span>
                </span>
              {/each}
            </span>
            <span class="name">{theme.label}</span>
          </button>
        {/each}

        {#if shelf.themes.some((theme) => theme.id === 'own')}
          <div class="own">
            <div class="parts">
              {#each OWN_PARTS as part (part)}
                <label>
                  <input
                    type="color"
                    value={own[part]}
                    onchange={(event) => {
                      onown({ ...own, [part]: event.currentTarget.value.toLowerCase() });
                      if (value !== 'own') onchange('own');
                    }}
                  />
                  <span>{OWN_PART_LABELS[part]}</span>
                </label>
              {/each}
            </div>
            <div class="code">
              <input
                type="text"
                class:bad
                aria-label={OWN_CODE_HINT}
                spellcheck="false"
                autocomplete="off"
                value={draft ?? code}
                onchange={oncode}
              />
              <button type="button" class="link" onclick={() => void copy()}>
                {copied ? OWN_COPIED : OWN_COPY}
              </button>
            </div>
            <p class="hint">{bad ? OWN_CODE_BAD : OWN_CODE_HINT}</p>
          </div>
        {/if}
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
    align-self: start;
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

  /* One's own: the five colours beside the swatch, the line to share below. */
  .own {
    grid-column: 2 / -1;
    display: grid;
    gap: 7px;
    min-width: 0;
  }

  .parts {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
  }

  .parts label {
    display: grid;
    gap: 3px;
    font-size: 10px;
    color: var(--ink-2);
    text-align: center;
  }

  input[type='color'] {
    width: 100%;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: none;
    box-shadow: 0 0 0 1px var(--hair-strong);
    cursor: pointer;
  }

  input[type='color']::-webkit-color-swatch-wrapper {
    padding: 0;
  }

  input[type='color']::-webkit-color-swatch {
    border: 0;
    border-radius: 4px;
  }

  .code {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .code input {
    flex: 1;
    min-width: 0;
    padding: 4px 6px;
    border-radius: 4px;
    background: var(--paper-2);
    box-shadow: inset 0 0 0 1px var(--hair);
    font-family: ui-monospace, 'Cascadia Mono', Menlo, monospace;
    font-size: 10.5px;
    color: var(--ink);
  }

  .code input.bad {
    box-shadow: inset 0 0 0 1px var(--ink);
  }

  .link {
    flex: none;
    font-size: 12px;
    color: var(--ink-2);
    text-decoration: underline;
    text-decoration-color: var(--hair-strong);
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--ink);
  }

  .hint {
    font-size: 11px;
  }
</style>
