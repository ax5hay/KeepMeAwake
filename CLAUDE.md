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
- The only `unsafe` is in `src-tauri/src/platform/windows.rs` and, once the
  macOS track lands, `src-tauri/src/platform/macos.rs`. Nowhere else.
- Two machines push to `main`: Windows work on the work laptop, macOS work on
  the personal Mac (`docs/specs/macos.md`). Pull first; keep platform work
  inside `platform/` and `cfg` blocks.
- `src/lib/types.ts` mirrors `src-tauri/src/core` field for field, `snake_case`.
  Change both or neither.
- UI copy that is more than a label lives in `src/lib/copy.ts`.

## Commands

- `npm run verify` before claiming anything works: lint, svelte-check, vitest,
  clippy (pedantic, `-D warnings`), `cargo test`.
- `npm run dev` runs the app. `npm run build` makes installers.

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
