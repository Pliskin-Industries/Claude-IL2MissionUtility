# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-12, after flights 1G, 1N and 1D and the handbook harvest.
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

Last recorded test run on the active branch: 544 passed, 0 failed, 10 ignored (`cargo test --offline`, 2026-10-12, after the last flight records). Warnings on 2026-10-12: 69 build, 251 clippy. Record each new run on this line.

## In progress: P14 Air Tasking (issues #28, #29)

Plan: `handoff/P14-air-tasking.md` (long; read §8 for steps and §10 for the build order). Its user decisions are §2.1 (U1 to U24).

- **Flight 0 was flown on 2026-10-03 and gate G0 passed.** Results: `handoff/P14-probe-results.md`. Video check: `References/Recordings/flight0-harvest/README.md`.
- **Step 3 is unblocked but not merged.** Its commit (`3224286`) reports 583 passed, 11 ignored. It still needs the lines marked `// P14 G0: default pending flight 0` settled, the probe fixes merged in (it is 24 commits behind), the shape and string guards extended to Probe 2, and a full review.
- **Flight 1A was flown on 2026-10-12 by Claude, as a flying check with an air start.** The T-c cells and ZA4 gave clear answers (c, the re-check and h confirmed; o = fires once; a zone fires for a player). Risk n (cruise speed) was not measured, and the country-filter cells were not in the build. Results: `handoff/P14-probe-results.md`, last section. **The user ruled that it counts as flight 1A: gate G1 is met and step 4 may start** (`docs/decisions/0009`).
- **Step 0e is built, merged (`aa3b048`) and flown.** Flights 1G, 1N and 1D were flown by Claude on 2026-10-12. Results: `handoff/P14-probe-results.md` (its last section is a one-table summary of step 0). Headlines: the country filter works on both sides (decision 0008, option 4); a lost aircraft raises OnEvent **13**, not 4, so the plan's loss counting must change; AI F-80Cs fly 733 km/h against the plan's 732; **one aircraft outlived its Delete**, which matters to the budget's hard limit (U22) and is not explained.
- **The game's handbook is harvested for the 12 aircraft:** `docs/game-handbook/aircraft-characteristics.md`. It ends with a comparison against `src/model_spec.rs`, whose 13 aircraft rows have no source. No code was changed.
- **Testing on this PC, with Claude flying (2026-10-12).** The game and editor run here on integrated graphics, and Claude can drive both. **Flight 0 was re-flown here as a calibration and matches the 2026-10-03 results** (`handoff/P14-probe-results.md`, last section), so this PC can fly the next probes. The Complex Trigger reference file is saved in `TemplateExamples/Reference/`. How to drive the game, and what is untested: `handoff/sessions/2026-10-12-il2-korea-on-this-pc.md`. Logs: `src/testdata/missionlog/real_4/`. Claude can also fly level legs, orbits and turns by keyboard.

## Waiting on the user

0. **Whether to change `src/model_spec.rs` from the handbook.** Proposal: take the handbook's practical ceilings for the 12 aircraft; leave the cruise speeds alone, because the handbook's "cruising speed" is an indicated economy speed and the app's figure is a transit speed that the game's AI was measured to match. A change to cruise speeds would change Template Builder output.
1. **Decision 0008: AI aircraft trigger check zones.** Its option 4 (filter by country) is now tested and works on both sides: players on 601 or 501, AI on 602 or 503. The decision is the user's; the record is still `Proposed`.
1a. **What to do about result g and the aircraft that outlived its Delete.** Both touch step 3 (budget, losses). Proposal: switch loss counting to event 13, and fly one more no-pilot probe on the Delete before step 3 is settled.
2. **Whether objective breadcrumbs show as map markers.** Nothing shows near K14 on the briefing map (2026-10-12). The far map corner, where the breadcrumbs sit, was not inspected.
3. **Go-ahead for the next steps below.** They were proposed on 2026-10-03 and not confirmed.
4. **Older confirmations still listed in the plan:** §10 item 23 (a smaller Auto-fill) and the "For the user to confirm" list.

## Next steps (proposed order)

1. Finish step 3 as listed above and merge it into `claude/p14-air-tasking`.
2. Merge upstream. It is now 6 commits ahead, including v0.7.0 and the split of the UI into per-tab files. Trial result and the one expected baseline change: `handoff/sessions/2026-10-07-flight-0-follow-up.md`.
3. Probe the Delete finding (no pilot needed), then settle step 3 with event 13 and the flight results.
4. Scope step 4 (threshold) with the country filter, if the user accepts decision 0008 option 4.

## Traps

- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
- This PC's game is version `261002` since 2026-10-12. A mission built here needs a server on the same version.
