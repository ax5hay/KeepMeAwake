# 0001. Tauri 2 with a Rust core and Svelte UI

Date: 2026-11-07 · Status: accepted

## Context

The app must idle in the tray at a few megabytes, have a UI I fully control
visually, and ship as one installer that needs no admin rights. The original
was a Python script; Python, Electron and .NET were the obvious alternatives.

| Option           | Idle memory                    | UI control | Notes                                    |
| ---------------- | ------------------------------ | ---------- | ---------------------------------------- |
| Python + tkinter | ~40 MB                         | poor       | ships an interpreter                     |
| Electron         | ~150 MB                        | full       | bundles Chromium                         |
| .NET WPF / WinUI | ~60 MB                         | good       | runtime always resident                  |
| Rust + egui/iced | ~10 MB                         | limited    | custom widgets, weak text layout         |
| Tauri 2          | ~5 MB tray, ~60 MB with window | full       | OS WebView2; the window can be destroyed |

## Decision

Tauri 2. Rust owns all behaviour; the UI is Svelte 5 compiled to a few tens of
kilobytes with no framework runtime; WebView2 lives only while the window is
open.

## Consequences

- Rust and the MSVC build tools are required to build (see
  `docs/development.md`).
- Two languages, two test suites, one `npm run verify`.
- Visual design is unconstrained; the resource budget is enforceable.
