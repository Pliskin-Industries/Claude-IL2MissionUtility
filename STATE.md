# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-16, after decision 0019 (every AI air formation at Safe density) was built.
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
| `codex/p14-3` | `Claude-IL2MissionUtility-0t` | P14 step 3, built before flight 0; superseded by `codex/p14-3b`. |
| `codex/p14-3b` | `Claude-IL2MissionUtility-3b` | P14 step 3: decisions 0017 and 0018, reviewed; flights 2 and 2B flown. Not merged: waits for the user. |
| `codex/p14-2c` | `.claude/worktrees/adoring-mahavira-fad55e` | `codex/p14-3b` plus the HVAR options note (`claude/p14-hvar-empty-target`), probe flight 2C (built by Astra, reviewed) and its results. 645 passed, 0 failed, 22 ignored; 69 / 251 warnings; 52 flown probe files byte-identical. Not merged. |
| `codex/p14-2d` | `.claude/worktrees/distracted-taussig-b9259d` | `codex/p14-2c` plus probe flight 2D (built by Astra, reviewed), its results and proposed decision 0019. 651 passed, 0 failed, 23 ignored; 69 / 251 warnings; 56 flown probe files byte-identical. Not merged. |
| `claude/formation-safe-0019` | `.claude/worktrees/distracted-taussig-b9259d` | `codex/p14-2d` plus **decision 0019 built** (`077e14b`): every AI air formation at `FormationDensity = 1` in Air Tasking placement, the six built-ins, Template Builder and every copy path; bug-report drafts. 663 passed, 0 failed, 23 ignored; 69 / 251 warnings; 40 flown probe files byte-identical (1M, 2B, 2C, 2D now frozen like flight 2). Not merged. |
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
  - Fighter Pack's event 7 hook did **not** send the flight home, so no decision is needed for it for now.
- **Step 3 is built on `codex/p14-3b`** (worktree `Claude-IL2MissionUtility-3b`, `129bf0d`), to decision 0017, reviewed, and **flown as flight 2** (2026-10-14; `handoff/P14-probe-results.md`, "Flight 2"; logs `src/testdata/missionlog/real_19/`). The budget works in the game at N = 2 and N = 1; the shell's own Delete removes the aircraft 60.6 s after `DONE HUB`; event 13 fires once for a crash with no damage. **But both sorties ended at WP 1 arrival:** Zone Out fires at once when no enemy aircraft is within 35 km of the target. That needs a ruling (below) before the merge, and the on-station part (attack, extension, RTB) is still unflown. On that branch: 629 passed, 0 failed, 21 ignored; 69 / 251 warnings; flown probe files (flight 2 now frozen in `handoff/p14-probes/flight2/`) byte-identical.
- **Step 3c (decision 0018) is flown as flight 2B** (2026-10-14; `handoff/P14-probe-results.md`, "Flight 2B"; logs `src/testdata/missionlog/real_20/`; flown files frozen in `handoff/p14-probes/flight2b/`). All four paths worked as intended, each `DONE HUB` with a logged cause: empty target HOT → `EXTEND`, empty target COLD → `COLD`, enemy present on arrival or arriving later → Zone Out 0.02 s after the enemy left. **New finding:** T-s1's HVAR 4-ship lost two aircraft to crashes on station over the empty target, no attacker; the cause is unknown. Still unseen: behaviour during the extension (risk v), the attack itself, the RTB turn; this PC records no Tacview (`tacviewrecord = 0`). 638 passed, 0 failed; 48 flown probe files byte-identical.
- **Flight 2B run 2 (2026-10-15; logs `real_21`):** the logic repeated run 1 to within 0.2 s. HVAR 4-ships over an empty target lost aircraft again (T-s1 two crashes, T-s4 an apparent collision of two wingmen); AirAlert never. No Tacview was written: the game had rewritten `startup.cfg` back to `tacviewrecord = 0`, and nobody pressed LCtrl+R.
- **Flight 2B run 3 (2026-10-15, host spectating; logs `real_22`):** still no Tacview file, for the same two reasons. T-s1 lost one more HVAR.
- **Flight 2C (2026-10-15, two runs, Tacview; logs `real_23`, `real_24`; tracks `handoff/tacview/`; results `handoff/P14-probe-results.md`, "Flight 2C"):** twelve sorties on empty targets, four variants (HVAR, no rockets, fix A′, AirAlert). Every variant comes down to a few hundred metres above the ground and holds an altitude set by the target's place (flight 2B's height difference was the places). **Every loss seen was a collision inside a formation** (7 aircraft in run 2, none in run 1); the no-rocket cell lost the most, so neither the order nor the rockets explain it. The test-bed server was overloaded (mission time 0.4× wall time); every MCU still fired at its mission time to the tick. Options now: `handoff/sessions/2026-10-15-hvar-empty-target.md`, "After flight 2C". This PC's `startup.cfg` now holds `tacviewrecord = 1` (set with the game closed).
- **Flight 2D (2026-10-15, three runs, Tacview; logs `real_25` to `real_27`; tracks `handoff/tacview/`; results `handoff/P14-probe-results.md`, "Flight 2D"):** four HVAR 4-ships, only `FormationDensity` (or Pairs) changed, 16 aircraft: **no server overload** (1.0× wall time). The as-shipped Dense Wedge still collided (two aircraft in two of three runs); **Safe (1) and Loose (2) had no pass under 20 m and no loss**; Pairs at Dense flew closest. Spacing: Dense about 100 m, Safe about 140 m, Loose about 500 m. **Decision 0019 accepted and built** on `claude/formation-safe-0019`: every AI air formation flies Safe until Dense is fixed.
- **Upstream (eszyman), 2026-10-15:** issue #1: he fixed all three bugs (event 13, Fighter Pack counts once, Delete by entity / none in Activate mode) plus a once-only strike go-home in his working tree; we asked him to publish it upstream and close the issue. Issue #2: he ran the old `P14_Load_2` file (one check zone per copy): SPS 50, tick delay 1.0 idle; all 240 switched-off aircraft present (`handoff/P14-probe-results.md`, "Load test on the upstream author's server").
- **The game is 1.005 (`261013`).** Its `MissionResaver.exe` crashes on every mission; build probes with `--no-resave`. How to run a no-pilot probe on this PC: `handoff/sessions/2026-10-14-handoff-flight-1n.md` ("Flying a no-pilot probe") and `handoff/sessions/2026-10-13-delete-probes.md`.

## Waiting on the user

0. **Merge `codex/p14-3b` (or `codex/p14-2c`, which contains it) into `claude/p14-air-tasking`?** Step 3 has flown in flights 2, 2B and 2C. Open from it: the formation collisions (decision 0019, below), and the extension and RTB turn, still unseen. Tacview now works (`docs/il2-loader-and-log-facts.md`, 2026-10-15).
0a. **Two game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`): AI wingmen collide at Dense; OnEvent 4 (`OnPlaneDestroyed`) never fires. Whether, where and when to post them is the user's call.
1. **The load test is with the upstream author:** https://github.com/eszyman/IL2MissionUtility/issues/2 (posted 2026-10-14; mission text log and Tacview recording requested). The text names the rebuilt file `P14_Load_3_20261014` (decision 0018 shell, 48 check zones per side); **the zip attached to the issue is still the old `P14_Load_2` and must be replaced** with `target/p14/missions/P14_Load_3_20261014/P14_Load_3_20261014.zip` (step 3b worktree). This PC is a test bed: its load numbers are not used.
2. **The upstream issue is posted** (2026-10-14, https://github.com/eszyman/IL2MissionUtility/issues/1; text in `handoff/upstream-issue-draft.md`). Waiting on the upstream author's reply; offering patches for 1 to 3 is the next step if they want them.
3. **The six built-in sorties** (`TemplateExamples/Historical1950/`) still carry the old Delete list. Air Tasking removes it from the copies it places and writes its own (decision 0017); whether to change the files is open.
4. **Upstream overlap:** upstream now places strikes and CAPs on Map targets. Proposed: Air Tasking schedules and budgets those instead of placing its own; to be planned before the upstream merge.
5. **Older items:** whether objective breadcrumbs show as map markers; the 2026-10-03 next-step list; plan §10 item 23.

## Next steps (proposed order)

1. Watch the upstream issue for a reply; send the patches for 1 to 3 if the author wants them.
2. Merge upstream (8 commits ahead, including the strike and CAP missions and a large `template.rs` rewrite). Carry the decision fixes onto it, or offer them upstream first.
3. Merge `claude/formation-safe-0019` (user; it contains `codex/p14-3b`, `-2c` and `-2d`). Then a confirming flight at Safe for what flight 2D did not cover: Pairs (Yak raid, F-51 ArmedRecon) and the propeller types (F-51, Yak, Il-10).
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
