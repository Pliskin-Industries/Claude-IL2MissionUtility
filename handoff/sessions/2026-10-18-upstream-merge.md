# Upstream merge plan: upstream v0.7.0 into `main` (2026-10-18)

Branch `claude/upstream-merge-0.7`, cut from `main` at `344d9d8`. Upstream is `eszyman/IL2MissionUtility` (`upstream`), 9 commits ahead (`967cbeb` to `842d27c`; merge base `c1d88c3`). **Not pushed, not merged into `main`.**

**Status: approved and committed (2026-10-18).** The user approved the plan as written ("Approved as planned: editor formation ids with our density rule (Column - Pairs counts as Pairs, bombers Safe), re-record template.Group and exclusive.Group, then commit the merge on claude/upstream-merge-0.7. Do not push."). The merge is one merge commit on `claude/upstream-merge-0.7`. Not pushed; not merged into `main`.

The old trial merge (`trial/upstream-merge` in `Claude-IL2MissionUtility-merge`) was not used.

## 1. What upstream's 9 commits bring

| Commit | What |
|---|---|
| `967cbeb` | Korea height grid baked into the app (`assets/korea_100m.hgt`, 31 MB; `src/heighthelper.rs`) |
| `af36b61` | Template Builder aircraft `FormationType` follows the Korea editor list (see §3) |
| `29e1da7` | Template and Map UI moved into `src/ui/builder.rs` and `src/ui/map.rs` |
| `1ccc44f` | Version 0.7.0 |
| `e18ee09` | Clear each side's fighters from the Forces dock |
| `f6b52e9` | Air starts and field spawn (`src/airstart.rs`, `src/ui/airfield.rs`); harvested missions keep doubled quotes |
| `160619d` | Template and Map place CAP and exclusive strikes on real targets; trains and truck leads sit on network corners |
| `a31ad0a` | NATO strike icon, a road overlay picture |
| `842d27c` | Korea 1.005 logic (our issue eszyman/IL2MissionUtility#1): OnKilled deaths, Spawn deletes by entity, Activate writes no Delete, Fighter Pack counts once, CAP and Strike drop bingo fuel, one `Go Home Once` counter per flight, `ALL DESTROYED`, Exclusive 3-hour window; roads from the editor `.ini` files |

## 2. Our decision fixes 0012 to 0016 against upstream's

| Ours | Upstream's version | Result in the merge |
|---|---|---|
| **0012** aircraft death event 13 | Same: every death counter listens for `OnKilled` (13) | Covered. Kept our code (`DEFAULT_DEATH_EVENT`); upstream's `EntityEvent::default_for` dropped because nothing calls it. Same output. |
| **0013** Spawn-mode Delete lists entities | Same | Covered. Kept our code. Same MCUs and indexes as ours; upstream's own build writes the Delete chain in a different block order (after the bring-up timers instead of after `Deactivate Units`), so our order is kept and our flown files stay byte-identical. |
| **0014** no Delete in Activate mode | Same | Covered, as 0013. |
| **0015** Air Tasking sorties `AiRTBDecision = 0` | Not in upstream (its Strike and CAP already write 0; Template Builder is unchanged) | Ours only, in `src/airtask/`; no conflict. |
| **0016** Fighter Pack death event targets `Delay Delete n` | Same wiring, done as a second pass (`point_kills_at_own_delay`) that skips silently when a link is missing | Covered. Kept ours: it fails with a message naming the aircraft when a link is missing (project rule: fail loudly). Upstream's pass removed; its test `killed_plane_ticks_delay_delete_once` kept and passes. `fighter.Group` is byte-identical. |

Upstream also fixed the two strike faults of our issue §3 (bingo fuel no longer an abort; the go-home step fires once per flight) and the Exclusive lock of §4 (a 3-hour window; `ALL DESTROYED` ends a strike that is shot down). We had no code for these; they come in as upstream wrote them.

## 3. Decisions 0019 to 0022 on upstream's editor formation ids

**What upstream changed.** Template Builder now writes the Korea editor's Formation list, 1-based: V-Form 1, Column - Pairs 4, Column - Flights wedge 5, Heavy - Wedge 8, up to Reserved - 15 at 19. Old Great Battles ids are rewritten when written and when a template is loaded (20 → 5, 21 → 6, 22 → 7, 23 → 8, 24 → 9, 26 → 11, 27 → 12, 0 → 1); 19 is left alone. New seats default by role: fighters Column - Pairs, attack aircraft Column - Flights wedge, bombers Heavy - Wedge. Upstream set density by role too: fighters Loose, attack Safe, **bombers Dense**.

**What we flew.** Flights 2D to 2I flew only `FormationType` 19 (Pairs) and 20 (Wedge), from the six built-ins. In the game they fly as a pair and a wedge, with different spacing (Pairs Loose about 200 m, Wedge Loose about 1,200 m). Ids 4, 5 and 8 have not been flown by us.

**How the trial resolution lands them (recommended):**

- **Air Tasking and the six built-ins: unchanged.** They keep 19 and 20, as flown. `set_air_formation_density` is unchanged.
- **Template Builder: upstream's editor ids, our density rule.** A formation flies Loose if it is a Pairs formation, Safe otherwise, and Safe under a Cover order (0019, 0020 A, 0022 A2). "Pairs" now means two ids: 19 (the built-ins' Pairs) and 4 (Column - Pairs): `is_pairs_air_formation` in `src/template.rs`. Upstream's role density (`air_formation_density`) is removed, so **bombers fly Safe, not Dense** (decision 0019: Dense collides). Fighters in Column - Pairs fly Loose, as upstream chose.
- The label for 19 in the Template Builder list is "Reserved - 15 (Pairs in the built-ins)".

**Rejected:** keep upstream's role densities (bombers Dense; contradicts 0019); revert Template Builder to the Great Battles ids (diverges from upstream, whose author says the editor list is 1-based and that 0 crashes the editor).

**Open, for the user:** treating Column - Pairs (4) as Pairs, and writing 4, 5 and 8 at all, is unflown. A short confirmation flight (Template Builder F-51 4-ship as Column - Pairs at Loose, an Il-10 Flights wedge and a B-29 Heavy - Wedge at Safe) would settle it. Not needed for the merge itself.

## 4. Conflicts and how they were resolved

`git merge upstream/main` stops on 8 files. `src/ui.rs` merges cleanly (our `main` never changed it); `src/ui/*.rs` are upstream-only. `src/weapon_range.rs`, `src/placement.rs`, `src/recon.rs`, `src/bombers.rs`, `src/ui_tests.rs` and `docs/src-guide.md` merge without conflict.

| File | Hunks | Resolution |
|---|---|---|
| `src/template.rs` | 21 | Delete chain, death event, block order and tests: ours. Formation tables and export: as §3. `Go Home Once` counters (upstream) added after the existing blocks; our `timer_random` and `pub(crate) counter` kept beside upstream's `go_home_once_name`. Upstream's unused `script` argument of `build_order` removed. |
| `src/flights.rs` | 2 | Ours (decision 0016 as built); upstream's duplicate pass removed. |
| `src/pack.rs` | 1 | Both tests kept. |
| `src/main.rs` | 1 | Both modules (`airstart`, `airtask`). |
| `src/mapnet.rs` | 1 | Upstream's corner headings, with our `pub(crate)` on `is_rtb_waypoint`. |
| `src/model_spec.rs` | 1 | Ours: upstream only reformatted the old ceilings; ours carries the sourced ceilings (`71a63b4`). |
| `USER_MANUAL.md` | 4 | Upstream's text (Missions, Disable Zone Out, Time on Target), with our formation-density sentence and our event 4 / 13 note. |
| `HANDOFF.md` | 2 | Ours: it is a pointer here (CLAUDE.md). Upstream's session-log rows stay in upstream's history. |

`src/airtask/` was not touched.

## 5. What Generate writes differently (each needs the user's approval)

Measured by recording every P14 baseline and running the flown-probe byte checks.

| Output | Change | Why |
|---|---|---|
| **Flown probe files** (`handoff/p14-probes/`) | **None.** 52 files byte-identical; the flight 1M and 1P frozen inputs match their generators. No new freezes. | Air Tasking, the built-ins and the probe writers are unchanged. |
| **Fighter Pack** (`fighter.Group`) | None | 0016 kept as built |
| `recon.Group`, `airfield_mp.Group`, `base_map.Group`, `base_map_ui.Group` | None | |
| **Template Builder** (`template.Group`) | One line: `FormationType = 23` → `8` (Heavy Wedge in the editor's numbering); density stays 1 | Upstream's editor ids (§3) |
| **Exclusive Activation** (`exclusive.Group`) | Two MCUs per plan (`{n} WINDOW`, a 3-hour timer, and `{n} WINDOW OFF`), linked from each plan's trigger and end timer; later indexes shift | Upstream's 3-hour window |
| Template Builder, any air formation | Great Battles ids rewritten to editor ids (20 → 5, 23 → 8, …; 19 kept) | §3 |
| Template Builder, new air seats | Default formation by role (Column - Pairs Loose for fighters, Flights wedge Safe for attack, Heavy - Wedge Safe for bombers), where every new seat was Heavy Wedge Safe | §3 |
| Template Builder, a group whose events Then a Goto order | One `Go Home Once n` counter per flight in front of that Goto timer | Upstream's fix of our issue §3 |
| New features | + Mission CAP and Strike, air starts and field spawn, map strikes and CAPs, roads and rails from the editor `.ini` files, trains and truck leads on network corners | New outputs only when used; not covered by a baseline |

**Done after approval:** re-recorded `template.Group` and `exclusive.Group` with `cargo test --offline write_p14_baseline -- --ignored`, updated the fingerprint in `baseline_tests::trace_off_changes_nothing` (now `0x1171aa00b3b1051c`) with a comment naming this merge, added the formation-id fact to `docs/il2-loader-and-log-facts.md`, re-ran everything (§6).

## 6. Test and warning counts (after the merge, 2026-10-18)

- `cargo test --offline`: **781 passed, 0 failed, 28 ignored.** (Before the baselines were re-recorded: 779 passed, 2 failed, the two baseline checks at `template.Group` byte 12557.) Main before the merge: 707 / 0 / 28. Upstream alone: 543 / 0 / 3.
- `flown_probes_regenerate_byte_for_byte_except_flight2`: passes, **52 files**.
- Warnings: **82 build, 307 clippy** (main: 69 / 251; upstream alone: 82 / 307).

## 7. Things to know before pushing

- **Repository size:** upstream's history adds about 150 MB uncompressed: two old release executables (`IL2MissionUtility-v0.3-Alpha.exe`, `-v0.5-Alpha.exe`, about 51 MB each), the height grid (31 MB) and `References/_gen.Mission` (17 MB). A merge keeps them in our history even if they are deleted later. Every file is under GitHub's 100 MB limit.
- **Decision number 0023 is taken** by the step 4 branch (`codex/p14-4`, `0023-step-4-threshold-one-complex-trigger-per-area.md`); the new Proposed record here was first numbered 0024 and is **0026** (allocated by the coordinating session: `codex/p14-4` keeps 0023 to 0025, `codex/p14-8` takes 0027 and 0028). The merge commit `c3792eb` still says 0024 in its message.

## 6b. Main merged in (2026-10-26)

`origin/main` (`141af6e`, PR #32: P14 steps 4 to 6, decisions 0023 to 0025, 0029, 0030) was merged into this branch, as asked by the coordinating session. Main's code changes are all in `src/airtask/` and merge cleanly with the upstream merge. The only conflict was the index in `docs/decisions/README.md` (rows put in number order). `STATE.md` and `src/airtask/` are as on `main`.

- `cargo test --offline`: **876 passed, 0 failed, 30 ignored** (781 / 0 / 28 before; the difference is main's step 4 to 6 tests).
- `flown_probes_regenerate_byte_for_byte_except_flight2`: passes, **52 files**; the flight 1M and 1P frozen inputs still match.
- Warnings: **81 build, 306 clippy** (82 / 307 before).
- No baseline changed in this merge; Generate output is as in §5.
- Decision numbers: 0026 (this branch) fits between main's 0025 and 0029. 0027 and 0028 are step 8's (`codex/p14-8`). **The next free number is 0031.**

## 7b. For the UI sessions (steps 8 and 9)

- `src/ui.rs`, `src/shell.rs` and `src/help.rs` merge with **no conflict**: every change in them is upstream's, taken as upstream wrote it. Nothing of ours was added to them, so `codex/p14-8` rebases onto upstream's versions only.
- **Rail registration** stays in `src/ui.rs`: `enum AppMode` (line 128) and the rail order `const MODES` (line 143; Ctrl 1–6 follow it), dispatched in the page `match` (about line 780). The rail widget is `shell::mode_rail` (`src/shell.rs`, line 852).
- **The Map dock is now in `src/ui/map.rs`**: `enum MapDock` (line 339; Period / Forces / References / Terrain), `map_dock_panel` (line 2715), `map_forces_tab` and `map_page` (line 2513). Template Builder's UI is in `src/ui/builder.rs`, the Airfield page in `src/ui/airfield.rs`.

## 7c. Rules from the coordinating session (2026-10-18)

- **Nothing goes to the upstream author** (no issues, comments, files, patches or PRs) until Air Tasking has a working concept and UI. Pulling and merging upstream's commits is fine. Decision 0026 stays in this repo; it is not an offer to upstream.
- `STATE.md` is not changed on this branch; a refreshed version is on `claude/state-refresh-2026-10-18`.

## 8. Facts for STATE.md (for the coordinating session)

- Upstream v0.7.0 merged on `claude/upstream-merge-0.7` (approved by the user 2026-10-18, committed, not pushed, not in `main`).
- Decisions 0012, 0013, 0014 and 0016 are now also upstream's; ours kept, outputs identical. 0015 stays ours.
- Template Builder writes Korea editor formation ids (upstream); 0019 to 0022 apply by formation type, with Column - Pairs (4) counted as Pairs. Unflown: ids 4, 5, 8.
- Generate changes from the merge: `template.Group` one line, `exclusive.Group` the 3-hour window; nothing for Air Tasking or the flown probes.
- New Proposed decision 0026 (Air Tasking and upstream's strikes and CAPs) replaces STATE "Waiting on the user" item 4.
- Test line after merging `main` (2026-10-26): 876 passed, 0 failed, 30 ignored; 81 build / 306 clippy warnings; 52 flown probe files byte-identical.
- Game fact added to `docs/il2-loader-and-log-facts.md`: `FormationType` 19 and 20 fly as Pairs and Wedge on game 1.005 (flights 2D to 2I), although upstream lists 19 as "Reserved - 15" in the Korea editor's list.
