# Auto-update

Version 0.3 · Size M · Touches app, ui, ci

## Why

A tray app that is never reinstalled is never updated. One signed check a day
against GitHub Releases keeps every install current without a store.

## Experience

- On launch and then every 24 hours, the app fetches
  `https://github.com/ax5hay/KeepMeAwake/releases/latest/download/latest.json`.
  Failures are silent (logged at `debug`).
- When a newer version exists: the tray status line gains a second line
  `Update ready: 0.3.1`; the About section shows
  `0.3.1 is available · Install and restart` as a link; a one-time whisper on
  the next window open says `A new version is up: 0.3.1.`
- Clicking the link downloads with a thin progress line under the About
  heading (`Downloading… 42%`), installs, and relaunches.
- Settings → About gains a switch **Check for updates automatically** with hint
  `Once a day and when you open this window, from GitHub.` Default on.

## Settings

```rust
pub check_for_updates: bool,        // true
pub last_update_check_unix_ms: u64, // 0; written by the app, not the UI
```

## Rust

- Add `tauri-plugin-updater = "2"` and `tauri-plugin-process = "2"`; register
  after `global-shortcut`. Capabilities: `updater:default`, `process:allow-restart`.
- `tauri.conf.json`:

  ```json
  "plugins": {
    "updater": {
      "pubkey": "<from tauri signer generate>",
      "endpoints": ["https://github.com/ax5hay/KeepMeAwake/releases/latest/download/latest.json"]
    }
  }
  ```

  and `bundle.createUpdaterArtifacts: true`.

- `app/update.rs` (new): `check_now(app) -> Option<UpdateInfo { version, notes }>`
  using the plugin; a background thread at setup that sleeps until the next
  due check (24 h after `last_update_check_unix_ms`), checks, stores the result
  in `AppState.update: Mutex<Option<UpdateInfo>>`, emits event `update` with
  the info, refreshes the tray, and writes `last_update_check_unix_ms`.
- Commands: `get_update` → `Option<UpdateInfo>`, `install_update` → downloads
  with progress events `update-progress { downloaded, total }` then calls
  `app.restart()`.
- Tray: `status_line` appends the update line when present.

## UI

- `types.ts`: `UpdateInfo`.
- Store: `update = $state<UpdateInfo | null>`, listens to `update` and
  `update-progress`.
- `Settings.svelte` About section: the link, the progress line, the switch.
- `copy.ts`: the whisper and the strings above.

## CI

- `release.yml`: set `includeUpdaterJson: true` on `tauri-action` and pass
  `TAURI_SIGNING_PRIVATE_KEY` / `_PASSWORD` (already wired). The job must fail
  if the secret is missing once this spec ships; add a step that checks.
- Generate the key pair once with `npx tauri signer generate`, written outside the repo;
  commit the public key in the config, store the private key and password as
  repository secrets. Document in `docs/releasing.md`.

## IPC

| Command          | Returns              | Events                          |
| ---------------- | -------------------- | ------------------------------- |
| `get_update`     | `UpdateInfo \| null` | `update` (info)                 |
| `install_update` |                      | `update-progress`, then restart |

## Tests

Rust: the due-time arithmetic (`next_check_due(last, now)`) is pure and
tested; `status_line` with an update present. UI: store accepts `update` and
progress events. The network path is verified by hand against a real release.

## Docs

Guide (About section), architecture security posture (one outbound HTTPS
request a day, to GitHub, opt-out), releasing (signing, `latest.json`),
changelog.

## Resource impact

One thread sleeping on a 24 h deadline; one HTTPS request a day. The updater
pulls in `reqwest` + TLS: ~1.5 MB binary. Record it; this is the one feature
allowed to cost that.

## Acceptance

- [ ] Publish 0.3.0, then a 0.3.1 draft with updater artifacts; a 0.3.0
      install shows the update within a minute of launch and installs in place.
- [ ] With the switch off, no request is made (verify with the log at debug).
- [ ] Signature mismatch (edit `latest.json` locally) is refused and logged.

## Commits

1. `build(tauri): add updater and process plugins, public key and endpoint`
2. `feat(app): check GitHub for updates once a day`
3. `feat(app): install an update with progress and restart`
4. `feat(ui): show available updates in About and the tray`
5. `ci: publish updater artifacts and latest.json on release`
6. `docs: updates in the guide, releasing and security posture`

## Out of scope

Release channels, delta updates, update notes rendering.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
