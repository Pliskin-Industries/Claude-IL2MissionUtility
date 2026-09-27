# P14 Air Tasking: pre-implementation review (Fable + Astra)

Status 2026-09-27. Review of `handoff/P14-air-tasking.md` at `ec1d3c0`. **No plan or code changes were made.** The user decides which findings to apply.

- **Fable review:** Claude Fable 5.1, in session. Read the whole plan and checked about 60 code anchors and three template files.
- **Astra review:** `gpt-6-astra`, `ultra` effort, read-only sandbox. Codex job `20260927054911-7a36f64e`, thread `01a0e169-0871-7d90-b1c5-35ae9fb90b2b`. The full report is in the appendix, unedited.

The user calls this work "Wing Commander (WG/CC)".

## 1. Combined verdict

**Not ready to build as written. One revision round of the plan is needed; no redesign.**

- The grounding is sound. Both reviews found the code anchors and template facts accurate.
- One defect disables the feature during a mission (C1). Several fallbacks, tests and the user-template path need tightening.
- Fable's first verdict was "ready with fixes". Astra's was "not ready". Astra's extra findings were checked and hold, so the combined verdict is the stricter one.

## 2. Triage

Status: **Confirmed** = traced in the plan or the source. **Needs a check** = depends on engine behaviour nobody has observed. No finding was refuted.

### Must fix before step 1a (the skeleton freezes types)

| ID | Finding | Found by | Status | Change to the plan |
|---|---|---|---|---|
| C1 | **Budget loses a release.** Two DONEs within the 0.1 s settle gap: the second finds no active `DEC` relay. The level stays one too high for the rest of the mission. Two sorties on one objective share a Zone Out, so one player leaving or dying ends both in one tick. The acceptance plan (line 1310) has that layout. | Both | Confirmed | Replace the level counter with per-slot tokens: a sortie holds slot k and releases slot k. Add a walker test with two DONEs 0.05 s apart. |
| C2 | **Index type.** `trace.rs` signatures use `i64` (lines 1034–1041). The AST and every builder use `i32` (`ast.rs:51`). | Both | Confirmed | Use `i32` in the 1a stubs. |
| C3 | **Run limit.** D12 says 3 (line 109); the skeleton says 1..=6 (line 607). | Astra | Confirmed | State the default (3) and the range once. |
| C4 | **Upstream.** `upstream/main` has `c1d88c3` (single 1500 m airstart; also removes Fighter Pack reinforcement logic). It rewrites parts of `template.rs`, `flights.rs`, `ui.rs` and `model_spec.rs`. | Both | Confirmed | User decides: merge it before the baselines are recorded, or record from `ec1d3c0`. |

### Fix in the plan before the step that uses it

| ID | Finding | Found by | Status | Change to the plan |
|---|---|---|---|---|
| C5 | **HOT goes off at every re-check,** for 0.1 s or more, with a player present. A stop probe or clock draw in that gap fails. Rare per event (about 0.04 % to 0.8 %), but the plan calls the gap harmless and calls N = 1 an exact match for U7. | Astra | Confirmed | Keep HOT on during the re-check; switch it off only when the re-check fails. |
| C6 | **Slot close does not stop the attempt in flight.** The last attempt always starts before `SLOT CLOSE` and can win up to A seconds after it. With a short period, it overlaps the next slot, and `validate` passes. Default settings are not affected. | Astra | Confirmed | Add A to the period check, or gate attempt outputs at close. |
| C7 | **Fallback matrix has holes.** Closer0Race needs an already-empty zone to fire at once (not tested) and needs b = ALL. The watchdog assumes `Restart`. The counter ring has no refill. The latch fallback reuses the primitive that J4 would have disproved. | Both | Confirmed | List the supported combinations of probe results. Name the combinations that stop the build. |
| C8 | **User templates have no compatibility contract.** A Spawn-mode template has `SPAWN UNITS`, not `MISSION BEGIN` (`template.rs:2675`). An escort that is first in the file has no waypoint. Locale ids from two templates collide and the first wins (`locale.rs:49`). | Astra | Confirmed | Drop user templates from v1 (they are a Claude addition, §10 item 3), or add one shared compatibility check. |
| C9 | **Replay follows state links as if they were pulses.** Deactivate and Activate `Targets` list nodes to switch, not nodes to fire (`flights.rs:981–997`). "First dead link" would flag outputs that were correctly closed. | Astra | Confirmed | Separate pulse links from state links. Until the probes, report firings and "unobserved successors" only. |
| C10 | **Cruise fallback changes existing output.** Setting `f80c10` cruise in `model_spec` changes Template Builder's suggested waypoint speed (`ui.rs:4401`). CLAUDE.md forbids that. | Astra | Confirmed | Keep the measured speed local to `airtask/timing.rs`. |
| C11 | **Tests that cannot pass or do not test the claim.** `clock_nobody_won_quiet_rate_is_small` (no-repeat and the run cap prevent 200 wins). DONE timing tests omit the 60.6 s cleanup. Probe 2 builds HVAR but checks AirAlert. T-t closes at 05:30 (line 956) and 05:00 (line 999). | Both | Confirmed | Correct each test definition. |
| C12 | **UI guard tests do not cover the new UI on their own.** The height guard skips disabled controls and unopened panels. The glyph guard reads four files only. | Astra | Confirmed | Add tests for a filled pool, the Replay panel and disabled states. Add the new files to the glyph guard. |
| C13 | **Simultaneous zone requests are normal, not rare.** PASS fans out to every zone entry on an area in one tick. One is granted; the rest are denied and wait 15 min. | Fable | Confirmed | Solved by C1's slot design, or queue zone entries per area. |
| C14 | **No limit on stamped copies.** Auto-fill with 5 targets per side gives about 90 copies and about 400 aircraft in the file. D5 limits active aircraft only. | Fable | Confirmed (effect on the server: needs a check) | Add a `validate` warning and a load check in Probe 2. |

### Needs a check in game

| ID | Finding | Found by | Proposed check |
|---|---|---|---|
| N1 | **Trace carrier is unverified when flight 1 flies.** Breadcrumb objectives may show on screen or disturb the same-tick cells. | Fable | Fly a solo 6-minute "flight 0": cell T-t only, text log on. It also gives the real log fixture (0L). |
| N2 | **The extension issues no orders.** The AttackArea time has run out when `EXTEND` starts. | Fable | Re-pulse each lead's AttackArea on `EXTEND`; observe in Probe 2. |
| N3 | **One copy granted twice.** Clock GO and zone FIRE request the same copy; needs `Twice` timers, `RunsOn`, and a DONE within about 0.1 s. | Astra | Add a per-copy pending latch (cheap). No flight needed. |
| N4 | **AI may trigger sorties.** If CheckZone counts AI and no player filter exists, step 11's "no AI-triggered sortie" check cannot pass. | Astra | Decide after T-a1: accept AI activation, or stop. |

### Low

- `ENii GO` marks the entry as last winner before the budget answers (line 411).
- Stale facts in the plan: repo root (line 5), "37 GB" (line 1454), test count 467 (the grep gives 479).
- U11 names an Opus session as the builder. Astra is now on the account.
- `suggested_waypoint_speed_kmh` returns 100 for unknown models, so the low-speed repair (line 216) does not help them.
- The trace grid is outside the parking area, not outside every possible front (line 1040).

## 3. Found sound by both reviews

- Code anchors and the six template files.
- Lead detection by entity `Targets`, and finding order timers by edge, not by name.
- The byte-identical approach: visibility-only edits, empty default pack list, baselines before any P14 code.
- `ui.rs` single ownership and the coordinator commit.

## 4. Decisions for the user

1. Budget: per-slot tokens (recommended), or keep the level counter with staggered DONEs?
2. Add flight 0 before flight 1?
3. Merge `c1d88c3` before the baselines?
4. User templates: drop from v1, or add the compatibility check?
5. Who builds: Astra through the Codex broker, or a separate Opus session (U11)?

---

## Appendix: Astra's report (verbatim)

**1. VERDICT: NOT READY**

The specified graphs can permanently miscount the budget and reject an extension despite uninterrupted player presence. Several fallbacks and acceptance tests contradict the behavior they are supposed to establish. Most existing-code anchors remain accurate, but these design defects need resolution before implementation.

Read all 1,465 lines and the four orientation documents. Review baseline: `ec1d3c0`; `c1d88c3` remains unmerged. No files changed or tests run. Below, **P** means [handoff/P14-air-tasking.md](<C:/Machine Intelligence/Claude-IL2MissionUtility/handoff/P14-air-tasking.md>); “verified” describes repository evidence or graph reasoning, never observed engine behavior.

**2. FINDINGS**

**F1 — Blocker — Budget transitions discard completion pulses.**  
**Plan:** §3.3, P:283–303; D25, P:122.

With two active sorties, DONE A at **100.000** disables L2 and schedules L1 for **100.100**. DONE B at **100.050** reaches only inactive DEC relays and disappears. At 100.100, the budget reports one occupied slot although no sorties remain. Identical cleanup chains following a shared-area exit also make clustered completions plausible.

The plan discusses requests lost during SETTLE and simultaneous UP/DOWN, but not lost releases or DOWN/DOWN. This causes permanent capacity loss.

**Smallest fix:** preserve every release across transitions, including multiplicity; serialize releases and admissions together. Add a walker case with two DONEs separated by 0.05 seconds.  
**Confidence: 0.99 — verified from the specified graph.**

**F2 — High — Both front ends can grant the same copy twice.**  
**Plan:** §3.3, P:291–295; §3.7, P:413–419; §3.8, P:443–445; P:1014.

Under the expressly supported `Twice`/`RunsOn` timer outcomes, copy 7 has a 0.45-second INC delay. Clock GO at **100.000** and zone FIRE at **100.230** both request that copy because its pointer advances only on GRANTED. The first output grants at **100.450**, enabling L2 at 100.550. An older sortie finishes at **100.560**, restoring L1 at 100.660. The second queued INC output arrives at **100.680**, finds its OUT relay reactivated, and grants the same copy again. Its single-use DONE cannot release both admissions.

**Smallest fix:** add a per-copy pending/consumed gate, closed before REQ, reopened only after DENIED, permanently closed after GRANTED. Test overlapping front ends explicitly.  
**Confidence: 0.95 — engine-conditional hypothesis; graph path verified.**

**F3 — High — The HOT recheck gap is consequential.**  
**Plan:** §3.5, P:358–370; §3.2, P:250–258; §3.4, P:315.

Continuous presence produces PASS at **120.100**. HOLD expires at **360.100**, clearing HOT; INNER cannot be repulsed before **360.200**. STOP PROBE at **360.150** loses its HOT-gated pulse and ends through STOP WAIT, even if presence never stopped. Later PASS cannot retry the closed probe. A clock draw during the same gap can exclude the occupied area for the entire slot.

This disproves both “That gap is harmless” and the claim that N=1 exactly implements U7.

**Smallest fix:** retain HOT while checking; clear it only when the recheck fails. Test both stop probes and draws during successful rechecks.  
**Confidence: 0.99 — verified from the specified graph.**

**F4 — High — SLOT CLOSE does not close the outstanding attempt.**  
**Plan:** §3.7, P:401–425; uncapped auto-fill, P:833.

For **144 clock entries**, L=72.5, A=74, and the capped slot window is 300 seconds. DRAW occurs at 0.3, 74.3, 148.3, 222.3 and **296.3**; the final attempt can emit through **368.3**. SLOT CLOSE disables only REROLL, so that attempt can still win after closure.

With jitter span 360 and period 671, validation passes, yet the previous delayed slot’s outstanding attempt can overlap the next slot. Both manipulate shared cancellation/randomizer state.

**Smallest fix:** prohibit attempts that cannot finish before closure, or gate every outstanding output at closure and account for its lifetime in schedule validation.  
**Confidence: 0.97 — verified timing counterexample.**

**F5 — High — Failed probes cannot safely select several advertised fallbacks.**  
**Plan:** §8, P:1006–1019; §9, P:1384–1398.

The fallback matrix is incomplete:

- **Counter ring:** with K=2, counters firing before 60 and 120 seconds leave the first counter already spent when rotation returns at 120. No reset/replenishment is specified. Reactivating a fired counter is itself unproven; E3 tests only a partial count.
- **Watchdog:** “each hit restarts” assumes `Restart`. Under `Ignore`, hits at 1, 2 and 3 seconds do not postpone a watchdog’s original expiry.
- **Latch fallback:** under `Twice`, delaying two simultaneous pulses by 0.05 seconds delivers two simultaneous pulses to another instance of the primitive J4 just disproved.
- **Closer0Race:** needs an already-empty zone to report immediately when pulsed. T-b tests departures after occupied activation, not this condition. No further fallback is specified if it fails.

**Smallest fix:** define supported combinations of probe outcomes and test each selected fallback. Keep N=1 where reusable counting fails; block unsupported combinations instead of promising a one-line default change.  
**Confidence: 0.98 — verified specification gaps; runtime branches are hypotheses.**

**F6 — High — Disk templates have no enforceable compatibility contract.**  
**Plan:** §1, P:44; §3.1, P:197–216; §3.2, P:224–242; §7, P:890.

A native single-plane, one-waypoint Spawn template passes the stated placement restrictions but has `SPAWN UNITS`, not the shell’s required `MISSION BEGIN`. See [template.rs:2675](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2675>). Likewise, permitting waypoint-less escorts while always selecting the first lead’s waypoint fails if that escort appears first.

Locale support also silently mismerges accepted inputs: two templates using LCText=13 for different messages retain the first string. [locale.rs:49](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/locale.rs:49>) explicitly implements first-wins merging; [duplicate.rs:28](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/duplicate.rs:28>) does not remap LC references.

**Smallest fix:** define and share one compatibility inspection across loading, readiness and generation. Initially reject unsupported bring-up/cleanup shapes and conflicting locale IDs; choose the navigation lead by capability.  
**Confidence: 0.99 — verified from files.**

**F7 — High — Replay mistakes successful suppression for a broken chain.**  
**Plan:** step 0T, P:1041, 1050, 1078.  
**Source:** [flights.rs:981](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/flights.rs:981>), especially 992–997.

At **t=0.5**, Random 1 fires its output and a Deactivate closer. That closer’s Targets identify later outputs to disable. Following every Targets edge through untraced nodes incorrectly creates causal paths to their Spawners. Those Spawners correctly remain silent, yet replay labels them “first dead links.”

Even genuine pulse adjacency does not establish failure across random rolls, disabled gates or counters.

**Smallest fix:** distinguish pulse links from state-control links. Before the probes, deliver raw firings/timeline and explicitly observational “unobserved successors”; reserve failure diagnosis for paths with specified conditions and deadlines. User-requested 0T tracing is reasonable; making this unproven diagnosis primary evidence is not.  
**Confidence: 1.00 — verified from files.**

**F8 — Medium — Cruise calibration changes existing output with P14 unused.**  
**Plan:** risk n, P:1395; A6, P:92.  
**Source:** [model_spec.rs:354](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/model_spec.rs:354>), [ui.rs:4400](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui.rs:4400>), [CLAUDE.md:15](<C:/Machine Intelligence/Claude-IL2MissionUtility/CLAUDE.md:15>).

Changing global F-80 cruise after T-n changes Template Builder’s suggested waypoint speed and consequently generated `Speed`, even when Air Tasking is unused.

**Smallest fix:** keep P14 transit calibration local; treat any global correction as a separate, explicitly accepted output change. Include an F-80 UI baseline case.  
**Confidence: 1.00 — verified consequence if this fallback is selected.**

**F9 — Medium — Several acceptance criteria cannot establish the claimed result.**  
**Plan:** steps 0, 3, 5 and 11.

P:1264 requires the sole HOT entry to win ≥97% of 200 non-quiet-roll slots. No-repeat forces intervening quiet slots; three available copies cap total grants at three. This cannot test the claimed probability without independently resetting state.

P:1197–1198 and 1369 omit cleanup latency from DONE expectations. DONE follows DONE HUB by **60.6 seconds**, using the retained timers: [template.rs:82](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:82>).

Probe 2 generates HVAR at P:1210 but asks to assess AirAlert at P:1212. T-t closes at 05:30 in P:1057 but its test still pins 05:00 at P:999. The run maximum is three at P:109 and six at P:607.

**Smallest fix:** separate independent probability trials from lifecycle tests; distinguish stop decision from budget release; reconcile fixtures, times and limits before freezing acceptance.  
**Confidence: 1.00 — verified contradictions.**

**F10 — Medium — Existing UI guards do not automatically cover the new UI.**  
**Plan:** §6.3, P:857; step 9T, P:1346.  
**Source:** [ui_tests.rs:1239](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui_tests.rs:1239>), [ui_tests.rs:1666](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui_tests.rs:1666>).

The height guard visits default tab states and skips disabled controls. It will not inspect unopened Replay panels or controls in an empty pool. The glyph guard scans four existing files, excluding generated messages from `airtask`/`missionlog`.

**Smallest fix:** explicitly exercise populated pools, expanded settings, Replay and disabled states; include their displayed strings in glyph checks.  
**Confidence: 1.00 — verified from files.**

**3. ANCHOR CHECK TABLE**

Fixture abbreviations below refer to the six exact Historical1950 filenames linked in the first six fixture rows. TRUE describes authored data/code, not proof of game behavior.

| Plan claim/location | Result | Evidence checked |
|---|---|---|
| Repository path/current revision, P:5–7 | STALE | Actual checkout is `C:\Machine Intelligence\Claude-IL2MissionUtility`, HEAD `ec1d3c0`; historical anchor revision is explicitly disclosed |
| AirAlert: four planes, lead 7, max Time 1080, WP 3050/660, P:23–30 | TRUE | [AirAlert:504](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F80_AirAlert_4ship.Group:504>); entities 566/628/690, commands 740/762, WP 788 |
| HVAR: four planes, lead 7, max Time 600, WP 3050/660 | TRUE | [HVAR:486](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group:486>); entities 548/610/672, command 718, WP 748 |
| CloseSupport: four planes, lead 7, max Time 900, WP 1500/520 | TRUE | [CloseSupport:486](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F51_CloseSupport_4ship.Group:486>); entities 548/610/672, command 718, WP 748 |
| ArmedRecon: leads 7/11, max Time 1200, WP 900/520 | TRUE | [ArmedRecon:522](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group:522>); entity 646, command 754, WP 822 |
| Yak: leads 7/11, max Time 300, shared WP 3050/500 | TRUE | [Yak:558](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group:558>); entity 682, commands 790/832, WP 862 |
| Il-10: leads 7/15, max Time 600, shared WP 1500/390 | TRUE | [Il-10:558](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_DPRK_Il10_KimpoAttack_2x4.Group:558>); entity 806, commands 1038/1080, WP 1110 |
| Ground AttackArea is at spawn, P:179 | TRUE | [template.rs:2815](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2815>); HVAR:723 versus WP:753 |
| Air AttackArea occupies a canvas slot, P:180 | TRUE | AirAlert:741 |
| Zone IN/OUT radii 16/35 km; HVAR Targets `[19,20,16,25]`, P:181 | TRUE | HVAR:66, 86 |
| HVAR WP→37→39→30 cleanup; approximately 3.6 seconds, P:182 | TRUE | HVAR:238, 274, 292, 398, 416, 748; graph arithmetic only |
| AirAlert duplicate `AttackArea 1` timers; WP targets both, P:183 | TRUE | AirAlert:398, 416, 434, 788 |
| ArmedRecon escort Cover Objects `[11]`, Targets `[7]`, no WP, P:184 | TRUE | ArmedRecon:794; sole WP:822 |
| Six fixtures contain no RTB, OnEvents or LCName/LCDesc, P:116/186 | TRUE | Full-file searches of all six fixtures |
| Wingmen target their lead and force Activate, P:109/197 | TRUE | [template.rs:2564](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2564>), 2572 |
| Disk templates can all use the specified `MISSION BEGIN` shell destination | FALSE | Spawn branch writes `SPAWN UNITS`: [template.rs:2675](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2675>) |
| Heading convention and rotation formula, P:178/200 | TRUE | [placement.rs:178](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/placement.rs:178>), 224 |
| Existing orientation scope is only Model nodes/linked entities, P:201 | IMPRECISE | Also explicitly includes Block/Ground: [placement.rs:152](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/placement.rs:152>), 195 |
| X/Z translation preserves Y, P:204 | TRUE | [placement.rs:101](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/placement.rs:101>); [ast.rs:221](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ast.rs:221>) |
| Path parking sorts indexes, preserves Y, excludes RTB, P:123/205/214 | TRUE | [mapnet.rs:328](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/mapnet.rs:328>), 892 |
| Ground AttackArea predicate and snapping, P:207/737 | TRUE | [weapon_range.rs:212](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/weapon_range.rs:212>), 274 |
| RTB Area/Priority/speed/altitude properties, P:210 | TRUE | [template.rs:2772](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2772>) |
| Duplication remaps indexes and event pointers, P:218 | TRUE | [duplicate.rs:28](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/duplicate.rs:28>), 62, 101 |
| Silencing Mission Begin clears Targets and Enabled, P:224 | TRUE | [recon.rs:1680](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/recon.rs:1680>); engine behavior is supported only by its comment |
| Dropcount convention/reset machinery, P:276 | TRUE | [template.rs:2638](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2638>), 2650; authored convention only |
| Event 4 is destroyed; 0/2 are pilot killed/crashed, P:277/1011 | TRUE | [template.rs:559](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:559>); attachment helper 3865 |
| F-80 cruise 732 and empty notes, P:467–468 | TRUE | [model_spec.rs:150](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/model_spec.rs:150>) |
| Suggested-speed fallback repairs low-speed user templates, P:216 | IMPRECISE | All-unknown scripts still return 100: [model_spec.rs:352](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/model_spec.rs:352>) |
| Fighter gate-cell Time-0 idiom, P:152 | TRUE | [pack.rs:54](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/pack.rs:54>) and gate construction |
| Randomizer 0.5-second waterfall and delayed reopening, P:382–384 | TRUE | [flights.rs:930](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/flights.rs:930>), 947, 984 |
| Activate/Deactivate closer link field, P:153 | TRUE | It is Targets: [flights.rs:985](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/flights.rs:985>), 997; recon.rs:1349 |
| Private helpers and timer Random=100, P:759–765 | TRUE | [template.rs:3786](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3786>); recon.rs:1681/1727/1733; placement.rs:204/213 |
| DPRK prefix recognized, P:120 | TRUE | [frontlines.rs:1878](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/frontlines.rs:1878>) |
| Rear AO corner helper, P:110–111 | TRUE | [mapfighters.rs:73](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/mapfighters.rs:73>) |
| Preview arrow front differs from exporter input, P:732 | TRUE | [ui.rs:8559](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui.rs:8559>); frontlines.rs:1187/1246. Agreement on selected test arrows does not establish general agreement |
| Placement seed uses system nanoseconds; Generate itself draws none, P:723/902 | TRUE | [ui.rs:9138](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui.rs:9138>), 11305 |
| Map readiness currently empty, P:824 | TRUE | [ui.rs:2005](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui.rs:2005>), 2067 |
| Undo needs pre-change recording, P:826 | TRUE | [shell.rs:915](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/shell.rs:915>) |
| Four dock tabs; widths do not shrink, P:830–831 | TRUE | [ui.rs:283](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ui.rs:283>); shell.rs:681 |
| Plain-number map shortcuts need separate Num7 handling, P:794 | TRUE | [shell.rs:985](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/shell.rs:985>); ui.rs:211 |
| Existing guards cover every new control automatically, P:857/1346 | FALSE | ui_tests.rs:1239 and 1666; F10 |
| Complete UI FrontOptions literal needs updating, P:888 | TRUE | [frontlines.rs:367](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/frontlines.rs:367>); ui.rs:11337 |
| Stamp after ground/before AO; empty iterator consumes no IDs, P:889 | TRUE | [frontlines.rs:1450](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/frontlines.rs:1450>), 2200 |
| Air Tasking otherwise imports as army, P:892 | TRUE | [frontlines.rs:1575](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/frontlines.rs:1575>), 1912 |
| Adding sidecar paths suffices for arbitrary user-template locales, P:890 | IMPRECISE | Conflicting LC IDs silently collapse: locale.rs:49/148; duplicate.rs:28 |
| Terrain leaves airborne planes/air waypoints unchanged, P:903 | TRUE | [terrain_apply.rs:184](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/terrain_apply.rs:184>); does not cover unspecified ground-start inputs |
| K14 origin and sole Airfield, P:949 | TRUE | [K14 AFB_mp.Group:20085](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/K14 AFB_mp.Group:20085>) |
| Willys NOICON Index117 reference, P:1039 | TRUE | [K14 AFB_mp.Group:1519](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/K14 AFB_mp.Group:1519>), linked entity 1564 |
| Existing CLI insertion point, P:1031 | TRUE | [main.rs:44](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/main.rs:44>) |
| Trace’s frozen `&mut i64` allocator directly fits existing helpers, P:1034–1041 | FALSE | AST/builders use i32: ast.rs:51; duplicate.rs:103; template.rs:3835. Unify before parallel branches |
| Outside parking bounds implies trace grid cannot meet front/units, P:1040 | IMPRECISE | Parking limits are not universal map limits: placement.rs:30; geo.rs:23; custom front input frontlines.rs:1182 |
| Every Targets edge represents downstream firing, P:1041 | FALSE | Deactivate Targets suppress nodes: flights.rs:997; F7 |
| DONE timing assertions, P:1197–1198 | FALSE | Retained cleanup adds 60.6 seconds: template.rs:82; HVAR:238/274/292 |

**4. AREAS FOUND SOUND**

- Historical fixture facts, edge-based AirAlert identification, and rigid X/Z placement are well grounded.
- Sequential N=1 threshold rearming and ON STATION deduplication work under the stated latch assumptions.
- Empty-air stamping and early baseline fixtures provide a sound output-preservation approach.
- UI file ownership and the coordinator’s FrontOptions edit are explicitly managed.
- Presentation-only architecture, DPRK/NATO wording, and minimum sizes are carried forward; guard coverage needs extending.

**5. OPEN QUESTIONS**

- Should baselines target `ec1d3c0` or include `c1d88c3` first? The upstream commit also removes Fighter Pack reinforcement logic; Historical1950 files are untouched.
- Which combinations of timer retrigger, latch, counter-reset and already-empty-zone behavior actually hold? Selected fallbacks need their own flights.
- If no player-only filter exists, is AI activation acceptable? The documented warning/dwell fallback cannot guarantee step 11’s “no AI-triggered sortie” requirement.
- Can objective breadcrumbs log repeatedly without affecting mission outcome, and does the log preserve enough identity and events for reliable replay?
---

## 6. User decisions after the review (2026-09-27)

**Status: applied to `handoff/P14-air-tasking.md` as revision round 4 (2026-09-27).** The plan is the authority; where this section and the plan differ, the plan is right. Astra's second review of the changed sections is section 7.

| # | Decision | Effect on the plan |
|---|---|---|
| R1 | **MCU economy is a design rule.** Every MCU costs tick budget. Choose the cleanest design with the fewest MCUs, not the smallest change. | New rule in §2. `validate` reports copies, aircraft, MCUs and check zones, and warns above a limit. |
| R2 | **Budget = head count.** | Replaces §3.3 and D25. See the design below. |
| R3 | **All testing solo where possible. Flight 0 accepted.** | Step 0 becomes three solo flights: 0 (automated cells and the trace test, about 10 min, no flying), 1A (NATO F-80C: cruise speed, zone activation), 1B (DPRK: kill events). |
| R4 | **Merge upstream `c1d88c3` in full.** | Done on branch `claude/upstream-sync` (merge `a34a00e`), not yet on `main`, not pushed. Baselines are recorded after it lands. Line anchors in `template.rs`, `flights.rs`, `ui.rs` and `model_spec.rs` have moved; find them by name. |
| R5 | **Multi-player testing comes last,** after the breadcrumbs and everything else are confirmed working. | New final step (after step 11): editor task 0a, the Complex Trigger path for N of 2 or more, cell T-d, cell T-b (Zone Out with several players). v1 ships with N = 1. |
| R6 | **Extension uses the area's state.** Extend once if a player was confirmed over the target at the last presence check. | Replaces D31, D32 and the stop-probe cell in §3.2 and §3.4. |
| R7 | **User templates stay in v1.** A template that is accurate and works is accepted. | The shell finds the bring-up timer by wiring, not by the name `MISSION BEGIN`. |
| R8 | **Astra builds.** Fable scopes each step, reviews each diff and re-runs the tests. | Replaces U11 and the fan-out table in §10. Steps run in the plan's serial order on one branch. |

### Head-count budget (R2), design for §3.3

- Per copy: `SNjj GATE` (relay, in the side's gate set) and `SNjj FLYING` (timer, `Time = 0.05·q`, q = the copy's order on its side; active only while the copy flies).
- Request path: front end → `ENii COPY r` → `SNjj GATE` → `SNjj START`. There is no REQ, GRANTED, DENIED or timeout per copy.
- `START` switches its own `FLYING` on (share the Activate that advances the copy pointer). `DONE HUB` switches it off (share its self-Deactivate). Both pulse `BUD s EVENT`.
- Recount, per side: shut every gate at once; reset `BUD s COUNT` (Counter = N, Dropcount 1) with a ModifierSetVal; 0.05 s later pulse every `FLYING`; each active one reaches `COUNT`. If `COUNT` fires, the gates stay shut. If it has not fired when the recount ends (`0.05 + 0.05·copies + 0.1` s), the gates reopen.
- A recount is never re-triggered while it runs. An event during a recount sets a `BUD s AGAIN` flag, and the end of the recount starts one more recount.
- A refused zone request is found by a 1 s race timer per zone entry, cancelled by the copy's `START`. (A "budget full" relay was considered and dropped: a started copy shuts the gates in the same tick, so such a relay could pass for a request that was granted.)
- Zone entries on one area fire staggered (recount length + 0.2 s per position), so simultaneous requests are each considered.
- 2 MCUs per copy (was 6 + 2N) and 18 per side at any N.
- New probe cell E7: three pulses in one tick into Counter(3). If counters count same-tick pulses, the `FLYING` spacing can be 0.
- Walker tests: two DONEs in one tick; two DONEs 0.05 s apart; a DONE during a recount; a request during a recount is refused; after every sortie has ended the gates are open (random event orders, many seeds).

### Stop cell (R6), design for §3.2 and §3.4

- The area state is two sets, HOT and COLD, switched make-before-break (the new set on, the old set off 0.05 s later), so a pulse is never lost.
- HOT stays on during a presence re-check and goes off only when the re-check fails (review finding C5).
- Per copy: `ON STATION` → `SNjj HOT` → `SNjj EXTEND` (T_ext) → `DONE HUB`, and `ON STATION` → `SNjj COLD` → `DONE HUB`.
- `HARD STOP` goes straight to `DONE HUB` and is the final stop: `t_arrive + T_attack + margin + T_ext`.
- `DONE HUB` releases the budget and the zone cooldown, about 60 s before the planes are deleted. The `DONE` relay is gone.
- 3 MCUs per copy and no check zone (was 15 and one check zone).
- The clock uses the same pair: `ENii HOT` → GO, `ENii COLD` → area excluded. The 1 s MISS race goes, and an attempt is `L + 0.5` s long.

### Still to apply from the triage

C2, C3, C6, C7, C9, C10, C11, C12, C14 and the Low items, as listed in section 2.

### Option not applied (needs the user)

One check zone per area in place of two (detect inside r_in, check again one transit time later). It saves about 6 MCUs and one check zone per area, but it changes user decision U3.

---

## 7. Astra's second review (round 4 text), and its triage

Codex job `20260927094211-595e56e3`, `gpt-6-astra`, `ultra`, read-only, 2026-09-27. Astra's verdict was **NOT READY**, on eight new findings. All eight were checked, confirmed and applied to the plan the same day. The full report follows the table, unedited.

**What Astra found sound:** the head count under every timer and delivery mode it tried (N = 1, N = 6, more flags than N, no copies); the HOT / COLD sets; the hard stop as the guarantee that a sortie ends; the clock's attempt timing; the zone stagger; and every MCU count stated in the plan. Of its ten round 3 findings, eight are fixed and two were partly fixed (those two are R4-1 and R4-4 below).

| ID | Finding | Status | What the plan now says |
|---|---|---|---|
| R4-1 | The counter once-gate fallback cannot be re-armed, so as a replacement for `BUD s CHECK` it would shut an empty side for good. | Confirmed | The once-gate replaces single-use latches only. Reusable latches stay relays. If J4 and E8 both fail, the build stops before step 3. |
| R4-2 | A clock launch of a `Both` entry could cancel the zone's refusal timer before the zone fired; a later refusal then started no cooldown and the zone never re-armed. | Confirmed | The zone arms `WAITING` and `DENY WAIT OUT` when the chance roll succeeds, and sends its request 0.05 s later. |
| R4-3 | The probe reads "fired twice in one tick" through a counter, which may itself count two same-tick pulses as one. | Confirmed | J4 and E8 are read with E7 or the trace log; otherwise "not settled". New cells E9 and "C2 again". |
| R4-4 | Template edge cases: an escort listed first, a template with its own RTB, event hooks that are not kill events, a waypoint wired straight to Mission Complete. | Confirmed from `template.rs` | Navigation lead throughout; on-target node can be a timer or a waypoint; RTB is reused; only counter-bound events are removed. |
| R4-5 | "One more flight than N" was wrong: ended flights fly home for 60 s while replacements start. | Confirmed | The budget counts sorties; up to 2N flights' aircraft for about a minute. A hard aircraft limit costs 1 MCU per copy (§10 item 17, for the user). |
| R4-6 | Exhaustion did not stop work already under way (a running RANDOM timer; an armed zone). | Confirmed | Exhaustion also switches off `ENii GO`, `ZNii ARMED`, `ZNii FIRE`. No extra MCU. |
| R4-7 | The extension fallback re-issued orders at the end of the extension, not the start. | Confirmed | `SNjj HOT` gives the order when the extension starts. |
| R4-8 | Four tests had impossible timings or outcomes. | Confirmed | Corrected; Probe 2 has cells T-k and T-k1. |
| — | MCU savings Astra proposed | Applied (U13) | 16 per copy (was 17), 16 per side (was 18; 14 at N = 1), 23 per area (was 26). |

**State after this pass.** Every finding of both reviews is applied. The fixes of this second pass have had no independent review. The build rule covers that: Fable re-reads each step's section when it writes that step's delegation prompt, and steps 3 and 4 wait for the probe flights.

### Astra's second report (verbatim)

**1. VERDICT: NOT READY**

The revised design still permits permanent lockouts in a supported budget fallback and in the interaction between clock and zone triggers. Template wrapping can also discard working event hooks and issue competing RTB orders. The probes and acceptance tests do not yet establish several guarantees that the build gates rely on.

Read-only review; no files changed. **P** means [handoff/P14-air-tasking.md](<C:/Machine Intelligence/Claude-IL2MissionUtility/handoff/P14-air-tasking.md>), using the latest **1,713-line** working copy; I incorporated the orphan-sweep edit made during review. “Verified” below means file or graph reasoning, never observed engine behaviour.

**2. STATUS OF ROUND 3 FINDINGS**

| Finding | Status |
|---|---|
| F1 — lost budget releases | **FIXED:** the primary head count replaces lossy level transitions; P:336–356. |
| F2 — delayed double grant of one copy | **FIXED:** delayed admission is removed; the pointer advances at START; P:472–478. |
| F3 — HOT re-check gap | **FIXED:** HOLD no longer clears HOT; P:429. |
| F4 — outstanding clock attempt overlaps next slot | **FIXED:** validation includes another attempt length; P:486–487. |
| F5 — unsupported fallbacks | **PARTLY FIXED:** empty-zone and restart conditions are explicit, but reusable counter latches remain broken; P:1125–1136. |
| F6 — template compatibility | **PARTLY FIXED:** bring-up lookup and navigation-lead selection improve, but shell hookup still uses the first lead; P:224, 256, 280. |
| F7 — replay follows state links | **FIXED:** pulse/state distinction and conditional diagnostics are explicit; P:1168, 1177–1178. |
| F8 — calibration changes existing output | **FIXED:** calibration stays local to P14; P:561. |
| F9 — previous acceptance contradictions | **FIXED:** independent probability trials, cleanup latency, AirAlert fixture and T-t closure corrected; P:1423–1425, 1345, 1361, 1115. |
| F10 — incomplete UI guards | **FIXED:** populated, disabled and Replay states explicitly covered; P:968–971. |

**3. NEW FINDINGS**

**R4-1 — High — Counter-latch fallback permanently closes an empty side.**  
**§3.3 / step 0d, P:350, 1127, 1136:** “a latch becomes a once-gate,” but `CHECK ON` only activates it.

For **N=1, C=2**, START at **10.000** fires the replacement `Counter=1, Dropcount=0` CHECK. The census keeps gates closed at **10.250**. DONE at **11.000** clears the only flying flag, but CHECK has already fired and receives no reset; no further recount occurs. The gates remain closed forever. Threshold ARM has the same reuse problem at P:424.

**Fix:** distinguish reusable latches from single-use gates. For CHECK, replace CHECK ON with a counter reset, remove its obsolete self-deactivation, and require E6. If reset is unavailable, this combination needs another design or a stop condition. Accepting failed J4 **and** E8 also cannot preserve “DONE exactly once.”

**Confidence 0.99 — verified graph; fallback selection depends on engine results.**

**R4-2 — High — A clock launch can permanently strand a zone in WAITING.**  
**§3.8, P:503, 510–515:** DENY cancellation comes from “every copy” START, including clock starts.

Use **C=4**, so `T_count=.35`; the second zone entry’s stagger is **.55**:

- **0.000:** PASS arms DENY WAIT OUT.
- **0.010:** clock starts this Both entry’s first copy; GRANTED disables that OUT.
- **0.550:** the copy dies; DONE reaches still-inactive WAITING. Recount closes gates until **.900**.
- **0.650:** zone FIRE activates WAITING; its second-copy request is refused.
- **1.650:** DENY WAIT reaches its disabled OUT.

No cooldown starts. Further PASS events encounter disarmed ARMED; if no later clock copy starts, the zone stays stuck despite an unused copy.

**Fix:** arm WAITING and the refusal output together when CHANCE succeeds, before issuing the request. The existing FIRE timer can provide explicit sequencing, avoiding another MCU. Test clock START/DONE throughout this interval under both target orders.

**Confidence 0.98 — verified under the specified walker; actual occurrence is an engine hypothesis.**

**R4-3 — High — G0 can falsely confirm its latch primitive.**  
**Step 0, P:1055–1056, 1104, 1107, 1128:** same-tick multiplicity is measured using `Counter(2)`.

Suppose E7 reveals `SameTickCounts::One`. At J4’s test instant, a broken latch emits twice; the readout counts those as **one**, and the video also shows one subtitle. J4 is recorded as passing. E8’s readout has the same blind spot.

Additionally, E7 tests **zero** spacing and the ordinary counters use **two seconds**; neither establishes the fallback’s **0.05-second** spacing claimed at P:375. G1’s C2 tests initial activation, not reusing an already-fired zone for another presence check.

**Fix:** condition J4/E8 conclusions on independently reliable multiplicity evidence; otherwise leave them unconfirmed. Add the actual spaced census and repeated presence-check cycles to the solo probes. These need no production MCUs.

**Confidence 0.98 — verified coverage defects; problematic engine outcomes remain hypotheses.**

**R4-4 — High — Accepted templates can be refused or have working behaviour changed.**  
**§3.1–3.2, P:237–240, 260–280:** “every” OnEvents entry is treated as a kill event; ON STATION still selects the “first lead.”

Concrete counterexamples:

- **Escort first:** the accepted reordered ArmedRecon has no Mission Complete for its first lead. Its Cover belongs to entity 11; its waypoint belongs only to entity 7. The shell’s ON STATION lookup therefore fails. See [ArmedRecon:799](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group:799>) and [ArmedRecon:825](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group:825>).
- **Existing RTB:** the old branch survives the sweep while placement adds another. At **DONE+0.65**, both destinations are issued to the same lead. The competing commands are verified; which wins is an engine hypothesis. See [template.rs:2700](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2700>) and [template.rs:3083](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3083>).
- **Non-kill events:** a valid BingoFuel→ForceComplete hook is erased; a fuel event at **1000 seconds** no longer invokes it. The generator supports these hooks explicitly: [template.rs:568](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:568>), [template.rs:3124](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3124>).
- **Waypoint predecessor:** Template Builder can wire WP directly to Mission Complete, which P:261’s order-timer rule misses. See [template.rs:7339](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:7339>).

**Fix:** normalize supported topology during inspection: use the navigation lead consistently, reuse existing RTB nodes, preserve non-cleanup hooks, and explicitly handle waypoint/TimeOnTarget predecessors. RTB reuse reduces MCU count.

**Confidence 1.00 — verified from files, with command precedence explicitly unverified.**

**R4-5 — Medium — N+1 is not a valid bound on aircraft still flying.**  
**D38 / §3.3, P:147, 367–368, 1675:** “one more flight than N.”

For **N=2**, two surviving sorties reach DONE at **100.000**. Their flags clear, but deletion waits until **160.600**. Two replacements starting at **101** and **102** produce **four** airborne sorties. For N=6, the equivalent overlap is twelve, not seven.

The separate same-tick admission claim is also unbounded as written: staggering is per area, so three unrelated areas can request simultaneously. Conversely, the walker’s immediate depth-first shutdown at P:1260–1261 serializes those requests and cannot reproduce the advertised overshoot.

**Fix:** distinguish counted sorties from surviving aircraft and test both totals. Correct the stated bound without adding MCUs; if a physical cap is required, retain occupancy through cleanup. Model batched admission explicitly before promising N+1.

**Confidence 1.00 for cleanup overlap; 0.90 for the engine-dependent admission hypothesis.**

**R4-6 — Medium — Exhaustion does not suppress pending front-end work.**  
**§3.7–3.8, P:474, 490–491, 512:** exhaustion disables RANDOM/AVAILABLE and zone REARM.

Under **RunsOn**, DRAW at **100.000** starts an entry’s RANDOM timer. Its last copy starts through the zone at **100.100**, disabling RANDOM. At **100.500**, the pending timer nevertheless emits: GO closes the slot, but every COPY relay is exhausted. Another eligible entry loses the slot.

Likewise, a runs=1 Both entry exhausted by the clock can remain ARMED and issue another zone request, contradicting P:1449.

**Fix:** add GO, zone ARMED and zone FIRE to their existing exhaustion Deactivate lists. No additional MCU is necessary.

**Confidence 0.97 — verified graph; clock manifestation depends on RunsOn.**

**R4-7 — Medium — Extension fallback reissues orders when extension finishes.**  
**§3.4 / step 0d, P:298, 386, 1140:** “EXTEND re-pulses” AttackArea timers.

If on-station ends at **900**, EXTEND starts its 600-second delay. Its targets fire at **1500**, simultaneously with DONE HUB—not during the extension.

**Fix:** issue renewed orders from the extension-entry path, and suppress that path after an earlier DONE. The need for renewed orders remains an engine question; the specified timing is already wrong.

**Confidence 1.00 — verified graph.**

**R4-8 — Medium — Several acceptance tests specify impossible timings or outcomes.**

| Plan location | Counterexample | Fix |
|---|---|---|
| Step 3, P:1345 | HVAR arriving at 300 starts ON STATION at **300.5**, so COLD DONE is **900.5**, not 900. Its retained timer is 0.50 seconds: [HVAR:409](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group:409>). | Measure from the on-target timer or include its delay. |
| Probe 2, P:1363, 1370 | Empty budget 2: requests at t,t,t+.02,t+5 yield two starts, not three. If three somehow started, two DONEs leave one occupied slot, so requests 5 **and** 6 cannot both fit. | Preload occupancy for the overshoot case; separate ordinary recovery. |
| Step 6, P:1446 | DENY is **1.1+stagger** after PASS, not one second. | Assert from FIRE or include CHANCE delay. |
| Step 5, P:1408 | A zone START at SLOT OPEN+.15 sets LAST before LAST CLEAR at +.20, contrary to “always comes before any new LAST SET.” | Specify and test concurrent Both-entry behaviour. |

**Confidence 1.00 — verified specification contradictions.**

**4. MCU COUNT TABLE**

Counts include implied Deactivate nodes and count check zones as MCUs.

| Cell | Stated | My count | Difference |
|---|---:|---:|---:|
| Shell, P:311 | 17 + one RTB waypoint/lead | Same | 0 |
| Budget/side, P:357 | 18 | 18 | 0 |
| Threshold/area, P:432 | 26, including 2 zones | Same | 0 |
| Clock entry, P:494 | 13 | 13 | 0 |
| Last waterfall entry | 12 | 12 | 0 |
| Zone entry, P:529 | 18 | 18 | 0 |

**U13 still permits material savings:** merge GATE→START (**−1/copy**); inline threshold’s ungated CHECK/PASS/FAIL fan-out relays (**−3/area**); combine budget CHECK OFF with SHUT and inline EVENT (**−2/side**). N=1 can use the counter-free SingleSlot graph even when counters work (**another −2/side**). These preserve the specified state transitions; optimize after correcting the races.

**5. LEFTOVERS**

- **P:1703:** says other end-timer links are moved to DONE HUB; current P:279 instead rejects remaining links.
- **P:1671:** still promises per-group attribution for repeated spawn names; P:1179 specifies per-name counts.
- **P:931:** readiness’s period description omits the attempt allowance required by P:487.
- **P:1130 versus 1315/1630:** LOSSES needs flight 1B, but formal G0 requires only flight 0.
- **P:1691–1692, 1695:** old level/SETTLE, stop-probe, flights 1/2, editor 0a and fan-out terminology remains **as revision history**, not active instructions. No active per-copy REQ/GRANTED/DENIED or clock MISS cell remains.

**6. AREAS FOUND SOUND**

- Under functioning latches and the stated walker semantics, the primary recount handles all three retrigger modes, both cancellation modes and target orders; I found no independent AGAIN, CHECK or partial-count failure, including N=1, N=6, excess flags and zero copies.
- Default threshold cycles separate state changes sufficiently for delayed OFFs to finish; HOT remains set during successful re-checks. The documented overlap ends the shell once when its latch works.
- HARD STOP independently guarantees eventual DONE under the working-latch assumptions.
- Clock attempt spacing clears the waterfall and input reopening before another attempt; period validation now includes the outstanding attempt.
- Stagger preserves CHANCE→MISS spacing, and DENY’s interval from FIRE.
- With default timing and no players, areas remain COLD and neither front end launches.
- The latest orphan sweep fixes the superseded repeat-mode lookup defect; ordinary Activate and nonrepeat Spawn bring-up lookup matches the repository.
---

## 8. Third review (the second-pass fixes), and its triage

2026-09-27. Asked for by the user before any commit. **The user then decided: apply all fifteen; user templates in tested shapes only (U21); add the hard limit on aircraft (U22); no further full review (U23).** All of it is applied to the plan (its §11, last entry). The "Recommended change" column below is what was applied, except where the user's decisions went further: T2 to T4 are settled by refusing those template shapes, and T11 by releasing the budget at deletion.

- **Astra:** Codex job `20260927105821-1e970592`, `gpt-6-astra`, `ultra`, read-only. Verdict NOT READY for the build as a whole; **step 1a (the skeleton) can start.** Its full report follows the tables, unedited.
- **Fable:** Claude Fable 5.1 read the revised sections and checked the new rules against `src/template.rs`.

**What both found sound.** The head count (including N = 1, overshoot, clustered ends); `START` as the copy's gate; the zone handshake after R4-2; the repeat-mode sweep; period validation; and every stated MCU count (16 per copy, 16 / 14 per side, 23 per area, 13 per clock entry, 18 per zone entry).

### Findings

Status: **Confirmed** = traced in the plan or the source. **Model only** = true under the walker's worst-case delivery rules; whether the game does it is unknown.

| ID | Finding | Found by | Status | First step it blocks | Recommended change |
|---|---|---|---|---|---|
| T1 | **Template rules are ambiguous for two built-ins.** Il-10 and Yak share one waypoint between two leads, so "the Mission Complete timer the waypoint reaches" finds two. | Astra (R5-3) | Confirmed from the files | 2 | Tie an order branch to a lead through the Objects of the command it targets. |
| T2 | **The sweep's roots leave out report hooks.** A Spawn-mode template can start its order chain from the "spawned" report (`attach_report`). The sweep would delete that whole chain. | Fable | Confirmed (`template.rs` `attach_report`) | 3 | Add the target of every kept `OnReports` entry to the roots. |
| T3 | **Event-ended templates.** An event or report that targets Mission Complete is kept while its target is deleted. | Astra (R5-3), Fable | Confirmed | 3 | Re-point such entries to `DONE HUB`. |
| T4 | **Templates with a time-on-target step** get that time plus the on-station time. | Astra (R5-3) | Confirmed | 3 | Start `ON STATION` from the timer that gives the attack order, not from the step before Mission Complete. |
| T5 | **RTB on only some leads.** Reuse leaves the other leads without an RTB. | Astra (R5-3) | Confirmed | 2 | Add RTB for the leads that have none. |
| T6 | **Interface gap.** The clock, zone and budget must add targets to `START`, `DONE HUB` and `EXHAUSTED`, which another builder owns. `SideLogic` has no way to carry that. `SideLogic` is part of the frozen skeleton. | Astra (R5-5) | Confirmed | **1a** | Add `links: Vec<(i32, i32)>` to `SideLogic`; `emit_side_logic` applies them. |
| T7 | **Skeleton details.** `MapAirPack` needs `Clone` and `Debug`; `mapnet::is_rtb_waypoint` is private; stub arguments raise unused-variable warnings; the step 12 enum variants raise dead-code warnings after step 9. | Astra | Confirmed | **1a** | Four one-line additions to the step 1a text. |
| T8 | **Trace interface.** `TraceSelect` cannot name an event type, and `TraceEntry` cannot record one. | Astra (R5-5) | Confirmed | 0T | Add an event-type field to both. |
| T9 | **Pass and timeout in one tick** switch both area sets off until the next check (at most 240 s). A sortie's end pulse in that time is lost; the hard stop still ends it. | Astra (R5-2) | Model only | 4 | The timeout switches the inner zone off first and fires 0.05 s later. One MCU more, offset by merging two Deactivates. Count stays 23. |
| T10 | **One copy can start twice** if the clock and the zone pick the same `Both` entry in the same tick. | Astra (R5-1) | Model only | 3 | Accept and document. It needs two unrelated triggers in one 0.02 s tick, and the second start only repeats orders already given. |
| T11 | **No fixed limit on aircraft.** "Up to 2N flights" is wrong: each ended flight lives 60 s more, and ends can come quickly. | Astra (R5-6) | Confirmed | 3 | State it truthfully, or release the budget at deletion (1 MCU per copy). **User decision.** |
| T12 | **Probe table gaps.** No branch for a failed E9; one breadcrumb fired twice in a tick is not tested; the `Closer0Race` fallback is not tested for re-use. | Astra (R5-4), Fable | Confirmed | 0b | Three automated cells and two table rows. No production MCU. |
| T13 | **Test errors.** Probe 2 cell T-k expects two same-tick starts, which depends on the game. Three timing labels are wrong. "Sends no request" should be "starts no copy". | Astra (R5-6) | Confirmed | 3, 6 | Correct the texts; space T-k's requests beyond the recount time. |
| T14 | **Renewed attack order after an earlier end** (only if the extension fallback is used). | Astra (R5-7) | Model only | 3 | Route the renewed order through a relay that `DONE HUB OFF` switches off. Only in the fallback. |
| T15 | **Stale text.** Step 0b names a "strip list" that §3.2 no longer has; the risk v row still says `EXTEND` renews the order; D5 says "about 8 aircraft" but an Il-10 sortie has 8, so a budget of 2 gives 16. | Fable, Astra | Confirmed | 0b | Correct the texts. |

**Further MCU saving Astra proposed:** make `ENii EXHAUSTED` one Deactivate that lists the targets of both front ends (1 fewer per Clock or Zone entry, 2 fewer per Both entry). It needs T6.

### Fable's reading of the three rounds

- The core logic has now been through three independent passes. The last pass found no fault in the budget, the area state, the clock or the zone under working latches.
- Eleven of the fifteen findings above are about **user templates, probes and tests**, not about the generated logic for the six built-ins.
- User templates have produced findings in every round (F6, R4-4, T1–T5). Template Builder can write many shapes. A rule set that tries to handle every shape will keep producing edge cases.

### Astra's third report (verbatim)

## 1. VERDICT: NOT READY

The revised plan still permits duplicate starts and conflicting threshold outcomes under its explicitly supported delivery model. Template inspection remains ambiguous for shipped files, and the probes and acceptance tests cannot establish several claimed guarantees. These need correction before their implementation steps, although I found no type or derive defect that prevents **step 1a itself from starting**.

**Step 1a: Yes**, with the stipulated function bodies and compiling stubs; the interface gaps below must be resolved before their owning steps.

Read all 1,748 lines. No files changed or build commands run. **P** below means [handoff/P14-air-tasking.md](<C:/Machine Intelligence/Claude-IL2MissionUtility/handoff/P14-air-tasking.md>); “verified” means file or specified-model reasoning, never observed engine behavior.

## 2. STATUS OF SECOND-ROUND FINDINGS

| Finding | Status and evidence |
|---|---|
| R4-1 | **FIXED** — P1148 limits counter once-gates to single-use latches and adds the failure stop. |
| R4-2 | **FIXED** — P514–528 arms WAITING and DENY together before issuing the request. |
| R4-3 | **PARTLY FIXED** — P1114–1116 adds the missing cells; P1159 still overinterprets T3, and 0d omits E9 failure handling. |
| R4-4 | **PARTLY FIXED** — P237,262,272 repairs the examples, but shared-waypoint ownership and event-owned completion remain defective. |
| R4-5 | **PARTLY FIXED** — P147 distinguishes release from deletion, but P1371’s replacement bound of 2N is still false. |
| R4-6 | **PARTLY FIXED** — P520 disables FIRE, but P516 makes FIRE a running timer, allowing its pending output under RunsOn. |
| R4-7 | **PARTLY FIXED** — P390 renews orders at extension entry; earlier DONE does not suppress renewal, and P1619 retains the old instruction. |
| R4-8 | **PARTLY FIXED** — P1362 and P1467 correct timings; T-k’s required outcome at P1388 still contradicts depth-first delivery. |

## 3. NEW FINDINGS

**R5-1 — High — First blocked step: 3 — One copy can START twice.**

**Plan:** §3.2/3.7, P286–293,484–486; supported semantics P1280; fallback P1148. Source: plan only.

At **100.000**, clock GO and zone FIRE for the same `Both` entry both encounter active COPY 1 and START under `Batched`. Both deliveries pass before their deactivations take effect. With `Twice`, the same copy schedules bring-up and HARD STOP twice, while contributing only one FLYING flag.

This is distinct from the accepted overshoot involving different copies. The q fallback protects DONE HUB and ON STATION IN, but leaves START unprotected.

**Fix:** require once-only admission. Replacing START with `Counter=1, Dropcount=0` adds no MCU, **provided** probes establish once-only output, dropping inactive requests, and no reset when a spent counter is reactivated. Otherwise this delivery combination needs a stop condition.

**Confidence: 0.98. Verified under the specified model; engine manifestation is a hypothesis.**

**R5-2 — High — First blocked step: 4 — Presence and timeout can switch both state sets off.**

**Plan:** §3.5, P411–430; invariant P399; test P1408. Source: plan only.

DWELL completes at **120**. The player’s first inside sample coincides with INNER CLOSE at **122**. Under `Batched`, both INNER and INNER CLOSE OUT initially remain enabled: PASS and FAIL propagate. Both ON switches fire, then **both OFF switches fire at 122.05**.

HOT and COLD are now both off. PASS also started HOLD while FAIL rearms ENTRY, creating overlapping cycles. A subsequent ON STATION pulse can reach neither ending branch.

**Fix:** timeout first disables INNER, then waits 0.05 seconds before pulsing the existing cancellable CLOSE OUT. Test exact boundary coincidence. The extra timer can be offset by combining ENTRY OFF with ARM’s latch Deactivate: `ENTRY ZONE → ARM → combined OFF(ENTRY ZONE, ARM)`, plus DWELL.

**Confidence: 0.98. Verified model counterexample; engine ordering is a hypothesis.**

**R5-3 — High — First blocked step: 2 — Template normalization still lacks a valid ownership rule.**

**Plan:** §3.1–3.2, P237,262–282; “The six built-ins pass” at P265.

Four current generator shapes contradict the rules:

- **Il-10 KimpoAttack and Yak AirfieldRaid:** the shared WP reaches two completion branches. Il-10 WP54 targets `[45,51]`; each branch reaches its own Mission Complete and the same end timer. P262 therefore finds two candidates. See [Il-10:1113](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_DPRK_Il10_KimpoAttack_2x4.Group:1113>), order branches at lines 401/473, and [Yak:865](<C:/Machine Intelligence/Claude-IL2MissionUtility/TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group:865>).
- **One-waypoint, event-ended attack:** BingoFuel → Mission Complete is supported by [template.rs:3124](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3124>) and tested at [7369](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:7369>). The generator suppresses the automatic completion edge. P271 deletes the event’s destination while P272 promises to preserve the non-counter event.
- **Partial RTB:** the generator permits RTB on only one of two leads ([template.rs:2704](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:2704>)). P237’s “add nothing” leaves the other lead without RTB.
- **AttackArea → TimeOnTarget → MissionComplete:** P262 chooses TimeOnTarget as the on-target node. A 180-second existing dwell followed by the new 600-second ON STATION produces 780 seconds after WP arrival. The generator’s corresponding wiring is demonstrated at [template.rs:7279](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:7279>).

**Fix:** select attack branches through command Objects ownership; distinguish attack initiation from completion; preserve event-owned end requests through DONE HUB; replace terminal duration machinery deliberately; add RTB only for uncovered leads. Add generated-template tests for these cases.

**Confidence: 0.99. Verified from files.**

**R5-4 — High — First blocked step: 0b — G0/G1 still lack sufficient observation and decision rules.**

**Plan:** step 0, P1114–1116,1134–1159,1204. Source: plan only.

- P1159 says the “trace log decides” J4/E8 when E7 fails. But T3 tests **two different breadcrumbs**, not two simultaneous firings of **one breadcrumb**. A carrier that coalesces repeated firings of one identity would falsely appear to prove a working latch.
- E9 tests the required 0.05-second census spacing, but 0d has no E9 failure branch. E7 failure still selects Spaced.
- “C2 again” failure stops step 4 at P1114, whereas 0d permits Closer0Race when initial c fails and z succeeds. T-z tests only one empty-zone activation, not reusable occupied/empty checks.

**Fix:** add same-carrier multiplicity calibration, explicit E9 handling, and repeated checks for the selected presence implementation. Failed spacing should select a confirmed alternative or counter-free N=1. Unresolved latch evidence must remain unresolved; it cannot justify the guarantees in R5-1/2.

**Confidence: 0.99. Verified coverage and decision-table gaps.**

**R5-5 — Medium — First blocked step: 0T — Required interfaces cannot express their operations.**

**Plan:** §5 P637,774–783; step 0T P1174–1181.

`TraceSelect` has only block types, prefixes and indexes, yet instrumentation requires “event type the selection names.” T-g selects five event types, and the type-4 test requires distinguishing them. `TraceEntry` also lacks event/report identity. The AST explicitly stores event Type independently from TarId ([template.rs:3822](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3822>)).

Separately, `budget::build_side(..., &SideLogic, ...) -> Vec<Il2Entity>` receives immutable IDs but must modify previously emitted START, DONE and FLYING nodes. No tree argument or returned link patches provide that capability.

**Fix:** define typed event selection/identity and a random-timer predicate; give budget construction explicit mutable-tree access or return link patches and typed IDs. These can remain owner-module interfaces without blocking the section-5 type skeleton.

**Confidence: 0.99. Verified interface gaps.**

**R5-6 — Medium — First blocked step: 3 — Several revised acceptance claims remain false.**

**Plan:** steps 3/6, P1371,1380–1389,1468–1470. Source: plan only.

- **T-k:** under DepthFirst, the first request at t immediately shuts every START. The second at t and the request at t+.02 drop; the census finds one flight and reopens. **The t+5 request starts**, contradicting P1388. Space ordinary admissions beyond T_count; retain simultaneous admission testing in T-k1.
- **2N bound:** N=1, starts at **0,2,4**, surviving DONEs at **1,3**. Three flights coexist until the first deletion at **61.6**. P1371 expressly permits rapid scripted ends, so its universal 2N assertion cannot pass. Count flying flags separately from all living, undeleted aircraft.
- **Exhaustion:** CHANCE at100 starts FIRE; the clock starts the last copy at100.02 and disables FIRE. Under RunsOn, FIRE still outputs at100.05. Exhausted COPY relays prevent another launch, but “sends no request” is false. Test the no-launch guarantee, or add an output guard if suppressing the request itself is required.
- P1468 still calls **0.65** FIRE time; it is CHANCE time. FIRE is **0.70**, with refusal at1.70.

T-k1 should also end **one** overshooting copy first and verify that N=1 stays shut.

**Confidence: 1.00 for specification contradictions; the RunsOn manifestation is conditional on that engine outcome.**

**R5-7 — Medium — First blocked step: 3 — Renewed orders survive an earlier DONE.**

**Plan:** §3.2/3.4, P307,390; active contradiction P1619. Source: plan only.

Suppose ON STATION expires at **900.5**, the area remains HOT from its last check, and Zone Out causes DONE at **900**. Cleanup begins, but ON STATION still reaches HOT at900.5 and reissues AttackArea during RTB. DONE HUB OFF disables only the hub and FLYING.

**Fix:** suppress renewed attack orders after DONE, using existing deactivation targets where possible, and test both pending-timer cancellation modes. Also replace P1619’s active instruction that **EXTEND** re-pulses the orders with the corrected extension-entry behavior.

**Confidence: 0.98. Competing command paths verified; their in-game precedence is a hypothesis.**

## 4. SKELETON TABLE

Owner-local declarations missing from §5 are distinguished from actual frozen-interface defects.

| Item | Problem | Fix |
|---|---|---|
| Exact `airtask/mod.rs` types, P648–841 | No invalid derive or Default dependency found. AirContext deliberately has no Default. | No type correction required. |
| `weighted_random_pct`; library/timing re-exports, P665–666,837 | The printed weighted body is a placeholder; re-exports need the explicitly required public stubs. | Supply mandated body and stub imports. |
| Stub arguments/re-exports | Literal stubs introduce unused warnings; `allow(dead_code)` does not suppress them, conflicting with P1043. | Narrow temporary scaffolding allowances or consume arguments. |
| `MapAirPack`, P646 | Bare declaration lacks traits required when added to `FrontOptions`, which derives Clone/Debug ([frontlines.rs:366](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/frontlines.rs:366>)). | Derive Clone/Debug; Il2Entity supports both ([ast.rs:48](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/ast.rs:48>)). |
| `mapnet::is_rtb_waypoint`, P237,263 | Private at [mapnet.rs:328](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/mapnet.rs:328>); omitted from visibility edits and allowed files. | Assign a `pub(crate)` change before step 2. |
| `SortieShape`, `inspect_sortie`, P251,634 | Shape fields and cross-module visibility unspecified. | Define in library: ownership, navigation lead, discovered nodes and optional parts. |
| `SortiePlacement`, `PlacedSortie`, `place_air_sortie`, P218–220 | Declared in §3, absent from exact §5 source. | Intended step-2 owner-module additions. |
| `ShellSpec`, `ShellIds`, `wrap_sortie`, P249 | Incomplete field/ID contract; return type cannot express P281’s refusal. | Define fields and `Result<ShellIds, …>` before step 3. |
| `SideCopies`, `assemble_side`, `emit_side_logic`, P638 | Return structures/signatures remain prose. | Specify copy trees, entry mapping and exported hook IDs. |
| `SideLogic` / `budget::build_side`, P637,774 | Missing writable wiring channel. | Explicit tree access or returned patches; see R5-5. |
| Threshold/clock/zone builder interfaces | Modules exist, callable interfaces are not specified. | Define in owning modules before steps 4–6. |
| `clock::validate_schedule`, P1449 | Not in frozen source. | Intended step-5 function; parent-visible access suffices. |
| `F80_CRUISE_OVERRIDE_KMH`; theatre rows, P642,873–880 | Constant and row schema not in exact source. | Intended timing/theatre module additions in step 1. |
| `trace`, `missionlog`, `baseline_tests`, P1171,1232 | Outside §5’s module tree. | Already explicitly assigned to main’s 1a module declarations. |
| `TraceSource`, `ObjectiveStyle` | Named without concrete definitions. | Define compiling owner-module types; finalize fields before 0T. |
| `TraceSelect`, `TraceEntry` | Missing event selection/identity and Random predicate. | R5-5. |
| Other trace API | `TraceCarrier`, `TraceMap`, `TraceEdge`; `instrument`, `breadcrumb`, `push`, `decision_points`, sidecar writer are outside §5; some signatures remain prose. | Explicit 1a stubs, completed in 0T. |
| Mission-log API | `MissionLog`, `Record`, `Replay`; parsers, replay, report and CLI signatures are outside §5. | Explicit 1a stubs, completed in 0T. |
| Walker/probe/test helpers | Delivery/retrigger/counter modes, probe generators/writers, baseline writers and test-plan helpers are not production skeleton declarations. | Intended test-only additions in their named steps. |
| Export fields, P1016,1021 | `FrontOptions.air_packs`, `ImportedBaseMap.air_tasking_found` absent. | Explicit step-7 ownership. |
| UI declarations, P899–969,1531–1534 | AppMode/Confirm/MapDock/HelpTopic variants; app plan/undo/draw state; MapForces plan; draw and replay helpers absent. | Explicit later UI additions, not 1a defects. |
| Declared items unused across the plan | None demonstrably unused. ComplexTrigger and counter-reset variants are deliberately reserved for step 12. | Reconcile deferred variants with step 9’s blanket removal of dead-code allowances. |

## 5. MCU COUNT TABLE

Counts include implied switches and count check zones as MCUs.

| Cell | Stated count | My count | Difference |
|---|---:|---:|---:|
| Shell/copy, P313 | 16 + RTB waypoint/lead | Same | 0 |
| Shell retaining existing RTB | 15 | 15 | 0 |
| Budget/side, N≥2, P360 | 16 | 16 | 0 |
| Budget/side, N=1 | 14 | 14 | 0 |
| Threshold/area, P439 | 23, including 2 zones | Same | 0 |
| Clock entry, P501 | 13 | 13 | 0 |
| Last waterfall entry | 12 | 12 | 0 |
| Zone entry, P541 | 18 | 18 | 0 |

The shell count includes the assembly-owned COPY relay. Assembly also adds **one EXHAUSTED relay per entry**, outside these counts; totals must include it.

Further savings: make EXHAUSTED itself one Deactivate containing both front ends’ target union: **−1 per Clock/Zone entry, −2 per Both entry**. Combining threshold ENTRY OFF with ARM’s latch Deactivate saves **one per area**, offsetting R5-2’s boundary delay.

## 6. AREAS FOUND SOUND

- With working latches and atomic depth-first fan-out, reopening spent START nodes is harmless; closed COPY relays prevent reuse.
- Under the stated walker rules, shutting START does not cancel downstream bring-up or HARD STOP already initiated.
- MARK/CLEAR/AGAIN conservatively handles clustered ends; N=1 stays shut while any overshooting copy remains flying.
- The revised zone handshake fixes the reported permanent strand; I found no independent strand under functioning latches.
- The repeat-mode orphan sweep removes the respawn machinery after the specified cuts; current generator end roots do not retain it ([template.rs:3058](<C:/Machine Intelligence/Claude-IL2MissionUtility/src/template.rs:3058>)).
- Ordinary Activate/Spawn lookup and second-inbound-end refusal match current generator wiring.
- Default no-player missions remain COLD; clock attempt spacing and period validation prevent overlapping slots.