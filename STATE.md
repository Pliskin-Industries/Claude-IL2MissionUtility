Stage: shared

# STATE — IL-2 Mission Utility (Pliskin-Industries fork)

> Read this first. It is **overwritten** at the end of every session and stays one page.
> Last updated: 2026-11-06: **Flights A4 and 2T flown on this PC** (run sheet `handoff/sessions/2026-11-05-p14-flights-a4-2t.md`): the Yak copy and the DPRK-side extension work in the game (A4 run 2, Replay: no dead links), and the quiet roll leaves 28 of 120 slots quiet (2T; JITTER 1 passed a little under its rate). Flight 2N is still pending with no date, so by decision 0061 the alpha is next, at the user's word. Nothing pushed.
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
| `main` | v0.6.0 + upstream v0.7.0; P14 steps 1a to 9T (PRs #31 to #38); decisions 0037 A (PR #39), 0043 to 0045 with flights A2, 2K, 2L (PR #40), 0042 A (PR #41); flight A3 (PR #43); standalone check L (PR #45); step 10 docs (PR #47); L's F-80 collision note and STATE (PR #48); step 12 parts 12a to 12e with flights 2M, 2P, S1, 2Q, 2R (PR #49); review fixes 1 to 9 with decisions 0056 and 0057 (PR #51); flight 2S and decisions 0053 (superseded), 0054, 0055, 0058 A (PR #52); decisions 0001 to 0058 | `origin/main` `7f5329d` |
| `claude/p14-flight-2n` (worktree `.claude/worktrees/airtask-fixes`) | Flight 2N (decision 0059), flights A4 and 2T (decision 0061), decision 0060 and the rewritten proposal draft: run sheets, frozen files in `handoff/p14-probes/flight-2n/`, `flight-a4/`, `flight-2t/`, test-only builders. No product code | Not pushed |

`upstream/main` has two commits not merged here: `571504a` (harvested coalition groups and ramp starts per country) and `fee0a79` (2026-11-02: the Airfield tab owns base-map fields and air starts, Map strikes wrapped in Exclusive Activation, anti-air in clearings). The `airtask-fixes` worktree holds this session; it goes when the session is archived. Older worktrees from earlier sessions (`p14-acceptance-*`, `p14-followup-1012`, `p14-shell-quiet-*`, `sad-cannon-*`) are unchecked.

Last recorded test run: **1132 passed, 0 failed, 45 ignored** (`cargo test --offline`, 2026-11-06) on `claude/p14-flight-2n` (main `7f5329d` plus the 2N, A4 and 2T checks and their ignored writers; the manual test now counts the quiet roll as flown). Clippy 306, none in the new code. Record each new run on this line.

## P14 Air Tasking ("Wing Commander")

Plan: `handoff/P14-air-tasking.md` (§8 steps, §10 build order, §2.1 user decisions U1 to U24). Flight results: `handoff/P14-probe-results.md` (flights 0 to 2S, A0 to A3, L, S1); step 12's scope and run sheets: `handoff/sessions/2026-10-31-p14-step12-scope.md` and the `2026-10-31-p14-flight-*` notes; run sheets for A3 and L: `handoff/sessions/2026-10-30-p14-acceptance-a3.md` and `2026-10-30-p14-standalone-l.md`. End-user manual for Air Tasking: `USER_MANUAL.md` (step 10). Facts that cost a flight: `docs/il2-loader-and-log-facts.md`. Step and flight notes: `handoff/sessions/`.

- **Built from Astra's review** (PRs #51, #52): Replay reads one session at a time and rescans on Run; trace builds refuse a non-.Group name and check index room (0056, 0057); a standalone base map moves the Map's AI off the player countries (0054); the clock passes RANDOM's output through the AVAILABLE gate, so a paused RANDOM cannot launch twice in one slot (0055). Air Tasking output changed on purpose for 0054 and 0055 only.
- **Built (on `main`):** steps 1a to 9T: the sortie shell, budget, threshold, clock, zone, export, the Air Tasking rail tab, the Map dock's Air tab and Map Generate, the Trace build checkbox, the Replay window and the Map's Replay layer. Also: a used copy goes quiet at `DONE HUB` (0038); the Map's own AI planes leave the player countries when Air Tasking is in the mission (0037 A); zone and clock end their timers through D11 relays, because a switched-off running timer is paused, not cancelled (0044, flight 2K); the DPRK air start and field spawns write 501 (0042 A).
- **Flown and seen in the game:**
  - A0 (no pilot): the group loads; clock slots open on time; nothing launches over an empty area.
  - A1 (NATO player at Kimpo): the DPRK cycle end to end; Zone Out ends a copy early; a spawn inside an area is not seen.
  - A2 (DPRK player, the first app-made mission, built through Map Generate): three NATO areas go HOT and COLD to the hundredth; **the first budget refusal**; **the single extension, then home** (P9); a clean COLD end; a used copy stays quiet (0038); the Map's own AI start nothing (0037); no dead links.
  - 2K: a switched-off running timer is paused and resumes when switched on; a pulsed check zone stays armed.
  - 2L (no pilot): the paused-timer fix holds; a zone rolls again after one cooldown.
  - A3 (DPRK player, built through Map Generate, logs `real_51`; run 1 crashed, `real_52`): **the Map's own air start on 501** (0042 A); **a NATO clock launch** (slot 1 at 600.10 s); **two sorties on one area end in one tick and are deleted in one tick** 60.60 s later (C1); **a later start** after one cooldown; the clock's no-repeat (`LAST`) keeps slot 2 quiet. A3 set the NATO clock to one jitter step and no quiet slots, so the 20 % quiet roll is not flown.
  - L, the standalone-file check (risk l, logs `real_53`): the Air Tasking tab's standalone file, imported (File › Import From File) into a Map base map that already has groups, is **re-indexed cleanly by the Mission Editor** (no Index twice, every link kept; a test compares the frozen editor save with the file). The saved mission flew with A3's timings. Seen in passing: one F-80 HVAR lost at 563.88 s to its own wingman in a steady circling turn over the empty area (Tacview; the 0021 steady-turn crossing), and the briefing map's front-line icons may show the mission title (seen on screen once, not saved).
- **Built: step 12, parts 12a to 12e** (PR #49):
  - **Threshold for 2 or 3 players** (0047 E): an area needs N players of the other side to go HOT; the count resets when any of them leaves (0047) or dies inside (0049 B; the Complex Trigger's Killed event is 70, flight 2P). N = 1 is wired as before.
  - **A player count per area** (0050 D): each sortie copy ends on its area's count of players. Air Tasking's AI is not counted. Known limit (0058): a wreck removed up to about 12 s after a busy area clears can empty the count under a player who has just entered; a gate (0058 C) is built only if logs show it. Generate builds no Zone Out for Air Tasking (0048 superseded); `Watchdog`, `ThresholdImpl` and `CounterResetImpl` are removed. Generate output changed on purpose for missions with Air Tasking.
  - **Generate refuses** a player spawn inside a count circle (0051).
  - **UI** (0052): N from 1 to 3; one count radius, 10 to 100 km (50 km default), one area per target; the Map draws the circles. Checked live.
  - **Flown, no pilot:** 2M (Zone Out is ALL-out; a kill or a Delete raises no Left); 2P (Killed is event 70); S1 and 2Q (AI on sides 3 and 4 attacks both real sides, so 0050 A failed; check zones do not see side 3); 2R (0050 D's count works as built); 2S (a wreck that crosses out of the circle raises Left, so 0053 failed and 0058 A went back to Left and Finished; a wreck's Finished comes 3 to 15 s after the kill).
- **Still open from the step 11 list** (plan §8 "Step 11"; items 1 to 3 done by A3, item 4 by L):
  5. Load time and tick delay next to `validate`'s totals: only on the real host, not this PC.
  - A4 (2026-11-06, one NATO player, logs `real_61`, run 1 `real_63`): **a Yak copy** and **a DPRK-side extension** (EXTEND + DONE HUB at ON STATION + 600.00); a copy ends when its last player is killed inside the circle (run 1). 2T (no pilot, `real_62`): **28 of 120 slots quiet** (20 % expected, range 15 to 33); `JITTER 1` passed 21 of 120 (17.5 % for 27 %, 2.3 sd low; a second 2T run would tell chance from bias).
  - Also unseen: a player who lands inside an area (0040: no change needed).
- **Formations:** AI losses in flights 2C to 2I were wingmen crossing their leader, a game bug (0021). Workarounds: Safe density (0019), Pairs at Loose (0020 A) except a Cover flight (0022 A2). Flight L lost one F-80 of a 4-ship Wedge at Safe to the steady-turn crossing; no formation setting removes that kind, so the user decided (2026-10-31) to change nothing and fly no probe. It waits for a game fix.
- **Built: step 10** (docs, PR #47): the Air Tasking manual, a Help topic "Trace builds and replay", `docs/src-guide.md` for every file. The manual covers step 12; its UNVERIFIED line lists the real-host timings, the 20 % quiet roll, and everything with two or more human players.
- **Remaining:** 12f, flight 2N, built (run sheet `handoff/sessions/2026-11-02-p14-flight-2n.md`, decision 0059): up to four pilots on eszyman's server, **pending, no date** (if it has not flown when A4 and 2T are read, the alpha goes live and trace-built operations stand in for it). Two areas 85 km apart: a DPRK pair over NATO area A, a NATO pair over DPRK area B, the same cues for both. Missions in `C:\Machine Intelligence\P14 Flight 2N\mission\` (trace build to fly, clean build to share); both load-tested on this PC on 2026-11-03 (run sheet, Files). Zip of both plus the run sheet: `C:\Machine Intelligence\P14 Flight 2N\P14_Flight_2N_20261102.zip`. Step 11 item 5 is measured on his server in the same session. Upstream integration (0026: Template Builder's Strike/CAP as sorties, his target types) waits for eszyman's answer to the proposal.

## Waiting on the user

1. **Send the Air Tasking proposal to eszyman?** Draft: `handoff/upstream-air-tasking-proposal-draft.md`, rewritten for decision 0060 (Air Tasking as a second mode beside Exclusive Activation, with how it could replace it). Planned for after the alpha has operation logs; when and where is yours.
2. **Merge `upstream/main` `571504a` and `fee0a79`?** About 6,000 lines, mostly the Map and Airfield tabs; `571504a` touches the airfield harvest that 0042 also changed, and `fee0a79` (2026-11-02) wraps Map strikes in Exclusive Activation. Decide before the alpha.
3. **Three game bug reports are drafted, not posted** (`handoff/il2-bug-report-drafts.md`). The paused-timer finding (2K) may be a fourth.
4. **The six built-in sorties** still carry the old Delete list in their files; Air Tasking replaces it on the copies it places (0017).
5. **Flight 2N:** attach `C:\Machine Intelligence\P14 Flight 2N\P14_Flight_2N_20261102.zip` as the first comment on [eszyman/IL2MissionUtility#3](https://github.com/eszyman/IL2MissionUtility/issues/3) (the issue says it is there; Claude could not attach it: Chrome would not load GitHub, and a gist was not allowed). After the flight, send Claude the server's `missionReport` text logs, the load time and the tick numbers; Tacview files if anyone recorded.

## Next steps

1. **Alpha (decision 0061), each step at the user's word:** the `/promote` audit for "shared"; a decision on upstream `571504a` and `fee0a79`; version 0.8.0; release notes with the UNVERIFIED list (`USER_MANUAL.md`); the `v0.8-Alpha` tag. Missions in operations are trace builds.
2. Optional: fly 2T again (same frozen files, a new log folder) to tell whether JITTER 1's low rate is chance.
3. Push `claude/p14-flight-2n` and open its PR when the user says so.
4. **Flight 2N, whenever it flies.** Claude copies the logs to `src/testdata/missionlog/real_60/`, reads them with Replay against `P14_Flight_2N.trace.json`, and records the load time and tick numbers (step 11 item 5). If the logs show 0058's race, 0058 C is built.
5. To build (decision 0060): a plan check that warns when a map has both Map missions under Exclusive Activation and Air Tasking.
6. The eszyman proposal (`handoff/upstream-air-tasking-proposal-draft.md`, rewritten for 0060) after the alpha has operation logs, if the user wants it sent.

## Traps

- **Send nothing to the upstream author** (`eszyman/IL2MissionUtility`: issues, comments, files, patches) unless the user says so for that send. On 2026-11-03 the user asked Claude to post flight 2N (issue #3); that covers only that post. Pulling and merging his commits is fine. `gh` here may default to upstream: always pass `--repo Pliskin-Industries/Claude-IL2MissionUtility`.
- **Astra writes `STATE.md` and decision records when not told otherwise:** say in every brief that Claude owns `STATE.md`. The review-fixes worktree still holds an uncommitted Astra rewrite of `STATE.md`; discard it (a hook stops Claude from doing so).
- **IL-2 launcher:** it needs `s3.il2korea.com` (a CDN). On 2026-11-01 that host first had no IPv4 address, then failed SSL, and worked again a few hours later; the game cannot start without the launcher. Retry later; do not change this PC's DNS.
- **Decision numbers:** sessions write records in parallel. Before writing one, check `main` **and** every open branch (`git ls-tree --name-only <branch> docs/decisions/`). Taken: 0001 to 0061 (0059 to 0061 on `claude/p14-flight-2n`). Next free: **0062**.
- Before generating any probe or mission file, read `docs/il2-loader-and-log-facts.md`. Each item there cost a flight attempt.
- **Editor saves:** the Mission Editor cuts Group names to 31 characters, and the host loads a `.msnbin` that sits next to the `.Mission` (L).
- **Timers:** switching off a running timer pauses it; switching it on resumes it (2K). Stop a timer only through a D11 relay (0044).
- `.gitattributes` is `* -text` because `.Group` fixtures are line-ending sensitive. Do not remove it.
- Do not push `v*` tags casually: they start the release build.
- **IL-2 on this PC:** since 2026-10-31 the user lets Claude start the game, host probe missions, copy a built probe into `game\data\Multiplayer\Dogfight`, and run the probe builder or the Mission Editor without asking each time. Pushing, merging and anything sent upstream still need the user.
- **Sides 3 and 4:** the game accepts AI countries there, but that AI attacks both real sides, and check zones do not see side 3 (flight 2Q).
- Nothing is pushed or merged to `main` without the user (`docs/decisions/0005`). A hook blocks `git push` (even a branch delete) while the checkout is on `main`.
- The game here is 1.005 (`261013`). Its `MissionResaver.exe` crashes: build missions with `--no-resave`. This PC is a test bed: its load times and tick delay are not used. It allows one spawn per game session.
- **Flying on this PC:** an air start may begin with the level-flight helper on; pressing LShift+A then switches it off (A2 run 1 crashed this way). **Never move the mouse or click in the cockpit**: it works the stick (A3 run 1 spun in). To regain keyboard focus, use PowerShell `AppActivate`. Tacview: `tacviewrecord = 1` with the game closed, then LCtrl+R in the mission.
- Flown probe flights regenerate byte for byte from frozen inputs (`src/testdata/p14_probe_inputs/`, `handoff/p14-probes/`), except those frozen as flown (2F, 2H, 2I, A0, A1, A2, A3, L; since 0050 D changed Generate's output, the A3 and L build tests check the run-sheet facts, not a byte-for-byte match): those are checked by parse-and-links tests only. Never edit those files or re-run their writers. Flight names are never reused. Log folders: `real_60` is reserved for flight 2N; `real_61` is A4 run 2, `real_62` is 2T, `real_63` is A4 run 1; the next free is `real_64`.
- Only one session writes `STATE.md` at a time; other sessions put their facts in a session note in `handoff/sessions/`.
- **Stage: shared** since 2026-11-06 (the first line; written by `/promote`): the alpha goes to another person (decision 0063). The shared checklist needs `.github/dependabot.yml`, the gitleaks job (`.github/workflows/secret-scan.yml`), `.pre-commit-config.yaml` (activate per clone: `pip install pre-commit`, `pre-commit install`) and `docs/BACKUP.md`. Only `/promote` changes the stage line.
