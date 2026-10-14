# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-14, after flights 1K, 1L and 1M and the fixes for decisions 0012, 0013/0014 and 0015.
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
| `claude/p14-air-tasking` | `Claude-IL2MissionUtility-fix` | **The active branch.** P14 steps 1a, 1, 1b, 0T, 0b, 2 and the probe steps 0e to 0j, plus the fixes for decisions 0012, 0013/0014 and 0015 (they change what Template Builder generates). Not pushed. |
| `codex/p14-3` | `Claude-IL2MissionUtility-0t` | P14 step 3, built but **held** and not merged. |
| `trial/upstream-merge` | `Claude-IL2MissionUtility-merge` | Uncommitted trial merge of upstream. Do not build on it. |
| `trial/upstream-strike-files` | `Claude-IL2MissionUtility-up` | A checkout of upstream `a31ad0a` with one ignored test that writes its strike and CAP files. Never pushed or merged. |
| `codex/p14-0b`, `codex/p14-0e*`, `codex/p14-0f` to `0j`, `codex/fixes-*` | `Claude-IL2MissionUtility-0b`, `-0e` | Already merged. Leftover worktrees. |
| `claude/docs-layout-trial` | `Claude-IL2MissionUtility-docs` | This document layout. Merged 2026-10-10. A leftover worktree. |

Last recorded test run on the active branch: 573 passed, 0 failed, 17 ignored (`cargo test --offline`, 2026-10-14, after the review fixes). Warnings on 2026-10-14: 69 build, 251 clippy. Record each new run on this line.

## In progress: P14 Air Tasking (issues #28, #29)

Plan: `handoff/P14-air-tasking.md` (long; read §8 for steps and §10 for the build order). Its user decisions are §2.1 (U1 to U24).

- **Flight 0 was flown on 2026-10-03 and gate G0 passed.** Results: `handoff/P14-probe-results.md`. Video check: `References/Recordings/flight0-harvest/README.md`.
- **Step 3 is unblocked but not merged.** Its commit (`3224286`) reports 583 passed, 11 ignored. It still needs the lines marked `// P14 G0: default pending flight 0` settled, the probe fixes merged in (it is 24 commits behind), the shape and string guards extended to Probe 2, and a full review.
- **Flight 1A was flown on 2026-10-12 by Claude, as a flying check with an air start.** The T-c cells and ZA4 gave clear answers (c, the re-check and h confirmed; o = fires once; a zone fires for a player). Risk n (cruise speed) was not measured, and the country-filter cells were not in the build. Results: `handoff/P14-probe-results.md`, last section. **The user ruled that it counts as flight 1A: gate G1 is met and step 4 may start** (`docs/decisions/0009`).
- **Step 0e is built, merged (`aa3b048`) and flown.** Flights 1G, 1N and 1D were flown by Claude on 2026-10-12. Results: `handoff/P14-probe-results.md` (its last section is a one-table summary of step 0). Headlines: the country filter works on both sides; a lost aircraft raises OnEvent **13**, not 4; AI F-80Cs fly 733 km/h against the plan's 732; **one aircraft outlived its Delete**, which matters to the budget's hard limit (U22).
- **The user ruled on four items on 2026-10-12.** (1) Decision 0008 is Accepted, option 4: players on 601 or 501, the app's AI on 602 or 503. (2) Losses count on event 13 (decision 0011), and the Delete gets one more probe. (3) The step 0e style findings were fixed (`67c5a13`, merged `4cd5d03`). (4) Ceilings: see "Waiting on the user", item 1.
- **Probe flights 1H to 1M (2026-10-13 and 14, no pilot).** Results: `handoff/P14-probe-results.md`; its last section is a summary table. Headlines:
  - A destroyed aircraft raises event 13, never 4 (every kind of loss tried). A Delete must list entities; one that lists the units' own block indexes removes nothing.
  - With `AiRTBDecision = 1`, all aircraft of a damaged formation but one stop answering commands and fly home. With 0 they stay controllable.
  - Upstream's new strike and CAP: the F-80C and F-86A raise event 7 (bingo fuel) 3 s after activation and go home at once; the Il-10 strike attacks but never gets home when its go-home timer fires more than once.
- **Fixed and merged (decisions 0012, 0013/0014, 0015, accepted by the user 2026-10-13):** aircraft death events are 13; Spawn-mode `Trigger Delete` lists entities and Activate mode writes no Delete; Air Tasking places aircraft with `AiRTBDecision = 0` and gives placed copies a working Delete. Flight 1M flew the generated output and confirmed each. **Not to be released until decision 0016 is settled:** with 0012 alone, Fighter Pack would count every kill twice.
- **An issue for the upstream author is drafted, not sent:** `handoff/upstream-issue-draft.md`. It waits on flight 1N for the go-home fix, then a `/hfe-mayer` pass, then the user's word.
- **The game updated to 1.005 (`261013`) on 2026-10-13.** Its `MissionResaver.exe` crashes on every mission; build probes with `--no-resave`. Leaving a client-hosted server: see `docs/il2-loader-and-log-facts.md`.
- **Aircraft ceilings are documented figures since 2026-10-13** (`src/model_spec.rs`; research in `docs/historical-reference/aircraft-service-ceilings.md`, with the game's own figures beside them).
- **Testing on this PC, with Claude flying (2026-10-12).** The game and editor run here on integrated graphics, and Claude can drive both. **Flight 0 was re-flown here as a calibration and matches the 2026-10-03 results** (`handoff/P14-probe-results.md`, last section), so this PC can fly the next probes. The Complex Trigger reference file is saved in `TemplateExamples/Reference/`. How to drive the game, and what is untested: `handoff/sessions/2026-10-12-il2-korea-on-this-pc.md`. Logs: `src/testdata/missionlog/real_4/`. Claude can also fly level legs, orbits and turns by keyboard.

## Waiting on the user

0. **Decision 0016 (`Proposed`): Fighter Pack counts each aircraft once.** Recommended: point each aircraft's death event at its own clean-up timer. It blocks releasing the 0012 fix.
1. **Flight 1N:** three copies of upstream's Il-10 strike (as written; with a once-only gate on the go-home step; without the Force Complete), and a Fighter Pack flight if 0016 is accepted. Proposed, not built.
2. **The upstream issue:** sending it, after 1N and the `/hfe-mayer` pass.
3. **The six built-in sorties** (`TemplateExamples/Historical1950/`) still carry the old Delete list. Air Tasking now corrects it on the copies it places; whether to change the files is open.
4. **Upstream overlap:** upstream now places strikes and CAPs on Map targets. Proposed: Air Tasking schedules and budgets those instead of placing its own; to be planned before the upstream merge.
5. **Older items:** whether objective breadcrumbs show as map markers; the 2026-10-03 next-step list; plan §10 item 23.

## Next steps (proposed order)

1. Settle 0016, build it, and fly flight 1N.
2. Finish the upstream issue and send it when the user says.
3. Merge upstream (8 commits ahead, including the strike and CAP missions and a large `template.rs` rewrite). Carry the decision fixes onto it, or offer them upstream first.
4. Settle step 3 (`codex/p14-3`, held): losses on event 13; its own clean-up with entity Deletes; sortie templates with or without a Delete chain.
5. Scope step 4 (threshold) on a Complex Trigger filtered by country (decision 0008).

## Traps

- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
- This PC's game is version `261013` (1.005) since 2026-10-13. A mission built here needs a server on the same version. Its resaver crashes: build with `--no-resave`.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`). Never edit those files or re-run their writers.
