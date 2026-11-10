# Claude

Version 0.5 · Size M · Touches core, app, ui

## Why

Two things in the app are easier said than set: what a week was like, and
how long to stay awake. A language model does both well. But an app that
keeps a laptop awake has no business talking to a server on its own, so the
whole of this is built around one rule: **AI is a choice**, made three times
over, and visible every time.

## The three locks

```mermaid
stateDiagram-v2
    direction LR
    Locked: Locked<br/>no API key
    Off: Off<br/>a key, Claude switched off
    FeatureOff: Feature off<br/>Claude on, this feature not
    On: On<br/>asked only on a click
    Locked --> Off: add a key
    Off --> On: Use Claude
    On --> FeatureOff: the feature's own switch
    FeatureOff --> On: the feature's own switch
    On --> Off: Use Claude, off
    Off --> Locked: remove the key
```

| Lock        | Default | Where it is                                                   |
| ----------- | ------- | ------------------------------------------------------------- |
| An API key  | none    | Settings → Claude; kept in the OS credential store            |
| Use Claude  | off     | Settings → Claude, and **Switch it on** wherever a feature is |
| The feature | on      | a switch beside the feature's name, where the feature is      |

A feature is never hidden. Without a key it is shown locked, with a lock and
`Locked: no API key yet.` **Add a key**. With a key and Claude off it says
`Claude is switched off.` **Switch it on**. With its own switch off it says
what it would do. Only with all three open does it show something to click,
and beside that, what clicking sends.

Nothing is ever sent on the app's own initiative: no request at launch, on a
timer, or in the background. Every request begins with a click.

## Experience

Every Claude feature sits in the same frame (`ClaudeGate`): a small `CLAUDE`
chip, the feature's name, its switch.

| Feature                 | Where                             | Asks with      | Sends                                           | Gives back                                     |
| ----------------------- | --------------------------------- | -------------- | ----------------------------------------------- | ---------------------------------------------- |
| **This week, in words** | Almanac, between week and month   | Read this week | the week being read, one line a day             | two or three sentences, in the whisper's voice |
| **Tell the sun**        | Home, under the rule, with Until… | Ask            | the sentence typed, and the local date and time | `Awake for 2 hours`, with **Start** and **No** |

Tell the sun never starts anything on its own: it shows what it understood
and waits. While a plan is being typed, Home's two rows and its doors step
aside to give it room.

The week is sent as the pane shows it: hours, first and last touch, time
over, the night, whether a day was leave or set by hand. Nothing else in the
almanac leaves the machine.

## Settings

Settings → new section **Claude**, after **Almanac**.

| Field              | Type   | Default | UI                                                                    |
| ------------------ | ------ | ------- | --------------------------------------------------------------------- |
| `ai.enabled`       | `bool` | `false` | **Use Claude**: `Off, nothing is ever sent to Anthropic.`             |
| `ai.week_in_words` | `bool` | `true`  | **This week, in words** (also beside the feature)                     |
| `ai.tell_the_sun`  | `bool` | `true`  | **Tell the sun** (also beside the feature)                            |
| (not a setting)    |        |         | **API key**: a password field and Save; then `A key is saved.` Remove |

The key is not in `settings.json`. It goes from the field to the app in one
call and into the OS credential store (Windows Credential Manager, macOS
Keychain); the window is only ever told whether there is one.

## Rust

### core/ai.rs (new, pure)

Builds request bodies and reads response bodies; no network.

| Item                   | What                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------- |
| `MODEL`                | `claude-opus-5-5`                                                                           |
| `week_request(digest)` | system prompt (say night, never slept; no advice), `effort: low`, `fallbacks: "default"`    |
| `plan_request(s, now)` | the same, with `output_config.format` a JSON schema: `kind`, `minutes`, `time`, `say`       |
| `text_of(response)`    | the text blocks; `stop_reason: "refusal"` is an error, checked before the content is read   |
| `understood(response)` | the plan; a length of no minutes or a time that is not one reads as `unclear`               |
| `AiError`              | `NoKey`, `Off`, `BadKey` (401), `Busy` (429, 529), `Declined`, `TooLong`, `Empty`, `Failed` |

Thinking is always on for this model and is never configured; effort is the
only control and is set low, since both jobs are small. Input over 12,000
characters is refused, never cut.

### app/ai.rs (new)

- `KeyStore` trait; `OsKeyStore` over the `keyring` crate.
- `Claude::unlock(settings, feature)`: the key, only if all three locks are
  open. `Claude::ask(key, body)`: one POST to `/v1/messages` over the HTTPS
  client the updater already ships, with `x-api-key`, `anthropic-version`
  and the fallback beta. The key is not logged.
- Raw HTTP, because there is no official Anthropic SDK for Rust; the shapes
  are the documented ones.

## UI

`claude.ts` (pure): `gateOf`, `planOf`, `nowLocal`. `ClaudeGate.svelte`,
`TellTheSun.svelte`; the week section in `Almanac.svelte`; Settings →
Claude. `almanac.ts::weekDigest` lays out what is sent.

## IPC

| Command        | Args              | Returns                                    |
| -------------- | ----------------- | ------------------------------------------ |
| `ai_status`    |                   | `{ has_key }`                              |
| `set_ai_key`   | `key \| null`     | `{ has_key }`                              |
| `ai_read_week` | `digest`          | the words, or a sentence saying why not    |
| `ai_plan`      | `sentence`, `now` | `Understood`, or a sentence saying why not |

## Tests

Core: the bodies name the model, ask for low effort and send no thinking or
sampling parameters; the schema's shape; nothing and too much are refused
before sending; a refusal is not read; a plan is read; half an answer is
unclear; statuses map to errors. App, with a key kept in memory: locked
without a key whatever the switches; off by default with one; each feature's
switch; removing the key; the window learns only `has_key`. UI: the gate's
four states; a plan from what was understood; the digest; the copy says
locked and off differently and names Anthropic wherever something is sent.

## Docs

Guide, architecture (security posture: the app now makes a second kind of
request, only when asked), design, changelog, roadmap, handoff.

## Resource impact

Nothing at rest and nothing at launch beyond one read of the credential
store when the window opens. One HTTPS request per click. `keyring` is the
one new crate; `reqwest` and `rustls` were already built for the updater.
The exe's growth is to be measured at the next release build.

## Acceptance

- [ ] No key: both features show locked, with the lock and Add a key.
- [ ] Add a key: both say Claude is switched off, with Switch it on.
- [ ] Switch it on: Read this week returns two or three sentences; Tell the
      sun understands `two hours` and `until half six`, and starts only on
      Start.
- [ ] A wrong key says `Anthropic did not accept that API key.`
- [ ] Each feature's switch turns only that feature off.
- [ ] Use Claude off: no request is made, by any path.
- [ ] The key is in the OS credential store and not in `settings.json`.

## Commits

1. `docs(specs): claude, as a choice`
2. `feat(app): Claude behind three locks, and a break reminder`
3. `feat(ui): Claude where its features are, a month as CSV`
4. `docs: Claude, the break reminder and CSV`

## Out of scope

Choosing the model, streaming the answer, a chat, anything sent without a
click, other providers, keys shared between machines, a monthly or yearly
reading, cost shown in the app.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
