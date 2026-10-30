# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-10-30: the standalone-file check (risk l) passed and is merged (PR #45); every step 11 item this PC can check is done. Next: step 10 (docs).
> Rules for keeping it current: `CLAUDE.md`. History: `handoff/archive/` and git log.

## What this is

A Windows desktop tool (Rust, egui) that builds IL-2 Great Battles **Korea** mission `.Group` files so the mission editor needs less hand wiring. Module map: `docs/src-guide.md`. End-user manual: `USER_MANUAL.md`.

- **Fork:** `Pliskin-Industries/Claude-IL2MissionUtility` (`origin`). **Upstream:** `eszyman/IL2MissionUtility` (`upstream`, the original creator).
- **Backlog:** GitHub Issues on the fork (#1 to #30). P14 is #28 (feature) and #29 (probe flights).
- **Build and test:** `cargo test --offline`, then `cargo build --release`. A fresh machine needs `cargo fetch`, rustup and the Visual Studio C++ Build Tools first.
- **Names:** the user calls P14 Air Tasking **"Wing Commander"** or **"WG/CC"**. That name is not in the code.

## Where the work is

| Branch | Holds | State |
|---|---|---|
| `main` | v0.6.0 + upstream v0.7.0; P14 steps 1a to 9T (PRs #31 to #38); decisions 0037 A (PR #39), 0043 to 0045 with flights A2, 2K, 2L (PR #40), 0042 A (PR #41); flight A3 (PR #43); standalone check L (PR #45); decisions 0001 to 0045 | `aa5bfcb`, equal to `origin/main` |

`upstream/main` has one commit not merged here (`571504a`, harvested coalition groups and ramp starts per country). Merged branches and their worktrees under `.claude/worktrees/` can be deleted once their idle sessions are archived.

Last recorded test run on `main` (`aa5bfcb`, the same tree as `24d6346`, where it ran): **1048 passed, 0 failed, 37 ignored** (`cargo test --offline`, 2026-10-30). Warnings: 81 build, 306 clippy (PR #45). Record each new run on this line.

## P14 Air Tasking ("Wing Commander")

Plan: `handoff/P14-air-tasking.md` (§8 steps, §10 build order, §2.1 user decisions U1 to U24). Flight results: `handoff/P14-probe-results.md` (flights 0 to 2L, A0 to A2); A3's and L's are in their run sheets, `handoff/sessions/2026-10-30-p14-acceptance-a3.md` and `2026-10-30-p14-standalone-l.md` (not yet in `P14-probe-results.md`). Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Step and flight notes: `handoff/sessions/`.

- **Built (on `main`):** steps 1a to 9T: the sortie shell, budget, threshold, clock, zone, export, the Air Tasking rail tab, the Map dock's Air tab and Map Generate, the Trace build checkbox, the Replay window and the Map's Replay layer. Also: a used copy goes quiet at `DONE HUB` (0038); the Map's own AI planes leave the player countries when Air Tasking is in the mission (0037 A); zone and clock end their timers through D11 relays, because a switched-off running timer is paused, not cancelled (0044, flight 2K); the DPRK air start and field spawns write 501 (0042 A).
- **Flown and seen in the game:**
  - A0 (no pilot): the group loads; clock slots open on time; nothing launches over an empty area.
  - A1 (NATO player at Kimpo): the DPRK cycle end to end; Zone Out ends a copy early; a spawn inside an area is not seen.
  - A2 (DPRK player, the first app-made mission, built through Map Generate): three NATO areas go HOT and COLD to the hundredth; **the first budget refusal**; **the single extension, then home** (P9); a clean COLD end; a used copy stays quiet (0038); the Map's own AI start nothing (0037); no dead links.
  - 2K: a switched-off running timer is paused and resumes when switched on; a pulsed check zone stays armed.
  - 2L (no pilot): the paused-timer fix holds; a zone rolls again after one cooldown.
  - A3 (DPRK player, built through Map Generate, logs `real_51`; run 1 crashed, `real_52`): **the Map's own air start on 501** (0042 A); **a NATO clock launch** (slot 1 at 600.10 s); **two sorties on one area end in one tick and are deleted in one tick** 60.60 s later (C1); **a later start** after one cooldown; the clock's no-repeat (`LAST`) keeps slot 2 quiet. A3 set the NATO clock to one jitter step and no quiet slots, so the 20 % quiet roll is not flown.
  - L, the standalone-file check (risk l, logs `real_53`): the Air Tasking tab's standalone file, imported (File › Import From File) into a Map base map that already has groups, is **re-indexed cleanly by the Mission Editor** (no Index twice, every link kept; a test compares the frozen editor save with the file). The saved mission flew with A3's timings. Seen in passing: one F-80 HVAR lost with no attacker (563.88 s), and the briefing map's front-line icons may show the mission title (seen on screen once, not saved).
- **Still open from the step 11 list** (plan §8 "Step 11"; items 1 to 3 done by A3, item 4 by L):
  5. Load time and tick delay next to `validate`'s totals: only on the real host, not this PC.
  - Also unseen: a Yak copy, a player who lands inside an area (0040: no change needed), a DPRK-side extension.
- **Formations:** AI losses in flights 2C to 2I were wingmen crossing their leader, a game bug (0021). Workarounds: Safe density (0019), Pairs at Loose (0020 A) except a Cover flight (0022 A2).
- **Remaining steps:** 10 docs → 12 multi-player. Step 11 item 5 waits for the real host. Upstream integration (0026: Template Builder's Strike/CAP as sorties, his target types) waits for eszyman's answer to the proposal.

## Waiting on the user

1. **Send the Air Tasking proposal to eszyman?** Draft: `handoff/upstream-air-tasking-proposal-draft.md` (decision 0026). The bar was a working concept and UI; A2 and A3 now run end to end from the app's Map Generate. Where to post it (new issue or issue #1) and when is yours.
2. **Merge `upstream/main` `571504a`?** It touches the airfield harvest that 0042 also changed.
3. **Three game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`). The paused-timer finding (2K) may be a fourth.
4. **The six built-in sorties** still carry the old Delete list in their files; Air Tasking replaces it on the copies it places (0017).

## Next steps

1. **Step 10 (docs):** the user manual for Air Tasking and the Map's Air tab, from what A0 to A3 and L showed. This includes adding A3's and L's results to `handoff/P14-probe-results.md`.
2. Step 12 (multi-player), then step 11 item 5 on the real host.

## Traps

- **Send nothing to the upstream author** (`eszyman/IL2MissionUtility`: issues, comments, files, load tests, patches) until the user says so. Pulling and merging his commits is fine. `gh` here may default to upstream: always pass `--repo Pliskin-Industries/Claude-IL2MissionUtility`.
- **Decision numbers:** sessions write records in parallel. Before writing one, check `main` **and** every open branch (`git ls-tree --name-only <branch> docs/decisions/`). Taken: 0001 to 0045. Next free: **0046**.
- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- **Editor saves:** the Mission Editor cuts Group names to 31 characters, and the host loads a `.msnbin` that sits next to the `.Mission` (L).
- **Timers:** switching off a running timer pauses it; switching it on resumes it (2K). Stop a timer only through a D11 relay (0044).
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`). A hook blocks `git push` (even a branch delete) while the checkout is on `main`.
- The game here is 1.005 (`261013`). Its `MissionResaver.exe` crashes: build missions with `--no-resave`. This PC is a test bed: its load times and tick delay are not used. It allows one spawn per game session.
- **Flying on this PC:** an air start may begin with the level-flight helper on; pressing LShift+A then switches it off (A2 run 1 crashed this way). **Never move the mouse or click in the cockpit**: it works the stick (A3 run 1 spun in). To regain keyboard focus, use PowerShell `AppActivate`. Tacview: `tacviewrecord = 1` with the game closed, then LCtrl+R in the mission.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`, `handoff/p14-probes/`), except those frozen as flown (2F, 2H, 2I, A0, A1, A2; A3 and L are also checked against their builds): those are checked by parse-and-links tests only. Never edit those files or re-run their writers. Flight names are never reused. The next free log folder is `src/testdata/missionlog/real_54`.
- Only one session writes `STATE.md` at a time; other sessions put their facts in a session note in `handoff/sessions/`.
- Stage is not declared yet (the session-start check treats the repo as `prototype`); `/promote` sets it.
