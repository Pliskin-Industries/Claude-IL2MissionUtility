# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-14, after decision 0016 and flight 1P (the plan's "flight 1N").
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
| `claude/p14-air-tasking` | `Claude-IL2MissionUtility-fix` | **The active branch.** P14 steps 1a, 1, 1b, 0T, 0b, 2 and the probe steps 0e to 0k, plus the fixes for decisions 0012 to 0016 (they change what Template Builder and Fighter Pack generate). Not pushed. |
| `codex/p14-3` | `Claude-IL2MissionUtility-0t` | P14 step 3, built but **held** and not merged. |
| `trial/upstream-merge` | `Claude-IL2MissionUtility-merge` | Uncommitted trial merge of upstream. Do not build on it. |
| `trial/upstream-strike-files` | `Claude-IL2MissionUtility-up` | A checkout of upstream `a31ad0a` with one ignored test that writes its strike and CAP files. Never pushed or merged. |
| `codex/fix-0016`, `codex/p14-0k` and the older `codex/*` branches | `Claude-IL2MissionUtility-0e` (on `codex/p14-0k`), `-0b` | Already merged. Leftover worktrees. |
| `claude/docs-layout-trial` | `Claude-IL2MissionUtility-docs` | This document layout. Merged 2026-10-10. A leftover worktree. |

Last recorded test run on the active branch: 577 passed, 0 failed, 19 ignored (`cargo test --offline`, 2026-10-14, after step 0k). Warnings on 2026-10-14: 69 build, 251 clippy. Probe writers: 11; the 44 flown probe files of flights 0 to 1P regenerate byte for byte. Record each new run on this line.

## In progress: P14 Air Tasking (issues #28, #29)

Plan: `handoff/P14-air-tasking.md` (long; read §8 for steps and §10 for the build order). Its user decisions are §2.1 (U1 to U24).

- **The planned probe flights are flown:** flights 0, 1A, 1N, 1D, 1G, 1H, 1J, 1K, 1L, 1M and 1P. Results: `handoff/P14-probe-results.md`; its last section is a summary table. Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Gates G0 and G1 are met (`docs/decisions/0009`).
- **Decisions 0012 to 0016 are built, merged and confirmed in flight:** aircraft death events are 13; Spawn-mode `Trigger Delete` lists entities and Activate mode writes no Delete; Air Tasking places aircraft with `AiRTBDecision = 0`; **Fighter Pack counts each loss once** (0016, merged `8210f64`; flight 1P: once with 0016, twice without it). Nothing known now blocks a release of these fixes.
- **Flight 1P (2026-10-14, Claude at the keyboard, no pilot; logs `src/testdata/missionlog/real_18/`)** was built as step 0k (merged `b92df87`). Renamed from 1N because flight 1N already exists. Headlines:
  - upstream's Il-10 strike goes home and completes with a once-only counter before its go-home timer; without it, the timer fired four times and the flight stayed at the target;
  - the F-80C strike without its event 7 hooks completes;
  - **event 7 at activation is not a property of the type:** twelve single aircraft as Template Builder writes them, and the Fighter Pack F-80Cs, did not raise it. Upstream's F-80C strike and F-86A CAP did. The cause is not known;
  - Fighter Pack's event 7 hook did **not** send the flight home, so no decision 0017 is needed for now.
- **Step 3 is unblocked but not merged** (`codex/p14-3`, commit `3224286`). It needs: losses on event 13, its own clean-up with entity Deletes, no abort on event 7, the lines marked `// P14 G0: default pending flight 0` settled, the probe fixes merged in, and a full review.
- **The game is 1.005 (`261013`).** Its `MissionResaver.exe` crashes on every mission; build probes with `--no-resave`. How to run a no-pilot probe on this PC: `handoff/sessions/2026-10-14-handoff-flight-1n.md` ("Flying a no-pilot probe") and `handoff/sessions/2026-10-13-delete-probes.md`.

## Waiting on the user

1. **The upstream issue is posted** (2026-10-14, https://github.com/eszyman/IL2MissionUtility/issues/1; text in `handoff/upstream-issue-draft.md`). Waiting on the upstream author's reply; offering patches for 1 to 3 is the next step if they want them.
2. **The six built-in sorties** (`TemplateExamples/Historical1950/`) still carry the old Delete list. Air Tasking corrects it on the copies it places; whether to change the files is open.
3. **Upstream overlap:** upstream now places strikes and CAPs on Map targets. Proposed: Air Tasking schedules and budgets those instead of placing its own; to be planned before the upstream merge.
4. **Older items:** whether objective breadcrumbs show as map markers; the 2026-10-03 next-step list; plan §10 item 23.

## Next steps (proposed order)

1. Watch the upstream issue for a reply; send the patches for 1 to 3 if the author wants them.
2. Merge upstream (8 commits ahead, including the strike and CAP missions and a large `template.rs` rewrite). Carry the decision fixes onto it, or offer them upstream first.
3. Settle step 3 (`codex/p14-3`, held), as listed above.
4. Scope step 4 (threshold) on a Complex Trigger filtered by country (decision 0008).
5. Optional probes, if they matter later: the go-home timer without Force Complete when it fires more than once (1P's H3 did not test it), and what makes the game raise event 7 at activation.

## Traps

- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`).
- This PC's game is version `261013` (1.005) since 2026-10-13. A mission built here needs a server on the same version. Its resaver crashes: build with `--no-resave`.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`). Never edit those files or re-run their writers.
- Probe flight names are never reused: the plan's "1N" for step 0k was flown as **1P**.
