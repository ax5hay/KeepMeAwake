# Development

How this repository is worked on: setup, the daily loop, the house style and
how things are tested. If something here drifts from reality, fixing the doc
is part of the change.

## Setup (Windows)

```mermaid
flowchart LR
    node["Node 22+"] --> install["npm install<br/>(wires git hooks)"]
    rust["Rust stable 1.90+<br/>rustup default stable-msvc"] --> install
    msvc["VS Build Tools<br/>VC.Tools.x86.x64 + Windows SDK"] --> install
    wv["WebView2<br/>(in Windows 11)"] --> dev
    install --> dev["npm run dev"]
```

```powershell
winget install OpenJS.NodeJS.LTS
winget install Rustlang.Rustup ; rustup default stable-msvc
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.26100"
npm install
npm run dev     # first Rust build takes a few minutes
```

Editor: VS Code with the extensions in `.vscode/extensions.json`.

## Daily commands

| Command             | Does                                                                  |
| ------------------- | --------------------------------------------------------------------- |
| `npm run dev`       | Vite dev server + `tauri dev`. UI hot-reloads; Rust rebuilds on save. |
| `npm run verify`    | What CI runs: lint, type-check, UI tests, clippy, Rust tests.         |
| `npm run lint`      | ESLint (0 warnings) and Prettier check. `npm run format` to fix.      |
| `npm run check`     | `svelte-check` with the strict tsconfig.                              |
| `npm test`          | Vitest once; `npm run test:watch` for watch mode.                     |
| `npm run lint:rust` | `cargo clippy --all-targets -D warnings` and `cargo fmt --check`.     |
| `npm run test:rust` | `cargo test`.                                                         |
| `npm run build`     | Release build plus NSIS and MSI installers.                           |

Rust-only loops are faster from `src-tauri`: `cargo test`, `cargo clippy`.

## Layout

| Path                      | Holds                                                              |
| ------------------------- | ------------------------------------------------------------------ |
| `.githooks/`              | pre-commit (prettier, eslint, rustfmt on staged files), commit-msg |
| `.github/`                | CI, release workflow, templates, Dependabot                        |
| `design/`                 | SVG sources for icons; regenerate with `tauri icon`                |
| `docs/`                   | this folder; `adr/` for decisions                                  |
| `legacy/`                 | the original script, kept as a souvenir                            |
| `src/`                    | Svelte UI                                                          |
| `src/lib/api.ts`          | the only file that calls `invoke()`                                |
| `src/lib/types.ts`        | mirrors of the Rust core types; keep in sync                       |
| `src/lib/*.ts`            | pure helpers, each with a `.test.ts` beside it                     |
| `src/lib/copy.ts`         | every piece of UI text that is more than a label                   |
| `src/lib/components/`     | one component per file, styles scoped inside                       |
| `src/styles/`             | `tokens.css` (palette, type, motion), `base.css` (reset)           |
| `src-tauri/src/core/`     | pure domain, no I/O                                                |
| `src-tauri/src/platform/` | OS traits; `windows.rs` is the only `unsafe`                       |
| `src-tauri/src/engine/`   | worker thread and policy                                           |
| `src-tauri/src/app/`      | Tauri shell: commands, tray, window                                |
| `src-tauri/capabilities/` | IPC permissions; keep minimal                                      |
| `src-tauri/icons/`        | generated; do not hand-edit                                        |

## Style

### Rust

- Edition 2024, `rustfmt` per `rustfmt.toml`. Clippy `pedantic` is on and CI
  treats warnings as errors; an `#[allow]` needs a comment saying why.
- `unsafe` only in `platform/windows.rs`, every block with a `SAFETY:` comment.
- Public items are documented. Doc comments say what and why, not how.
- `thiserror` enums at the platform boundary; `Result<_, String>` only at the
  Tauri command boundary.
- No `unwrap` / `expect` outside tests. Prefer `let … else`.
- `log`: `info` for state changes, `warn` for degraded-but-recovered,
  `error` for things the user will notice.

### TypeScript and Svelte

- Svelte 5 runes only; no `svelte/store`.
- `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`.
- One component per file; styles scoped; tokens from `tokens.css`, never raw
  colours in components.
- Pure logic lives in `src/lib/*.ts`, not in components, so it is testable
  without a DOM.
- Field names that cross IPC stay `snake_case`.

### Commits

[Conventional Commits](https://www.conventionalcommits.org), checked by the
`commit-msg` hook. Subject ≤ 72 characters.

```
feat(engine): pause below the battery threshold
fix(tray): rebuild menu after settings change
docs(design): explain why there are no gradients
```

Scopes: `core`, `platform`, `engine`, `app`, `tray`, `ui`, `brand`, `docs`,
`ci`, `build`, `deps`. Small commits, each buildable, each about one thing.
Linear history on `main`.

### Changes that touch settings

Fields must default, so adding one needs no migration. If a field changes
meaning, bump `SETTINGS_VERSION` and migrate in `persistence.rs`.

## Testing

```mermaid
flowchart LR
    subgraph rust["cargo test (110)"]
        core_t["core<br/>transitions · expiry · serde shape · defaults ·<br/>schedule windows, overnight, DST"]
        engine_t["engine<br/>policy table · worker with fakes:<br/>apply/release · expiry events · battery pause/resume ·<br/>OS failure surfaced · shutdown"]
        platform_t["platform<br/>pure decoders · one real power-request round trip"]
        tray_t["tray · notify<br/>labels · tooltip · menu ids · icons decode ·<br/>toast text per event and switch"]
    end
    subgraph ui["vitest (54)"]
        fmt["format · sun · theme · copy · hotkey<br/>pure helpers · DayPills"]
        store_t["store<br/>against a fake backend, fake timers"]
    end
```

Rules: a bug fix comes with the test that would have caught it. Tests with
fakes never sleep for fixed times; they wait on a condition with a bounded
timeout (`wait_for_guard` in `engine/mod.rs`).

## Debugging

| Need                  | Do                                                                           |
| --------------------- | ---------------------------------------------------------------------------- |
| see the log           | dev builds log at `debug` to stdout and the file; release at `info`          |
| prove the request     | `powercfg /requests` (elevated) lists "KeepMeAwake is keeping this PC awake" |
| inspect the UI        | right click → Inspect in dev builds                                          |
| reset settings        | delete the JSON file                                                         |
| screenshot the window | see `docs/assets/`; captured with a PowerShell `CopyFromScreen` helper       |

## Working with an AI assistant

`CLAUDE.md` at the root gives Claude Code these conventions in a form it reads
automatically. Keep the two in step.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
