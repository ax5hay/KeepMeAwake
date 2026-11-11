<script lang="ts">
  import { ADD_APP, ADD_APP_PLACEHOLDER, WATCH_ROWS } from '$lib/copy';

  interface Props {
    /** The apps watched, as written. */
    apps: string[];
    /** Apps running now (from the latest look), to mark which are live. */
    running: string[];
    /** Names offered by the field; asked for when the field is focused. */
    choices: string[];
    onfocus: () => void;
    onchange: (apps: string[]) => void;
  }

  const { apps, running, choices, onfocus, onchange }: Props = $props();

  let draft = $state('');
  // Offer only what is not on the list already.
  const offered = $derived(
    choices.filter((name) => !apps.some((app) => app.toLowerCase() === name.toLowerCase())),
  );

  function add(event: SubmitEvent) {
    event.preventDefault();
    const name = draft.trim();
    if (name === '') return;
    draft = '';
    if (apps.some((app) => app.toLowerCase() === name.toLowerCase())) return;
    onchange([...apps, name]);
  }

  function remove(name: string) {
    onchange(apps.filter((app) => app !== name));
  }
</script>

<div class="apps">
  <div class="label">{WATCH_ROWS.apps.label}</div>
  <p class="hint">{WATCH_ROWS.apps.hint}</p>
  {#if apps.length > 0}
    <ul class="chips">
      {#each apps as app (app)}
        <li class:live={running.includes(app)}>
          <span class="dot" aria-hidden="true"></span>
          <span class="name">{app}</span>
          <button
            type="button"
            class="remove"
            aria-label="Stop watching {app}"
            title="Stop watching {app}"
            onclick={() => remove(app)}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg
            >
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <form class="add" onsubmit={add}>
    <input
      type="text"
      list="running-apps"
      aria-label={ADD_APP}
      placeholder={ADD_APP_PLACEHOLDER}
      autocomplete="off"
      spellcheck="false"
      maxlength="64"
      bind:value={draft}
      onfocus={() => onfocus()}
    />
    <datalist id="running-apps">
      {#each offered as name (name)}
        <option value={name}></option>
      {/each}
    </datalist>
    <button type="submit" class="save" disabled={draft.trim() === '' || apps.length >= 16}>
      Add
    </button>
  </form>
</div>

<style>
  .apps {
    padding: 11px 0;
    border-top: 1px solid var(--hair);
  }

  .label {
    color: var(--ink);
  }

  .hint {
    margin: 4px 0 0;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }

  .chips li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 2px 2px 9px;
    border: 1px solid var(--hair-strong);
    border-radius: 999px;
    font-size: 12px;
    color: var(--ink);
  }

  /* A ring while it is not running; the sun's colour while it is, since
     that is what is keeping the laptop awake. */
  .dot {
    width: 6px;
    height: 6px;
    border: 1px solid var(--ink-3);
    border-radius: 50%;
  }

  .live .dot {
    border-color: var(--sun);
    background: var(--sun);
  }

  .remove {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--ink-3);
  }

  .remove:hover,
  .remove:focus-visible {
    color: var(--ink);
    background: var(--paper-2);
  }

  .remove svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
  }

  .add {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }

  .add input {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--hair-strong);
    border-radius: var(--radius-sm);
    background: var(--paper);
    font-size: 12px;
  }

  .add input::placeholder {
    color: var(--ink-3);
  }

  .save {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    background: var(--ink);
    color: var(--paper);
    font-size: 12px;
  }

  .save:disabled {
    opacity: 0.4;
  }
</style>
