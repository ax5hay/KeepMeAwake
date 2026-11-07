# Handoff

The one page every agent or session reads first and updates last. Keep it
short; link out for detail.

**Last updated:** 2026-11-07 06:20 (IST) · by Claude Fable 5.1 (scoping session)
**Repo state:** `main` is green (`npm run verify`), pushed, 0.1.0 released locally (unsigned installers in `src-tauri/target/release/bundle/`).

## Where to start

1. Read [`specs/BRIEF.md`](specs/BRIEF.md), then [`specs/schedule.md`](specs/schedule.md).
2. Start the **Schedule** spec at commit 1 (`feat(core): add Schedule settings and pure window arithmetic`).
3. Work through the 0.2 specs in the order given in [`specs/README.md`](specs/README.md).

## In progress

| Spec     | Commit reached | Notes |
| -------- | -------------- | ----- |
| none yet | –              | –     |

## Done this cycle

| Date       | What                                                                    | By               |
| ---------- | ----------------------------------------------------------------------- | ---------------- |
| 2026-11-07 | 0.1.0 built end to end; docs, CI, specs for 0.2 and 0.3; roadmap to 0.5 | Claude Fable 5.1 |

## Open questions for Akshay

- Signing certificate for installers (needed before auto-update ships).
- Confirm the default for **Check for updates daily** (spec says on).

## How to update this page

At the end of every session: set **Last updated**, move your spec's row in
**In progress** (commit number reached, anything half-done), append to **Done
this cycle**, and rewrite **Where to start** so the next agent can begin in
one step. Commit it as `docs(handoff): …` and push.

---

_Keep me awake · [Akshay Bajpai](https://akshaybajpai.com)_
