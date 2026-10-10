# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-10, from the repo at `86c5dbf` and the 2026-10-03 to 2026-10-08 sessions.
> Rules for keeping it current: `CLAUDE.md`. History: `handoff/archive/`.

## What this is

A Windows desktop tool (Rust, egui) that builds IL-2 Great Battles **Korea** mission `.Group` files so the mission editor needs less hand wiring. Module map: `docs/src-guide.md`. End-user manual: `USER_MANUAL.md`.

- **Fork:** `Pliskin-Industries/Claude-IL2MissionUtility` (`origin`). **Upstream:** `eszyman/IL2MissionUtility` (`upstream`, the original creator).
- **Backlog:** GitHub Issues on the fork (#1 to #30).
- **Build and test:** `cargo test --offline`, then `cargo build --release`. A fresh machine needs `cargo fetch`, rustup and the Visual Studio C++ Build Tools first.
- **Names:** the user calls P14 Air Tasking **"Wing Commander"** or **"WG/CC"**. That name is not in the code.

## Where the work is

| Branch | Worktree folder | Holds |
|---|---|---|
| `main` | `Claude-IL2MissionUtility` | v0.6.0 plus the P14 plan. 3 ahead and 1 behind `origin/main`. |
| `claude/p14-air-tasking` | `Claude-IL2MissionUtility-fix` | **The active branch.** P14 steps 1a, 1, 1b, 0T, 0b and 2, and the flight 0 results. Has commits that are not pushed. |
| `codex/p14-3` | `Claude-IL2MissionUtility-0t` | P14 step 3, built but **held** and not merged. |
| `trial/upstream-merge` | `Claude-IL2MissionUtility-merge` | Uncommitted trial merge of upstream. Do not build on it. |
| `codex/p14-0b` | `Claude-IL2MissionUtility-0b` | Already merged. A leftover worktree. |
| `claude/docs-layout-trial` | `Claude-IL2MissionUtility-docs` | This document layout. Merged 2026-10-10. A leftover worktree. |

Last recorded test run on the active branch: 540 passed, 8 ignored; 69 build and 251 clippy warnings (2026-09-28). Not re-run since. Record each new run on this line.

## In progress: P14 Air Tasking (issues #28, #29)

Plan: `handoff/P14-air-tasking.md` (long; read §8 for steps and §10 for the build order). Its user decisions are §2.1 (U1 to U24).

- **Flight 0 was flown on 2026-10-03 and gate G0 passed.** Results: `handoff/P14-probe-results.md`. Video check: `References/Recordings/flight0-harvest/README.md`.
- **Step 3 is unblocked but not merged.** Its commit (`3224286`) reports 583 passed, 11 ignored. It still needs the lines marked `// P14 G0: default pending flight 0` settled, the probe fixes merged in (it is 24 commits behind), the shape and string guards extended to Probe 2, and a full review.
- **Flights 1A and 1B are not flown.** Their generators exist (step 0b); their mission files have not been built. Step 4 waits for flight 1A (gate G1).

## Waiting on the user

1. **Decision: AI aircraft trigger check zones.** Flight 0 showed it. Options and the recommendation are in `docs/decisions/0008-ai-aircraft-trigger-check-zones.md`. It is not needed before flight 1A.
2. **A Complex Trigger reference file** saved from the editor to `TemplateExamples/Reference/ComplexTrigger_reference.Group`, with a screenshot of its properties. It is not in the repo yet. Flight 1A's country-filter cells need it.
3. **Whether objective breadcrumbs show as map markers.** The flight 0 video shows nothing in the cockpit view, but the map was never opened.
4. **Go-ahead for the next steps below.** They were proposed on 2026-10-03 and not confirmed.
5. **Older confirmations still listed in the plan:** §10 item 23 (a smaller Auto-fill) and the "For the user to confirm" list.

## Next steps (proposed order)

1. Finish step 3 as listed above and merge it into `claude/p14-air-tasking`.
2. Merge upstream. It is now 6 commits ahead, including v0.7.0 and the split of the UI into per-tab files. Trial result and the one expected baseline change: `handoff/sessions/2026-10-07-flight-0-follow-up.md`.
3. Build flights 1A and 1B with the country-filter cells, each under a new mission name.

## Traps

- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
