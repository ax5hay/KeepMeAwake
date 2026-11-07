# Keep me awake — notes for AI-assisted work

Personal project by Akshay Bajpai (akshaybajpai.com). Start at
`docs/HANDOFF.md` (where the last session stopped), then `docs/specs/BRIEF.md`
if you are building a spec. Read `docs/development.md` (conventions) and
`docs/design.md` (UI rules) before changing anything. This file is the short
version.

## Shape

- Tauri 2. Rust in `src-tauri/src` owns behaviour; Svelte 5 in `src` is a
  view over a `Snapshot`.
- Layers: `app → engine → core`, `engine → platform`. `core` has no I/O.
- The only `unsafe` is in `src-tauri/src/platform/windows.rs` and
  `src-tauri/src/platform/macos.rs`. Nowhere else.
- Development happens on two machines, a Windows PC and a Mac, both pushing
  to `main`. Pull first; keep platform work inside `platform/` and `cfg`
  blocks.
- Anything built on one machine is not done until it is built and checked on
  the other: `npm run verify` there, and the spec's acceptance items that
  apply. Until then it is listed under **Waiting for the other machine** in
  `docs/HANDOFF.md`.
- Every change updates the docs it touches in the same series of commits,
  and every session ends by updating `docs/HANDOFF.md` so the other machine
  knows exactly where to start.
- `src/lib/types.ts` mirrors `src-tauri/src/core` field for field, `snake_case`.
  Change both or neither.
- UI copy that is more than a label lives in `src/lib/copy.ts`.
- Copy that names the OS, the tray or a shortcut is written per platform
  (`platformCopy(mac)`, `tips(mac)` in `copy.ts`, `isMac` from `platform.ts`);
  tests forbid "tray"/"Windows" in Mac strings and "menu bar"/⌘ in Windows ones.

## Commands

- `npm run verify` before claiming anything works: lint, svelte-check, vitest,
  clippy (pedantic, `-D warnings`), `cargo test`.
- `npm run dev` runs the app. `npm run build` makes installers.
- Neither machine can compile the other's build (Tauri's build script needs
  that OS's SDK). CI runs on `windows-latest` and `macos-latest`: watch it
  (`gh run watch`) after pushing platform code.
- Release: bump, move Unreleased to the version in `CHANGELOG.md`, `git tag
vX.Y.Z` and push the tag; `release.yml` drafts it. Check all four assets
  are attached, then `gh release edit vX.Y.Z --draft=false [--prerelease]`.
- Mac: Homebrew's rustup is keg-only; `/opt/homebrew/opt/rustup/bin` must be
  on PATH or npm scripts and the git hooks cannot find cargo.

## Rules

- Small Conventional Commits (subject ≤ 72 chars), each buildable. Scopes:
  core, platform, engine, app, tray, ui, brand, docs, ci, build, deps.
- Tests accompany behaviour. Engine tests use `platform::fakes` and wait on
  conditions, never fixed sleeps.
- No new timers, polling or animation loops without a note in
  `docs/resource-budget.md`.
- UI: tokens only, no raw colours, no gradients, one accent (the sun). Copy is
  short and names what the laptop is doing. Nothing should look generic.
- Docs use Mermaid for diagrams, tables for anything list-like, and end with
  the one-line author footer.
- Settings fields must default; a change of meaning bumps `SETTINGS_VERSION`.
- Keep `docs/` and `CHANGELOG.md` current in the same change.
- End every session by updating `docs/HANDOFF.md` (last updated, where to
  start, in progress) and pushing it.
- Do not add input injection of any kind; see ADR 0002 and the roadmap.
- The repo is public. Files, docs and commit messages never carry secrets
  or where they are kept (the updater key lives only in repository secrets
  and with the owner), nor personal or machine details: no local paths,
  usernames, emails other than the noreply address, account names, drive or
  hardware details. Describe a machine by its role ("the Windows PC").
- Nothing personal in the repo beyond the author's name and website: no
  machine models or specs, emails, places, employers. Commit as Akshay
  Bajpai with the ax5hay GitHub noreply address (repo-local git config).
- Commit subjects need a type: `feat(brand): …`, never `brand: …`.
- A roadmap item gets a spec in `docs/specs/` (indexed in its README,
  linked from the roadmap) before it is built.

## Gotchas

- Engine steps send their `Snapshot` first and other events after. To show
  something did _not_ happen, use the tests' `settled()` (waits for the
  snapshot, then `shutdown()` joins the worker), never `drain()`'s 50 ms
  quiet. Positive checks use `events_until`.
- Listeners that must hear the first step (startup plan, a window open at
  launch) go in `Engine::start_with`, not `subscribe`.
- The UI build targets Safari 15 (macOS 12): no `Array.prototype.at`,
  `structuredClone` or other 15.4+ APIs; the target lowers syntax, not methods.
- On macOS, tray-icon's `set_icon` drops the template flag; use
  `set_icon_with_as_template`.
- Clippy's `doc_markdown` rejects product names in docs; add them to
  `src-tauri/clippy.toml` `doc-valid-idents`.
