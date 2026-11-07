# Handoff

The one page every agent or session reads first and updates last. Keep it
short; link out for detail.

**Last updated:** 2026-11-07 07:35 (IST) · by Claude Opus 5.5 (0.2 build session)
**Repo state:** `main` is green (`npm run verify`: vitest 54, cargo test 111), pushed. All four 0.2 specs are built and in `CHANGELOG.md` under Unreleased; 0.2.0 is not tagged yet (version is still 0.1.0).

## Where to start

1. Tag 0.2.0 when Akshay is happy with it: follow [`releasing.md`](releasing.md) (bump the version, move Unreleased to `[0.2.0]`, build, tag).
2. Then read [`specs/BRIEF.md`](specs/BRIEF.md) and start [`specs/auto-update.md`](specs/auto-update.md) at commit 1. It needs a signing decision first (see open questions).
3. Before testing on the real app: `taskkill /IM keepmeawake.exe /F` (a release build is running hidden in the tray). Setting `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` before `npm run dev` lets you drive and inspect the UI over the DevTools protocol.

## In progress

| Spec                     | Commit reached | Notes                                                                                    |
| ------------------------ | -------------- | ---------------------------------------------------------------------------------------- |
| schedule                 | 7 of 7         | done; also a separate engine fix (lost wake-up) before commit 3                          |
| notifications            | 5 of 5         | done; battery toasts verified by unit tests only (no unplugging in the session)          |
| global-hotkey            | 3 of 3         | done                                                                                     |
| tray-icons-light-taskbar | 4 of 4         | done; paused-state ink icon verified by unit test only (could not pause without battery) |

## Done this cycle

| Date       | What                                                                                            | By               |
| ---------- | ----------------------------------------------------------------------------------------------- | ---------------- |
| 2026-11-07 | 0.1.0 built end to end; docs, CI, specs for 0.2 and 0.3; roadmap to 0.5                         | Claude Fable 5.1 |
| 2026-11-07 | 0.2 built: schedule, notifications, global hotkey, taskbar-aware tray icons; engine wake-up fix | Claude Opus 5.5  |

## Open questions for Akshay

- Signing certificate for installers (needed before auto-update ships).
- Confirm the default for **Check for updates daily** (spec says on).
- Toasts name "Windows PowerShell" as their source unless the app is installed
  (the plugin only sets the app id outside `target/`). Expected, but worth a
  look in an installed 0.2 build.
- Spec choices made on the way, each noted in its commit body: skip is stored
  as wall-clock ms, not an `Instant`; the wait for the next window is capped at
  5 minutes so sleep cannot make it hours late; the tray's skip item shows only
  while a window is open; no `notification:default` capability (toasts are sent
  from Rust); ink tray icons are monochrome because the shipped ones have no
  amber.

## How to update this page

At the end of every session: set **Last updated**, move your spec's row in
**In progress** (commit number reached, anything half-done), append to **Done
this cycle**, and rewrite **Where to start** so the next agent can begin in
one step. Commit it as `docs(handoff): …` and push.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
