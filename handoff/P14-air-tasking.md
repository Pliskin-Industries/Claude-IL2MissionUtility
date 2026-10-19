# P14: Air Tasking (historical air sorties against Map objectives)

Status 2026-09-27. **Fully planned, nothing built.** Revised after review rounds 1 to 3, the trace and replay extension (U12, step 0T), and **round 4**: three Fable + Astra reviews (`handoff/P14-review-fable-astra.md`) and the user decisions U13–U23. §11 lists the changes. This file is the whole brief for the build. **Astra builds it through the Codex broker, one step at a time; Fable scopes each step, reviews each diff and re-runs the tests (U19, §10).** A build session reads this file and the repo, and nothing else from the planning conversation.

Repo root: `C:\Machine Intelligence\Claude-IL2MissionUtility`. All paths below are relative to it.

**Code anchors.** Every anchor is given by item or function name first. Line numbers are approximate, for `main` @ `bf1a356`. `main` is now `a34a00e`, which includes the creator's upstream commit `c1d88c3` (U15). That commit changed the line numbers in `src/template.rs` (about 43 lower from `generate_template` onward), `src/flights.rs`, `src/model_spec.rs` and `src/ui.rs` (about 35 lower from `readiness_checks` onward). So find each anchor by its name and use the number only as a starting point.

**Design rule (U13).** Every MCU costs tick budget in the game. Where two designs do the same job, the plan uses the one with fewer MCUs and fewer check zones. Each builder's MCU count is stated in §3 and pinned by a test, so a rise is a deliberate change.

Sizes: S = under a day, M = a few days, L = larger.

Related items: R3 F1 (linked-flight respawn), F3 (break off after losses), F4 (EXT START / EXT DONE hooks), F5 (in-theatre dates), F7 (waypoint speed); R4 P9 (logic checker), P11 (historical order of battle). P14 builds the parts of F4 and F5 it needs, in a form those items can reuse later.

---

## 1. Goal and scope

**Goal.** In Map mode, turn the historical air templates in `TemplateExamples/Historical1950/` into air sorties that attack Map objectives on their own during the mission. This works the way Map mode already places ground units against objectives (`recon::snap_placed_attack_areas`, `recon.rs` ≈824).

There are six built-in templates, all Template Builder output:

| File | Side | Planes | Leads (entity index) | T_attack (largest AttackArea `Time`) |
|---|---|---|---|---|
| `1950_US_F80_AirAlert_4ship.Group` | NATO | 4 × F-80C | 7 | 1080 s |
| `1950_US_F80_HVAR_Strike_4ship.Group` | NATO | 4 × F-80C | 7 | 600 s |
| `1950_US_F51_CloseSupport_4ship.Group` | NATO | 4 × F-51D | 7 | 900 s |
| `1950_US_F51_ArmedRecon_2plus2.Group` | NATO | 2 + 2 × F-51D | 7, 11 (escort, no WP 1) | 1200 s |
| `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` | DPRK | 2 + 2 × Yak-9P | 7, 11 | 300 s |
| `1950_DPRK_Il10_KimpoAttack_2x4.Group` | DPRK | 2 × 4 × Il-10 | 7, 15 | 600 s |

(Entity indexes, times and WP data were read from the files; step 2 pins them in tests. WP 1 altitude / speed: AirAlert 3050 m / 660, HVAR 3050 / 660, CloseSupport 1500 / 520, ArmedRecon 900 / 520, Yak 3050 / 500, Il-10 1500 / 390 km/h.)

**v1 includes:**
- A shared back end:
  - `place_air_sortie` moves and rotates a whole template onto a target.
  - A sortie shell gives each placed copy one START in and one DONE out.
  - An air budget allows at most N active sorties per side. It is a head count (§3.3, U20).
  - A player-threshold trigger for one player (N = 1, §3.5).
  - One engaged extension, decided from the target area's state (§3.4, U17).
  - Totals: `validate` reports the copies, aircraft, MCUs and check zones the plan adds, and warns above a limit (D36, U13).
- Front end A, the **mission clock** (primary). Slots come from a timer chain started at Mission Begin. Each slot draws from a weighted pool, with jitter, a quiet chance, no back-to-back repeat, and a reroll when the threshold check fails.
- Front end B, the **check zone**. A threshold zone at the target, with a chance to fire, a cooldown, a repeat limit, and drop-and-re-arm when the budget is full.
- One pool and one budget shared by both front ends.
- A 7th rail tab **Air Tasking** and a 5th Map dock tab **Air**.
- Date filtering from a new in-theatre table keyed on `model_spec` ids.
- Built-in library of the six files, plus user templates added from disk (U18). v1 accepts an Activate-mode user template only if it has one of the tested shapes S1–S4 or S7 (§3.2, U21, D39, decision 0017); every other template is refused with a message.
- Export inside Map's Generate Base Map, and as a standalone file from the Air Tasking tab.
- Step 0: generated in-game MCU probe missions that settle the risk register. All are flown **solo** (U14): flight 0 (automated cells and the trace test), flight 1A (NATO) and flight 1B (DPRK).
- Step 0T, built **before** the probe flights (U12): **trace builds** (breadcrumb MCUs that write a line to the server's text mission log when a chosen MCU fires, plus a `.trace.json` sidecar) and the **replay core** of R4 P10 (parse `missionReport*.txt`, timeline, fired / never fired, unobserved successors, templates that never spawned; a Markdown report from a CLI). The probe flights test both. The replay UI (Trace checkbox, report view, Map overlay) is step 9T, before the step 11 acceptance flight.

**v1 explicitly excludes:**
- A threshold of two or more players (N ≥ 2), and every test that needs more than one player. They are step 12, which starts after step 11 is confirmed (U16). The designs are kept in §3.9.
- Takeoffs from airfields. v1 uses air starts only. The airfield database (`src/harvest.rs`) is not populated yet.
- Any external server helper, or anything that reads logs while the mission runs. The user ruled this out because it cannot run alongside the dedicated server. (Step 0T's replay is different: it reads the server's text logs in the app **after** a session, and nothing runs next to the server.)
- A budget scaled by server population. The budget is a fixed per-side setting.
- Reconstructing the plan from a loaded `.Group`. v1 recognises and skips a loaded Air Tasking group and warns (§7).
- Saving the plan between app runs. Map objectives are preview-only today (`USER_MANUAL.md` ≈467), so the plan follows the same rule.
- Changing `frontlines::aircraft_ids` (the La-11 / MiG-15 date errors). That stays with R3 F5.
- Fixing the AirAlert template's two AttackAreas firing at once (§3.1, D28).
- Multi-leg routes. v1 accepts one path waypoint per lead (D26).
- Terrain clearance. Altitudes come from the template file.

---

## 2. Decisions

"user, 2026-09-24" = decided by the user. "Claude default, confirmed by user" = the architecture agreed in the design discussion. **"Claude default"** = set by Claude to close an open point. Each has a rationale, and each is a setting or a constant the user can change after seeing it in game. None of these are open questions: implement them as written. The ones that change or extend what the user said are also listed in §10 "For the user to confirm".

### 2.1 User decisions

| # | Decision | Source |
|---|---|---|
| U1 | Player threshold is **Option A**: in-mission only, built from Complex Trigger + Counter + window W + dwell check. There is no external helper, because it cannot run alongside the server. v1 builds the one-player path (a check zone and the dwell check); the Complex Trigger, the counter and the window are step 12 (U16). | user, 2026-09-24 |
| U2 | The budget is **not** scaled by player population. It is a fixed setting per side. | user, 2026-09-24 |
| U3 | Window W and dwell time come from the time an **F-80 takes to fly through the zone at cruising speed**. | user, 2026-09-24 |
| U4 | Threshold N is decided later. It must be a setting with a sensible default (see D1). v1 fixes it at 1 (U16). | user, 2026-09-24 |
| U5 | Slot timing is a **timer chain counted from Mission Begin**, not `MCU_DateTime`. | user, 2026-09-24 |
| U6 | If a clock slot's chosen entry fails the threshold, reroll among the other pool entries whose areas pass. If none pass, the slot stays quiet. | user, 2026-09-24 |
| U7 | "Hard stop while players still engaged: if N players are still within the inner zone at the stop, allow ONE extension; after that delete regardless." How the build approximates this is in §3.4 (D31, D32, U17). | user, 2026-09-24 |
| U8 | If a zone fires while the budget is full, drop the trigger and re-arm after the cooldown. | user, 2026-09-24 |
| U9 | UI: a rail tab plus a Map dock tab. The user originally asked for "an air force tab like the Army tab". | user, 2026-09-24 |
| U10 | v1 uses air starts. Takeoffs from airfields are a later phase. | user, 2026-09-24 |
| U11 | Nothing is pushed or merged without the user. (The first half of U11, a separate Opus ultracode session as the builder, is replaced by U19.) | user, 2026-09-24 |
| U12 | Build **trace builds** (breadcrumbs in the server's text mission log) and the **P10 replay** core **before** the probe flights, and use the flights to test that they work too (step 0T, cell T-t). | user, 2026-09-25 |
| U13 | **MCU economy.** Every MCU costs tick budget, and the game's logic is poorly optimised. Choose the cleanest design with the fewest MCUs, not the smallest change to an earlier draft. | user, 2026-09-27 |
| U14 | All in-game testing is solo where possible. A short solo "flight 0" comes first. | user, 2026-09-27 |
| U15 | Merge the creator's upstream commit `c1d88c3` in full before the baselines are recorded. (Done: `main` @ `a34a00e`, 469 tests pass.) | user, 2026-09-27 |
| U16 | Multi-player testing, and with it the threshold for two or more players, is planned for after the breadcrumbs and everything else are confirmed working (step 12). | user, 2026-09-27 |
| U17 | The extension is decided from the target area's state: extend once if a player was confirmed over the target at the last presence check. | user, 2026-09-27 |
| U18 | User templates stay in v1. The utility places the Mission Begin and links it to the templates; a template that is accurate and works is accepted. | user, 2026-09-27 |
| U19 | Astra builds, through the Codex broker. Fable scopes, reviews and integrates. | user, 2026-09-27 |
| U20 | The budget is a head count (§3.3). | user, 2026-09-27 |
| U21 | **User templates: tested shapes only.** v1 accepts a template only if its shape has a test, and refuses the rest with a clear message. | user, 2026-09-27 |
| U22 | **Hard limit on aircraft.** The budget is released when a sortie's aircraft are deleted, not when the sortie ends. It costs 1 MCU per copy. | user, 2026-09-27 |
| U23 | No further full review before the build. Fable re-reads each step's section when it scopes that step for Astra. | user, 2026-09-27 |
| U24 | **Try one check zone per area (§10 item 22), provisional.** Revisit after testing (the triggers are in §10 item 22). §3.5 is rewritten when step 4 is scoped. | user, 2026-09-27 |

### 2.2 Agreed architecture

| # | Decision | Source |
|---|---|---|
| A1 | Shared back end (§3.1–3.6) with two front ends (§3.7, §3.8). | Claude default, confirmed by user |
| A2 | Front end A (clock) is primary. Front end B (zone) is secondary. Pool entries are tagged `Clock`, `Zone` or `Both`. | Claude default, confirmed by user |
| A3 | The sortie shell guarantees DONE exactly once, on the first of: mission complete (end of on-station time, after at most one extension), all aircraft killed, Zone Out, or hard stop. DONE then deletes what is left and reopens the budget. | Claude default, confirmed by user |
| A4 | The budget is a head count of per-copy flying flags, recounted at every launch and every end (§3.3). It replaces the level counter of earlier drafts, which followed the Fighter Pack NodeGates pattern generalised to N. | Claude default; changed by the user, 2026-09-27 (U20) |
| A5 | Role is detected from template orders: an air AttackArea = patrol / air alert, a ground AttackArea = strike / CAS, Cover = escort. | Claude default, confirmed by user |
| A6 | Existing Generate output stays byte-identical when the feature is unused. | Claude default, confirmed by user (also CLAUDE.md) |

### 2.3 Claude defaults (settings, changeable)

| # | Decision | Default | Rationale |
|---|---|---|---|
| D1 | Threshold N | **1**, fixed in v1 | Never leaves the sky empty for a lone player. The threshold uses a plain CheckZone path with no Complex Trigger (§3.5), so v1 does not depend on the least-known MCU (risk d). N ≥ 2 cannot be tested solo; it is step 12 (U16, §3.9). |
| D2 | Threshold radius R | **12 km**, one cell per (side, area), shared by both front ends | Inside the 10–16 km band for front end B. One cell per area halves the MCU count. |
| D3 | Inner (dwell) zone radius | R / 2 | Someone flying straight through has left the inner zone by the time the dwell ends. |
| D4 | W, dwell D, hold H | W = D = 2R / v_cruise rounded up to 10 s; H = 2W | U3. Formulas and numbers are in §4. |
| D5 | Budget per side | **2** (range 1–6) | The budget limits the flights whose aircraft exist on a side (D38). With 4-plane sorties a budget of 2 is 8 AI aircraft; two Il-10 sorties are 16. Keeps server load modest. |
| D6 | Clock: first slot, period, jitter, quiet chance | first slot 10 min; period **20 min**; 3 jitter steps of 3 min (0 / 3 / 6 min); quiet chance 20 % | A new F-80 flight was launched every 20 min [S9 p.101], reference line 26 area. The first slot at 10 min gives players time to reach the front. |
| D7 | Spawn distance | Clock: arrive in **M = 5 min** at the template's waypoint speed. Zone: **30 km** from the target. | Clock sorties must already be on the way. Zone sorties must be seen arriving (original proposal). |
| D8 | Hard stop | arrival time + on-station time + **10 min** margin + the extension time (**10 min**). It is final: it goes straight to the end of the sortie | A backstop for a late or stuck flight. The normal end is the end of on-station time, with at most one extension (D31). |
| D9 | "Mission complete" | One timer `SNjj ON STATION` per sortie, equal to T_attack, started when the navigation lead's attack timer fires (0.5 s after WP 1) | The shipped chain ends the sortie about 1 s after WP 1 (§3.1). The AttackArea completion report (type 2) is UNVERIFIED in game. |
| D10 | Once-only gates (latches) | Self-deactivating relay, not a Counter | "Self Deactivate" is an existing pattern. Same-tick double pulses are risk q (probe J4). The fallback is a once-gate made of an `MCU_Counter` with `Counter = 1`, `Dropcount 0` (probe E8). |
| D11 | Cancel pattern | Every cancellable delay is a timer followed by a 0 s `… OUT` relay. Cancelling means deactivating the OUT relay. The node that starts the next cycle re-activates the OUT relay first (§3.0). | Whether deactivating a *running* timer cancels it is UNVERIFIED. Deactivating a relay that has not yet been pulsed is the pattern `Close_Remaining_Output(s)` already relies on (`recon.rs` `build_randomizer`). If probe J1 shows a running timer is cancelled, the OUT relays can be dropped later; keep them in v1. |
| D12 | Re-running a sortie | **One stamped copy per run**, with a copy pointer per entry. Runs per entry: default **3**, range 1–6, for both front ends together. | A linked flight is forced to Activate mode and cannot respawn (R3 F1, `template.rs` `has_linked_wingmen`). The cap also limits the clock (§10 list). Each run is a full copy of the template, so runs drive the totals (D36). |
| D13 | RTB | Hand-insert one `RTB n` waypoint per lead at `mapfighters::rtb_ao_point` (`mapfighters.rs` ≈73). Set `DELAYED END ORDERS` to 60 s, as the generator does for RTB groups. | None of the files has RTB. Regenerating through `load_template` is lossy (R3 F2). |
| D14 | Approach heading | Arrow target: tail → tip. Objective target: from the side's rear point (`rtb_ao_point`) toward the target. Each entry can override it. | Sorties come from the friendly side. |
| D15 | Weights | 1–10, default 5 (equal). Entry order is shuffled with the plan seed (`AirPlan.seed`, §5). | The shuffle spreads the waterfall bias described in §3.7. |
| D16 | New entry mode | `Clock` | A2 |
| D17 | Clock slot and budget full | The slot is quiet, with no reroll. The entry is not marked as the last winner. | The budget is per side, so rerolling cannot help. |
| D18 | Zone chance and cooldown | 50 %, 15 min | Mid values. Both are settings. |
| D19 | Built-in library | `include_str!` of the six files. User templates via "Add template…" (Ctrl O on the tab). | The release exe is self-contained. The six files contain no `LCName` / `LCDesc`, so they need no locale sidecar (checked with grep). |
| D20 | Date data | New module `src/in_theatre.rs`, keyed on `model_spec` ids and seeded from the reference with a citation per row. R3 F5 and R4 P11 reuse it later. | F5 requires one table, not two. |
| D21 | Output | Map's Generate Base Map includes the air pack when the plan has at least one enabled, valid entry and "Include in base map" is ticked (it ticks itself when the first entry is added). The Air Tasking tab's "Generate File" writes the air pack alone. | Keeps the `every_tab_has_a_primary_button_with_its_shortcut` rule (`ui_tests.rs` ≈293), and lets the user regenerate air tasking without a new base map. |
| D22 | Loading a base map that contains Air Tasking | Recognise it by group name and skip it, so it is not imported as an army. Show a warning. Do not rebuild the plan. | See §7. |
| D23 | Names in new `.Group` groups | Use DPRK / NATO. | Only legacy groups keep "Eastern". `coalition_is_eastern` accepts names starting "DPRK" (`frontlines.rs` ≈1878-1889). |
| D24 | How each front end uses the threshold | Clock reads the area's HOT state. Zone reacts to the area's PASS event. | One cell serves both front ends (§3.5). |
| D25 | Budget timing | The budget fails shut: a request during a recount is dropped. The one way over budget is two requests in the same tick; the next recount keeps the gates shut until enough sorties have ended. No error can persist (§3.3). | The head count keeps no running total (U20). Earlier drafts accepted a race that left the level wrong for the rest of the mission (review finding C1). |
| D26 | User templates with more than one path waypoint per lead | Rejected in v1 with a clear message. | `park_path_waypoints` fills waypoints in index order (`mapnet.rs` ≈892). One target per lead is well defined. |
| D27 | Tab labels and shortcuts | Rail: "Air Tasking", Ctrl 7. Dock: "Air". No plain-7 Map tool; the Map tool keys stay 1–6. | Adding a plain 7 would panic `MAP_TOOLS[i]` (§6.1). |
| D28 | AirAlert's two AttackAreas firing together | Left as the file has them. Flagged for the in-game check. | Changing a curated template is out of scope. |
| D29 | Timeline display length | 3 h (display only). The clock loops until the mission ends. | Typical server rotation. |
| D30 | AI setting off the other side's threshold | Orange warning when a sortie's spawn → target → RTB path passes within R of an area that triggers the *other* side. If flight 0 shows that check zones count AI, the warning becomes an error (§8 step 0d, §10 item 15). | Known weakness of Option A (risk a). |
| D31 | When the extension is decided | Once, at the **end of on-station time**. The hard stop is final and never extends. | One decision point needs no latch. A flight that arrives very late is ended by the hard stop (§3.4). |
| D32 | Extension condition | The target area is HOT: a player was confirmed inside r_in at the area's last presence check, at most H + 2 s earlier (U17). | It costs 3 MCUs per copy (`HOT`, `COLD`, `EXTEND`) and no check zone. The fresh check of earlier drafts cost 15 MCUs and one check zone per copy (U13). Nothing in the mission can count N planes inside a zone. |
| D33 | Clock draw attempts | Fixed length A = L + 0.5 s. A COLD area excludes every entry on that area at once. A win ends the slot at once. | Overlapping attempts could give two winners in one slot (§3.7). |
| D34 | MiG-15 date | `mig15bis` stands in for the MiG-15 family from 1950-11-01 [S9 p.241]. The row notes that the bis variant itself appeared in Nov 1951 [S9 p.434]. La-9 has no `model_spec` id and gets no row. | `model_spec` has no plain MiG-15. The reference already uses `mig15bis` as the stand-in (reference ≈902). |
| D35 | Dock label fallback | If five dock tabs do not fit 288 px, "References" becomes "Refs" when the count has 3 digits. | One fallback, chosen now, so the fit test has a fixed expectation (§6.2). |
| D36 | Totals and limits | `airtask::totals` counts what the plan adds: copies, aircraft, MCUs and check zones. The Air Tasking tab shows them. `validate` warns (orange) above **96 aircraft, 1500 MCUs or 40 check zones**. | U13. The limits are estimates; set them from the Probe 2 load check (§10 item 18). |
| D37 | "Else" branches | Where the condition is a state that something else switches (an area's HOT / COLD), a consumer pulses a pair of relays, one in each set, and exactly one passes. A race timer (D11) is used only where the pulse itself causes the condition (zone refusal, chance roll, presence check). | A pair costs 2 MCUs and no delay. A race costs 3 or 4 MCUs and 1–2 s. |
| D38 | Release point | A copy leaves the head count when its aircraft are deleted: the template's delete timer switches the copy's flag off and starts a recount. The zone cooldown still starts at `DONE HUB`. | U22: a hard limit on the aircraft that exist. The second review showed that a release at `DONE HUB` gives no fixed limit, because ended flights fly home for 60 s while replacements start. Cost: 1 MCU per copy (`SNjj LANDED`). |
| D39 | User template acceptance | `library::inspect_sortie` finds the template's parts by wiring and accepts only Activate-mode S1–S4 and S7 (§3.2, decision 0017). Every refusal has its own message and its own test. The **navigation lead** is the first lead, in file order, that has a path waypoint. | U18, U21. Template Builder can write many shapes; three review rounds each found one that the rules mishandled. A short list of tested shapes ends that. A shape is added later by adding its test. |
| D40 | Cruise calibration | The measured F-80C cruise (cell T-n) is a P14 constant, `timing::F80_CRUISE_OVERRIDE_KMH: Option<f64>`. `model_spec` is not changed. | Changing `model_spec` would change Template Builder's suggested waypoint speed and so its output (review finding C10, CLAUDE.md). |
| D41 | Locale ids of user templates | P14 uses the existing first-wins merge (`locale.rs` `LocaleTable::merge`), as the Army Generator does. `validate` warns when two templates in the pool carry different text under one id. | Re-numbering locale ids is a separate feature. |

---

## 3. Architecture

### 3.0 Conventions

**Names.** Every generated MCU name uses these prefixes. `s` is the side letter, `D` or `N`, and numbers are two digits, 1-based, per side. Tests assert on these exact strings.

| Prefix | Meaning | Example |
|---|---|---|
| `AT s` | Per-side housekeeping | `AT N INIT`, `AT N INIT OFF` |
| `BUD s` | Budget (head count) | `BUD N CHECK`, `BUD N COUNT` |
| `CLK s` | Clock | `CLK N TICK`, `CLK N ATTEMPT END` |
| `TH s aa` → `THNaa` | Threshold cell for area aa | `THN02 PASS`, `THN02 HOT ON` |
| `ENii` / `EDii` | Pool entry ii | `EN05 OUT`, `EN05 COPY 2` |
| `ZNii` / `ZDii` | Zone trigger of entry ii | `ZN05 ARMED` |
| `SNjj` / `SDjj` | Placed sortie copy jj | `SN07 START`, `SN07 FLYING`, `SN07 DONE HUB` |

- **Relay** = `MCU_Timer` with `Time = 0` and `Random = 100`, used as a gate: it passes a pulse only while it is active. This is the Fighter Pack gate-cell idiom (`pack.rs` ≈54, all timers Time 0).
- **Set ON / set OFF** = one `MCU_Activate` / `MCU_Deactivate` whose MCU links list every relay in a named set. Use the same link field as `Close_Remaining_Output(s)` / `CloseInput` in `recon.rs` / `flights.rs` `build_randomizer`. The implementer checks whether that field is Targets or Objects and uses the same one.
- **Latch** = a relay that also targets a Deactivate of itself: the first pulse passes, later ones are dropped.
- **Pair (D37)** = two relays owned by one consumer, one in each of two sets of which exactly one is on. The consumer pulses both; one passes. This is the plan's "else" branch where the condition is a state.
- **Make-before-break.** A change between two such sets switches the new set on at once and the old set off 0.05 s later, so a pair never has both relays off.
- **Start states.** At Mission Begin + 0.5 s, `AT s INIT` → `AT s INIT OFF` (a Deactivate) switches off every relay that must start inactive. Everything else starts active. No reliance on an `Enabled` flag on timers.
- **Cancel pattern (D11).** `X` (timer, T s) → `X OUT` (relay) → consequences. Cancel = deactivate `X OUT`.
- **Re-arm rule.** Every `X OUT` that a success path deactivates is re-activated by the node that starts the next cycle, **before** `X` can fire again. The diagrams mark each re-arm edge with ⟲. A cell with a cancel but no ⟲ edge is used once per copy (the sortie shell) and says so. A test checks the rule (step 1b, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`).
- **Emit order.** When one node pulses several relays of a set (for example `BUD s CENSUS` → every `SNjj FLYING`), its Targets list them in ascending order. The walker uses this order; the game may not (risk q).
- **Cross-module wiring (`SideLogic`, §5).** Builders never edit an MCU another builder owns. Each side has one `SideLogic` value that every builder appends to:
  - `links`: pairs (from, to). A builder that needs "MCU `from`, which I do not own, must also target my MCU `to`" pushes the pair. `emit_side_logic` adds each `to` to the Targets of its `from`, across every tree of the side. Examples: `SNjj START` → `BUD s MARK`; the copy's delete timer → `BUD s CHECK`; `SNjj START` → `ENii LAST SET`; `SNjj DONE HUB` → `ZNii WAITING`; `ENii EXHAUSTED` → the relays each front end wants switched off;
  - `mission_begin`: ids to pulse from `Translator Mission Begin`;
  - `init_off`: relays that `AT s INIT OFF` switches off;
  - `hot_consumers` and `cold_consumers`: area → the `ENii HOT` / `SNjj HOT` and `ENii COLD` / `SNjj COLD` relays that the area's HOT and COLD sets list;
  - `pass_subscribers`: area → the `ZNii ARMED` relays that the area's `THsaa INNER` targets (the PASS event, §3.5);
  - `gates`, `flags`, `budget_events`: the side's `SNjj START` relays (each is its copy's gate), its `SNjj FLYING` timers, and the `START` relays and delete timers that must pulse `BUD s MARK` and `BUD s CHECK`.
- **Build order per side** (step 7, `build_air_packs`): `assemble_side` (place, wrap, copy pointer; registers each copy's pair, gate, flag and events) → clock (registers each `ENii` pair) → zone (registers `ZNii ARMED`) → threshold (reads the consumers and subscribers, so its lists are complete) → `budget::build_side` (reads `gates`, `flags` and `budget_events`) → `assemble::emit_side_logic`, which applies `links` and creates `Translator Mission Begin`, `AT s INIT` (0.5 s) and `AT s INIT OFF` from the collected lists. **Owner of `AT s *` and the side's `Translator Mission Begin`: step 3 (`assemble.rs`).**

**Groups in the output:**

```
Air Tasking                                  <- recognised by name prefix on Load (§7)
├─ Air Tasking DPRK Logic                    <- Translator Mission Begin, AT D *, BUD D *, CLK D *, THD** *, ED** *, ZD** *
├─ Air Tasking NATO Logic                    <- same for NATO
├─ Air Tasking DPRK Sorties
│   └─ SD01 1950_DPRK_Il10_KimpoAttack_2x4 #1     <- one placed template copy (its own Logic/Units/Orders/Waypoints kept)
└─ Air Tasking NATO Sorties
    └─ SN07 1950_US_F80_HVAR_Strike_4ship #2
```

The sortie shell MCUs `SNjj *`, the copy's `FLYING` flag included, live inside the copy's own `Logic` group. `BUD s *` live in the side's Logic group, in a subgroup `Budget`.

### 3.1 `place_air_sortie` (move + rotate a whole template)

What the templates are today (verified from the files and `template.rs`):
- The lead plane sits on `ORIGIN = (40000, 40000)` (`template.rs` ≈75-76). `WP 1` is 4 km north, at `(44000, alt, 40000)` (≈94, ≈2938-2958). So the lead → WP 1 bearing is 0°, where heading 0 = +X = north and 90 = +Z = east (`placement.rs` ≈224-232).
- **The ground AttackArea sits on the spawn point, not on WP 1** (`template.rs` ≈2815-2821). README step 3 is wrong about this.
- The air AttackArea sits at a canvas slot.
- Zone IN (16 km, Closer 1) and Zone Out (35 km, Closer 0) are centred on the spawn. Zone IN's Targets are its Self Deactivate, `Zone Out ReActivate`, `PULSE OUT` and the bring-up timer (HVAR: `[19, 20, 16, 25]`). Nothing else arms Zone Out.
- **The bring-up timer has two names.** Template Builder calls it `MISSION BEGIN` in Activate mode and `SPAWN UNITS` in Spawn mode (`template.rs` `bring_up_name`). All six built-ins are Activate mode. It is not the `Translator Mission Begin`, which is the `MCU_TR_MissionBegin` that starts the template. §3.2 finds every part by its wiring (U18).
- **Mission Complete fires about 1 s after WP 1.** `chain_delay_to` (`template.rs` ≈3369-3404) chains `AttackArea n` → `Mission Complete n` at the next order's 0.5 s delay. In the HVAR file: `WP 1(40) → AttackArea 1(37) → [AttackArea(38), Mission Complete 1(39)] → MISSION END(30)`. The flight is deleted about 3.6 s after WP 1, and the AttackArea `Time` never plays out. The wiring is VERIFIED from the files; how it plays in game is UNVERIFIED.
- **Timer names are not unique.** AirAlert has two `MCU_Timer`s named `AttackArea 1` (37 → [38, 39] and 39 → [40, 41 = Mission Complete 1]); WP 1 targets both. Never find an order timer by name. Find it by its edges.
- **Not every lead has a waypoint.** In Armed Recon, `WP 1` Objects = [7]. The escort lead 11 has only `Cover 3` (Cover cmd Objects [11], Targets [7]) and no `Mission Complete 3`.
- The Il-10 and Yak share one `WP 1` for both leads (Il-10: Objects [7, 15], Targets [45, 51]). The Il-10 order timers are numbered by seat (`AttackArea 1`, `AttackArea 5`).
- There is no RTB and there are no `OnEvents` in any of the six files.

New function in `src/airtask/place.rs`:

```rust
pub struct SortiePlacement { pub target: (f64, f64), pub heading_deg: f64, pub spawn_dist_m: f64, pub rtb: (f64, f64) }
pub fn place_air_sortie(root: &mut Il2Entity, p: &SortiePlacement) -> Result<PlacedSortie, String>
pub struct PlacedSortie { pub spawn: (f64, f64), pub leads: Vec<i32>, pub wp_speed_kmh: f64, pub attack_time_s: f64, pub planes: Vec<i32>, pub warnings: Vec<String> }
```

Every placed `Plane` is written with `AiRTBDecision = 0`, leaving the source template unchanged and rejecting a copy missing that key (decision 0015).

Steps, in this order:
1. **Find the leads.** A lead is a `Plane` whose linked `MCU_TR_Entity` has **empty Targets**, meaning it does not target another plane's entity. A wingman's entity targets its lead's entity (`template.rs` `generate_template` sets the follower's targets to `[lead_eid]`). Expected: HVAR leads = [7]; Il-10 = [7, 15]; Armed Recon = [7, 11]. The **first lead** is the one first in file order. The **navigation lead** is the first lead, in file order, that has a path waypoint (D39); in all six built-ins it is the first lead. Pivot = navigation lead XZ. Template bearing β = `heading_toward(navigation lead, its non-RTB MCU_Waypoint)`; a waypoint belongs to a lead when its Objects contain that lead's entity. `library::inspect_sortie` (§3.2) has already found the leads and refused a template that cannot be placed.
   - Reject a template in which no lead has a waypoint, or in which a lead has more than one non-RTB waypoint (D26). A lead with no waypoint (an escort) is allowed.
2. **Rotate the whole tree** by `θ = p.heading_deg − β` about the pivot. Add a new `placement::rotate_tree(root, pivot, θ)`:
   - Rotate every node that has `XPos`/`ZPos`: planes, entities, every MCU, waypoints, commands, checkzones, icons. Use the formula from `apply_group_heading` (`placement.rs` ≈116-134): `nx = px + dx·cos − dz·sin`, `nz = pz + dx·sin + dz·cos`.
   - Add θ to `YOri` only on the nodes `apply_group_heading` already turns (its own node list: Model nodes, Block and Ground objects, and linked entities; walk at ≈178-202). MCU `YOri` is left alone; whether it is ever read is UNVERIFIED (risk m).
   - Never touch `XOri`/`ZOri` or `YPos`.
   - Reuse `set_coord` / `add_yori` (`placement.rs` ≈204-222; made `pub(crate)` in the 1a skeleton), so decimals are written the same way.
3. **Translate.** `spawn = target − d·(cos h, sin h)`, then `placement::move_anchor_to(root, navigation lead, spawn)` (`placement.rs` ≈101). This keeps YPos, so file altitudes survive (`ast.rs` ≈221-227).
4. **Snap the waypoint.** `mapnet::park_path_waypoints(root, &vec![target; n_path_wps])` (`mapnet.rs` ≈892). It sets XZ only, so WP altitudes (3050 / 1500 / 900 m) are kept. A shared WP 1 (Il-10, Yak) is fine.
5. **Snap AttackAreas.**
   - Ground: `weapon_range::snap_ground_attack_areas(root, x, z)` (`weapon_range.rs` ≈274). **Required**, because it currently sits on the spawn point.
   - Air: new `weapon_range::snap_air_attack_areas`. Same code, with the predicate `AttackAir == 1`.
   - Cover and Formation have no world meaning, so they only rotate and translate.
6. **RTB (D13).** A lead that has its own RTB waypoint (`mapnet::is_rtb_waypoint`) **keeps it**: move it to `p.rtb` and keep its altitude and speed. Two RTB orders to one lead would compete (second review, R4-4). **A lead that has none gets one** (third review, T5; the generator allows RTB on some leads only): a new `MCU_Waypoint` `RTB n` at `p.rtb`, `Area 1000`, `Priority 2`, `Speed` = WP speed, Objects = [lead entity]. `YPos` = that lead's WP 1 altitude; a lead with no waypoint (the Armed Recon escort) uses its own plane's `YPos`. Copy the property set of the generator's RTB waypoints (`template.rs` RTB waypoint block). If the template has an RTB timer, the new waypoints are added to its Targets; if it has none, the first two sub-steps below add one.
   - Add a timer `RTB DELAY` (0.5 s) → all `RTB n`.
   - Add `MISSION END ORDERS` → `RTB DELAY`.
   - Set `DELAYED END ORDERS.Time` to 60 if it is lower.
   - `mapnet::is_rtb_waypoint` (`mapnet.rs` ≈328) skips names starting "RTB", so later waypoint parking leaves these alone.
7. **Return** the spawn point, leads, planes, WP `Speed` (km/h, read from the file) and on-station time T_attack = the largest `MCU_CMD_AttackArea` `Time` whose Objects contain any lead.
   - If a user template has `Speed ≤ 150` (R3 F7's 100 km/h default), use `model_spec::suggested_waypoint_speed_kmh` (`model_spec.rs` `suggested_waypoint_speed_kmh`) instead and push a warning. That function returns 100 when it knows none of the models. If the speed is still 150 or less, refuse the template with a message that names the models (D39).

Index merge: each placed copy goes through `duplicate::duplicate_template(copy, &mut next_id)` (`duplicate.rs` ≈101) before it is stamped.

### 3.2 Sortie shell (START in, DONE out exactly once)

New `src/airtask/shell.rs`: `wrap_sortie(root, prefix: &str, shape: &SortieShape, spec: &ShellSpec, next_id: &mut i32) -> Result<ShellIds, String>`. It is written generically so R3 F4 can later call it with the prefix `EXT`. `ShellSpec` carries T_attack, T_final, T_ext, the target, the copy's census delay (§3.3), and the `ZoneOutImpl` and `LossesImpl` strategies.

**Finding the template's parts (U18, D39).** Every part is found by its wiring, never by its name. `library::inspect_sortie(root) -> Result<SortieShape, Vec<String>>` (§5) does the lookup once, for loading, `validate` and generation alike:

| Part | How it is found | Template Builder's name |
|---|---|---|
| Zone IN | The `MCU_CheckZone` with `Closer = 1` that the template's `MCU_TR_MissionBegin` reaches through one timer | `Zone IN` |
| **Bring-up timer** | The `MCU_Timer` among Zone IN's Targets whose own Targets hold an `MCU_Activate` with plane entities in its Objects | `MISSION BEGIN` (Activate mode); a counter into a Spawner is refused (decision 0017) |
| Zone Out | The `MCU_CheckZone` with `Closer = 0` | `Zone Out` |
| Zone Out arming pair | The `MCU_Activate` and the `MCU_Timer` that target Zone Out | `Zone Out ReActivate`, `PULSE OUT` |
| **End timer** | The `MCU_Timer` among Zone Out's Targets that reaches an `MCU_CMD_ForceComplete` within two links | `MISSION END` |
| **Optional template Delete chain** | The `MCU_Timer` that targets the sole `MCU_Delete`; both are removed by the shell | `DELETE DELAY`, `Trigger Delete`; regenerated Activate templates have neither |
| Mission Complete timers | Every `MCU_Timer` that targets the end timer | `Mission Complete n` |
| **Attack timer of a lead** | The `MCU_Timer` that targets an `MCU_CMD_AttackArea` whose Objects hold that lead's entity, **and** targets a Mission Complete timer. A branch belongs to a lead through the Objects of its command, so two leads that share one waypoint are told apart (Il-10, Yak) | `AttackArea n` (in AirAlert the second of the two, timer 39) |
| Template RTB | Every `MCU_Waypoint` that `mapnet::is_rtb_waypoint` accepts, and the timer that targets them | `RTB n`, `RTB DELAY` |

**Accepted shapes (U21, D39).** v1 accepts Activate-mode templates with shapes S1–S4 or S7 only (decision 0017). Each shape has a test that builds it, wraps it and walks it (step 2 and step 3). Every other template is refused with a message that names what is not supported. This is a deliberate limit: Template Builder can write many shapes, and three review rounds each found a shape the rules mishandled.

| # | Shape | Example |
|---|---|---|
| S1 | One lead, Activate mode, one AttackArea | HVAR, CloseSupport |
| S2 | One lead, two AttackAreas in a chain | AirAlert |
| S3 | A strike lead and an escort lead with Cover and no waypoint, in either file order | ArmedRecon |
| S4 | Two leads that share one waypoint, each with its own AttackArea | Yak, Il-10 |
| S7 | Any of S1–S4 on Activate mode with its own RTB on every lead, or on some leads | written by `generate_template` in the test |

**Refused, each with its own message and test:**
- Spawn mode, Repeat off or on: "Air Tasking needs an Activate-mode template", followed by `REGENERATE_TEMPLATE`;
- a part of the table above is missing or is found twice;
- a unit that is not a plane;
- no lead has a path waypoint, or a lead has more than one (D26);
- the navigation lead has no attack timer (no AttackArea, or a step such as a time-on-target delay between the attack order and Mission Complete). Message: "Air Tasking needs the attack order to be the last order before Mission Complete";
- an event hook or a report hook (`OnEvents` / `OnReports`). Message: "Air Tasking does not support event or report hooks yet";
- a waypoint speed that stays at 150 km/h or less (§3.1 step 7).

**Stripped from the copy.** `recon::silence_clone_starts` (`recon.rs` ≈1681, made `pub(crate)`) sets `Enabled=0` and empties the Targets of the `MCU_TR_MissionBegin`. IL-2 still runs Mission Begin when `Enabled=0` (comment at `recon.rs` ≈1680). The side's own Mission Begin (§3.0) is the only one that starts anything.

Then the shell cuts the template's own start and end logic, in this order:
1. Keep Zone IN and replace its Targets with Zone Out's arming pair, cutting its old bring-up and self-deactivation links. Replace Zone Out's Targets with `[SNjj DONE HUB]` (below, decision 0018).
2. Delete every Mission Complete timer (the single `SNjj ON STATION` replaces them, D9).
3. Remove the optional template delete timer and `MCU_Delete`, and all links to them. The shell writes its own chain whether or not the input had one. Accepted templates have no event or report hooks; the shell attaches only type 13 into `LOSSES`.
4. **Orphan sweep.** Delete every logic MCU (`MCU_Timer`, `MCU_Activate`, `MCU_Deactivate`, `MCU_Counter`, `MCU_ModifierSetVal`, `MCU_CheckZone`) that no pulse link or state link reaches from one of the roots: the bring-up timer, every waypoint, Zone IN, Zone Out, its arming pair and the end timer. Repeat the sweep until nothing more is deleted, then drop every link to a deleted MCU.
   - For the six built-ins the sweep removes `ENABLE / PULSE IN`, both `Self Deactivate`, `Zone In ReActivate` and `COOLDOWN`. Zone IN stays.

**Kept and rewired:**
- Zone Out is re-centred on the target (radius unchanged, 35 km) and its Targets are replaced with `[SNjj DONE HUB]`.
- Zone IN is kept under its template name, like Zone Out. It is re-centred on the target with Zone Out's 35 km radius; `Closer = 1` and `PlaneCoalitions` stay unchanged. Its only Targets are Zone Out's Activate and 0.1 s pulse timer.
- **The navigation lead's `WP 1` pulses Zone IN**, which arms Zone Out only after an enemy is seen (decision 0018). Zone IN is absent from `SideLogic.init_off`, and nothing in the copy deactivates it. WP 1 no longer targets the arming pair.
- **After the cut, the end timer has exactly one inbound link, from `SNjj DONE HUB`.** If another link into it is left, `wrap_sortie` returns an error and the template is refused.
- **ON STATION starts from the navigation lead's attack timer.** Its edge to the Mission Complete timer is replaced by → `SNjj ON STATION IN` (latch) → `SNjj ON STATION`. Other leads' attack timers keep their AttackArea commands and simply lose their Mission Complete target. Only the navigation lead's timer feeds ON STATION, so a shared WP 1 cannot send two same-tick pulses into the latch (risk q). That timer itself can be pulsed twice: in AirAlert, `WP 1` targets both 37 and 39, and 37 (0.5 s) also targets 39, so 39 is pulsed at WP 1 and again 0.5 s later. The `ON STATION IN` latch passes only the first of those, so `ON STATION` starts exactly once per sortie under every re-trigger behaviour (risk p). The template's own edges are left as they are (D28). An attack timer has `Time = 0.5`, so HVAR starts `ON STATION` 0.5 s after WP 1. AirAlert starts it 0.5 to 1.0 s after WP 1 across the three walker re-trigger modes: Restart can postpone timer 39 to +1.0 s. Its two AttackAreas still supply 1080 s of on-station time.
- **The shell owns `SNjj DELETE DELAY` (0.5 s) and `SNjj DELETE`.** `DELAYED END ORDERS` targets the new delay in place of the optional template chain. The delay targets `SNjj DELETE` and `SNjj LANDED`; its registration in `SideLogic.budget_events` adds `BUD s MARK` and `BUD s CHECK` through the budget builder. Delete Objects list every plane entity, including escorts (flight 1P, decision 0017). The copy leaves the budget in the tick its aircraft are deleted.

```
 ENii COPY r (copy pointer, §3.7) ─► SNjj START
 SNjj START (relay; it is also the copy's gate: it is in the side's gate set and is open while the budget has room, §3.3)
   ├─► bring-up timer (0.5 s) ─► Activate Units, AFTER BRING UP ─► Formation n ─► Goto WP 1 ─► WP 1
   ├─► SNjj HARD STOP (timer, T_final) ────────────────────────────────────────────────────────────────────►┐
   ├─► SNjj STARTED ON  (MCU_Activate: SNjj FLYING, and ENii COPY r+1 if there is one)                       │
   ├─► SNjj STARTED OFF (MCU_Deactivate: ENii COPY r)                                                        │
   ├─► BUD s MARK, BUD s CHECK (§3.3)                                                                        │
   ├─► ENii LAST SET (clock entries only, §3.7), ZNii GRANTED (zone entries only, §3.8)                      │
   └─► (last copy only) ENii EXHAUSTED                                                                       │
                                                                                                             │
 WP 1 (navigation lead's) ─► attack timers (unchanged) ─► AttackArea cmd (on target)                         │
                          └─► Zone IN (enemy seen) ─► Zone Out arming pair ─► Zone Out                      │
 navigation lead's attack timer ─► SNjj ON STATION IN (latch) ─► SNjj ON STATION (timer, T_attack)           │
                                                                                                             │
 End of on-station time (D31, D32):                                                                          │
   SNjj ON STATION ─► SNjj HOT  (relay in the area's HOT set, §3.5)  ─► SNjj EXTEND (timer, T_ext) ─────────►│
                   └► SNjj COLD (relay in the area's COLD set, §3.5) ──────────────────────────────────────►│
 Other DONE causes:                                                                                          │
   each Plane entity OnEvent Type 13 ► SNjj LOSSES (Counter = planes, Dropcount 0 = fire once, stay) ──────►│
   Zone Out (Closer 0, 35 km, at target; armed only after Zone IN fires) ──────────────────────────────────►│
                                                                                                             ▼
 SNjj DONE HUB (latch; active from mission start; used once)
   ├─► SNjj DONE HUB OFF (MCU_Deactivate: SNjj DONE HUB)
   ├─► ZNii WAITING (if the entry has a zone trigger, §3.8)
   └─► end timer ─► MISSION END ORDERS (0.05 s) ─► Force Complete - High, RTB DELAY ─► RTB n
                 └► DELAYED END ORDERS (60 s) ─► Deactivate Units, SNjj DELETE DELAY (0.5 s)
 SNjj DELETE DELAY ─► SNjj DELETE                                        <- the aircraft are gone
              ├► SNjj LANDED (MCU_Deactivate: SNjj FLYING)               <- the copy leaves the head count (D38)
              └► BUD s MARK, BUD s CHECK (§3.3)                          <- and the side recounts
```

**MCUs the shell adds per copy: 19, plus one `RTB n` waypoint per lead that has none.** They are `START`, `HARD STOP`, `STARTED ON`, `STARTED OFF`, `ON STATION IN`, `ON STATION IN OFF`, `ON STATION`, `HOT`, `COLD`, `EXTEND`, `LOSSES`, `DONE HUB`, `DONE HUB OFF`, `FLYING`, `LANDED`, the new `DELETE DELAY` and `DELETE`, `RTB DELAY`, and the assembly's `ENii COPY r` relay. `wrap_sortie` itself adds 17; placement and assembly provide the last two. A template with its own RTB timer keeps it and receives 18, plus any missing RTB waypoints. The HVAR fixture removes 8 template MCUs; the generated S7 fixture without a Delete chain removes 7. Keeping Zone IN adds no new MCU: it removes one fewer. HVAR holds 43 MCUs (42 without Zone IN), and S7 holds 47 (46 without Zone IN), including the fixture copy pointer and missing RTB waypoint. Every wrapped built-in has two check zones, both at the target with the same radius.

Notes:
- **Empty targets stay on station.** Zone IN waits for an enemy; only after it fires can Zone Out end the sortie. An empty target reaches COLD at the end of on-station time or extends once through HOT. AI aircraft also count as seen. Keeping Zone IN means 48 check zones per side in the 24-copy load file, 96 total (decision 0018).
- **One decision point.** Only `ON STATION` reaches the `HOT` / `COLD` pair, and `ON STATION` starts once, so the extension needs no latch. `HARD STOP` goes straight to `DONE HUB`.
- **`START` closes itself in the tick it passes a pulse** (`START` → `BUD s CHECK` → `BUD s SHUT`). This is the latch idiom: a relay whose own output switches it off still fires all its targets. Probe J4 tests a latch, and risk q covers it.
- **HOT and COLD are never both off** (§3.5, make-before-break), so the end-of-station pulse always reaches `DONE HUB` or `EXTEND`. For 0.05 s at each change both are on. A pulse in that window reaches both: `COLD` ends the sortie at once and the later `EXTEND` pulse finds the hub closed. That is accepted.
- **Two moments, two names.** "DONE" is the `DONE HUB` pulse: the sortie stops attacking and turns for home, and the zone cooldown starts. "Deleted" is `SNjj DELETE DELAY`'s pulse, about 60.6 s later (`MISSION END` 0.1 s + `DELAYED END ORDERS` 60 s + `SNjj DELETE DELAY` 0.5 s): the aircraft are gone and the copy leaves the head count (D38). Tests and the step 11 checks name which of the two they measure.
- A copy is used for one run only (D12). `DONE HUB` never needs re-arming.
- Delete does not raise event 13 in the flown controls (flights 1G/1P). Any late loss event reaches a closed hub.
- **Dropcount convention.** The codebase uses `Dropcount = 1` for a counter that resets after firing and can fire again (`template.rs` `SpawnCount` `drop = i32::from(repeat)`, `DeathCount` 1 in repeat mode; `flights.rs` `DeathCount` 1), and `Dropcount = 0` for one that fires once and stays. `LOSSES` (used once per copy) is `Dropcount 0`. `BUD s COUNT` (§3.3) fires at every full recount, so it is `Dropcount 1`.
- `SNjj LOSSES` needs `attach_event(entity, 13, losses_idx)` (type 13, not 4: flight 1G showed that a lost aircraft raises 13 once and never 4; decision 0011) (`template.rs` `attach_event`) on each plane's `MCU_TR_Entity`. The escort lead's planes count too. No other OnEvent or OnReport is written: no event-7 abort and no 4/2/0 latch (decisions 0011/0015/0017). A crash without damage remains for flight 2; the hard stop still ends the sortie.
- **Spawn-mode templates (risk u; former shapes S5/S6) are refused.** Activation after respawn can retain a broken entity association (flight 1L V2, decision 0013). Decision 0017 keeps Air Tasking on Activate mode; flight 2 has no Spawn sortie.
- The end timer is no longer reachable from anywhere except `DONE HUB`. A test asserts this.

### 3.3 Air budget: a head count (at most N active per side; U20)

**Why not a running total.** Earlier drafts kept a level per side: add one at each launch, subtract one at each end. Each change took 0.1 s. A second end that arrived inside that 0.1 s found no active relay and was lost, and the level stayed one too high for the rest of the mission (review finding C1). Two sorties on one objective share a Zone Out, so one player leaving or dying ends both in the same tick. The fault was the normal case, not a rare one.

**The head count keeps no total.** The truth is one flag per copy, and only that copy switches its own flag. The side counts the flags again after every launch and every deletion.

**Per copy** (inside the copy's own `Logic` group):
- `SNjj START` is the copy's **gate**: it is a relay in the side's gate set, and a request passes it only while the gate is open (§3.2). The budget adds no gate MCU of its own.
- `SNjj FLYING`: an `MCU_Timer` with `Time = 0.05·q` s, where q is the copy's 1-based order on its side. It is active from `START` until the copy's aircraft are deleted (`SNjj LANDED`, §3.2), and inactive otherwise (it is in `SideLogic.init_off`).

**Per side** (subgroup `Budget` of the side's Logic group). An **event** is a pulse from any `SNjj START`, from any copy's `SNjj DELETE DELAY`, or from `BUD s AGAIN`. Each of them targets `BUD s MARK` and `BUD s CHECK`:

```
 event ─► BUD s MARK (MCU_Activate: BUD s AGAIN)
       └► BUD s CHECK (latch; BUD s SHUT closes it)

 BUD s CHECK                                                                       <- one recount, never re-triggered while it runs
   ├─► BUD s SHUT (MCU_Deactivate: every SNjj START, and BUD s CHECK)              <- fails shut, at once
   ├─► BUD s REOPEN ARM (MCU_Activate: BUD s REOPEN OUT)                           ⟲ re-arm, first
   ├─► BUD s ZERO (MCU_ModifierSetVal, ParamIndex 0, Data0 0; target BUD s COUNT)
   ├─► BUD s CLEAR (timer 0.02 s) ─► BUD s CLEAR OFF (MCU_Deactivate: BUD s AGAIN)
   ├─► BUD s CENSUS (timer 0.05 s) ─► every SNjj FLYING (only the active ones answer, 0.05 s apart)
   │        SNjj FLYING ─► BUD s COUNT (MCU_Counter: Counter = N, Dropcount 1) ─► BUD s STAY SHUT (MCU_Deactivate: BUD s REOPEN OUT)
   └─► BUD s END (timer T_count)
          ├─► BUD s REOPEN OUT (relay) ─► BUD s OPEN (MCU_Activate: every SNjj START)      <- fewer than N are flying
          ├─► BUD s CHECK ON (MCU_Activate: BUD s CHECK)                                   ⟲ re-arm
          └─► BUD s AGAIN DELAY (timer 0.05 s) ─► BUD s AGAIN (relay) ─► an event          <- only if an event came during the recount
```

- `T_count = 0.05 + 0.05·C + 0.10` s, where C is the number of copies on the side. Example: 9 copies give 0.6 s.
- **Start state.** Every gate is open. `FLYING`, `AGAIN` and nothing else of the budget are in `SideLogic.init_off`. No recount runs at Mission Begin.
- **`BUD s AGAIN`** makes sure no event is missed. `MARK` switches it on at every event. The recount that the event starts switches it off 0.02 s later, before the flags are read at 0.05 s. An event that arrives later in the recount switches it on again, and the end of the recount then starts one more recount.
- **`BUD s OPEN` also re-activates the `START` relay of a copy that has already flown.** That does no harm: a request reaches a `START` only through its `ENii COPY r` relay, and that relay is off for good once the copy has started.
- **N = 1 needs no counter.** When a side's budget is 1, every `SNjj FLYING` targets `BUD s STAY SHUT` directly, and `ZERO` and `COUNT` are left out. The builder does this whether or not counters work in game.
- **The links into another builder's MCUs** (each `START` and each `SNjj DELETE DELAY` → `MARK` and `CHECK`) are not written by editing those MCUs. The budget builder pushes them to `SideLogic.links` (§3.0).
- **16 MCUs per side for N ≥ 2, 14 for N = 1, and 2 per copy** (`FLYING`, `LANDED`). Earlier drafts used 6 + 2N per copy.

**Worked cases.**
1. *Launch with room.* Budget 2, one sortie flying. A request passes `SN07 START`, which pulses `MARK` and `CHECK`. The gates shut at once. The census finds two flags, `COUNT` fires at the second, and the gates stay shut.
2. *Two sorties are deleted in one tick.* Both `LANDED` switches take their own flags off, and both `SNjj DELETE DELAY` timers pulse `CHECK`. The first pulse starts a recount and closes `CHECK`; the second is dropped. The census runs 0.05 s later and finds no flag. The gates reopen at `T_count`.
3. *A deletion during a recount.* The flag goes off, but the census may already have read it. `AGAIN` is on, so a second recount starts 0.05 s after the first ends and reads the true state.
4. *A request during a recount.* The gates are shut, so the request is dropped. A clock slot stays quiet (D17). A zone trigger starts its cooldown (U8, §3.8).

**Properties** (each is a walker test, step 3):
- No state outlives a recount, so no error can persist. When nothing is flying, a recount always reopens the gates.
- It fails shut. While a recount runs, no launch is possible, so a recount never reads fewer flags than are flying.
- **What the budget counts: aircraft that exist (U22, D38).** A copy is counted from `START` until its aircraft are deleted, about 60.6 s after `DONE HUB`. So the budget is a hard limit on the flights whose aircraft exist on a side. A sortie that is flying home still holds its place.
- **The one way to go over budget** is two or more requests passing open gates in the same tick; each of them starts. The clock sends one request per slot, and zone entries on one area fire `T_count + 0.2` s apart (§3.8), so it needs unrelated triggers (the clock and a zone, or zones on different areas) in one tick. The next recount then finds more than N flags: `COUNT` fires at N and the gates stay shut until enough copies are deleted. The error heals itself.
- **One copy can be started twice in one tick (accepted; third review, T10).** If the clock and the zone pick the same `Both` entry in the same tick, both pulses can pass the copy's `START` before its pointer moves. The copy then gets its bring-up order and its hard stop twice. It still has one flag, so the head count stays right, and the `DONE HUB` latch still ends it once. It needs two unrelated triggers in one 0.02 s tick. The manual states it.
- A doubled event pulse (risk q) only starts a recount, or is dropped by the `CHECK` latch. It cannot leave the gates shut when nothing is flying.

**What it relies on.** All of it is behaviour the shipped Template Builder and Fighter Pack already use, and probe cell T-e tests each line:
- a counter counts pulses and fires at its `Counter` value (E1);
- a `Dropcount 1` counter fires again (E2);
- `MCU_ModifierSetVal` clears a partial count (E4);
- new: pulses 0.05 s apart are counted one by one. Cell E7 tests whether pulses in the *same* tick are also counted one by one. If they are, `CensusImpl::SameTick` sets every `FLYING` `Time` to 0 and `T_count` to 0.2 s.

**Fallback (`BudgetImpl::SingleSlot`).** If E1, E2 or E4 fails, the budget is fixed at 1 per side and needs no counter: every `SNjj FLYING` targets `BUD s STAY SHUT` directly. The UI then shows the budget as 1 with the explanation "Counters are not confirmed in game".

### 3.4 Hard stop and extension (U7, U17)

- `T_final = t_arrive + T_attack + margin + T_ext`, with margin 10 min and `T_ext` 10 min (D8). `SNjj HARD STOP` runs from `START` and goes straight to `DONE HUB`. It is final and never extends.
- The extension is decided once, at the end of on-station time (D31): if the target area is HOT, the sortie stays `T_ext` longer; if it is COLD, the sortie ends now (D32).
- **HOT** means a player was confirmed inside r_in at the area's last presence check. With the defaults that check is at most 242 s old (H + the 2 s check window, §3.5).
- **Deviation from U7, agreed by the user (U17).** U7 asks for N players inside the inner zone at the stop. The build reads the area's state, which says that at least one enemy plane was inside r_in at the last check. This costs 3 MCUs per copy and no check zone. The fresh check of earlier drafts cost 15 MCUs and one check zone per copy.
- A flight that reaches its target more than `margin + T_ext` late is ended by the hard stop before its on-station time is over.
- The extension adds time only. It gives no new order: the AttackArea `Time` has run out by then. What the aircraft do in those minutes is UNVERIFIED (risk v). Probe 2 (step 3) observes it. If they leave the area, the shell gets one more relay per copy, `SNjj RENEW`: `SNjj HOT` → `SNjj RENEW` → the navigation lead's attack timer, so the attack order is given again **when the extension starts**. `SNjj DONE HUB OFF` also switches `RENEW` off, so a sortie that has already ended gets no new attack order (third review, T14). That change is made in `shell.rs` after the probe, and only if the probe calls for it.
- The UI labels it "Extend once if a player was over the target in the last few minutes."

### 3.5 Player-threshold cell (per side × area; N = 1 in v1)

**As built (step 4, 2026-10-18; decisions 0023, 0024 and 0025, flight 2J).** This section replaced the two-check-zone cell of earlier drafts. Details, the review and the flight: `handoff/sessions/2026-10-18-p14-step4.md`; code and tests: `src/airtask/threshold.rs`.

- An **area** is a target point: an objective, or an attack-arrow tip. The cell for side s watches the **other** side's **players**: `THNaa` (NATO sorties) watches country 501, `THDaa` watches 601. The app's own AI flies 602 / 503 (decision 0008; the rewrite of placed copies is step 7).
- **Entry:** one `MCU_TR_ComplexTrigger` per area (`THsaa TRIGGER`, radius r_in, filtered to the players' country, the flown 1N shape). Its Object Entered closes the `ARM` latch, which switches the trigger off and starts `DWELL` (D).
- **Presence check:** a check zone at r_in (`THsaa INNER`, Closer 1, the other side's coalition), switched on by `CHECK ON` and pulsed 0.1 s later by `INNER PULSE` (flight 1A's c). A switched-on Complex Trigger does **not** report a plane already inside (flight 2J), so the trigger is used for the entry only.
- **PASS** (`INNER` fires): `PASS OFF`, HOT on at once, COLD off 0.05 s later, `HOLD` (H) re-checks while HOT, and the area's `pass_subscribers`. **FAIL** (`CLOSE DELAY`, 0.05 s after the 2 s `CLOSE` switched the zone off): COLD on, HOT off 0.05 s later, `REARM ON` re-opens `ARM` and the trigger. A pass and a timeout in one tick give one outcome only.
- **State:** HOT and COLD sets of consumer relays (§3.0 pairs, make-before-break); COLD starts on, the HOT set is in `init_off`. Every poll in flight 2J passed exactly one relay of a pair.
- **Timing (U24):** D = F-80 transit of r_in rounded up to 10 s (60 s at R = 12 km), H = 2D (120 s); `timing::derived_timing`.
- **19 MCUs, 1 Complex Trigger and 1 check zone per (side, area).**
- Known weaknesses: an enemy-coalition AI plane inside r_in at a check passes like a player (but the app's AI alone never starts a cycle); a plane already inside when the mission loads is not seen until it re-enters (unflown); a straight pass slower than about 720 km/h through the centre passes; HOT stays up to H after the player leaves; a mission maker's own AI on 601 or 501 counts.

### 3.6 Weighted re-armable randomizer

New `airtask::weighted_random_pct(weights: &[u32]) -> Vec<u8>` in `airtask/mod.rs` (step 1): `p_i = round(100 · w_i / Σ_{j≥i} w_j)`, with the last entry at 100. The existing `recon::random_pct` (`recon.rs` ≈463) is equal-odds only.

The structure copies the re-armable `flights::build_randomizer` (`flights.rs` ≈918-1004):
- `Randomizer:INPUT` → `CloseInput`, `Wait for Output`, and every Random timer. Random k (k = 1..n, 1-based) has `Time = 0.5·k` s, the same times as `build_randomizer`'s `0.5·(i+1)` with 0-based i, so the last output comes at 0.5·n = L − 0.5 s.
- `Wait for Output` (fires at L = 0.5·(n+1) s) → `ReOpen Outputs` and re-opens INPUT, a clean 0.5 s after the last output, never in the same tick. A DRAW that arrives before L is dropped, so attempts must be at least L long (§3.7).

Differences: names carry the `CLK s` prefix, and exclusion acts on `ENii RANDOM` (not on the Outs), so `ReOpen Outputs` does not undo it.

### 3.7 Front end A: mission clock (per side)

**As built (step 5, 2026-10-18; decision 0029, option B).** Same behaviour as the original design below, with a smaller inside, because flight 0 settled what the original hedged against (f: a timer's Random % is rolled per trigger; J1: deactivating a running timer cancels it; J3: an Activate and a pulse in one tick drop the pulse; J4: a relay latch drops a same-tick double). Code and tests: `src/airtask/clock.rs`; scope, diagram and test list: `handoff/sessions/2026-10-18-p14-step5.md`.

- **Quiet roll and jitter are J random timers into a latch.** `TICK` → `SLOT ON` (re-arms the `SLOT OPEN` latch) and `JITTER k` (`Time` 0.1 + (k−1)·step, `Random` p_k = `weighted_random_pct([100−q; J] ++ [q·J])[k]`, 27 / 36 / 57 % at the defaults) → `SLOT OPEN`. The first that passes opens the slot; about q % of slots stay quiet. No `QUIET ROLL`, no jitter randomizer, no `JITTER k DELAY`.
- **`ATTEMPT END` is cancelled directly** by `WIN` (no `ATTEMPT ARM`, no `ATTEMPT END OUT`); `REROLL ON` re-arms it at `SLOT OPEN`.
- **The entry randomizer is `INPUT` → `ReOpen Outputs` and the `RANDOM` timers**; attempts never overlap, so there is no `CloseInput` or `Wait for Output`. A = L + 0.5 s and the slot window min(300, (a + 10)·A) are unchanged.
- **`DRAW` is the 0.3 s timer** and pulses `LAST CLEAR`; the accepted exception below is 0.3 s, not 0.2 s.
- **`ENii LAST SET` is one Activate.** The next slot excludes every entry started since the last slot opened, not only the most recent. Step 6 measures whether that matters on a busy area against the alternative (B′); the bar is half a launch per 3-hour mission (0029).
- **`ENii GO` → `CLK s WIN`, then the `COPY r` pointers last run first**, so a pointer activated by the previous copy's `START` cannot take `GO`'s pulse in the same tick.
- **No `Seeded` variant; `RandomImpl` is removed.**
- **MCUs: 11 per clock entry (10 for the last in the waterfall), 17 + J + a per side** (20 + a at J = 3). The three-entry, three-area test plan has 55 (the original design: 76).
- The entry order is shuffled per side from `AirPlan.seed` by `clock::waterfall_order` (SplitMix64); step 6 uses the same order for its stagger. `clock::validate_schedule(side, schedule, entries)` holds the schedule checks and the two §3.7 rules below.

**Original design (2026-09-27),** kept for its reasoning; where it differs from the list above, the list above is what is built.

```
 Translator Mission Begin ─► CLK s FIRST (timer, first slot) ─► CLK s TICK (relay)
 CLK s TICK ─► CLK s PERIOD (timer, P) ─► CLK s TICK          (loop; jitter is downstream so it never accumulates)
            └► CLK s QUIET ROLL (0.1 s, Random = 100 − quiet%) ─► CLK s JITTER Randomizer:INPUT
 CLK s JITTER k (equal-odds waterfall, k = 1..J) ─► CLK s JITTER k DELAY (timer (k−1)·step) ─► CLK s SLOT OPEN

 CLK s SLOT OPEN (relay)
   ├─► every ENii AVAILABLE (relay; OFF once the entry's copies are used up) ─► ENii RANDOM ON (Activate ENii RANDOM)
   ├─► CLK s NO REPEAT (0.1 s) ─► every ENii LAST (relay; only the last winner's is ON) ─► ENii EXCLUDE (Deactivate ENii RANDOM)
   ├─► CLK s LAST CLEAR (0.2 s) ─► Deactivate every ENii LAST       (after the exclusion; a quiet slot leaves no LAST set)
   ├─► CLK s REROLL ON (Activate CLK s REROLL)                                                   ⟲ re-arm
   ├─► CLK s SLOT CLOSE (timer = slot window) ─► CLK s REROLL OFF (Deactivate CLK s REROLL)
   └─► CLK s DRAW DELAY (0.3 s) ─► CLK s DRAW

 CLK s DRAW (relay)
   ├─► CLK s ATTEMPT ARM (Activate CLK s ATTEMPT END OUT)                                        ⟲ re-arm, first
   ├─► CLK s Randomizer:INPUT ─► ENii RANDOM p% (0.5·k s, k = waterfall position 1..n) ─► ENii OUT ─► (closer: later ENxx OUT)
   └─► CLK s ATTEMPT END (timer A = L + 0.5 s) ─► CLK s ATTEMPT END OUT ─► CLK s REROLL
 ENii OUT ─► ENii HOT  (relay in the area's HOT set, §3.5)  ─► ENii GO
          └► ENii COLD (relay in the area's COLD set, §3.5) ─► CLK s AREA aa OUT (Deactivate ENxx RANDOM for every entry on area aa)
 CLK s REROLL (relay) ─► CLK s DRAW                                      (only while the slot window is open)
 ENii GO ─► CLK s WIN (MCU_Deactivate: CLK s ATTEMPT END OUT, CLK s REROLL)   (the slot ends at once; one per side)
         └► every ENii COPY r ─► SNjj START (the copy's gate, §3.3)      (the copy pointer, below)
 SNjj START (a copy of clock entry ii) ─► ENii LAST SET: Deactivate every ENxx LAST except ENii; Activate ENii LAST
 ENii EXHAUSTED (MCU_Deactivate, from the copy pointer): ENii AVAILABLE, ENii RANDOM, ENii GO  (the clock adds these targets through SideLogic.links)

 Copy pointer per pool entry (built by assemble_side, step 3; shared by both front ends):
 ENii COPY r (one-hot relays, r = 1..runs; only r = 1 starts ON) ─► SNjj START    (copy r of this entry; START is the copy's gate)
 SNjj START (copy r of entry ii) ─► SNjj STARTED OFF (Deactivate ENii COPY r), SNjj STARTED ON (Activate ENii COPY r+1)
                                 └► (last copy) ENii EXHAUSTED (one MCU_Deactivate per entry; clock and zone each add their targets to it)
```

- **The HOT / COLD pair replaces the MISS race of earlier drafts (D37).** `ENii OUT` pulses both relays and exactly one passes. A miss excludes the area at once, with no 1 s timer. This saves 2 MCUs per entry and makes each attempt 1 s shorter.
- **Attempts are fixed length and do not overlap (D33).** Every attempt ends at `ATTEMPT END` (A = L + 0.5 s after DRAW): the last `OUT` comes by L − 0.5 s (RANDOM k at 0.5·k s) and the randomizer INPUT re-opens at L. Only `ATTEMPT END` starts the next DRAW, so there is never more than one attempt in flight. A win cancels `ATTEMPT END OUT` and switches `REROLL` off, so the slot sends exactly one request.
- **A failed area is excluded once.** `ENii COLD` deactivates the RANDOM relay of every entry on that area (`CLK s AREA aa OUT`), because they share one state. So at most (distinct areas) + 1 attempts end with a winner or an exclusion. **Nobody-won attempts are not bounded:** once the last waterfall entry is excluded (as LAST, or by an area miss), an attempt can end with no OUT at all, and each such attempt is a random event. Example: 3 equal-weight entries on 3 areas, #3 excluded as LAST, only #1's area HOT: #1 wins about 1/3 of attempts, so a window of 6 attempts leaves about 9 % of such slots quiet although #1 passes.
- **Slot window** = `min(300 s, (a + 10)·A)`, where a = number of distinct areas among the side's clock-eligible entries, L = `0.5·(n+1)` s for n clock-eligible entries, A = L + 0.5 s. After it closes, `REROLL` is off and no new attempt starts (U6). The +10 is sized for the example above: 13 attempts leave about 0.5 % of those slots quiet ((2/3)^13). With many entries the 300 s cap can bind, and the quiet rate with a passing entry rises. This deviation from U6 is §10 item 12.
- **The attempt in flight at SLOT CLOSE runs to its end** and can still win (review finding C6). So a slot is busy for up to `slot window + A` seconds after it opens.
- `validate` gives an **error** when `period ≤ (J − 1)·jitter_step + slot window + A + 10 s` (one slot's last attempt would run into the next slot), and a **warning** when `(a + 10)·A > 300 s` (some areas may go untried, and nobody-won attempts may use up the window). Both checks are `clock::validate_schedule` (step 5), which `validate` calls (step 7).
- **Distribution caveat.** When entry k is excluded, the waterfall passes k's share to the entries after k. The shuffled order (D15) spreads that bias. If the last entry is excluded, "nobody won" is possible; that attempt excludes nothing and `ATTEMPT END` rerolls, so a slot can close quiet while a passing entry exists (see "Slot window"). Document this in the manual.
- **No repeat.** The last entry that **flew** is excluded for one slot only. `LAST SET` comes from the copy's `START`, not from `GO`, so an entry whose request the budget refused is not excluded from the next slot. `NO REPEAT` (0.1 s) excludes the last one, then `LAST CLEAR` (0.2 s) switches every `LAST` off, before `DRAW` (0.3 s) can produce a new winner. A quiet slot therefore sets no `LAST`, and the entry is eligible again in the slot after. One exception is accepted: a zone launch of a `Both` entry inside the first 0.2 s of a slot sets its `LAST` before `LAST CLEAR`, which clears it, so that entry is not excluded from the next slot. If its area is the only HOT one, the slots alternate win / quiet (§10 list; walker test `clock_only_one_area_hot_alternates_with_quiet`). A `Both` entry launched by its zone trigger also sets its `LAST`.
- **Exhaustion.** The last copy's `START` (from either front end) pulses `ENii EXHAUSTED`. It is one `MCU_Deactivate` per entry, and it switches off `ENii RANDOM`, `AVAILABLE` and `GO` (and the zone's relays, §3.8). So a used-up entry cannot win a later slot and waste it, and a RANDOM timer that was already running when the last copy started cannot end the slot (second review, R4-6).
- **Copy pointer ownership.** `ENii COPY r`, the `START` → advance edges and `ENii EXHAUSTED` are built by `assemble_side` (step 3) for every pool entry that has copies, `Zone`-only entries included, and their ids are returned in `SideCopies`. The clock (step 5) adds `GO → every COPY r`, and pushes `START → LAST SET` and its `EXHAUSTED` targets to `SideLogic.links`. The zone (step 6) adds `FIRE → every COPY r`, and pushes `START → GRANTED`, `DONE HUB → WAITING` and its `EXHAUSTED` targets. Neither front end creates or edits the other's MCUs, or the shell's.
- **A request needs no answer.** If the gate is shut, the pulse is dropped and the slot stays quiet (D17). There is no REQ, GRANTED or DENIED relay per copy.
- **Starting states** (`SideLogic.init_off`): `ENii LAST`, `CLK s REROLL` (clock); `ENii COPY r` for r ≥ 2 (assemble).
- **MCUs: 12 per clock entry** (11 for the last one in the waterfall, which has no closer): `AVAILABLE`, `RANDOM ON`, `LAST`, `EXCLUDE`, `RANDOM`, `OUT`, its closer, `HOT`, `COLD`, `GO` and the two `LAST SET` switches. `ENii EXHAUSTED` is one more MCU per entry, owned by `assemble_side` and shared by both front ends. One `CLK s WIN` Deactivate per side serves every `GO`. The per-side count is about 40 with three jitter steps, plus one `AREA aa OUT` per area; step 5 pins the exact number.
- `QUIET ROLL`, `JITTER` and `RANDOM` rely on the Timer `Random` % being rolled each time the timer is triggered (risk f, confirmed in flight 0; decision 0029 removed `RandomImpl` and the `Seeded` variant). The original builder took `RandomImpl::{InMission, Seeded}`:
  - `InMission` (primary) is the diagram above.
  - `Seeded` (fallback if f fails) makes every choice at Generate time from the plan seed (`AirPlan.seed`). The clock is unrolled into S = ceil(6 h / period) slots. Each slot gets a fixed jitter step, a fixed quiet flag, and an ordered entry list (a weighted sample without replacement over the clock-eligible entries). In the mission, a slot tries its list in order through the same `ENii HOT` / `ENii COLD` pairs: the first HOT, available, not-LAST entry wins, and "reroll" means the next entry in the list (U6). After slot S the clock stops. The zone chance becomes a fixed per-zone sequence of 8 fire/skip outcomes, stepped by a one-hot pointer at each arming and cycled.

### 3.8 Front end B: check zone (per entry tagged Zone or Both)

```
 THsaa INNER (the PASS event, §3.5) ─► ZNii ARMED (relay; ON = armed)
 ZNii ARMED ─► ZNii ARMED ARM (MCU_Activate: ZNii MISS WAIT OUT)                              ⟲ re-arm, first
            ├► ZNii DISARM (MCU_Deactivate: ZNii ARMED)
            ├► ZNii CHANCE (timer 0.1 s + stagger, Random = chance%)
            └► ZNii MISS WAIT (timer 1 s + stagger) ─► ZNii MISS WAIT OUT (relay) ─► ZNii COOLDOWN     (the chance roll failed)
 ZNii CHANCE ─► ZNii FIRE ARM (MCU_Activate: ZNii WAITING, ZNii DENY WAIT OUT)                ⟲ re-arm, first
             ├► ZNii FIRE OFF (MCU_Deactivate: ZNii MISS WAIT OUT)
             └► ZNii FIRE (timer 0.05 s)                          <- so WAITING and DENY WAIT OUT are on before the request goes out
 ZNii FIRE ─► every ENii COPY r (the copy pointer, §3.7) ─► SNjj START                                  (the budget has room)
           └► ZNii DENY WAIT (timer 1 s) ─► ZNii DENY WAIT OUT (relay) ─► ZNii WAITING                 (no copy started: U8)
 SNjj START (every copy of entry ii) ─► ZNii GRANTED (MCU_Deactivate: ZNii DENY WAIT OUT)
 ENii EXHAUSTED (MCU_Deactivate, §3.7): ZNii REARM, ZNii ARMED, ZNii FIRE                     (the zone adds these targets through SideLogic.links)
 SNjj DONE HUB (every copy of entry ii) ─► ZNii WAITING (relay, starts OFF)
                                             ├─► ZNii WAITING OFF (MCU_Deactivate: ZNii WAITING)
                                             └─► ZNii COOLDOWN
 ZNii COOLDOWN (timer C) ─► ZNii REARM (relay; OFF after the last copy has started) ─► ZNii ARM (MCU_Activate: ZNii ARMED)
```

- **`WAITING` and `DENY WAIT OUT` are armed when the chance roll succeeds, not when the zone is armed** (second review, R4-2). Armed earlier, a clock launch of the same `Both` entry could cancel `DENY WAIT OUT` before the zone had fired. A later refusal then started no cooldown, and the zone never re-armed.
- If a clock launch of the same entry cancels `DENY WAIT OUT` after `FIRE ARM`, `WAITING` is already on, so that copy's `DONE HUB` starts the cooldown. The zone cannot be left without a way back.
- **Exhaustion also switches off `ZNii ARMED` and `ZNii FIRE`** (R4-6), so an entry whose last copy the clock has started **starts no further copy**. If `FIRE` was already running, it can still give its output (when a running timer is not cancelled by a Deactivate), but every `ENii COPY r` is off by then, so the pulse goes nowhere; the refusal timer then starts the cooldown, and `REARM` is off.

- The threshold cell's `PASS` keeps firing every H seconds while players stay. So after the cooldown, a still-busy area offers the trigger again.
- **Stagger (review finding C13).** `PASS` reaches every zone entry on the area in the same tick. Entry number p on its area (0-based, in the shuffled order of D15) adds `p·(T_count + 0.2)` s to its `CHANCE` and `MISS WAIT` timers, with `T_count` from §3.3. So each request arrives after the recount that the request before it caused, and each is judged against the true budget.
- **Refusal is a race timer here, not a HOT / COLD style pair.** A started copy itself shuts the gates in the same tick, so a "budget full" relay read at that moment could pass for a request that was granted. The 1 s `DENY WAIT`, cancelled by the copy's `START`, has no such fault. It costs 3 MCUs per zone entry, not per copy.
- `ZNii WAITING` lets only a refusal or a `DONE HUB` that follows a zone FIRE start the cooldown. A clock copy's end does not restart the cooldown of an idle zone.
- The cooldown starts at `DONE HUB`, when the sortie turns for home. The budget is released about 60 s later, when the aircraft are deleted (D38).
- **Residual edge case, accepted and documented (§10 list):** for a `Both` entry, if the zone has fired and a clock-launched copy of the same entry ends first, the zone starts its cooldown early and can re-arm while its own copy is still flying.
- Zone sorties use the zone spawn distance (30 km, D7). Copies are shared by both front ends, so each copy is placed once. Rule: copies of a `Zone` or `Both` entry are placed with the **zone** distance, because that is the tighter case. Copies of a `Clock` entry use the clock distance.
- "Max repeats" = the entry's run count (D12).
- Starting states (`SideLogic.init_off`, switched off by `AT s INIT OFF`): `ZNii WAITING`.
- The zone builder registers each `ZNii ARMED` in `SideLogic.pass_subscribers` for its area; the threshold builder (run after it) makes `THsaa INNER` target them.
- **MCUs: 17 per zone entry**, plus the entry's shared `ENii EXHAUSTED` (§3.7).

### 3.9 Designs for step 12 (two or more players; not built in v1)

U16 moves everything that needs a second player to step 12. The designs are kept here so step 12 starts from them. Nothing in this section is built, tested or emitted before step 12.

**Entry path for N ≥ 2 (Complex Trigger).** It replaces `THsaa ENTRY ZONE` and feeds the same `THsaa ARM`:

```
 THsaa ENTRY (MCU_TR_ComplexTrigger: radius R, planes only, other side's countries, event "entered alive")
   └─► THsaa COUNT (MCU_Counter: Counter = N, Dropcount 0 = fire once, stay until ZERO) ─► THsaa ARM
 Mission Begin ─► THsaa WINDOW (timer W) ─► THsaa ZERO (MCU_ModifierSetVal, ParamIndex 0, Data0 0; target COUNT)
                                        └─► THsaa WINDOW                      (loop; the loop contains a timer, so P9-safe)
 THsaa REARM also targets THsaa ZERO                                          (a fresh count after a failed dwell)
```

- It needs the user's editor reference file (12a) and a flight with two players (cell T-d).
- `THsaa COUNT` needs a **fired** `Dropcount 0` counter to fire again after `ZERO` resets it. Nothing in the codebase does that today. Probe cell T-e E6 tests it in flight 0, so the answer is known before step 12 (risk e). If E6 fails, `CounterResetImpl::Ring` is the fallback: K counters, and a timer every W/K deactivates the oldest and activates one that has not yet fired. A ring counter is used once, so the ring needs `ceil(mission length / (W/K))` counters; step 12 sizes it.
- Known weaknesses: the window is a fixed bucket, not a true sliding window; a re-entry at the zone edge counts twice; a player who leaves still counts until the next reset; keeping an area HOT only needs one plane present after the first pass.
- The extension condition stays as in §3.4 (the area's state).

**Zone Out with several players (risk b).** Whether a `Closer 0` zone fires when ALL watched planes have left or when ANY one leaves cannot be tested solo. Cell T-b (two NATO players) is in step 12. v1 uses `ZoneOutImpl::CheckZone`, the same zone every shipped template uses. If T-b shows ANY-out, step 12 switches to `ZoneOutImpl::Watchdog`, which needs probe result p = "restart" (J5).

---

## 4. Timing formulas

### 4.1 Speed source

- `v_cruise` = F-80C-10 cruise **732 km/h = 203.3 m/s**, from `model_spec::spec_for("f80c10").cruise_kmh` (`model_spec.rs` ≈156, `spec_for` ≈302).
- **UNVERIFIED / unsourced.** Every `notes` field in that table is empty. The reference gives F-80 altitudes only ("cruise above 4,570 m", "CAP orbit 3,050 m" [S9 p.81, p.53], reference §2.2) and no speeds (§9 Gaps).
- Implement one helper, `airtask::f80_transit_s(radius_m) -> f64` in `timing.rs`, and `airtask::derived_timing(&ThresholdSpec) -> DerivedTiming` on top of it (W, D, H, r_in with the Manual timing overrides applied; the §4.4 UI and every builder read this one function). Both are re-exported from `airtask/mod.rs` (§5).
- **Speed used (D40):** `timing::F80_CRUISE_OVERRIDE_KMH` if it is `Some`, else the `model_spec` value read at run time. The constant starts as `None`. P14 never writes to `model_spec`, so Template Builder's suggested waypoint speed, and with it existing output, cannot change (review finding C10).
- Probe cell **T-n** (§8 step 0, flight 1A) measures the value: the tester flies an F-80C at the in-game cruise setting over a timed 24 km leg. The measured speed, not an AI flying a commanded speed, decides whether 732 km/h stays. If it is off by more than 10 %, the override is set to the measured value (risk n).

### 4.2 Threshold timing (U3, D4)

```
transit(R) = 2R / v_cruise           (straight through the centre)
W = D = ceil_to_10s(transit(R))      window (step 12 only) and dwell
H = 2W                               time between presence re-checks; the area stays HOT meanwhile
r_in = R / 2                         inner (dwell) zone
```

| R | transit at 732 km/h | W = D | H | r_in | (for reference: at the 660 km/h waypoint speed) |
|---|---|---|---|---|---|
| 10 km | 98.4 s | 100 s | 200 s | 5 km | 109 s |
| **12 km (default)** | 118.0 s | **120 s** | **240 s** | **6 km** | 131 s |
| 16 km | 157.4 s | 160 s | 320 s | 8 km | 175 s |

Why D = transit: a player flying straight through at cruise has left the whole zone by the time D ends, and so is certainly outside the inner zone. A player still inside the inner zone after D is loitering.

### 4.3 Sortie timing

```
spawn distance, clock   d = v_wp · M            (v_wp = the template's WP Speed, M = 5 min)
spawn distance, zone    d = 30 km               t_arrive = d / v_wp
T_ext = 600 s                                   margin = 600 s
T_final = t_arrive + T_attack + margin + T_ext  (the hard stop; it is final, §3.4)
```

| Template | v_wp (file) | clock d (M = 5) | zone t_arrive (30 km) |
|---|---|---|---|
| F-80 (both) | 660 km/h | 55.0 km | 164 s |
| F-51 (both) | 520 km/h | 43.3 km | 208 s |
| Yak-9P | 500 km/h | 41.7 km | 216 s |
| Il-10 | 390 km/h | 32.5 km | 277 s |

Examples:
- F-80 HVAR, clock: T_attack 600 s → T_final = 300 + 600 + 600 + 600 = 2100 s (35 min). Normal end: 900 s, or 1500 s with the extension.
- F-51 Armed Recon, clock: T_attack 1200 s → T_final = 2700 s (45 min). Normal end: 1500 s, or 2100 s with the extension.

Take `T_attack` from each file (§3.1 step 7). Do not hard-code it.

Clock slot times: first slot 600 s, then every 1200 s, plus a jitter of 0 / 180 / 360 s.

### 4.4 In the UI

- The Threshold section shows R (editable) and the derived D, H and r_in as read-only values, with the label "From F-80C cruise 732 km/h" (or the override value, marked "measured"). N is shown as 1 and cannot be changed in v1.
- A "Manual timing" checkbox makes D and H editable. They are stored as `Option<f64>` overrides, and `None` means derived. (W has an override field too; the UI shows it from step 12.)
- The Schedule section shows M, the zone spawn distance, T_ext and the hard-stop margin.
- Each pool row shows its computed T_final in a tooltip.
- The Output section shows the totals (D36): copies, aircraft, MCUs and check zones, per side and in all.

---

## 5. Data model

New module tree `src/airtask/` (the step 1 skeleton adds `mod airtask;` and `mod in_theatre;` to `main.rs`). The UI only calls into it (CLAUDE.md rule).

| File | Contents | Owner step |
|---|---|---|
| `airtask/mod.rs` | Types below (exact source), `weighted_random_pct`, the `pub use` lines the UI needs, stubs of `build_air_packs`, `preview`, `validate` (bodies in step 7) | 1a (types frozen), 7 (bodies) |
| `airtask/library.rs` | Built-in six (`include_str!("../../TemplateExamples/Historical1950/…")`), `builtin_library()`, `load_user_sortie(path)`, `inspect_sortie(root) -> Result<SortieShape, Vec<String>>` (the wiring lookup and compatibility check of §3.2, D39), role and side detection. The 1a skeleton writes the two re-exported `pub fn` signatures with stub bodies (`vec![]`, `Err(..)`) so `mod.rs` can re-export them | 1a (signatures), 2 (bodies) |
| `airtask/place.rs` | `place_air_sortie` (§3.1) | 2 |
| `airtask/shell.rs` | `wrap_sortie` (§3.2); under `#[cfg(test)]`, the ignored Probe 2 writer `write_p14_sortie_probe` (step 3) | 3 |
| `airtask/budget.rs` | §3.3: `build_side(side, n, logic: &mut SideLogic, strategies, next_id) -> Vec<Il2Entity>`. It returns the side's `BUD` MCUs and pushes its links into other builders' MCUs to `logic.links` | 3 |
| `airtask/assemble.rs` | `assemble_side`: per side, stamps one copy per run of each eligible entry (jj numbering, census order q, spawn distance rule of §3.8), calls `inspect_sortie` + place + wrap, builds each entry's copy pointer (`ENii COPY r`, the `START` advance, `ENii EXHAUSTED`, §3.7), registers each copy's `HOT` / `COLD` pair, `START` (the gate), `FLYING` and `DONE HUB` in `SideLogic`, and returns `SideCopies` (copy ids per entry, the `START` / `DONE HUB` ids per copy, `COPY r` ids and the `EXHAUSTED` id per entry). Also `emit_side_logic` (§3.0): the side's `Translator Mission Begin`, `AT s INIT`, `AT s INIT OFF` | 3 |
| `airtask/threshold.rs` | §3.5 | 4 |
| `airtask/clock.rs` | §3.7 | 5 |
| `airtask/zone.rs` | §3.8 | 6 |
| `airtask/timing.rs` | §4 helpers: `F80_CRUISE_OVERRIDE_KMH`, `f80_transit_s(radius_m)`, `derived_timing(&ThresholdSpec) -> DerivedTiming`. The 1a skeleton writes both `pub fn` signatures with stub bodies (`0.0`, `DerivedTiming::default()`) so `mod.rs` can re-export them | 1a (signatures), 1 (bodies) |
| `airtask/testkit.rs` (`#[cfg(test)]`) | `assert_links_resolve` (skeleton) and the dry-run walker (step 1b) | 1 skeleton, 1b |
| `airtask/probe.rs` (`#[cfg(test)]`) | Step 0 probe missions: flights 0, 1A and 1B (§8) | 0b |
| `src/in_theatre.rs` | D20 table | 1 |
| `src/frontlines.rs` | `#[derive(Debug, Clone)] pub struct MapAirPack { pub root: Il2Entity }` next to `MapFighterPack` / `MapShipPack` / `MapGroundPack`. `FrontOptions` derives `Clone` and `Debug`, so the pack must too. Added in the step 1 skeleton so the frozen API compiles; the stamping and `FrontOptions` field come in step 7. | 1 skeleton, 7 |

**Exact skeleton source of `airtask/mod.rs`** (the step 1a commit writes this; later steps only add bodies, and a type change is its own small commit by Fable, §10). Every type derives `Debug`, `Clone` and `PartialEq`, because the Undo fingerprints use `format!("{:?}", …)` (`map_fingerprint`, `ui.rs`) and tests compare plans for equality. Field-less enums are also `Copy` and `Eq`. No type derives `Hash` (the plan holds `f64`s); the fingerprint is the `{:?}` string.

```rust
#![allow(dead_code)] // P14: removed in step 9
//! Air Tasking (P14): historical air sorties against Map objectives. See handoff/P14-air-tasking.md.
use std::collections::BTreeMap;
use std::path::PathBuf;
use crate::frontlines::MapAirPack;
use crate::mapclip::WorldAabb;

mod library; mod place; mod shell; mod budget; mod assemble;
mod threshold; mod clock; mod zone; mod timing;
#[cfg(test)] mod testkit;
#[cfg(test)] mod probe;

// What `ui.rs` may call besides the functions at the bottom. The submodules stay private.
// library.rs / timing.rs get these signatures with stub bodies in the 1a skeleton (§5 table).
pub use library::{builtin_library, load_user_sortie};   // fn builtin_library() -> Vec<AirSortie>; fn load_user_sortie(path: &std::path::Path) -> Result<AirSortie, String>
pub use timing::{derived_timing, f80_transit_s};        // fn derived_timing(t: &ThresholdSpec) -> DerivedTiming; fn f80_transit_s(radius_m: f64) -> f64

/// Index convention for every per-side array (`schedule`, `AirBudget::per_side`, preview slot rows).
pub const DPRK: usize = 0;
pub const NATO: usize = 1;

pub type Ymd = (u16, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange { pub from: Ymd, pub to: Ymd }          // inclusive; also used by in_theatre

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirSide { Dprk, Nato }                              // map to the existing `eastern: bool` at call sites
impl AirSide {
    pub fn idx(self) -> usize { match self { AirSide::Dprk => DPRK, AirSide::Nato => NATO } }
    pub fn letter(self) -> char { match self { AirSide::Dprk => 'D', AirSide::Nato => 'N' } } // §3.0 names
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirRole { AirPatrol, GroundAttack, Escort }         // per lead

#[derive(Debug, Clone, PartialEq)]
pub enum SortieSource { BuiltIn(&'static str), File(PathBuf) }

#[derive(Debug, Clone, PartialEq)]
pub struct AirSortie {                                       // one library item
    pub id: String, pub name: String, pub source: SortieSource,
    pub side: AirSide, pub models: Vec<String>, pub planes: usize,
    pub lead_roles: Vec<Vec<AirRole>>, pub wp_speed_kmh: f64, pub wp_alt_m: f64, pub attack_time_s: f64,
    pub valid: DateRange, pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetRef { Objective { side: AirSide, xz: (f64, f64) }, Arrow { tail: (f64, f64), tip: (f64, f64) } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriggerMode { #[default] Clock, Zone, Both }       // D16

#[derive(Debug, Clone, PartialEq)]
pub struct PoolEntry {
    pub sortie_id: String, pub target: TargetRef, pub weight: u8, pub mode: TriggerMode,
    pub runs: u8, pub heading_override: Option<f64>, pub enabled: bool,   // runs: default 3, range 1..=6 (D12)
}
impl PoolEntry {                                             // no Default: an entry always has a target
    pub fn new(sortie_id: String, target: TargetRef) -> Self {
        Self { sortie_id, target, weight: 5, mode: TriggerMode::Clock, runs: 3, heading_override: None, enabled: true }
    }
}                                                            // weight 1..=10 (D15)

#[derive(Debug, Clone, PartialEq)]
pub struct AirBudget { pub per_side: [u8; 2] }               // [DPRK, NATO], each 1..=6
impl Default for AirBudget { fn default() -> Self { Self { per_side: [2, 2] } } }      // D5

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdSpec {
    pub players_n: u8,                                        // 1 in v1; the UI lets it rise from step 12 (U16)
    pub radius_m: f64, pub inner_ratio: f64,
    pub window_s: Option<f64>, pub dwell_s: Option<f64>, pub hold_s: Option<f64>,   // None = derived (§4.2)
}
impl Default for ThresholdSpec {
    fn default() -> Self { Self { players_n: 1, radius_m: 12_000.0, inner_ratio: 0.5, window_s: None, dwell_s: None, hold_s: None } }
}                                                            // D1, D2, D3, D4

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DerivedTiming { pub window_s: f64, pub dwell_s: f64, pub hold_s: f64, pub inner_m: f64 }   // W, D, H, r_in (§4.2); overrides applied

#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {                                        // one per side, indexed DPRK / NATO
    pub enabled: bool, pub first_slot_min: f64, pub period_min: f64,
    pub jitter_steps: u8, pub jitter_step_min: f64, pub quiet_pct: u8, pub arrive_min: f64,
}
impl Default for Schedule {
    fn default() -> Self { Self { enabled: true, first_slot_min: 10.0, period_min: 20.0, jitter_steps: 3, jitter_step_min: 3.0, quiet_pct: 20, arrive_min: 5.0 } }
}                                                            // D6, D7

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneSpec { pub chance_pct: u8, pub cooldown_min: f64, pub spawn_dist_km: f64 }
impl Default for ZoneSpec { fn default() -> Self { Self { chance_pct: 50, cooldown_min: 15.0, spawn_dist_km: 30.0 } } } // D18, D7

#[derive(Debug, Clone, PartialEq)]
pub struct StopSpec { pub margin_min: f64, pub extension_min: f64 }
impl Default for StopSpec { fn default() -> Self { Self { margin_min: 10.0, extension_min: 10.0 } } }                 // D8

#[derive(Debug, Clone, PartialEq)]
pub struct TotalsLimits { pub aircraft: u32, pub mcus: u32, pub check_zones: u32 }
impl Default for TotalsLimits { fn default() -> Self { Self { aircraft: 96, mcus: 1500, check_zones: 40 } } }         // D36

// Strategy enums (§8 step 0d table). The Default is the primary design.
// ThresholdImpl::ComplexTrigger and CounterResetImpl are reserved for step 12 (§3.9); v1 builders accept only ZoneOnly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum ThresholdImpl    { #[default] ZoneOnly, ComplexTrigger }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum ZoneOutImpl      { #[default] CheckZone, Watchdog }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum LossesImpl       { #[default] Type13 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum CounterResetImpl { #[default] SetVal, Ring }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum RandomImpl       { #[default] InMission, Seeded }      // removed by decision 0029 (flight 0 confirmed f)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum BudgetImpl       { #[default] HeadCount, SingleSlot }      // §3.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum CensusImpl       { #[default] Spaced, SameTick }           // §3.3, probe E7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum PresenceCheckImpl { #[default] PulseInside, Closer0Race }   // risk c; removed by decision 0025 (one presence check)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Strategies {
    pub threshold: ThresholdImpl, pub zone_out: ZoneOutImpl, pub losses: LossesImpl,
    pub counter_reset: CounterResetImpl, pub random: RandomImpl, pub budget: BudgetImpl,
    pub census: CensusImpl, pub presence: PresenceCheckImpl,
    pub ai_counts: bool,                                      // probe result a; true makes the D30 warning an error (§8 step 0d)
}

/// Cross-module wiring for one side (§3.0). Every builder appends; nothing edits another builder's MCU by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SideLogic {
    pub(crate) mission_begin: Vec<i32>,                        // pulsed by the side's Translator Mission Begin
    pub(crate) init_off: Vec<i32>,                             // switched off by AT s INIT OFF (Mission Begin + 0.5 s)
    pub(crate) hot_consumers: BTreeMap<usize, Vec<i32>>,       // area index → ENii HOT / SNjj HOT relays (THsaa HOT ON / OFF)
    pub(crate) cold_consumers: BTreeMap<usize, Vec<i32>>,      // area index → ENii COLD / SNjj COLD relays (THsaa COLD ON / OFF)
    pub(crate) gates: Vec<i32>,                                // SNjj START relays; each is its copy's gate (BUD s SHUT / OPEN)
    pub(crate) flags: Vec<i32>,                                // SNjj FLYING timers, in census order (BUD s CENSUS)
    pub(crate) budget_events: Vec<i32>,                        // SNjj START relays and the copies' delete timers; each gets BUD s MARK and BUD s CHECK
    pub(crate) links: Vec<(i32, i32)>,                         // (from, to): emit_side_logic adds `to` to the Targets of `from` (§3.0)
    pub(crate) pass_subscribers: BTreeMap<usize, Vec<i32>>,    // area index → ZNii ARMED relays (targets of THsaa INNER, the PASS event)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AirPlan {
    pub library: Vec<AirSortie>,                             // Default is empty; the UI fills it with the built-ins (step 2)
    pub pool: Vec<PoolEntry>, pub budget: AirBudget, pub threshold: ThresholdSpec,
    pub schedule: [Schedule; 2],                             // [DPRK, NATO]
    pub zone: ZoneSpec, pub stop: StopSpec, pub limits: TotalsLimits, pub include_in_base_map: bool,
    pub strategies: Strategies,
    pub seed: u64,                                           // every Generate-time random choice; see "Seed" below
    pub debug_subtitles: bool,                               // false; not in the UI; set only by the step 11 test mission writer
}

pub struct AirContext<'a> {                                  // supplied by the UI from Map state; no derives needed
    pub east_objectives: &'a [(f64, f64)], pub nato_objectives: &'a [(f64, f64)],
    pub arrows: &'a [((f64, f64), (f64, f64))],
    pub front: &'a [(f64, f64)],                             // the polyline the Map preview colours arrows with (see below)
    pub aabb: WorldAabb, pub date: Ymd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirLine { pub side: AirSide, pub entry: usize, pub from: (f64, f64), pub to: (f64, f64) }   // spawn → target
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirRing { pub side: AirSide, pub centre: (f64, f64), pub radius_m: f64, pub inner: bool }  // R, or r_in when inner
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirPoint { pub side: AirSide, pub entry: usize, pub at: (f64, f64), pub target: (f64, f64) } // spawn or RTB dot; RTB draws a thin line to target
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotMark { pub t_s: f64, pub jitter_span_s: f64, pub quiet_pct: u8 }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AirPreview {
    pub lines: Vec<AirLine>, pub rings: Vec<AirRing>, pub spawns: Vec<AirPoint>, pub rtbs: Vec<AirPoint>,
    pub slot_times: [Vec<SlotMark>; 2],                      // [DPRK, NATO], 0–3 h (D29)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AirDrawCounts { pub lines: usize, pub rings: usize, pub spawns: usize, pub rtbs: usize }
impl AirPreview {
    pub fn counts(&self) -> AirDrawCounts {
        AirDrawCounts { lines: self.lines.len(), rings: self.rings.len(), spawns: self.spawns.len(), rtbs: self.rtbs.len() }
    }
}

/// What the plan adds to a mission (D36). Index DPRK / NATO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AirTotals { pub copies: [u32; 2], pub aircraft: [u32; 2], pub mcus: [u32; 2], pub check_zones: [u32; 2] }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity { Error, Warning }                    // Error blocks Generate; Warning is orange
#[derive(Debug, Clone, PartialEq)]
pub struct AirIssue {
    pub severity: IssueSeverity, pub text: String,
    pub rail_only: bool,   // true only for "no enabled valid entry": an Error on the Air Tasking tab, an orange note on Map (§6.1)
}

pub fn weighted_random_pct(weights: &[u32]) -> Vec<u8> { /* step 1a: real body, §3.6 */ }
pub fn build_air_packs(plan: &AirPlan, ctx: &AirContext) -> Result<Vec<MapAirPack>, String> { Ok(vec![]) } // step 7
pub fn preview(plan: &AirPlan, ctx: &AirContext) -> AirPreview { AirPreview::default() }                // step 7
pub fn validate(plan: &AirPlan, ctx: &AirContext) -> Vec<AirIssue> { vec![] }                            // step 7; feeds readiness_checks
pub fn totals(plan: &AirPlan, ctx: &AirContext) -> AirTotals { AirTotals::default() }                   // step 7; counted from build_air_packs output
```

Only fields may be added to `AirPreview` and `AirIssue` in step 7, as a small commit of their own. `AirDrawCounts` lives here so that `ui.rs` (`self.air_drawn`) and the `air_dock_draws_preview` test compare against `preview(..).counts()`.

**Seed.** Map placement draws a fresh time-based seed on every use (`placement_seed()`, `ui.rs` ≈9138, SystemTime nanos), and neither the app nor `FrontOptions` stores one. So the plan carries its own: `AirPlan.seed`.
- The UI sets it once from `placement_seed()` when it creates the plan at startup, and again on Reset. Nothing else changes it, so Generate twice gives the same output (CLAUDE.md rule).
- It is part of the plan, so it is in the Air Tasking undo snapshot, the `{:?}` fingerprint and the Map fingerprint.
- `load_base_map` keeps it (it keeps the library and settings, §6.2).
- Tests set it explicitly (step 7 test plan: `seed = 7`). UI tests that compare generated output with air entries set `app.air_plan.seed` before Generate.

**Target resolution.**
- An `Objective` target stores its XZ. It resolves to the nearest current objective of that side within 50 m. If none is found, the entry is orphaned (orange, skipped at Generate).
- An `Arrow` target resolves to an arrow with the same tip within 50 m. The arrow side comes from `mapclip::point_north_of_front(ctx.front, tail.0, tail.1)` (`mapclip.rs` ≈984; north = DPRK).
- `ctx.front` is the same polyline `draw_korea_map` uses to colour arrows (`arrow_front` in `ui.rs` ≈8519-8525: the composite front when it has ≥ 2 points, else the densified full front), so a sortie's side always matches the arrow colour the user sees. `generate_front` decides ground-arrow sides with `dense_base` (`frontlines.rs` ≈1246). A step 7 test checks the two agree for the test arrows; if they ever disagree, report it to the user rather than pick one silently.
- Objective meaning is confirmed in code: `east_objectives` are DPRK targets and `nato_objectives` are NATO targets.

**Library and role detection.**
- Side = the template's CheckZone `PlaneCoalitions`: `[1]` means it watches DPRK, so it is a NATO sortie; `[2]` means DPRK. This is verified in all six files.
- Per lead: every `MCU_CMD_*` whose Objects contain the lead's entity index. An AttackArea with `AttackAir == 1` gives `AirPatrol`. `AttackGround` or `AttackGTargets` gives `GroundAttack` (`weapon_range::is_ground_attack_area`, `weapon_range.rs` ≈212-222). `Cover` gives `Escort`. Detection is by these Objects edges only, never by timer names (§3.1). Do **not** depend on `load_template` layout detection (R3 F2).

| File | Leads → roles | Display role |
|---|---|---|
| F-80 AirAlert | 7: AirPatrol + GroundAttack | Air alert |
| F-80 HVAR | 7: GroundAttack | Strike |
| F-51 CloseSupport | 7: GroundAttack | Close support |
| F-51 ArmedRecon 2+2 | 7: GroundAttack; 11: Escort | Armed recon (escorted) |
| Yak AirfieldRaid 2+2 | 7, 11: GroundAttack | Strike |
| Il-10 KimpoAttack 2×4 | 7, 15: GroundAttack | Strike |

A unit test pins this table.

**`in_theatre.rs` (D20, D34).**
- Rows: `model_spec id → DateRange, side, citation, sourced: bool`. One row per plane id in `model_spec` (b29, simpleb29, c47b, f51d, f80c10, f84e, f86a5, il10, la11, li2t, mig15bis, tu2, yak9p at `bf1a356`).
- Seed it from `docs/historical-reference/korea-1950-53-unit-reference.md` §2.2, §3 and §9. Sourced dates known today:
  - `la11` from 1953-04-15 [S9 p.685], [S9 p.686];
  - `mig15bis` from 1950-11-01 [S9 p.241], as the stand-in for the MiG-15 family; citation text: "stand-in for MiG-15; the bis variant appeared Nov 1951 [S9 p.434]".
  - La-9 (first seen 1951-11-30 [S9 p.437]) has no `model_spec` id, so it has no row. Leave a comment for R4 P11.
- A row without a sourced date uses the war span (1950-06-25 → 1953-07-27) and is marked `sourced = false`. Do not invent dates.
- An entry is valid when `ctx.date` (from the Map `TimelineMark`'s year/month/day, `frontlines/timeline.rs` ≈25-35) falls inside the range of every model in its template.

**Builders.** Make these `pub(crate)` in place, without moving them, so the diff stays small and output does not change. This is done **in the step 1a skeleton commit** (a visibility-only diff), so the step 0b probe and the step 1b walker can use them from the start:
- `mcu`, `timer`, `counter`, `checkzone`, `modifier_set_val`, `attach_event` (`template.rs` ≈3786-3880; all private today);
- `clone_named` / `synthesize_mcu` (`recon.rs` ≈1727-1733);
- `silence_clone_starts` (`recon.rs` ≈1681);
- `set_coord` / `add_yori` (`placement.rs` ≈204-222);
- `is_rtb_waypoint` (`mapnet.rs` ≈328; step 2 needs it to find a template's own RTB).

Note that `timer` hard-codes `Random=100` (`template.rs` ≈3803). The skeleton commit also adds `timer_random(name, time, pct, …)` next to it (item-level `#[allow(dead_code)] // P14`). Nothing calls it from a Generate path, so output does not change.

---

## 6. UI

Rules: presentation only. Build on `theme.rs` / `shell.rs`. Text at least 12 px, controls at least 28 px. Use ✔ ✖, never ✓ ✗. User text says DPRK / NATO.

### 6.1 Rail tab "Air Tasking" (7th, Ctrl 7)

**Enum, arrays and exhaustive matches** (the compiler flags these; `ui.rs`, approximate lines at `bf1a356`):
- `enum AppMode` (≈169) → add `AirTasking`.
- `MODES: [(AppMode,&str); 6]` (≈184) → 7, add `(AppMode::AirTasking, "Air Tasking")`. Also update its doc comment.
- `tab_status: [Status; 6]` (≈592) → 7.
- Page dispatch (≈1918-1925).
- `page_help_topic` (≈1930).
- `readiness_checks` (≈2007).
- `primary_action_inner` (≈2073).
- `load_action` (≈2088): "Add template…".
- `page_header_bar` (≈2104): title, file label, primary label, and its right-hand buttons.
- `settle_undo` (≈5394), `undo_label` (≈5455), `undo_current_tab` (≈5467).
- `idle_hint` (≈10392).

**`_`-fallback matches** (check by hand):
- Reset → confirm (in `page_header_bar`): add an `AirTasking` arm and a `Confirm::ResetAirPlan` variant (`enum Confirm` ≈274).
- `idle_problem` (≈10418).
- `mark_saved` / `is_dirty`: no change. The plan is not a file (§1).

**Shortcuts (`shell.rs`):**
- `read_shortcuts` (≈970-995): add `Num7` for **Ctrl** only. Do not put it in the plain-number `map_tool` path, because `handle_shortcuts` indexes `MAP_TOOLS[i]` (`ui.rs` ≈211, 6 entries) and would panic.
- `mode_rail` caption "Ctrl 1–6" (`shell.rs` ≈723) → "Ctrl 1–7".
- **Only "Ctrl 1–6" changes.** The plain map-tool keys stay "1–6" everywhere (D27). Edits:
  - `help.rs` ≈363 key list: `"Ctrl 1–6"` → `"Ctrl 1–7"`; keep the separate `"1–6"` entry (the help test looks for the substring "1–6", which only the map-key entry will still carry).
  - `USER_MANUAL.md` ≈28 and ≈54: "Ctrl 1–6" → "Ctrl 1–7". Leave ≈61 and ≈453 (map tools 1–6) as they are, except that ≈453 "The right dock has four tabs" becomes five, with **Air**.
  - `docs/src-guide.md` ≈68: "Ctrl G / O / Z / Y / 1–6" → "… / 1–7"; keep "map keys 1–6".
  - `docs/ui-redesign/README.md` ≈83 ("six 32 px rows", "Ctrl 1–6" → seven, "Ctrl 1–7") and ≈265 (`Ctrl 1–6` → `Ctrl 1–7`). Keep ≈268 (`1–6` map tools). Leave the §9 phase history (≈329, ≈333) as the record of what was checked then.

**`help.rs`:** `HelpTopic::AirTasking`, `ALL` 11 → 12 (≈38-50), `title()` "Air Tasking" (≈52-65). Add a `## Air Tasking` section in `USER_MANUAL.md` longer than 80 chars (test at `help.rs` ≈331).

**Layout:**
- **Left panel: Library.** The built-in six plus user templates. Each row shows a side marker, name, role chips, planes, and a date-validity badge (✔ valid for the Map date / ✖ with the reason and citation). Buttons: "Add template…" and "Remove". A built-in item cannot be removed (the button is disabled).
- **Centre: Pool table.** Columns: Sortie, Side, Target (objective n / arrow n / orphaned), Weight, Mode (Clock/Zone/Both), Runs, Heading (auto/override), Valid, ✖ remove. The Runs cell's tooltip states the cap: "At most `runs` sorties of this entry per mission (clock and zone together)".
- **Right panel: settings sections**, all with `shell::section_title`:
  - BUDGET: DPRK / NATO max active (1–6). Under `BudgetImpl::SingleSlot` both show 1 and cannot be changed, with the explanation "Counters are not confirmed in game".
  - THRESHOLD: N (shown as 1, not editable in v1, with the explanation "Two or more players comes with the multi-player phase"), R, derived D / H / r_in, Manual timing.
  - SCHEDULE DPRK / SCHEDULE NATO: enabled, first slot, period, jitter steps × step, quiet %, arrive in M min, and the side's **slot timeline strip** (the same draw helper as the dock tab, §6.2).
  - ZONE TRIGGERS: chance %, cooldown, spawn distance.
  - HARD STOP: margin, extension, and the label "Extend once if a player was over the target in the last few minutes."
  - OUTPUT: "Include in base map", and the totals (D36): copies, aircraft, MCUs and check zones per side and in all. A total above its limit is orange.

**Primary:** "Generate File" writes the air pack alone. Readiness is `airtask::validate` as `Check`s:
- a front is loaded;
- at least one enabled entry that is valid and not orphaned;
- budget ≥ 1 on each side that has entries;
- every template placed without error;
- each enabled side's period is longer than its jitter span plus its slot window plus one attempt (§3.7).

The D30 AI-trigger check, the slot-window cap warning, the totals warning (D36) and the locale-id warning (D41) are orange warnings, not blockers. If `strategies.ai_counts` is true, the D30 check is an error (§8 step 0d).

**Map readiness.** `readiness_checks`' Map arm is empty today ("any period makes a valid base map"). Add one check when `include_in_base_map` is ticked and the pool is not empty: "Air Tasking plan is valid (see the Air Tasking tab)", true when `validate` has no `Error` with `rail_only == false` (budget, placement, period). The one `rail_only` issue, "no enabled valid entry", does not block Map: the plan then yields no air pack and the base map is generated as usual (D21, step 7 `generate_front_with_disabled_air_plan_matches_baseline`), so Map shows it as an orange note "Air Tasking has no valid entries and will be left out". On the Air Tasking tab the same issue is an Error, because "Generate File" would write an empty file. Update the comment above `readiness_checks`. This keeps Map's checks in step with the new `build_air_packs` error in `generate_front_file` (CLAUDE.md rule).

**Undo:** `air_undo: Undo<AirPlan>` with arms in `settle_undo`, `undo_label` and `undo_current_tab`. The fingerprint is `format!("{:?}", self.air_plan)`, like `map_fingerprint` (the plan has `f64`s, so no `Hash`). Reset also draws a new `AirPlan.seed` from `placement_seed()` (§5 "Seed"). `Undo::record` keeps a snapshot only when it is called before a change; ordinary edits just settle. So call `air_undo.record(..)` before every edit that Ctrl Z must undo on this tab: a weight / mode / runs / heading change (on the first frame of the change), an entry removal, Reset, and Remove on a library item.

### 6.2 Map dock tab "Air"

- `enum MapDock` (≈283) → add `Air`. `tabs(refs) -> [...; 4]` (≈292) → 5. Dispatch in `map_dock_panel` (≈7301) → `MapDock::Air => self.map_air_tab(ui)`.
- **Width (UNVERIFIED):** "Period / Forces / References 123 / Terrain / Air" in 288 px. `dock_tab_widths` (`shell.rs` ≈681) does not shrink tabs. The fallback is fixed now (D35): if they do not fit, `MapDock::tabs` gives "Refs" instead of "References" when the count has 3 digits. `DOCK_TAB_PAD` stays as it is. Test edits for the fallback: `map_dock_tabs_fit_a_three_digit_reference_count` (`ui_tests.rs` ≈1619) looks up and clicks "Refs 123" instead of "References 123", and adds "Air" to its label list; `map_dock_tabs_all_render` keeps "References" at 2 digits. Measure first; make the change only if the 5-tab fit test fails without it.
- **Sections:**
  - SORTIES: a DPRK / NATO toggle. "Auto-fill from objectives" adds one entry per date-valid sortie of that side for each objective **and each arrow** of that side that has no entry for that sortie yet, with default weight and mode Clock. Running it again adds nothing. A compact list of that side's entries has weight, mode and ✖.
  - SHOW: checkboxes for approach lines, threshold rings, spawn points, RTB points.
  - TIMELINE: slot strip, one row per side, 0–3 h (D29). It shows slot ticks with the jitter span shaded and the quiet % as the tick's opacity. One helper, `draw_slot_strip`, serves this and the rail tab.
- **Drawing:** `draw_map_air` is called in `draw_korea_map` right after `draw_map_objectives` (call ≈8560). It paints `airtask::preview(...)` output:
  - dashed approach line spawn → target;
  - solid ring R and dashed ring r_in per area;
  - spawn dot, and RTB dot linked by a thin line;
  - side colours from `theme.rs` tokens.

  No geometry is computed in `ui.rs`. For tests, `draw_map_air` stores the counts it painted in `self.air_drawn: AirDrawCounts` (lines, rings, spawns, rtbs), so a headless test can assert the draw path ran.
- **State:** the `AirPlan` field on the app is shared by both tabs.
  - Add it to `struct MapForces` (≈241), `map_forces()` (≈5197), `restore_map_forces` (≈5212), the Map fingerprint in `settle_undo`, and `map_fingerprint` (≈5529).
  - Dock edits undo with Map's Ctrl Z only where `record_map_undo` (≈5244, "Call before a Map Clear / Remove") is called first. Call it before "Auto-fill from objectives" and before each ✖ remove in the dock list.
  - `load_base_map` (≈11407) keeps the library and settings and clears the pool. Its targets were preview objectives, and Load clears those.

### 6.3 `ui_tests.rs` changes

**Extend:**
- `rail_and_ctrl_numbers_switch_every_tab` (≈267): key list `[Num1..Num6]` → Num7.
- `every_tab_has_a_primary_button_with_its_shortcut` (≈293): Air Tasking has "Generate File".
- `button_labels_stay_on_one_line` (≈1725) and `collect_tall_buttons` (≈1743): add dock "Air".
- `side_panel_controls_stay_inside_their_panel`: `PANELS: [..; 6]` (≈1690) → 7.
- `map_dock_tabs_all_render` (≈1223): add `("Air", "SORTIES")`.
- `map_dock_tabs_fit_a_three_digit_reference_count` (≈1619): 5 labels (and the D35 edits if the fallback is needed).
- `interactive_widgets_are_at_least_28_px_tall` (≈1235) and `every_ui_glyph_is_in_a_loaded_font` (≈1661) **do not cover the new UI on their own** (review finding C12). The height guard visits each tab in its start state and skips disabled controls, so it never sees a filled pool, the Replay panel or a disabled button. The glyph guard reads four files only. Edits:
  - `every_ui_glyph_is_in_a_loaded_font`: add every new file that holds text the user can see: `airtask/mod.rs`, `airtask/library.rs`, `airtask/clock.rs` (the `validate` messages), `in_theatre.rs`, `trace.rs`, `missionlog.rs`.
  - New guard `air_tasking_widgets_are_28_px_with_a_filled_pool` (step 8): the pool holds one entry per built-in, every settings section is open, and **disabled controls are checked too**.
  - New guard `map_air_dock_widgets_are_28_px_with_entries` (step 9), and `replay_panel_widgets_are_28_px` (step 9T, with a replay loaded).

**New (step 8, part a: needs only the types):**
- `ctrl_7_opens_air_tasking`.
- `plain_7_on_map_does_not_panic`.
- `air_tasking_undo_restores_weight_change` (a pool entry built with `PoolEntry::new`; edit a weight, Ctrl Z, pool equal to before).
- `air_tasking_threshold_n_is_fixed_at_1`.
- `air_plan_first_entry_ticks_include_in_base_map`.

**New (step 8, part b: needs step 2's library and step 7's `validate` / `build_air_packs` / `totals` bodies):**
- `air_tasking_tab_shows_builtin_library` (six names present).
- `air_tasking_builtin_remove_disabled`.
- `air_tasking_generate_disabled_until_validate_passes` (empty pool, then an orphaned-only pool, then budget 0 on a side with entries: the button is disabled each time and the matching Check text is shown).
- `air_tasking_generate_file_writes_air_pack` (uses `dialog::answer`; parses the file and asserts a single top-level `Air Tasking` group, both side Logic groups, `assert_links_resolve`, and indexes starting at 1; `assert_group_file` alone only checks that it parses).
- `air_tasking_shows_totals` (the numbers in the OUTPUT section equal `airtask::totals`) and `air_tasking_totals_above_limit_are_orange_and_do_not_block_generate`.
- `air_tasking_widgets_are_28_px_with_a_filled_pool` (above).

**New (step 9):**
- `air_dock_auto_fill_adds_entries_for_objectives`, `air_dock_auto_fill_includes_arrows`, `air_dock_auto_fill_twice_is_idempotent`.
- `air_dock_draws_preview` (asserts `air_drawn` equals the `airtask::preview` counts).
- `map_undo_restores_air_pool_after_auto_fill`, `map_undo_restores_air_entry_after_remove`.
- `map_readiness_includes_air_check_when_included`.
- `map_readiness_orphaned_only_air_pool_warns_but_allows_generate` (Include ticked, every entry orphaned: the orange note shows, Map's Generate is enabled, and the output equals the baseline).
- `map_generate_with_air_includes_air_tasking_group`.
- `map_generate_with_include_unticked_matches_baseline` (same inputs as the step 1 fixture, plus a pool entry with "Include in base map" unticked).
- `load_base_map_with_air_tasking_is_not_an_army`, `load_base_map_keeps_air_library_and_settings` (drive `load_base_map` in `ui.rs`; need step 7's `inspect_base_map` recognizer).

**Already there from step 1, not new here:** `write_p14_baseline_ui` (ignored) and `map_generate_without_air_matches_baseline` (§7). Steps 8 and 9 keep them green.

---

## 7. Export integration

- `FrontOptions` (`frontlines.rs` `FrontOptions`) gets `pub air_packs: Vec<MapAirPack>`, and its `Default` gets `Vec::new()`. The test constructions use `..FrontOptions::default()`. Only the literal in `generate_front_file` (`ui.rs`) lists every field with no `..Default`, so it needs the new line. **Step 7 adds that one line (`air_packs: Vec::new(),`) in the same commit as the field.** It is the only `ui.rs` line step 7 touches.
- **Stamping.** In `generate_front` (`frontlines.rs` ≈1116), the packs are stamped near ≈1450-1473. Stamp the air packs **after the ground packs and before `aoi_border_group`**, with the same `stamp_rooted_packs(iter, &mut next_id)` (≈2200-2210). When the list is empty the loop does nothing and `next_id` does not change, so the output is byte-identical.
- **`generate_front_file`** (`ui.rs`, step 9). After the ground packs: `let air_packs = if self.air_plan.include_in_base_map { airtask::build_air_packs(&self.air_plan, &ctx)? } else { vec![] };`. The built-in templates need no sidecar (D19). Add user-template paths to the `paths` passed to `merge_template_sidecars`. That merge keeps the first text when two templates share a locale id (D41); `validate` has already warned about it.
- **Standalone file** (Air Tasking tab): `airtask::build_air_packs`, then a root holding only the `Air Tasking` group with indexes from 1, then `serialize_group`, the save dialog and `write_sidecars`. Other tabs' standalone files already rely on the editor re-indexing when groups are imported into a mission. Whether it does that is UNVERIFIED (risk l, R4 P13 "first check"); step 11 checks it.
- **Load.** `inspect_base_map` (`frontlines.rs` ≈1543-1638) must recognise a group whose name starts with `Air Tasking` **before** `group_has_units` (≈1912). Otherwise the sorties come back as an `ImportedArmy`.
  - Add `ImportedBaseMap.air_tasking_found: bool`.
  - The UI shows the orange warning: "Loaded map had Air Tasking; Generate will replace it with the current plan (removed if the plan is empty)."
- **Determinism.** Given the same plan (which includes `AirPlan.seed`) and context, `build_air_packs` returns identical trees. Every random choice made at Generate time (the entry shuffle, and all `RandomImpl::Seeded` choices) uses `AirPlan.seed` (§5 "Seed"), never `placement_seed()` directly.
- **Byte-identical guarantee.** Before any P14 code, step 1 records baseline fixtures from unmodified `main` (`a34a00e` or later, so with the upstream commit `c1d88c3`, U15):
  - Directory `src/testdata/p14_baseline/`, committed. Files: `template.Group`, `recon.Group`, `fighter.Group`, `exclusive.Group`, `airfield_mp.Group`, `base_map.Group`, `base_map_ui.Group`, plus each file's locale sidecars where the generator writes them.
  - Test module `src/baseline_tests.rs` (`#[cfg(test)] mod baseline_tests;` in `main.rs`). Each fixture's inputs are written as code in that module: fixed seat lists, a fixed seed, the templates under `TemplateExamples/` it loads, and for `base_map.Group` a `FrontOptions` with fighters, ships, ground packs, armies and attack arrows. The first six call the same `generate_*` function each tab's primary button calls. The `template.Group` seat list includes an F-80C with one waypoint at the suggested speed (`suggested_waypoint_speed_kmh(["f80c10"])` = 660), so a change to the `model_spec` cruise value would show in the baseline (D40).
  - `base_map_ui.Group` is produced through the UI harness via `generate_front_file`, so it covers the `ui.rs` path. `Harness` and `dialog::answer` are private to `src/ui_tests.rs`, so this fixture's writer and test live there, and **step 1 makes that small `ui_tests.rs` addition**:
    - ignored `write_p14_baseline_ui` writes it; `map_generate_without_air_matches_baseline` compares against it. Both are active from step 1.
    - Inputs are direct field sets on `h.app`, no clicks: `east_objectives` and `nato_objectives` with two fixed points each, one `attack_arrows` entry, a fixed `front_t`, and `terrain_apply = false` (the Harness default is already false, `ui.rs` ≈1781; set it explicitly anyway, because `generate_front_file` runs `apply_terrain` on the pack, `ui.rs` ≈11369). The save path comes from `dialog::answer`.
    - No seed is involved: Map's Generate path draws none. (`placement_seed()` is used only by the placement tools.)
    - Terrain note: `terrain_apply::apply_terrain_heights` never touches airborne planes, and moves only waypoints whose Objects are ground entities (`terrain_apply.rs` ≈4-7, ≈197). So the air packs are unaffected even when a user ticks terrain; the fixture keeps it off so the baseline does not depend on the height store.
  - An ignored test `write_p14_baseline` writes the other files. It and `write_p14_baseline_ui` are run once, on unmodified `main` code, in step 1. After that the fixtures change only with the user's approval.
  - `existing_outputs_match_baseline` compares current output to them byte for byte. It must pass after every step.

---

## 8. Build steps

**General rules for every step:**
- Branch from the P14 integration branch `claude/p14-air-tasking`, which is cut from `main`.
- `cargo build`, `cargo clippy` and `cargo test --offline` must pass.
- The test count must not drop below the recorded baseline. Re-record it at step 1 (`cargo test --offline` gave 469 passed and 3 ignored at `a34a00e`).
- **Warnings.** This is a binary crate with no `lib.rs` (`main.rs` header), so new code is `dead_code` until `ui.rs` calls it. The step 1 skeleton therefore puts a temporary `#![allow(dead_code)] // P14: removed in step 9` at the top of `airtask/mod.rs` (it covers the submodules) and `in_theatre.rs`, and an item-level `#[allow(dead_code)] // P14` on each new helper outside those modules (`rotate_tree`, `snap_air_attack_areas`, `timer_random`, `MapAirPack`, and the `FrontOptions.air_packs` field if needed). `trace.rs` gets a module-level `#![allow(dead_code)] // P14: removed in step 9T` (its only non-test caller is the 9T UI); `missionlog.rs` is reachable through the CLI and needs an allow only on items the CLI does not use, removed in 9T. Test-only modules (`testkit`, `probe`, `baseline_tests`) are `#[cfg(test)]` and need none. With those in place, warnings must not rise above the recorded baseline at any step. Step 9 removes every `// P14` allow once the code is reachable from `main` (9T removes the `trace.rs` / `missionlog.rs` ones); step 10's acceptance greps that none remain.
- The baseline-fixture test (§7) must stay green.
- **Every acceptance line is a named test.** The names are given below. Walker scenarios live next to the builder they test.
  - **MCU counts are pinned (U13).** Each builder has one test that counts the MCUs and check zones it adds (`shell_adds_19_mcus_per_copy`, `budget_structure`, `threshold_has_23_mcus_and_2_check_zones`, `clock_mcu_counts`, `zone_has_17_mcus_per_entry`). If a step needs more, it says so in its stop report, and the number in §3 and in the test change together.
- Stop after each step and list the changes (CLAUDE.md phase rhythm).
- Never `git add -A`. `.gitattributes * -text` stays.

### Step 0. In-game MCU probe — S (code) + three short solo flights. Branch `claude/p14-probe`

**All three flights are solo (U14).** One tester, called **P** below, flies each of them alone on the dedicated server. Cells that need a second player (T-b, T-d) are in step 12 (U16).

| Flight | File | P flies | Cells | Length |
|---|---|---|---|---|
| **0** | `P14_Probe_0_traced.Group` | NATO, any aircraft. P may stay on the ground at K14 | T-a1, T-e, T-f, T-t, T-j, T-z (all automated) | 11 min |
| **1A** | `P14_Probe_1A.Group` | NATO, F-80C from K14 | T-c, T-n (with ZA4) | 26 min |
| **1B** | `P14_Probe_1B.Group` | DPRK, from the `P14 DPRK SPAWN` icon | T-g | up to 30 min |

**Flight 1B was retired on 2026-10-12** (decision 0010). Flights 1N, 1D and 1G in step 0e replace it; its rows below are kept as the record of the first design.

**Order.** Flight 0 comes first. Its results choose the trace carrier (step 0T), and flights 1A and 1B are then written with that carrier. If no carrier logs, 1A and 1B are written plain and read from their subtitles.

**0b. Probe generators.** `airtask/probe.rs` (`#[cfg(test)]`) with `generate_probe_0() -> Il2Entity`, `generate_probe_1a() -> Il2Entity` and `generate_probe_1b() -> Il2Entity`, plus their locale strings (`src/locale.rs` ≈89).
- **Depends on:** the step 1a skeleton only. The builders it needs (`mcu`, `timer`, `timer_random`, `counter`, `checkzone`, `modifier_set_val`, `attach_event`, `silence_clone_starts`) are made `pub(crate)` in that commit (§5 "Builders").
- Ignored writers: `cargo test --offline write_p14_probe_0 -- --ignored` writes `target/p14/P14_Probe_0.Group` (plain), `P14_Probe_0_traced.Group`, `.trace.json` and `.eng`. `write_p14_probe_1a` and `write_p14_probe_1b` write the other two files, plain and traced.
- It touches no existing Generate path.
- Every observation is an `MCU_TR_Subtitle` with `Coalitions [0,1,2]` and a unique `LCText`, using the block shape from `TemplateExamples/BomberMissions/B29Mission.Group` ≈758-785. A subtitle MCU may fire several times; the video shows how often.
- **Readouts.** IL-2 counters show nothing in game, so every count is read through threshold counters, each driving its own subtitle. Readout counters are `Dropcount 0` (fire once). **No pass/fail criterion counts repeats of one subtitle:** "exactly once" is read as "`X` shows and `X ≥2` never shows", with a `Counter(2)` readout on that output (E1, E2, E6, J4, J5, each T-g plane's type 4, C5). They depend on counters working (risk e), so every counted event also drives a per-event subtitle that the user can count on the video if T-e shows odd counter behaviour.
- **Subtitle duration.** Per-event and readout subtitles use `Duration = 1` (s), below the shortest pulse spacing in the probe (2 s, T-e), so two firings of the same subtitle show as two separate appearances. Cue subtitles use `Duration = 20`, as in the B29 block, so a player can read them. A same-tick double firing still shows as one appearance; only the `≥2` readout can see it.
- **AI flights** come from the built-in templates through a test-only helper `probe_flight(src, keep_planes, start, exit, next_id)`:
  1. `parse_group_file(include_str!("../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group"))`;
  2. keep the first `keep_planes` planes in file order (the lead first); delete the others with their `MCU_TR_Entity`s, and drop their indexes from every Targets and Objects list;
  3. `duplicate::duplicate_template(copy, &mut next_id)`;
  4. `placement::move_anchor_to(root, lead_xz, start)`, then `mapnet::park_path_waypoints(root, &[exit])` and `weapon_range::snap_ground_attack_areas(root, exit.0, exit.1)`, so the flight does not turn back to attack its spawn point (§3.1). WP 1 `Speed` stays as in the file. Every probe flight flies north (start → exit on a bearing of 0°, the templates' own bearing), so no rotation is needed; `rotate_tree` is not available to 0b;
  5. `recon::silence_clone_starts`, then delete these MCUs of the HVAR file, found by name, and every link to them: `ENABLE / PULSE IN`, `Zone IN`, both `Self Deactivate`, `Zone In ReActivate`, `COOLDOWN` and `Mission Complete 1`. 0b may use names because it only ever reads the HVAR file; `inspect_sortie` (step 2) is not available to it;
  6. a probe timer at the cell's start time pulses the template's `MISSION BEGIN` bring-up timer. A cell's close deletes the flight with its own `MCU_Delete` on the flight's plane entities.
- **Cue subtitles and timing.** Every cell is armed, cued and closed by timers started directly by the probe's `Translator Mission Begin`, at fixed mission times. A step that depends on P being in place starts at a fixed cue time: the cue subtitle tells P to be there, and the cell's timers run from that same time. No zone is used to detect P, so the run sheet does not rely on risks a or c. At its close time, every check zone of the cell is deactivated, so crossing a finished cell later changes nothing.
- All positions and times are constants in `probe.rs` (`K14_ORIGIN`, `RUN_SHEET_0`, `RUN_SHEET_1A`, `RUN_SHEET_1B`, `CUES`), and the tests pin them to the tables below.

**Run sheets.** Origin = the `K-14_Kimpo_AF` airfield in `TemplateExamples/K14 AFB_mp.Group` (X 105934, Z 269477; the file's only airfield). Offsets are km north (+X), km east (+Z). Times are mm:ss from Mission Begin. Each file carries `MCU_Icon` markers for the places P must find.

**Flight 0** (`P14_Probe_0_traced.Group`; P joins as NATO at K14 and may stay on the ground):

| Cell | Centre (N, E km) | Armed | Closed | Cues (subtitle at the time shown) |
|---|---|---|---|---|
| T-a1 | (0, +60); AI start (−30, +60), exit (+30, +60) | 00:05 | 06:00 (AI deleted) | 00:00 "Flight 0: stay within 20 km of K14 until 11:00" |
| T-e | automated, at (0, +5) | 01:00 | 02:30 | 01:00 "T-e start" |
| T-f | automated, at (0, +5) | 02:30 | 09:00 | 02:30 "T-f F1 start"; 05:30 "T-f F2 start" |
| T-t | automated, at (0, +5); breadcrumbs on the trace grid (step 0T) | 04:30 | 05:30 | 04:30 "T-t start: note any objective message or map marker in the next 60 s" |
| T-j | automated, at (0, +5) | 09:00 | 10:00 | 09:00 "T-j start" |
| T-z | (−40, −40), a place with no DPRK aircraft | 10:00 | 10:30 | 10:00 "T-z start"; 11:00 "Flight 0 done. End the mission normally" |

**Flight 1A** (`P14_Probe_1A.Group`; P takes off from K14 in an F-80C):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-c | C0–C3, C5 at (+40, −20), icon "T-c C"; C4 at (+40, −8), icon "T-c C4" | 10:00 | 14:00 | 00:00 "T-c: take off, fly to the T-c C icon, orbit within 5 km at 3050 m by 10:00"; 12:00 "T-c: fly east into C4 now"; 14:00 "T-c done" |
| T-n (+ ZA4) | START (+30, +30), END (+54, +30), icons "T-n START" and "T-n END"; ZA4 at (+42, +30) | 14:00 | 26:00 | 14:00 "T-n: fly to T-n START, then north through START and END, level at 3050 m, F-80C cruise setting"; 26:00 "Flight 1A done. End the mission normally" |

**Flight 1B** (`P14_Probe_1B.Group`; the user adds a DPRK spawn at the `P14 DPRK SPAWN` icon, K14 + (170, 0)):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-g | G2, G3: (+130, +50); G1: (+130, +80), icon "T-g G1" on the DPRK map | 12:00 | 30:00 | 00:00 "T-g: take off and fly to the T-g G1 icon"; 12:00 "T-g: shoot down both G1 planes at the T-g G1 icon; leave the other flights alone"; 30:00 "Flight 1B done. End the mission normally" |

**0c. The flights.** Before flight 0: turn on the server's text mission log (step 0T "Server setting") and note the time the mission starts. Import each probe into a blank Korea mission with `K14 AFB_mp.Group` (the NATO spawn); flight 1B also needs a DPRK spawn at the `P14 DPRK SPAWN` icon. Run it on the dedicated server and record it (video or Tacview). End the mission normally, not by killing the server, so the last log file is written; then copy that session's `missionReport*.txt` files into `target/p14/logs/flight0/` (or `flight1a/`, `flight1b/`).
- If a T-e or T-j result in flight 0 looks wrong, fly the plain file `P14_Probe_0.Group` once (11 min). That shows whether the breadcrumbs changed the result.

**Cells:**

| Cell | Flight | Tests | Setup and procedure | Observe | Confirmed if |
|---|---|---|---|---|---|
| T-a1 | 0, 1A | AI vs player (a) | ZA1: Closer 1, `[2]`, 12 km, pulsed at 00:05. ZA2 is the same with `[1]` (control). At 00:10 the 4-plane HVAR flight (`probe_flight`, all 4 planes) starts 30 km south of the centre and flies north through it. ZA4 (a copy of ZA1, on the T-n leg of flight 1A) belongs to cell T-n: armed and pulsed at 14:00, deactivated at T-n's 26:00 close, and P flies through it. Each zone's output → its own subtitle. | Does ZA1 fire for the AI (flight 0)? Does ZA4 fire for P (flight 1A)? | a confirmed (only players count) if ZA1 stays silent until 06:00 and ZA4 fires for P. ZA2 stays silent either way. |
| T-n | 1A | F-80C cruise speed (n) | START and END: Closer 1, `[2]`, radius 2 km, 24 km apart on a north line, pulsed at 14:00, each self-deactivating. P flies level at 3050 m (the templates' F-80 altitude) at the F-80C cruise setting from the game's aircraft specifications. | Time between the "T-n START" and "T-n END" subtitles on the video; speed = 24 km / Δt. Both zones fire 2 km before their point, so the offset cancels. | n confirmed if the measured speed is within ±10 % of 732 km/h. Otherwise risk n's fallback (§9) sets the P14 cruise override to the measured value. |
| T-c | 1A | Activation while a player is inside (c, h, o) | C0 (sentinel: Closer 1, 5 km, active, pulsed at 11:00; shows P really is inside) and C1–C4 (Closer 1, 5 km), deactivated at start. P orbits inside from 10:00. At 11:00: C1 Activate only; C2 Activate + pulse 0.1 s; C3 pulse while deactivated. **C2 again (the threshold's re-check):** at 11:10 C2 is deactivated, at 11:15 it is activated and pulsed 0.1 s later, with P still inside; its second firing drives the subtitle "C2 fired again". **C4 (risk h):** at 11:30, while P still orbits C (C4's edge is 2–12 km from P), C4 Activate, then pulse 0.1 s later, so it is active and waiting; at 11:45 C4 Deactivate; at 12:00 the cue "fly east into C4" sends P into it, after the Deactivate. **C5:** Closer 1, pulsed once at 10:30. Its output → "C5 fired" (per event), and into Counter(2) → "C5 fired ≥2" and Counter(10) → "C5 fired ≥10". | Which fire, and after how long; how often C5 fires | c confirmed if C2 fires within 1 s. The re-check is confirmed if "C2 fired again" shows within 1 s of 11:15; if it does not, a check zone cannot be used twice, and the build stops at step 4. h confirmed (deactivating a waiting zone cancels it) if C4 never fires, including after P enters it at 12:00. o = "fires once" if "C5 fired" shows once and "C5 fired ≥2" never shows; "repeats" otherwise. |
| T-z | 0 | An empty Closer 0 zone, used once and used again (needed by the `Closer0Race` fallback) | Z1: Closer 0, `[1]`, 5 km, at a place with no DPRK aircraft (flight 0 has none). Activate + pulse at 10:00. Its output → "Z1 fired". **Z1 again:** Deactivate at 10:10, then Activate + pulse at 10:15; its second firing → "Z1 fired again". | Whether and when Z1 fires, both times | z confirmed if "Z1 fired" shows within 1 s of 10:00 **and** "Z1 fired again" shows within 1 s of 10:15. |
| T-e | 0 | Counter (e, r) | Automated loop of 2 s pulses from 01:00. E1 Counter 3 / Dropcount 0, 6 pulses; E2 Counter 3 / Dropcount 1, 6 pulses; E3 2 pulses → Deactivate → Activate → 1 pulse; E4 2 pulses → `ModifierSetVal` → 1 pulse; E5 2 pulses while deactivated → Activate → 1 pulse; **E6** Counter 3 / Dropcount 0, 3 pulses (fires) → `ModifierSetVal` → 3 pulses. **E7** (01:40): one timer with three targets, each a 0 s relay into one Counter 3 / Dropcount 1, so three pulses arrive in one tick. **E8** (01:50): one timer with two targets, each a 0 s relay into one Counter 1 / Dropcount 0. **E9** (02:00): three pulses 0.05 s apart into one Counter 3 / Dropcount 1 (the census spacing of §3.3). Each counter's output → its own subtitle ("E1 fired", …), and E1, E2, E6 and E8 outputs also → Counter(2) readouts ("E1 fired ≥2", …). The pulse times are constants, so the video time gives the pulse number. | When each fires | Confirmed if E1 fires on the 3rd pulse and "E1 fired ≥2" never shows; E2 fires on the 3rd pulse and "E2 fired ≥2" shows at the 6th (Dropcount 1 resets, the codebase convention); E4 does **not** fire on its 3rd pulse (the reset worked); "E6 fired ≥2" shows at E6's 6th pulse. **E7:** r = "counted one by one" if "E7 fired" shows at 01:40. **E8:** "once-gate works" if "E8 fired" shows and "E8 fired ≥2" never shows (read with the rule in 0d). **E9:** confirmed if "E9 fired" shows at 02:00.10; the budget's default census relies on this. |
| T-f | 0 | Timer Random % (f) | F1: Random 50, pulsed 40 times at 3 s from 02:30. Each hit → "F1 hit" (per event), and into Counter(12) → "F1 reached 12" and Counter(29) → "F1 reached 29". F2: 4-output waterfall re-run 40 times at 5 s from 05:30 (ends 08:45, before T-j at 09:00); each output k → "F2 out k". | Hits | Confirmed if "F1 reached 12" shows and "F1 reached 29" does not (12–28 hits), and each "F2 out k" shows at least once. |
| T-g | 1B | Kill events (g) | Three 2-ship AI F-80C flights (`probe_flight`, 2 planes) start at 12:00, each 10 km south of its own centre and attacking ground there: G2 and G3 at (+130, +50), G1 at (+130, +80), 30 km east. Each plane has OnEvents 0, 2, 4, 5, 13 → its own subtitle per type ("G1a ev4", …), and its type 4 also → a Counter(2) readout ("G1a ev4 ≥2", …). G1 is shot down by P. G2: probe `MCU_Delete` at 22:00. G3: the template's `Deactivate Units` at 22:00, then its `Trigger Delete` 0.5 s later (the `template.rs` Deactivate → Delete chain). G1 is deleted at the 30:00 close if still alive. | Which types fire, how many per plane, whether Delete fires them | Confirmed if, for each G1 plane shot down, "…ev4" shows and "…ev4 ≥2" never shows, and no G2 or G3 "ev4" subtitle shows. |
| T-j | 0 | Timer gating (j, p, q) | Automated from 09:00. J1: Deactivate a timer while it is running (Time 10 s) at 5 s. J2: pulse a deactivated relay, then Activate 1 s later (does the pulse replay?). J3: Activate + pulse in the same tick. **J4:** two pulses in the same tick (one timer with two targets into one relay) into a latch; its output → "J4 out" (per event) and a Counter(2) readout "J4 out ≥2". **J5:** a 10 s timer re-triggered at 5 s; output → "J5 out" (per event) and a Counter(2) readout "J5 out ≥2". | Each outcome | j confirmed if J2 does not replay. J1 "cancels" or "runs on" (either is handled). q confirmed if "J4 out" shows and "J4 out ≥2" never shows. p = "twice" if "J5 out ≥2" shows; otherwise "restart" (the one "J5 out" comes 15 s after the first trigger) or "ignore" (10 s after). J3 is recorded; §3.3 is safe either way. |

T-k (budget) is in Probe 2 (step 3), so it is built from the real `budget.rs`. T-b and T-d are in step 12.

**Step 0 acceptance (tests):**
- `probe_files_parse_and_links_resolve`: each of the three files parses back with `parse_group_file`, and `assert_links_resolve` passes.
- `probe_subtitles_are_unique`: within each file, every `MCU_TR_Subtitle` has a unique `LCText`.
- `probe_cells_match_the_table`: for each cell, the key MCUs exist with the table's properties, in the file the table names. For example: T-a1's flight has 4 planes, and ZA1 / ZA2 are Closer 1 with radius 12000 and coalitions `[2]` / `[1]`; ZA4 (in the flight 1A file, under T-n) is Closer 1, radius 12000, `[2]`; T-n's START and END are 24 km ± 1 m apart with radius 2000; C4 is activated at 11:30, pulsed at 11:30.1 and deactivated at 11:45, and the "fly east into C4" cue is at 12:00; C5 has one pulse and readout counters 2 and 10, each driving its own subtitle; Z1 is Closer 0, `[1]`, radius 5000; T-e E1 `Counter = 3`, `Dropcount 0`, E2 `Dropcount 1`, 6 pulses each; E6 `Counter = 3`, `Dropcount 0`, 3 pulses, a `ModifierSetVal`, 3 pulses; E7 has one source timer with three targets that all reach one `Counter = 3`; E8 one source timer with two targets that reach one `Counter = 1`, `Dropcount 0`; E9 has three pulses 0.05 s apart into one `Counter = 3`, `Dropcount 1`; C2 has a Deactivate at 11:10 and a second Activate and pulse at 11:15; T-f F1 `Random = 50`, a 40-pulse 3 s loop, readout counters 12 and 29 (`Dropcount 0`) and a per-hit subtitle; T-g has 6 planes with OnEvents 0, 2, 4, 5, 13, G1's WP 1 and AttackArea at its own centre 30 km ± 1 m east of G2's / G3's, and a "T-g G1" `MCU_Icon` at G1's centre; T-j J1 timer `Time = 10` with its Deactivate at 5 s; T-t has T1–T5 and T7 breadcrumbs at the T-t times, T4's six variants 4 s apart, T5's two firings, and T7's one breadcrumb with two same-tick inputs at 05:25; Z1 has a Deactivate at 10:10 and a second Activate and pulse at 10:15. Every `Counter(2)` readout named in 0b "Readouts" exists with `Dropcount 0` and drives its own subtitle; every per-event and readout subtitle has `Duration = 1`, and every cue subtitle `Duration = 20`.
- `probe_run_sheets_match_the_tables`: in each file, every cell centre is at `K14_ORIGIN` + its offset (± 1 m); every cue subtitle is fired by a timer from Mission Begin at the run-sheet time; every check zone of a cell is deactivated at the cell's close time (ZA4 belongs to T-n, so it is deactivated at 26:00 in the flight 1A file); T-t is armed at 04:30 and closed at 05:30.
- The ignored tests `write_p14_probe_0`, `write_p14_probe_1a` and `write_p14_probe_1b` write the files, and the user has them.

**Needs the user in game:** the server's text-log setting, 0L (an existing log, optional), and the three solo flights.

**0d. Results and supported outcomes.** Run `replay` on each flight's logs first (step 0T "0d addition"); it gives exact times for everything that logged. Write `handoff/P14-probe-results.md`, one row per cell, observed (video, cross-checked with the replay report) vs the "Confirmed if" column. Then set each strategy from the table below. This is a mechanical comparison.

| Result | Used by | If confirmed | If not confirmed | Build stops? |
|---|---|---|---|---|
| a (flight 0 + 1A) | Step 4, D30 | Only players count. The D30 warning is information only | AI counts. The D30 warning becomes an **error**: an entry is refused when another side's sortie path passes within R of its area. The user decides whether that is acceptable (§10 item 15) | No |
| c (1A) | Step 4 (CHECK and the re-check) | `PresenceCheckImpl::PulseInside`, if C2 fires **and** "C2 fired again" shows (a check zone can be used twice) | `PresenceCheckImpl::Closer0Race` is allowed only if z is confirmed, both firings. It also depends on b, which is unknown until step 12, so the manual then says that the threshold is confirmed for one player only | **Yes, at step 4, if c (both parts) and z both fail** |
| e: E1, E2, E4 (0) | Step 3 (budget) | `BudgetImpl::HeadCount` | `BudgetImpl::SingleSlot`: budget 1 per side, no counter (§3.3) | No |
| e: E6 (0) | Step 12 only | `CounterResetImpl::SetVal` | `CounterResetImpl::Ring` (§3.9) | No |
| r: E7 (0) | Step 3 (budget) | `CensusImpl::SameTick` may be chosen (shorter recount) | `CensusImpl::Spaced` (the default), if E9 is confirmed | No |
| E9 (0) | Step 3 (budget) | `CensusImpl::Spaced`: pulses 0.05 s apart are counted one by one | If E7 is confirmed, use `CensusImpl::SameTick`. If E7 and E9 both fail, a counter cannot take the census: `BudgetImpl::SingleSlot` (budget 1 per side, no counter) | No |
| f (0) | Steps 5, 6 | `RandomImpl::InMission` | `RandomImpl::Seeded` (§3.7) | No |
| g (1G; flight 2 T-l for a crash without damage) | Step 3 (LOSSES) | `LossesImpl::Type13` only (decisions 0011/0017); no 4/2/0 latch | If a crash raises no type 13, on-station end, Zone Out and the hard stop still end the sortie | No |
| h (1A) | Step 4 | INNER is deactivated as written | INNER's output goes through a relay `THsaa INNER GATE`, and `INNER OFF` / `INNER ON` switch that relay | No |
| j: J2 (0) | Steps 3–6 (every relay gate) | D11 as written | A pulse into an inactive relay is replayed when the relay is activated. Every gate in the design, and in the shipped Fighter Pack, then misfires | **Yes, before step 3** |
| j: J1 (0) | Steps 3–6 | Either outcome is handled: the `… OUT` relays cover "runs on" | — | No |
| o (1A) | Step 4 | Either outcome is handled by ENTRY OFF and the ARM latch | — | No |
| p: J5 (0) | Steps 3–6 | Generated timers are never re-triggered while running (latches, fixed-length attempts, the budget's CHECK latch) | If J5 = "twice", a once-gate goes in front of any timer the walker shows can be re-triggered. `ZoneOutImpl::Watchdog` is allowed only if J5 = "restart" | No |
| q: J4 (0) | Steps 3, 4 | Relay latches as written | If J4 outputs twice: a **single-use** latch (`DONE HUB`, `ON STATION IN`) becomes a once-gate, an `MCU_Counter` with `Counter = 1`, `Dropcount 0`, allowed only if E8 is confirmed. A **reusable** latch (`BUD s CHECK`, `THsaa ARM`) stays a relay: a once-gate cannot be re-armed by an Activate. A doubled pulse through a reusable latch starts a doubled recount or a doubled dwell; both can refuse a launch that had room, and neither can leave a side shut when nothing is flying (§3.3) | **Yes, before step 3, if J4 fails and E8 fails too:** "DONE exactly once" (A3) cannot then be kept |
| n (1A) | §4 timing | 732 km/h stays | The measured speed becomes the P14 cruise override (§4.1) | No |
| t1–t6 (0) | Step 0T defaults, step 9T, step 11 | `TraceCarrier::Objective` with the silent, logged `ObjectiveStyle` from T4; the tick rate fitted from the cue breadcrumbs | `Spawn` carrier if objectives log once only or not at all, or show on screen; debug subtitles only if neither carrier logs (the replay still reports spawns, kills and never-spawned groups) | No |
| k (Probe 2) | Step 3 | `BudgetImpl::HeadCount` | `BudgetImpl::SingleSlot` | No |
| u (decision 0017) | Step 3 | Activate-mode templates only; Spawn/Repeat templates (former S5/S6) are refused (§3.2) | No Spawn sortie in flight 2; risk u must be settled before support can be reconsidered | No |
| v (flight 2) | Step 3 | The extension needs no new order | If the order needs renewing, the shell adds `SNjj RENEW`, so the order is given again when the extension starts, and not after the sortie has ended (§3.4) | No |
| b, d | Step 12 | — | — | — |

- Every builder whose fallback is a strategy enum (`ZoneOutImpl`, `RandomImpl`, `BudgetImpl`, `CensusImpl`; `PresenceCheckImpl` until decision 0025 removed it) takes it from `AirPlan.strategies`. **Each variant of these enums, primary and fallback, gets its own structural test and at least one walker scenario** (names in the steps below). `LossesImpl` also comes from the plan but has only `Type13` (decision 0017).
- The h, p and q fallbacks are **not** strategy variants: they are small local code changes made after the probe results, only if a result calls for them. The walker already runs every scenario under `ReverseTargets` and all three re-trigger modes, so the cases they guard are exercised before then.
- **A "build stops" row means: stop, write the result into `handoff/P14-probe-results.md`, and ask the user.** Do not invent a new fallback during the build.
- **Gates (§10).** Step 3 starts after the flight 0 results are in. Step 4 starts after the flight 1A results are in. Step 2 has no engine assumptions and does not wait. Flight 1G settled result g on event 13; step 3 builds only `LossesImpl::Type13` (decisions 0011/0017). Flight 2 T-l checks a crash without damage.
- **How many times did it fire? (second review R4-3, third review T12.)** A `Counter(2)` readout proves "fired twice in one tick" only if counters count same-tick pulses one by one (E7). So the J4 and E8 results are read like this: if E7 is confirmed, the `≥2` readouts decide. If E7 is not confirmed, the trace log decides, but only if T-t **T7** showed that **one** breadcrumb fired twice in one tick writes two lines. (T3 is not enough: it fires two different breadcrumbs.) If neither holds, q is recorded as **not settled**: the build keeps the relay latches, which are the pattern every shipped template uses, and the manual says that same-tick double pulses are unverified.

### Step 0e. Probes reshaped for an autopilot pilot — S–M. Branch `codex/p14-0e`

**Why (decisions 0009 and 0010, 2026-10-12).** Claude flies the probes on the second PC from an air start with the level-flight helper: straight legs, turns and orbits, no takeoff and no shooting. Flight 1A was flown that way and gate G1 is met. Three things are left: risk n (cruise speed), the country filter (decision 0008, option 4) and risk g (kill events). **Flight 1B is retired**; flights 1N, 1D and 1G replace it. Flights 0 and 1A do not change: their files were flown and their tests pin them.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1N** | `P14_Probe_1N_traced.Group` | NATO, F-80C, air start at the `1N NATO` spawn | T-n2, Y-N | 20 min |
| **1D** | `P14_Probe_1D_traced.Group` | DPRK side as USSR, MiG-15bis, air start at the `1D USSR` spawn | Y-D | 20 min |
| **1G** | `P14_Probe_1G_traced.Group` | none; a player may sit on the ground at K14 | T-g2 | 16 min |

**Spawn points.** Each probe has a spawn list, written next to its `.Group` as `<stem>.spawns.json` (the plain and the traced file get the same list): `name`, `country`, `x`, `z`, `heading_deg`, `altitude_m`, `script`. `tools/p14/build_probe_mission.py --spawns <file>` turns each entry into a spawn airfield copied from the game's own dogfight test mission (the `fakefield` airfield of that country with its `MCU_TR_Entity`), moved to `x`, `z`, turned to `heading_deg` (`YOri`), and holding one `Plane` entry: that airfield's `StartType = 0` entry for `script`, with `Altitude = altitude_m`. Flight 1G has an empty list and uses `--ref "TemplateExamples/K14 AFB_mp.Group"` as flight 0 does.

**Run sheets.** Origin, offsets and times as in the sheets above.

**Flight 1N** (spawn `1N NATO`: country 601, `f80c10`, at (−10, +30), heading 0, 3050 m):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-n2 | START (+30, +30), END (+54, +30), icons "T-n START" and "T-n END" | 00:30 | 20:00 | 00:00 "1N: join NATO at the 1N NATO spawn at 04:00 or later. Level flight on at once, hold north at 3050 m, F-80C cruise setting" |
| Y-N | (+100, +30), icon "Y-N" | 00:10 | 20:00 | 20:00 "Flight 1N done. End the mission normally" |

**Flight 1D** (spawn `1D USSR`: country 501, `mig15bis`, at (+40, −30), heading 0, 3050 m):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| Y-D | (+140, −30), icon "Y-D" | 00:10 | 20:00 | 00:00 "1D: join the 1D USSR spawn at 04:00 or later. Level flight on at once, hold north at 3050 m"; 20:00 "Flight 1D done. End the mission normally" |

**Flight 1G** (no spawn list):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-g2 | G2: (+130, +50); G3: (+130, +53); G1: (+130, +80); G4: (+130, +110) | 01:00 | 15:00 | 00:00 "1G: no flying needed. Stay on the ground at K14 until 15:30"; 15:30 "Flight 1G done. End the mission normally" |

**Cells (flights 1N, 1D, 1G):**

| Cell | Flight | Tests | Setup and procedure | Observe | Confirmed if |
|---|---|---|---|---|---|
| T-n2 | 1N | F-80C cruise speed (n) | START and END: Closer 1, `[2]`, **radius 5 km**, 24 km apart on a north line, pulsed at 00:30, each self-deactivating (as T-n). The pilot spawns 40 km south of START on the line and holds north. On a straight track both zones fire the same distance before their point, so the radius cancels. No ZA4. | Log time between "T-n START" and "T-n END"; speed = 24 km / Δt. The results record the throttle figure and the height flown. | n confirmed if the speed is within ±10 % of 732 km/h; otherwise risk n's fallback. |
| Y-N | 1N | Country filter, NATO side (decision 0008) | At the centre: Complex Trigger **YN-CT** (cylinder, 10 km, planes, `Country = 601`, Object Entered and Object Left on) and Complex Trigger **YN-CT0** (the same with no `Country` line). Each has `OnEvent` 58 → a 0 s relay timer → its "entered" subtitle, and `OnEvent` 60 → a 0 s relay timer → its "left" subtitle; a Counter(2) and a Counter(3) readout hang off the "entered" relay. The relay timers are the traced sources, because nothing shows that a subtitle passes its input on. Both triggers are live from Mission Begin: nothing deactivates or activates a Complex Trigger before the cell close. Three plain check zones at the centre, Closer 1, `[2]`, 10 km: **YN-Z1** pulsed 00:10, **YN-Z2** pulsed 05:25, **YN-Z3** pulsed 10:40, each → its own subtitle. AI pair **YN-A** (`probe_flight`, 2 planes, both `Country = 602`) starts 00:20 at 30 km south of the centre, exit 30 km north, deleted 05:20. AI pair **YN-B** (the same with `Country = 601`) starts 05:30, deleted 10:30. The pilot arrives after 10:40. | Which triggers and zones fire in each window: A about 02:20 to 04:20, B about 07:30 to 09:30, the pilot after 10:40 | The filter works if YN-CT stays silent in window A and fires for B and for the pilot, while YN-CT0 and the Z zones fire in all three. If YN-CT0 is silent for AI, a Complex Trigger does not see AI at all, and that is the result to report instead. |
| Y-D | 1D | Country filter, DPRK side | As Y-N, mirrored: **YD-CT** `Country = 501`, **YD-CT0** unfiltered, zones **YD-Z1** to **YD-Z3** `[1]`, pair **YD-A** `Country = 503`, pair **YD-B** `Country = 501`, same times. The AI pairs are the same F-80C `probe_flight` with the country changed; the aircraft type does not matter to the trigger. | as Y-N | as Y-N, with 501 for 601 |
| T-g2 | 1G | Kill events (g) | Four 2-ship AI F-80C flights (`probe_flight`, 2 planes) start at 01:00, each 10 km south of its own centre. Each plane has OnEvents 0, 2, 4, 5, 13 → its own subtitle per type, and type 4 also → a Counter(2) readout (as T-g). **G1** is attacked: a hunter pair **G1 HUNTER** (`probe_flight`, 2 planes, `Country = 503`, `AILevel` raised to the highest value the Template Builder writes) starts at 01:00 5 km south of G1's start, and at 01:30, 03:00, 04:30 and 06:00 the same `MCU_CMD_AttackTarget` (`AttackGroup = 1`, `Priority = 1`, Objects = the hunter lead's entity, Targets = G1a's entity; shape from `TemplateExamples/CorrectedFlightfromTemplate.Group`) fires. It is repeated because the hunter keeps its template orders, and its ground AttackArea at G1's centre could replace a single Attack order. **G3 sits 3 km east of G2**, so the two flights do not start on the same point. **G4** is destroyed at 04:00 by one `MCU_CMD_Damage` per plane (`Damage = 2`, `Type = 1`, Objects = that plane's entity; shape from `TemplateExamples/Reference/CommandDamage_reference.Group`). **G2:** probe `MCU_Delete` at 10:00. **G3:** the template's `Deactivate Units` at 10:00, then its `Trigger Delete` 0.5 s later. Everything still alive, the hunters included, is deleted at the 15:00 close. | Which types fire, how many per plane, and whether a gun kill, a Damage command and a Delete differ | g confirmed if each plane that dies (G1 by guns, G4 by Damage) shows its "ev4" and never its "ev4 2+", and no G2 or G3 "ev4" shows. If the hunters do not kill G1 in time, G4 alone decides, and the results say so. |

**Step 0e acceptance (tests):**
- The step 0 tests cover the three new files in place of flight 1B's: `probe_files_parse_and_links_resolve`, `probe_strings_are_loader_safe`, `probe_blocks_match_shipped_shapes`, `probe_subtitles_are_unique`, `probe_cells_match_the_table`, `probe_run_sheets_match_the_tables`, `probe_traced_every_subtitle_source_has_a_breadcrumb`.
- `probe_blocks_match_shipped_shapes` checks `MCU_TR_ComplexTrigger` against `TemplateExamples/Reference/ComplexTrigger_reference.Group` (the same keys in the same order, `Enabled` twice), `MCU_CMD_Damage` against `CommandDamage_reference.Group` and `MCU_CMD_AttackTarget` against `CorrectedFlightfromTemplate.Group`.
- `probe_spawn_lists_match_the_run_sheets`: each flight's spawn list has the entries in the run sheets above, at `K14_ORIGIN` + the offset (± 1 m).
- Ignored writers `write_p14_probe_1n`, `write_p14_probe_1d`, `write_p14_probe_1g` write the plain file, the traced file, its `.trace.json`, the `.eng` files and the `.spawns.json` files into `target/p14/`. `write_p14_probe_1b` is removed.
- Flights 0 and 1A generate byte-for-byte the same files as before.

### Step 0f. Delete probe (flight 1H) — S. Branch `codex/p14-0f`

**Why (decision 0011).** In flight 1G one aircraft was still flying 12 minutes after the Delete that should have removed it. That Delete's Objects list also held its dead wingman. The budget releases a slot when an aircraft is deleted (U22), so a Delete that can leave an aircraft behind breaks the hard limit. Flight 1H finds out when that happens and which wiring avoids it. No pilot is needed.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1H** | `P14_Probe_1H_traced.Group` | none; a player may sit on the ground at K14 | T-d | 6 min |

**Flight 1H** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` as flight 0):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-d | D0 to D7 at (+130, +20 + 8·n) for n = 0 to 7; D8 at (+114, +20); D9 at (+114, +28) | 01:00 | 08:00 | 00:00 "1H: no flying needed. Stay on the ground at K14 until 05:30"; 05:30 "Flight 1H done. End the mission normally" |

**Cell T-d.** Ten 2-ship AI F-80C flights (`probe_flight`, 2 planes), D0 to D9, start at 01:00, each 10 km south of its own centre. Each plane's `Name` is its label (`D0a`, `D0b`, … `D9b`), so the log's end-of-mission list names every aircraft that still exists. Each plane has OnEvent 13 → its own subtitle ("D2b ev13"), traced.

| Flight | Killed at 02:00 (`MCU_CMD_Damage`, Complete) | Removed at 04:00 by | Asks |
|---|---|---|---|
| D0 | nobody | nothing | control: both must be in the end-of-mission list |
| D1 | nobody | one `MCU_Delete`, Objects = [D1a, D1b] | control: a Delete of two live aircraft removes both |
| D2 | D2b (the wingman) | one `MCU_Delete`, Objects = [D2a, D2b] | the flight 1G case: live aircraft first, dead second |
| D3 | D3a (the lead) | one `MCU_Delete`, Objects = [D3a, D3b] | dead aircraft first, live second |
| D4 | D4b | two `MCU_Delete`s, one per aircraft, fired by the same timer | candidate fix: one Delete per aircraft |
| D5 | D5b | one `MCU_Delete`, Objects = [D5a] only | candidate fix: leave the dead aircraft out of the list |
| D6 | D6b | the template's own `Deactivate Units`, then its `Trigger Delete` 0.5 s later (as G3 in flight 1G) | what a sortie's own clean-up does with a dead wingman |
| D7 | nobody | nothing | control for the detector below |
| D8 | nobody | one `MCU_Delete`, Objects = the two `Plane` blocks' own indexes (not their entities) | what Template Builder writes today: does a Delete that names the Plane blocks remove anything? |
| D9 | nobody | one `MCU_Deactivate`, Objects = [D9a, D9b]; no Delete | control for D6: what the detector and the end-of-mission list show for an aircraft that is deactivated and not deleted |

**Why D8 and D9 (found while reviewing the build, 2026-10-12).** `generate_template` fills `Trigger Delete` with the `Plane` blocks' indexes and `Deactivate Units` with their entities (`template.rs`, `delete.set_objects(object_ids)`). The six built-in sorties carry the same lists. Every example file saved from the editor lists entities in a Delete (20 files checked, none lists a Plane block). So D6, which uses the template's own chain, cannot tell "deleted" from "only deactivated" without these two.

**Detector.** At 04:30 one `MCU_CMD_Damage` (Complete) per aircraft hits every aircraft of D1 to D9. An aircraft that still exists dies and raises event 13 at 04:30; an aircraft that was removed raises nothing. D7's two aircraft must both raise it, which shows the detector works. D0 is never touched and must be in the end-of-mission list, which shows the list works. The two readings are independent.

**Confirmed if.** D1: no event at 04:30 and neither aircraft in the list. D7: two events at 04:30. D0: both in the list. Then each of D2 to D5 and D8 reads "removed" (no 04:30 event, not in the list) or "left behind" (a 04:30 event). D9 shows how a deactivated aircraft reads; D6 is then read against D9: the same as D9 means the template's Delete did nothing more than the Deactivate. If D2 or D3 leaves an aircraft behind and D4 or D5 does not, the fix is the wiring of D4 or D5. If D1 fails or D7 is silent, the flight says nothing and the cell must be redesigned.

**Caveat for reading it (from the spec review of the build).** Every Plane copied from the sortie template carries `DeleteAfterDeath = 1`, as in flight 1G. The game may remove an aircraft killed at 02:00 by itself before 04:00. D2 and D3 then test a Delete whose list names an aircraft that no longer exists, which is still the flight 1G case. D5b is killed and named in no Delete: if it raises no event at 04:30 and is not in the end-of-mission list, the game removed it by itself.

**The cell close at 08:00** removes whatever is left with one `MCU_Delete` per aircraft. The mission is ended at about 06:00, before it.

**Step 0f acceptance (tests):** the step 0 tests cover the new file (`probe_files_parse_and_links_resolve`, `probe_strings_are_loader_safe`, `probe_blocks_match_shipped_shapes`, `probe_subtitles_are_unique`, `probe_cells_match_the_table`, `probe_run_sheets_match_the_tables`, `probe_traced_every_subtitle_source_has_a_breadcrumb`); `probe_cells_match_the_table` checks every row of the table above against literals (who is damaged and when, each Delete's Objects list in order, the two Deletes of D4, D6's use of the template's own MCUs, D8's Delete listing the two Plane indexes, D9's Deactivate with no Delete, the detector's eighteen Damage commands at 04:30, one per aircraft of D1 to D9, and the plane names). An ignored writer `write_p14_probe_1h` writes the files. Flights 0, 1A, 1N, 1D and 1G generate byte-for-byte the same files as before.

### Step 0g. Delete probe with single aircraft (flight 1J) — S. Branch `codex/p14-0g`

**Why.** Flight 1H (2026-10-13, `handoff/P14-probe-results.md`) answered the Plane-index question and left the main one open: its Damage command, aimed at one aircraft of a pair, killed both, so no flight held one live and one dead aircraft when its Delete fired. Flight 1J repeats the mixed cases with aircraft that are each a flight of their own, so that a Damage command kills exactly one. No pilot is needed.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1J** | `P14_Probe_1J_traced.Group` | none | T-d2 | 6 min |

**Flight 1J** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` and `--no-resave`: the game's `MissionResaver.exe` crashes since update 1.005, and the game hosts a plain `.Mission` with its `.eng`):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-d2 | J0 to J6 at (+130, +20 + 8·n) for n = 0 to 6 | 01:00 | 08:00 | 00:00 "1J: no flying needed. Stay on the ground at K14 until 05:30"; 05:30 "Flight 1J done. End the mission normally" |

**Cell T-d2.** Seven pairs J0 to J6. Each pair is **two separate 1-ship AI F-80C flights** (`probe_flight`, 1 plane each), `x` and `y`, with their centres 1 km west and 1 km east of the pair's centre, each starting 10 km south of its own centre at 01:00. Each plane's `Name` is its label (`J0x`, `J0y`, … `J6y`). Each plane has OnEvent 13 → its own subtitle ("J2y ev13"), traced. Times as flight 1H: kills at 02:00 by `MCU_CMD_Damage` (Complete), removal at 04:00, detector at 04:30, close at 08:00 with one `MCU_Delete` per aircraft.

| Pair | Killed at 02:00 | Removed at 04:00 by | Detector at 04:30 | Asks |
|---|---|---|---|---|
| J0 | nobody | nothing | no | control: both must be in the end-of-mission list |
| J1 | nobody | one `MCU_Delete`, Objects = [J1x, J1y] | both | control: one Delete removes two live aircraft of two flights |
| J2 | J2y | one `MCU_Delete`, Objects = [J2x, J2y] | both | the flight 1G case: live aircraft first, dead second |
| J3 | J3x | one `MCU_Delete`, Objects = [J3x, J3y] | both | dead aircraft first, live second |
| J4 | J4y | two `MCU_Delete`s, one per aircraft, fired by the same timer | both | candidate fix: one Delete per aircraft |
| J5 | J5y | one `MCU_Delete`, Objects = [J5x] only | both | candidate fix: leave the dead aircraft out of the list |
| J6 | J6y | nothing | both | control: the Damage command kills only J6y (J6x must raise event 13 at 04:30, not at 02:00), and whether a dead aircraft raises it again |

**Confirmed if.** J0: both in the end-of-mission list. J1: no event at 04:30, neither in the list. J6: J6y's event at 02:00 only, J6x's event at 04:30 only. Then, for the live aircraft of J2 to J5 (J2x, J3y, J4x, J5x): "removed" is no event at 04:30 and not in the list; "left behind" is an event at 04:30. If J2 or J3 leaves its live aircraft behind and J4 or J5 does not, a sortie's clean-up must not name a dead aircraft in a Delete that also names a live one. If J2 and J3 both remove it, a dead aircraft in the list is harmless and flight 1G's survivor has another cause.

**Step 0g acceptance (tests):** the step 0 tests cover the new file; `probe_cells_match_the_table` checks every row above against literals (fourteen 1-ship flights, who is damaged, each Delete's Objects list in order, the two Deletes of J4, twelve detector commands for J1 to J6, the plane names). An ignored writer `write_p14_probe_1j` writes the files. Flights 0, 1A, 1N, 1D, 1G and 1H generate byte-for-byte the same files as before.

### Step 0h. What can keep an aircraft from being deleted (flight 1K) — S. Branch `codex/p14-0h`

**Why.** Flight 1J showed that a dead aircraft in a Delete's list is harmless, so flight 1G's survivor (G1a) has another cause. That case had three things no probe has isolated: the two aircraft were one formation and one of them was gone; the survivor had been damaged or had decided to return to base; and it was in a fight. Flight 1K takes each alone, and then repeats flight 1G's fight twice with two ways of writing the Delete. No pilot is needed.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1K** | `P14_Probe_1K_traced.Group` | none | T-d3, T-d4 | 16.5 min |

**Flight 1K** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` and `--no-resave`):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-d3 | K0 to K7 at (+130, +20 + 8·n) for n = 0 to 7 | 01:00 | 20:00 | 00:00 "1K: no flying needed. Stay on the ground at K14 until 16:00" |
| T-d4 | K8 at (+60, +20); K9 at (+60, +60); K10 at (+60, +100) | 01:00 | 20:00 | 16:00 "Flight 1K done. End the mission normally" |

Every pair in both cells is **one 2-ship formation** of AI F-80Cs (`probe_flight`, 2 planes), starting 10 km south of its centre at 01:00. Plane `Name`s are the labels (`K0a`, `K0b`, … `K10b`); each plane has OnEvent 13 → its own subtitle, traced. The cells close at 20:00 with one `MCU_Delete` per aircraft.

**Cell T-d3: one thing at a time.** Times as flights 1H and 1J unless the row says otherwise.

| Pair | At 02:00 | At 04:00 | Detector at 04:30 | Asks |
|---|---|---|---|---|
| K0 | nothing | nothing | no | control: both in the end-of-mission list |
| K1 | nothing | one `MCU_Delete`, Objects = [K1a] (the lead only) | both | does a Delete of the lead take the wingman with it? |
| K2 | nothing | one `MCU_Delete`, Objects = [K2b] (the wingman only) | both | does a Delete of the wingman take the lead with it? |
| K3 | one `MCU_Delete`, Objects = [K3b] | one `MCU_Delete`, Objects = [K3a, K3b] | both | a formation that has lost a member, and a list that still names it (flight 1G without the gunfire) |
| K4 | `MCU_CMD_Damage` with `Damage = 1`, Objects = [K4a] | one `MCU_Delete`, Objects = [K4a, K4b] | both | does a Delete remove a damaged formation? |
| K5 | `MCU_CMD_Damage` with `Damage = 0`, Objects = [K5a] | one `MCU_Delete`, Objects = [K5a, K5b] | both | the same at the lower level |
| K6 | `MCU_CMD_Damage` with `Damage = 1`, Objects = [K6a] | nothing | no | control for K4: what level 1 does. Event 13 at 02:00 means it kills; if both are in the end-of-mission list, their height and place show whether they turned for home |
| K7 | `MCU_CMD_Damage` with `Damage = 0`, Objects = [K7a] | nothing | no | control for K5 |

`Damage = 2` is "Complete" in the editor (the reference file). The editor's other two levels are 0 and 1 by position ("Minor" is the default); which name goes with 1 was not read. `Type = 1` as in the reference file.

**Cell T-d4: the fight.** Each pair has two DPRK-country (503) MiG-15bis hunters at ace skill, written as in flight 1G: they start 5 km behind the pair's start, and an `MCU_CMD_AttackTarget` (hunter lead → the pair's lead) is issued at 01:30 and every 90 s after it (K8: to 03:00; K9 and K10: to 13:30), and K9 and K10 get one more at 14:30. The 14:30 order is off the 90 s step: the build prompt asked for it and the flight was flown with it. Hunter plane `Name`s are `K8Ha`, `K8Hb`, and so on, with OnEvent 13 → a traced subtitle each. The cells are 40 km apart and 70 km from T-d3 so that no fight spills over.

| Pair | Delete | Detector | Asks |
|---|---|---|---|
| K8 | 04:00: one `MCU_Delete`, Objects = [K8a, K8b] | 04:30, both | does a Delete remove aircraft that are being attacked? |
| K9 | 15:00: one `MCU_Delete`, Objects = [K9a, K9b] | 15:30, both | flight 1G again: whatever the fight has done by 15:00 |
| K10 | 15:00: one `MCU_Delete` per aircraft, fired by one timer | 15:30, both | the same fight with the other wiring |

The hunters are never deleted before the close and get no detector.

**How to read it.** K0 must be in the list. For each of K1 to K5 and K8 to K10: an aircraft with no event at its detector time and not in the list was removed; an event at the detector time means it was still there. K9 and K10 depend on what the hunters do, so they may show nothing: they count only if one aircraft of the pair has died by gunfire before 15:00 and the other has not. If K9's survivor is left behind and K10's is not, one Delete per aircraft is the fix. If both are left behind, the cause is the aircraft's state, and T-d3's rows say which.

**Step 0h acceptance (tests):** the step 0 tests cover the new file; `probe_cells_match_the_table` checks every row of both tables against literals (eleven formations and six hunters, each command's time, kind, `Damage` level and Objects list in order, the two Deletes of K10, the detector lists, the Attack times of each hunter pair, plane names and countries). An ignored writer `write_p14_probe_1k` writes the files. Flights 0, 1A, 1N, 1D, 1G, 1H and 1J generate byte-for-byte the same files as before.

### Step 0i. The app's own logic in the game (flight 1L) — M. Branch `codex/p14-0i`

**Why.** Three things wait on one flight.
1. **Decision 0012** (which event counts a destroyed aircraft). The user asked for the test that settles it. It runs what `generate_template` writes for a repeat-spawn group, as written today (event 4) and with event 13, and watches for the next wave.
2. **Decision 0013** (Accepted: `Trigger Delete` lists entities). Today that Delete does nothing, so no re-activation or respawn has ever met units that are really gone. The flight runs both bring-up modes with the corrected list before the app is changed.
3. **Flight 1K's reading** that an AI aircraft which has decided to return to base stops taking commands. The flight repeats the damaged pairs with `AiRTBDecision = 0`.

No pilot is needed.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1L** | `P14_Probe_1L_traced.Group` | none | T-r, T-v, T-w | 24.5 min |

**Flight 1L** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` and `--no-resave`):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-r | R0 to R3 at (+130, +20 + 8·n) for n = 0 to 3 | 01:00 | 26:00 | 00:00 "1L: no flying needed. Stay on the ground at K14 until 24:00" |
| T-v | V1 at (+130, +52); V2 at (+130, +60); V3 at (+130, +68) | 01:00 | 26:00 | none |
| T-w | W1 to W6 at (+90, +20 + 8·n) for n = 0 to 5 | 01:00 | 26:00 | 24:00 "Flight 1L done. End the mission normally" |

The cells close at 26:00 with one `MCU_Delete` per aircraft (entities).

**Cell T-r: the AI's own decision to return to base.** Each pair is one 2-ship formation of AI F-80Cs (`probe_flight`, 2 planes), starting 10 km south of its centre at 01:00. Plane `Name`s are the labels (`R0a` … `R3b`); each plane has OnEvent 13 → its own subtitle, traced.

| Pair | `AiRTBDecision` on both planes | At 02:00 | At 05:00 | Detector at 05:30 | Asks |
|---|---|---|---|---|---|
| R0 | 1 (as the template has it) | `MCU_CMD_Damage`, `Damage = 1`, Objects = [R0a] | one `MCU_Delete`, Objects = [R0a, R0b] | both | flight 1K's K4 with three minutes for both to decide: are both left? |
| R1 | **0** | the same | the same | both | does switching the decision off make them deletable? |
| R2 | **0** | the same | nothing | no | control for R1: with the decision off, do they still go home? (end-of-mission place and height) |
| R3 | 1 | the same | one probe-owned `MCU_Deactivate`, Objects = [R3a, R3b] | no | does a Deactivate reach an aircraft that is going home? (frozen near where it was at 05:00, or near K14) |

**Cell T-v: every event, for three ways to end.** Each is one single-aircraft flight (`probe_flight`, 1 plane) with OnEvents **0 to 13** on its entity, each type → its own subtitle (`V1 ev0` … `V1 ev13`), traced.

| Aircraft | What happens to it | Asks |
|---|---|---|
| V1 | 02:00: `MCU_CMD_Damage` at `Damage = 2` | the full set of events for an outright kill (flight 1G listened for five types) |
| V2 | written with `Fuel = 0.03` | what a fuel-starved aircraft raises: bingo fuel (7), a landing (5), a crash (2), a kill (13) |
| V3 | 02:00: `MCU_CMD_Damage` at `Damage = 1`; `AiRTBDecision` stays 1; nothing else | what an aircraft raises when it is damaged, goes home and lands; and whether it is still in the end-of-mission list at 24:30 (each plane has `DeleteAfterLand = 1`) |

**Cell T-w: what `generate_template` writes.** Six groups, each built by `generate_template` from two **independent** F-80C seats with no orders (a `Follows` seat forces Activate mode), then placed and started by the probe the way `probe_flight` treats a sortie: the group's `Translator Mission Begin` is cut off, its two check zones are never pulsed, and the probe pulses the group's own nodes. Nothing else in the group is removed; in particular `DeathCount`, `COOLDOWN`, `SpawnCount` and `Trigger Spawner` stay. Plane `Name`s are `W1a`, `W1b`, and so on. Each unit entity also gets a traced OnEvent 13 subtitle; that adds a listener and changes none of the group's own links. The probe traces the output of `DeathCount`, `COOLDOWN` and `SpawnCount` in each group that has them.

| Group | Options | Changed after generation | The probe pulses | Asks |
|---|---|---|---|---|
| W1 | `bring_up = Spawn`, `allow_multiple_spawns = true` | `COOLDOWN` `Time` set to 60 | 01:00 `SPAWN UNITS`; 03:00 a kill (`Damage = 2`) on each unit entity; 06:00 detector on each | today's wiring: does a second wave come after the first is killed? |
| W2 | the same | the same, and each unit's `OnEvent` to `DeathCount` changed from type 4 to **13** | the same | the same with event 13 |
| W3 | the same | `COOLDOWN` 60; OnEvent type 13; nothing else | 01:00 `SPAWN UNITS`; 03:00 `MISSION END`; 04:00 `SPAWN UNITS` again; 06:00 detector on each | today's Delete (Plane indexes) in the spawn loop: a clean-up, then a new wave |
| W4 | the same | as W3, and `Trigger Delete` `Objects` set to the unit **entities** | the same | the corrected Delete in the spawn loop: does the new wave still come? |
| W5 | `bring_up = Activate` | nothing | 01:00 `MISSION BEGIN`; 03:00 `MISSION END`; 04:00 `MISSION BEGIN` again; 06:00 detector on each | today's Delete with re-activation: the group comes back when a player returns |
| W6 | `bring_up = Activate` | `Trigger Delete` `Objects` set to the unit **entities** | the same | the corrected Delete with re-activation: does the group still come back? |

**How to read T-w.** "A wave is there" means its aircraft die to a kill or detector command (event 13 and Kill records at that time) or are in the end-of-mission list; a spawned wave should also show as new objects in the log.
- **0012:** W1 against W2. If W2's `DeathCount` and `COOLDOWN` fire after 03:00 and a second wave dies at 06:00, while W1 shows none of that, event 13 is the fix. If W1 also sends a wave, event 4 does fire for spawned aircraft and decision 0012 needs no change.
- **0013, spawn mode:** W4 against W3. If W4's second wave dies at 06:00, the corrected Delete does not break the spawn loop.
- **0013, activate mode:** W6 against W5. If W5's aircraft die at 06:00 and W6's do not, the corrected Delete ends a group for good the first time its players leave, where today it comes back. That would be a change in behaviour for the user to rule on before the app is changed.
- Whether a Damage command aimed at a unit's entity reaches a **spawned** copy is not known. If the 03:00 kills in W1 and W2 kill nothing, the row cannot be read and the kill must be made another way.

**Step 0i acceptance (tests):** the step 0 tests cover the new file; `probe_cells_match_the_table` checks every row of the three tables against literals (the `AiRTBDecision` and `Fuel` values written, each command's time, kind, level and Objects, the fourteen event types on V1 to V3, each W group's options as seen in its nodes, the event type on its units, its `Trigger Delete` list, its `COOLDOWN` time, and the nodes the probe pulses with their times). An ignored writer `write_p14_probe_1l` writes the files. The earlier flights generate byte-for-byte the same files as before. **`src/template.rs` is not changed by this step.**

### Step 0j. Upstream's strike and CAP, and the fixes as generated (flight 1M) — M. Branch `codex/p14-0j`

**Why.** Four things in one no-pilot flight.
1. **Upstream's new strike and CAP missions** send the flight home on entity event 7 (bingo fuel). In flight 1L event 7 fired three seconds after activation on aircraft with full tanks. The flight runs upstream's own generated files, with listeners, to see whether a strike turns for home before it attacks. The result goes into an issue for the upstream author (`handoff/upstream-issue-draft.md`).
2. **Decisions 0012 and 0014** are now in `generate_template`. The flight runs groups the changed code generates, with nothing edited afterwards.
3. **Decision 0015:** a sortie placed by `place_air_sortie` has `AiRTBDecision = 0`. Flight 1L tested one pair; this adds a 4-ship, the same 4-ship with the setting on, and a single aircraft with it on.
4. **Decision 0016 (proposed)** leans on a re-triggered timer restarting and firing once. That was measured on game 1.004; cell T-j is flown again on 1.005.

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1M** | `P14_Probe_1M_traced.Group` | none | T-u, T-x, T-y, T-j | 14.5 min |

**Flight 1M** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` and `--no-resave`):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-u | U1 (+130, +20); U2 (+50, +100); U3 (+130, +60); U4 (+90, +20); U5 (+90, +60) | 01:00 | 16:00 | 00:00 "1M: no flying needed. Stay on the ground at K14 until 14:00" |
| T-x | X1 (+10, +60); X2 (+10, +68); X3 (+10, +76) | 01:00 | 16:00 | none |
| T-y | Y1 (+50, +20); Y2 (+50, +36); Y3 (+50, +52) | 01:00 | 16:00 | 14:00 "Flight 1M done. End the mission normally" |
| T-j | as in flight 0 (automated from 09:00) | as in flight 0 | as in flight 0 | none |

The cells close at 16:00 with one `MCU_Delete` per aircraft (entities).

**Cell T-u: upstream's own files.** Each group is a copy of a file in `src/testdata/p14_probe_inputs/upstream_a31ad0a/` (written by upstream's code at commit `a31ad0a`; see the README there). The probe places the copy so that the group's origin (its `Zone IN`) is at the row's centre, cuts its `Translator Mission Begin` off, never pulses its check zones, and pulses its own `MISSION BEGIN` at 01:00. That is what `Zone IN` would do when a player arrives.

Changes to a copy are limited to: new indexes and positions; plane `Name`s (`U1a` … `U5d`, so the log's end-of-mission list can be read); **added** listeners; and, in the two control rows, the removal named in the table. No link the app wrote is changed otherwise.

Listeners added: on every plane entity one traced OnEvent subtitle for each of the row's four abort types and for 13 and 5 (`U1a ev7`, …); and a trace on the output of each order timer in the group (`Formation 1`, `Goto WP 1`, `AttackArea 1`, `OnAreaAttacked 1`, `Goto WP 2`, `Mission Complete 1`, `MISSION END`, `DELAYED END ORDERS` in a strike; `Goto WP 1`, `Goto WP 2`, `Goto WP 3`, `Return to WP 1`, `Goto WP 5` in the CAP).

| Row | File | Change beyond the list above | Asks |
|---|---|---|---|
| U1 | `Upstream_Strike_NATO_F80C_4ship.Group` | none | does the strike reach its `AttackArea`, or does `Goto WP 2` (home) fire first, and on which event? |
| U2 | `Upstream_Strike_DPRK_Il10_4ship.Group` | none | the same for a DPRK strike |
| U3 | `Upstream_Strike_NATO_F80C_4ship.Group` | the four `OnEvent` type 7 hooks to `Goto WP 2` are removed | control: without the bingo-fuel abort, does the strike attack and come home? |
| U4 | `Upstream_CAP_NATO_F86A_4ship.Group` | none | does the CAP patrol, or does `Goto WP 5` (home) fire at once? |
| U5 | `Upstream_CAP_NATO_F86A_4ship.Group` | the four `OnEvent` type 7 hooks to `Goto WP 5` are removed | control for U4 |

**How to read T-u.** For each strike: the times of `Goto WP 1`, `AttackArea 1`, `OnAreaAttacked 1`, `Goto WP 2`, `Mission Complete 1` and `MISSION END`, and the time of each abort event on each aircraft. "Aborted at once" is `Goto WP 2` firing before `AttackArea 1`, within a second of an event 7. For each strike that reaches `MISSION END`: its four aircraft are then deactivated; if they are in the end-of-mission list, the upstream `Trigger Delete` removed nothing (decision 0013, on upstream's own file). For the CAP: whether `Goto WP 5` fires, and when.

**Cell T-x: the changed generator, as generated.** Three groups written **once** by the changed `generate_template` into `src/testdata/p14_probe_inputs/flight1m/` (two independent F-80C seats, no orders; `spawn_cooldown_min` set as an option so that `COOLDOWN` is 60 s) and loaded from there. A test checks that the current generator still writes the same bytes, so a later change to the generator is seen. Nothing in a group is edited after generation except indexes, positions, plane `Name`s and added traced listeners (OnEvent 13 per unit; the outputs of `DeathCount`, `COOLDOWN`, `SpawnCount`).

| Group | Options | The probe pulses | Must show |
|---|---|---|---|
| X1 | `Spawn`, `allow_multiple_spawns` | 01:00 `SPAWN UNITS`; 03:00 a kill on each unit entity; 06:00 detector on each | 0012: `DeathCount` at 03:00, `COOLDOWN` at 04:00, a second wave dies at 06:00 |
| X2 | the same | 01:00 `SPAWN UNITS`; 03:00 `MISSION END`; 04:00 `SPAWN UNITS`; 06:00 detector on each | 0014, Spawn mode: the new wave dies at 06:00, and the end-of-mission list holds one X2a and one X2b |
| X3 | `Activate` | 01:00 `MISSION BEGIN`; 03:00 `MISSION END`; 04:00 `MISSION BEGIN`; 06:00 detector on each | 0014, Activate mode: no `DELETE DELAY` or `Trigger Delete` in the file, and both aircraft die at 06:00 (the group came back) |

**Cell T-y: return to base, larger groups.** F-80C HVAR sortie copies, each starting 10 km south of its centre at 01:00, OnEvent 13 traced per plane.

| Row | Built by | `AiRTBDecision` | At 02:00 | At 05:00 | Detector at 05:30 | Asks |
|---|---|---|---|---|---|---|
| Y1 | `place_air_sortie` on the 4-ship (decision 0015 in the code) | 0, set by `place_air_sortie` | `MCU_CMD_Damage`, level 1, Objects = [Y1a] | one `MCU_Delete`, Objects = the four entities | all four | are all four removed? |
| Y2 | `probe_flight`, 4 planes | 1 (as the template has it) | the same | the same | all four | how many of four stop answering? |
| Y3 | `probe_flight`, 1 plane | 1 | the same on Y3 | one `MCU_Delete`, Objects = [Y3] | Y3 | does a lone damaged aircraft stop answering? |

**Cell T-j** is flight 0's cell, unchanged (J1 to J5). It is here for J5: a 10 s timer re-triggered at 5 s must fire once, 15 s after the first trigger.

**Step 0j acceptance (tests):** the step 0 tests cover the new file; `probe_cells_match_the_table` checks every row of the tables against literals. For T-u it checks that, apart from the listed changes, each copy's nodes, links, properties and plane blocks equal the source file's (compare after undoing the index and position shift), and that U3 and U5 differ from U1 and U4 by exactly the four removed hooks. An ignored writer `write_p14_probe_1m` writes the files. The earlier flights generate byte-for-byte the same files as before.

### Step 0k. Upstream's go-home fix and Fighter Pack's count (flight 1N) — M. Branch `codex/p14-0k`

> **Built, merged (`b92df87`) and flown on 2026-10-14 as flight 1P**, not 1N: flight 1N already exists (2026-10-12) and its files must not change. Results: `handoff/P14-probe-results.md`, "Flight 1P". Two changes to the tables below were made before the build. In cell T-f the 03:00 kill is one level 2 Damage command per aircraft, because Fighter Pack aircraft are independent formations. Cell T-b's aircraft come from `bundled_catalog()`, because `builtin_plane_catalog()` holds only the seven fighters; `simpleb29` is left out.

**Why.**
1. **Upstream's Il-10 strike never got home** in flight 1M: its go-home timer fired two or three times, and the flight was left with no orders. The proposed fix for the upstream issue (`handoff/upstream-issue-draft.md`, section 3) is a once-only gate before that timer. This flight proves or refutes it, with a second variant that drops the Force Complete instead.
2. **Decision 0016** (accepted 2026-10-14) changes Fighter Pack so that each aircraft is counted once. This flight flies a Fighter Pack flight with and without the change.
3. **Fighter Pack also sends a flight home on event 7** (`FIGHTER_EVENTS` in `flights.rs`: critical damage, pilot wounded, bingo guns, **bingo fuel**). In flights 1L and 1M event 7 fired on the F-80C and F-86A 3 s after activation. If it does the same in a Fighter Pack, every jet flight goes home and is cleaned up and replaced about a minute after it spawns. Suspected from the wiring and the flight data; not flown.
4. **Which aircraft raise event 7 at once.** Only the F-80C, F-86A-5 and Il-10 have been watched. Cell T-b watches every aircraft type the app offers.

No pilot is needed. Before the probe is built, 0016 is built (prompt in `handoff/P14-step-0k-codex-prompts.md`, job 1).

| Flight | File | Pilot | Cells | Length |
|---|---|---|---|---|
| **1N** | `P14_Probe_1N_traced.Group` | none | T-h, T-f, T-b | 14.5 min |

**Flight 1N** (no spawn list; built with `--ref "TemplateExamples/K14 AFB_mp.Group"` and `--no-resave`):

| Cell | Centre (N, E km) | Armed | Closed | Cues |
|---|---|---|---|---|
| T-h | H1 (+50, +20); H2 (+50, +60); H3 (+50, +100); H4 (+130, +20) | 01:00 | 16:00 | 00:00 "1N: no flying needed. Stay on the ground at K14 until 14:00" |
| T-f | F1 (+90, +20); F2 (+90, +60); F3 (+90, +100) | 01:00 | 16:00 | 14:00 "Flight 1N done. End the mission normally" |
| T-b | B1 to B12 at (+10, +20 + 8·n) for n = 0 to 11 | 01:00 | 16:00 | none |

The cells close at 16:00 with one `MCU_Delete` per aircraft (entities).

**Cell T-h: upstream's go-home step.** Copies of the files in `src/testdata/p14_probe_inputs/upstream_a31ad0a/`, placed and started exactly as cell T-u of flight 1M (Zone IN on the row centre, Mission Begin cut, `MISSION BEGIN` pulsed at 01:00, planes renamed, listeners added on every abort event, 13 and 5, and on every order timer).

| Row | File | Change | Asks |
|---|---|---|---|
| H1 | `Upstream_Strike_DPRK_Il10_4ship.Group` | none | control: as in flight 1M, does the go-home fire more than once and the flight stay near the target? |
| H2 | the same | **once-only gate**: one new `MCU_Counter` (`Counter = 1`, `Dropcount = 0`) whose `Targets` are `[Goto WP 2]`; every abort `OnEvent` that targeted `Goto WP 2` now targets the counter | does the flight go home and reach `Mission Complete`? (the fix proposed upstream) |
| H3 | the same | `Goto WP 2`'s `Targets` lose `Force Complete - High`; nothing else | does the flight go home without the Force Complete, even when the timer fires more than once? |
| H4 | `Upstream_Strike_NATO_F80C_4ship.Group` | the four event 7 hooks removed **and** H2's once-only gate | the whole proposed fix on a jet strike: attack, go home, `Mission Complete` |

The new counter is written from an existing probe counter block (`p.count` or the reference it uses), not from memory.

**Cell T-f: Fighter Pack's count.** One Fighter Pack flight of four F-80Cs per row, generated by the app's own Fighter Pack code (`flights.rs`, `configure_aircraft` / `install_pack_logic`) and frozen as a fixture under `src/testdata/p14_probe_inputs/flight1n/` like flight 1M's T-x groups. Started by pulsing its own bring-up node at 01:00, as T-x did. Each plane entity gets a traced OnEvent 13 subtitle; the outputs of the flight's `DeathCount`, `COOLDOWN` and each `Delay Delete n` are traced.

| Row | Wiring | At 03:00 | Must show |
|---|---|---|---|
| F1 | as the app generates it with decision 0016, unchanged | one `MCU_CMD_Damage` (level 2) on whatever aircraft of the flight still exist | **suspected fault:** does event 7 at about 01:03 send the flight to `CLEANUP n`, and is the flight then switched off, counted and replaced about a minute later? |
| F2 | F1 with the four event 7 hooks removed (a named change in the probe) | one `MCU_CMD_Damage` (level 2) on the flight's lead; it kills the whole formation (flight 1H) | **0016:** `DeathCount` fires **once**, about the clean-up delay after the kills; one replacement |
| F3 | F2 with the four death events pointed back at `DeathCount` (what 0012 alone would ship) | the same | control: `DeathCount` fires **twice** (at the kill and again after the clean-up delay) |

If a pack's `DeathCount` `Counter` is not the number of aircraft in the flight, or if the replacement chain needs a zone, the builder reports it and the row is redesigned before the flight.

**Cell T-b: which aircraft raise event 7.** Twelve single aircraft, one of each plane in `builtin_plane_catalog()` (F-80C-10, F-84E, F-86A-5, F-51D, B-29, C-47B, MiG-15bis, La-11, Yak-9P, Il-10, Tu-2, Li-2T), each generated by `generate_template` (one independent seat, Activate mode, an air start at the template's default height, no orders) and frozen as a fixture under `src/testdata/p14_probe_inputs/flight1n/`. Started at 01:00 by pulsing `MISSION BEGIN`. Each entity gets traced OnEvent subtitles for types 7, 8, 9, 13 and 5. Plane `Name`s `B1` … `B12` in catalog order; the README beside the fixtures lists which is which.

**How to read it.** T-b: which aircraft raise event 7, and when, without any damage. T-h: for each row the times of `AttackArea 1`, each `Goto WP 2` firing, `Mission Complete 1` and `MISSION END`, and where the aircraft are at the end. T-f: the number of `DeathCount` outputs and their times, and the replacement spawns. Leave the server after 14:00 and before 16:00. If the server reloads the mission as it is left, let the second run go on: it is a repeat sample (flight 1M's was).

**Step 0k acceptance (tests):** the step 0 tests cover the new file; `probe_cells_match_the_table` checks every row against literals, and for T-h that each copy equals its source apart from the listed changes, as for flight 1M. A test generates the T-f pack and the T-b aircraft again and compares them with the frozen files. An ignored writer `write_p14_probe_1n` writes the files. The earlier flights generate byte-for-byte the same files as before.

### Step 0T. Trace builds and replay core (P10) — M. Branch `claude/p14-trace`

**Why (U12).** Subtitles show what fired only to whoever is watching, with no file and no "never fired". The server's text mission log records spawns, kills, takeoffs and landings anyway. A Mission Objective MCU writes an `AType:8` line when it fires, so a small objective hung on any MCU turns that MCU's firing into a log line (a **breadcrumb**). The app knows the whole MCU graph, so it can report what fired, what never fired, and where each chain stopped. It is built **before** probe flight 0 so the flight produces a real log, and cell T-t (below) tests whether it works.

Everything in this step is **UNVERIFIED until flight 0** (risks t1–t6, §9): the log line format, what `AType:8` carries, and whether an objective shows anything to players. Build against the community-documented format, keep the parser tolerant, and treat the flight 0 log as the real fixture.

**0L. A real log first (user, optional but preferred).** If the dedicated server already writes text logs, the user copies one set of `missionReport*.txt` files from a past session into `src/testdata/missionlog/real_1/` (trimmed by the implementing agent to under 200 kB). If none exist, build against the documented format with a synthetic fixture, and add the flight 0 log as `real_1` in 0d.

**Server setting (user, before flight 0).** Turn text logging on for the dedicated server. The community-documented setting is `mission_text_log = 1` in the server's `startup.cfg` (`[KEY = system]`); the files land in `data\logs\text\`. The exact key and folder are **UNVERIFIED**: the implementing agent confirms them from current IL-2 documentation and writes them into the step's stop report and the manual.

**Files:** new `src/trace.rs` (instrumentation + sidecar) and `src/missionlog.rs` (parser, replay, report, CLI). `main.rs` gets `mod trace; mod missionlog;` and one CLI line next to the existing `heightprobe::run_cli` hook (`main.rs` ≈44-47): `if let Some(code) = missionlog::run_cli(&args) { std::process::exit(code); }`. Both are general tools, not P14-only, so they sit at the top level of `src/`, not under `airtask/`. The `mod` lines, the CLI line and stubs with the signatures below are part of the **1a skeleton** (§10), so 0T, 0b and step 1 each fill their own files.

**0T-1. Trace instrumentation** (`trace.rs`):
- `pub struct TraceSelect { pub block_types: Vec<String>, pub name_prefixes: Vec<String>, pub indexes: Vec<i32>, pub event_types: Vec<i32>, pub random_timers: bool }`. An MCU is traced if any rule matches. `event_types` names the entity event types to trace (T-g passes 0, 2, 4, 5, 13); `random_timers` selects every `MCU_Timer` with `Random < 100`, which the three lists cannot express. `TraceSelect::decision_points()` = `MCU_CheckZone`, `MCU_Counter`, every `MCU_Timer` with `Random < 100`, `MCU_TR_MissionBegin`, `MCU_Spawner`, plus every name prefix the caller adds (P14 passes its START / DONE HUB / EXTEND / SLOT OPEN / GO / FIRE / PASS / `BUD s CHECK` names).
- `pub fn instrument(root: &mut Il2Entity, sel: &TraceSelect, next_id: &mut i32, carrier: TraceCarrier, map: &mut TraceMap)`. For each traced source it adds exactly one breadcrumb, registered in `map` (numbering continues from the entries already there). An MCU source gets the breadcrumb's index appended to its `Targets`, so the breadcrumb fires whenever the MCU outputs. An **entity source** (a plane's or vehicle's `MCU_TR_Entity` whose events are wired through `OnEvents` / `OnReports`, as in T-g) gets a new `OnEvent` with the same `Type` and the breadcrumb as `TarId`, once per event type the selection names. It changes nothing else.
- `pub fn breadcrumb(carrier: TraceCarrier, map: &mut TraceMap, next_id: &mut i32, source: TraceSource) -> Vec<Il2Entity>` builds one breadcrumb (the blocks to add: one objective, or Spawner + object + Delete) and registers it. `instrument` uses it, and so do hand-built probes such as T-t. `TraceMap::push(entry)` registers a breadcrumb built elsewhere. Both signatures are in the 1a stubs.
- `pub enum TraceCarrier { Objective(ObjectiveStyle), Spawn }`:
  - `Objective(style)` (primary): an `MCU_TR_MissionObjective` with `TaskType 0`, `IconType 0`, and the `Coalition` / `Success` / `LCName` choice given by `ObjectiveStyle` (default = the variant cell T-t shows is silent to players and logged; until then `Coalition 0, Success 1, no LCName`).
  - `Spawn` (fallback, risk t1 or t3): an `MCU_Spawner` that spawns a small vehicle named `TRACE <n>`, then an `MCU_Delete` of it after 1 s. Use a `Vehicle` with a linked `MCU_TR_Entity`, copied from a sample such as the WillysMB `"NOICON"` in `TemplateExamples/K14 AFB_mp.Group` ≈1519 (Index 117, entity ≈1564), set to a neutral country and not engageable. Nothing in IL-2 is truly hidden, so every spawn-carrier object appears at one fixed dry-land point `TRACE_SPAWN_POINT` far from the front (checked with the water map), and the log line is matched by **name**, not position. The spawn writes the log's spawn line; a deleted object may write nothing. It costs one short-lived object per firing, so use it only if objectives fail.
- **Breadcrumb IDs.** Objective breadcrumb n sits on a **trace grid**: (`TRACE_X0` + 10·(n mod 3000) m, `TRACE_Z0` + 10·(n div 3000) m), with `TRACE_X0` = `TRACE_Z0` = 5 000. That is inside the whole-map bounds of `geo.rs` ≈23-24 (0 to 499 200) and outside the placement area of `placement.rs` ≈30-32 (40 000 to 470 000), so it never meets the units the app places or the parking grid that starts at `placement::MAP_MIN` (≈43). A front line or an imported army can lie anywhere on the map, so the grid is kept clear only of what the app itself places. Breadcrumbs are matched by index or by their own position, so an object near one does no harm. Rows are 30 km long. Pin the constants and both bound checks in a test. The log line is matched by the objective's index if `AType:8`'s `OBJID` turns out to be the MCU index, otherwise by position (± 1 m). Which one works is risk t2; the sidecar holds both.
- `pub struct TraceMap { pub entries: Vec<TraceEntry>, pub edges: Vec<TraceEdge> }`, `TraceEdge { from: i32, to: i32, unconditional: bool, delay_s: f64 }`, `TraceEntry { breadcrumb_index, source_index, source_name, source_type, event_type: Option<i32> (set for an entity source), group_path, pos: (f64, f64), carrier, spawn_name: Option<String>, expected_s: Option<f64> }`, serialised as `<mission>.trace.json` next to the `.Group` with `write_trace_sidecar(path, &TraceMap)`. `expected_s` is the mission time at which the source is known to fire, filled for timers started directly by Mission Begin (the probe's cue timers: their `Time`); the replay fits the tick rate from these. `edges`: for each traced source, the traced sources that its **pulse** can reach downstream through untraced MCUs. The walk follows **pulse links only**: the `Targets` of timers, counters, check zones, translators and waypoints, and entity `OnEvents` / `OnReports` `TarId`s. It does **not** follow the `Targets` of `MCU_Activate`, `MCU_Deactivate`, `MCU_ModifierSetVal`, `MCU_Delete` or commands: those are **state links**, which switch or change a node and do not fire it (review finding C9; `flights.rs` `build_randomizer` closers). It keeps a visited set, because timer loops exist (T-e, T-f, the clock). An edge is `unconditional` when every MCU on its path is a timer with `Random = 100` that no state link in the file can deactivate, and no counter is on the path; `delay_s` is the sum of the timers' `Time` values.
- **Byte-identical.** Nothing calls `instrument` unless trace is requested. With trace off, every Generate path is unchanged (the step 1 baseline test covers it). With trace on, the only changes are added MCUs with fresh indexes and appended `Targets` entries.

**0T-2. Log parser and replay** (`missionlog.rs`):
- `pub fn parse_log_dir(dir) -> Result<MissionLog, String>` and `parse_log_files(&[PathBuf])`. A session's files are `missionReport(<date>)[n].txt`; sort by the `[n]` suffix and concatenate. One line = one record: `T:<ticks> AType:<n> key:value …`. Parse into `Record { t_ticks, atype, fields: Vec<(String, String)> }` with typed views for the types the replay uses: 0 (mission start), 3 (kill), 5 (takeoff), 6 (landing), 8 (mission objective), 10 (player plane), 12 (object spawn: id, type, name, position), 16 (removed), 20 / 21 (player join / leave). **Unknown types and unknown fields are kept and skipped, never fatal**; the report counts them.
- **Time.** `T` is documented as game ticks (50 per second). **UNVERIFIED** (risk t4): in the traced probe every cue timer carries a breadcrumb with `expected_s` set, so the replay fits a line `T = offset + rate·s` from those (log `T:0` is not necessarily Mission Begin), reports both values, and uses the fit when it has one.
- `pub fn replay(log: &MissionLog, group: &Il2Entity, trace: Option<&TraceMap>) -> Replay`:
  - **Timeline:** breadcrumb firings (by source MCU name) merged with spawns, kills, takeoffs, landings and removals, in time order.
  - **Fired / never fired:** per traced MCU, its firing times and count.
  - **Unobserved successors:** a traced MCU that never fired while a traced MCU upstream of it (from `edges`) did fire. This is an observation, not a fault: the path may hold a closed gate, a failed random roll or a counter below its value. The report says so under the heading. Everything downstream of an unobserved successor is grouped under it rather than listed again.
  - **Dead links:** an unobserved successor whose edge is `unconditional`, where the log runs at least `delay_s + 5` s past the upstream firing. Only these are reported as faults.
  - **Never spawned:** entities in the `.Group` (planes, vehicles, ships, trains) whose `Name` never appears in a spawn record. Log lines carry no group path and names repeat across copies, so it reports counts **per name** ("`F-80C`: 6 spawned, 8 in the mission") and attributes a name to a group path only when that name is unique in the `.Group`. Whether the spawn line's name field is the mission's `Name` for AI planes is UNVERIFIED (risk t6); flights 0 and 1B settle it.
- `pub fn report_markdown(&Replay) -> String`: header (log files, first and last `T`, fitted tick rate, unknown-record count), then Dead links, Unobserved successors, Never fired, Never spawned, Timeline.
- **CLI:** `IL2MissionUtility.exe replay --group <file.Group> [--trace <file.trace.json>] --logs <dir | files…> [--out <report.md>]`. It prints the report (or writes it with `--out`) and returns exit code 0, or 2 on bad arguments or unreadable files. Follow `heightprobe::run_cli`'s style (it returns `None` when the first argument is not its own).

**Probe hook-up (with 0b).** Each ignored writer (`write_p14_probe_0`, `write_p14_probe_1a`, `write_p14_probe_1b`) writes a plain file and a traced file with its `.trace.json`. The traced file is `instrument(probe, &TraceSelect { indexes: <the source of every per-event, readout and cue subtitle, except T-t's>, block_types: vec![], name_prefixes: vec![] }, next_id, carrier, &mut map)`. For flight 0 the carrier is `Objective(default)` and `map` already holds cell T-t's own breadcrumbs (registered by `probe.rs` when it builds T-t). For flights 1A and 1B the carrier is the one the flight 0 results chose. T-g's per-plane sources are entities, so they get `OnEvent` breadcrumbs. **Flight 0 flies the traced file.** Every subtitle the video shows should then have a matching log line.

**Cell T-t (flight 0, automated): does tracing work?** Centre (0, +5), armed 04:30, closed 05:30, in the gap between T-f's F1 (last pulse 04:27) and F2 (moved to start at 05:30 for this cell). Cue at 04:30: "T-t start: note any objective message or map marker in the next 60 s". **Owner:** step 0b builds T-t in `probe.rs` (and its one `LCName` string in `locale.rs`), using `trace::breadcrumb` from the 1a stubs. Each T-t breadcrumb is registered in the probe's `TraceMap` with `TraceMap::push`, so the replay can name it; T-t's timers are **excluded** from the `instrument` selection, so no T-t source gets a second breadcrumb. Each breadcrumb's source timer also drives its own per-event subtitle, so the video and the log can be compared line by line.

| Sub | Setup | Confirmed if (from the replay report) |
|---|---|---|
| T1 | One objective breadcrumb pulsed once at 04:35 | exactly one `AType:8` line for it (t1) |
| T2 | One breadcrumb pulsed 5 times, 2 s apart, from 04:37 | 5 lines (t1 "every firing"); 1 line = "once only", then the `Spawn` carrier becomes the default |
| T3 | Two breadcrumbs pulsed in the same tick at 04:47 | both logged (t5) |
| T4 | Six objective variants, 4 s apart from 04:50 (a message stays on screen for a few seconds, so each variant needs its own window): `Coalition` 0 / 1 / 2 × `Success` 0 / 1, all `IconType 0`, one with an `LCName` | which variants log, and which show anything to players on the video (t3). The default `ObjectiveStyle` becomes the silent variant that logs |
| T5 | One `Spawn`-carrier breadcrumb fired twice, at 05:15 and 05:20 | two spawn lines named `TRACE <n>` (fallback carrier works and re-spawns on each firing). A deleted object may write no line at all, so any removal line is recorded as information only |
| T6 | The whole traced probe | every per-event subtitle seen on the video has a log line within ± 2 s of its video time, and no breadcrumb logs without its subtitle (t6). The cue breadcrumbs give the tick-rate fit (t4) |
| T7 | **One** breadcrumb pulsed twice in the same tick at 05:25 (one timer with two targets, each a 0 s relay into the same breadcrumb) | two lines = the log can show that one MCU fired twice in a tick; one line = it cannot, and the log is then no evidence for J4 or E8 (0d) |

**0T acceptance (tests):**
- `trace_off_changes_nothing` (added by the 0T agent after step 1's fixtures have merged, like the traced-probe test): every baseline fixture is unchanged, and `instrument(` appears only in `trace.rs`, `probe.rs` and tests. From step 9T the grep also allows the one call behind the trace flag in `ui.rs`, and `trace_checkbox_off_output_matches_baseline` covers the UI side.
- `trace_instrument_adds_exactly_one_breadcrumb_per_source`: on `Exclusive_Activation_6plan.Group` with `decision_points()`, the breadcrumb count equals the number of matching MCUs; each traced MCU's `Targets` gains exactly its breadcrumb; no other property of any existing block changes (compare the serialised trees with the breadcrumbs removed).
- `trace_breadcrumbs_are_unique_and_inside_the_map`: unique indexes and positions on the trace grid, inside the `geo.rs` bounds and outside the `placement.rs` area.
- `trace_entity_source_gets_an_onevent`: tracing a plane entity's type-4 event adds one `OnEvent { Type 4, TarId = breadcrumb }` and nothing else.
- `trace_edges_follow_onevents_and_stop_on_loops`: an OnEvents link is followed, and a timer loop terminates.
- `trace_sidecar_round_trips` and `trace_edges_skip_untraced_mcus` (a traced → untraced timer → traced chain gives one edge).
- `trace_spawn_carrier_deletes_its_object`: the `Spawn` carrier has a Spawner, a neutral vehicle named `TRACE <n>` with its linked entity at `TRACE_SPAWN_POINT`, and a 1 s Delete.
- `missionlog_parses_synthetic_fixture` (`src/testdata/missionlog/synthetic/`: at least one record of each typed kind, one unknown `AType`, one unknown field, two files `[0]` and `[1]`): record counts per type, correct order across files, unknown kept and counted.
- `missionlog_real_fixture_parses` (`#[ignore = "needs a real log (0L or flight 0)"]` until `real_1` exists): no errors; unknown-record count reported.
- `trace_edges_do_not_follow_state_links`: on a `flights.rs` randomizer, no edge runs from a Random timer through its Deactivate closer to a later output's Spawner.
- `trace_edge_is_unconditional_only_without_gates_and_counters`.
- `replay_unobserved_successor_behind_a_gate_is_not_a_dead_link`: a synthetic log in which A fires and B (behind a relay that a Deactivate targets) never fires lists B under Unobserved successors and reports no dead link.
- `replay_dead_link_on_an_unconditional_path`: a synthetic log in which A fires, B (one plain timer downstream of A) never fires and C (downstream of B) never fires reports B as a dead link and groups C under it.
- `replay_never_spawned_counts_per_group` and `replay_tick_rate_fit` (breadcrumbs at known times give 50 ticks/s ± 1 %).
- `replay_cli_bad_args_exit_2` and `replay_cli_writes_report` (run `run_cli` in-process on the synthetic fixture with `--out` to a temp file).
- `probe_traced_every_subtitle_source_has_a_breadcrumb` (runs after 0b is merged): in the traced probe, every per-event, readout and cue subtitle's source has **exactly one** breadcrumb, T-t's sources have none from `instrument`, every cue breadcrumb has `expected_s`, and the sidecar has an entry for each T-t breadcrumb.

**0d addition.** After flight 0: add the log as `real_1` (trimmed), un-ignore `missionlog_real_fixture_parses`, run `replay` on the traced probe, and put the report next to `handoff/P14-probe-results.md` as `handoff/P14-probe-replay.md`. Record t1–t6 in the results and set the `TraceCarrier` / `ObjectiveStyle` defaults and the tick rate from them. If objectives do not log at all (t1 false), switch the default to `Spawn`; if neither carrier logs, trace builds fall back to debug subtitles only and the replay still reports spawns, kills and never-spawned groups.

### Step 1. Skeleton, core helpers and baseline — S–M. Branch `claude/p14-core`

**First commit: the skeleton (step 1a).** It is merged into `claude/p14-air-tasking` before any other step starts:
- `main.rs`: `mod airtask;`, `mod in_theatre;`, `mod trace;`, `mod missionlog;`, the `missionlog::run_cli` line (step 0T), `#[cfg(test)] mod baseline_tests;`.
- `airtask/mod.rs` exactly as the §5 source: every type with its derives and `Default`s (including `DerivedTiming` and `SideLogic`), the `DPRK` / `NATO` index constants, `weighted_random_pct` (real body), the three stub functions, the strategy enums, every `mod` line and the two `pub use` lines.
- An empty stub file for each submodule (a doc comment only), so each later step only fills its own file. Two exceptions carry `pub fn` signatures with stub bodies, so the `pub use` lines compile: `library.rs` (`builtin_library() -> Vec<AirSortie>` returning `vec![]`, `load_user_sortie(path: &Path) -> Result<AirSortie, String>` returning `Err`) and `timing.rs` (`f80_transit_s(radius_m: f64) -> f64` returning `0.0`, `derived_timing(t: &ThresholdSpec) -> DerivedTiming` returning the default). **A stub names its unused arguments with a leading underscore** (`_path`, `_plan`, `_ctx`), here and in the three stub functions of `mod.rs`, because `allow(dead_code)` does not silence unused-variable warnings.
- `src/baseline_tests.rs` stub (a doc comment only), so `cargo test` compiles after the skeleton commit.
- `in_theatre.rs` stub (a doc comment and the dead-code allow).
- `testkit.rs` with `assert_links_resolve(root)` (every Targets / Objects index resolves to a node in the tree) and a walker placeholder.
- `MapAirPack` in `frontlines.rs`, with `#[derive(Debug, Clone)]`.
- The visibility-only changes of §5 "Builders": `pub(crate)` on `mcu`, `timer`, `counter`, `checkzone`, `modifier_set_val`, `attach_event` (`template.rs`), `clone_named`, `synthesize_mcu`, `silence_clone_starts` (`recon.rs`), `set_coord` / `add_yori` (`placement.rs`) and `is_rtb_waypoint` (`mapnet.rs`), plus the new `timer_random` next to `timer`. None of this changes output; the step 1 baseline fixtures are still recorded from `main` code, because nothing on a Generate path calls these differently.
- The enum variants that only step 12 uses (`ThresholdImpl::ComplexTrigger`, `CounterResetImpl::Ring`, `ZoneOutImpl::Watchdog`) get an item-level `#[allow(dead_code)] // P14 step 12` on their enums. These allows stay after step 9.
- The dead-code allows (§8 general rules).

**Rest of step 1. Files:** `src/placement.rs` (`rotate_tree`), `src/weapon_range.rs` (`snap_air_attack_areas`), `src/airtask/timing.rs`, `src/in_theatre.rs`, `src/baseline_tests.rs`, `src/testdata/p14_baseline/`, and a small addition to `src/ui_tests.rs`: `write_p14_baseline_ui` (ignored) and `map_generate_without_air_matches_baseline` (§7). This is the only `ui_tests.rs` change before step 8.

**Depends on:** the skeleton commit.

**Acceptance (tests):**
- `existing_outputs_match_baseline`: the fixtures are generated from unmodified `main` code by `write_p14_baseline` and committed, and the test passes.
- `map_generate_without_air_matches_baseline` (`ui_tests.rs`): `base_map_ui.Group` is written by `write_p14_baseline_ui` from the §7 inputs and committed, and the test passes. Active from step 1, so the `ui.rs` Map export path is guarded before any air pack goes through it.
- `rotate_tree_quarter_turn`: by 90° about (40000, 40000), WP 1 moves from (44000, h, 40000) to (40000, h, 44000). YPos is unchanged. Plane `YOri` gets +90. MCU `YOri` is unchanged.
- `snap_air_attack_areas_moves_only_air`: only nodes with `AttackAir = 1` move. The ground AttackArea is untouched.
- `weighted_random_pct_values`: `(&[5,5,5]) == [33,50,100]` and `(&[1,3]) == [25,100]`.
- `f80_transit_and_derived_timing`: `f80_transit_s(12000)` is within 0.1 s of 118.0. `derived_timing` gives W for 10 / 12 / 16 km of 100 / 120 / 160, H = 2W and r_in = R/2, and applies the `Option` overrides.
- `timing_override_wins_and_model_spec_is_not_written`: with `F80_CRUISE_OVERRIDE_KMH` passed as `Some(700.0)` to the inner helper, `f80_transit_s(12000)` is within 0.1 s of 123.4; `timing.rs` holds no write to `model_spec` (D40).
- `in_theatre_la11_invalid_in_1950`: `la11` on 1950-08-01 is invalid and cites [S9 p.686].
- `in_theatre_mig15bis_from_1950_11_01`: invalid on 1950-10-31, valid on 1950-11-01, and the citation says "stand-in".
- `in_theatre_all_builtins_valid_on_1950_07_01`: every model in the six built-ins is valid on 1950-07-01.
- `in_theatre_every_row_is_cited_or_unsourced`: each row has a citation or `sourced = false`, and every `model_spec` plane id has a row.
- The test count is re-recorded, and warnings do not rise.

### Step 1b. Dry-run walker — M. Branch `claude/p14-walker`

**Files:** `src/airtask/testkit.rs` only (test-only code).

**Depends on:** the step 1 skeleton. Steps 3–7 use it.

An event-queue simulator over an `Il2Entity` tree. Its semantics are explicit, and each rule names the risk it assumes:

| MCU | Walker rule | Assumes (risk) |
|---|---|---|
| `MCU_Timer` | A pulse into an active timer schedules its Targets at now + Time; `Random` < 100 rolls a seeded RNG on **every** trigger. A pulse into an inactive timer is dropped. | f, j (J2 no replay) |
| Timer re-trigger while running | Configurable: `Restart` (default), `Ignore`, `Twice`. Tests that depend on it run under all three. | p |
| Deactivate a running timer | Configurable: `RunsOn` (default, so D11 OUT relays matter) or `Cancels`. | j (J1) |
| `MCU_Activate` / `MCU_Deactivate` | Takes effect immediately, within the tick, on every node in its link list. | j, q |
| Same-tick order | Depth-first, Targets in listed order. A switch `ReverseTargets` runs the same scenario with each Targets list reversed. | q |
| `MCU_CheckZone` | Fires when, while active and pulsed, a scripted track of the watched coalition is inside the radius (Closer 1) or none is (Closer 0). Checked at each scripted track sample (1 s). Fires once per pulse (`o = once`); a `Repeat` switch fires every sample while inside. | b, c, o |
| `MCU_TR_MissionBegin` | Fires its Targets at t = 0 **regardless of `Enabled`** (the game runs Mission Begin even when `Enabled = 0`; comment above `silence_clone_starts`, `recon.rs` ≈1680). A silenced copy has empty Targets, so it does nothing. | — |
| `MCU_Counter` | Counts pulses while active; fires at Counter. `Dropcount 1` = reset to 0 after firing, so it can fire again; `Dropcount 0` = fire once and stay (the codebase convention: `template.rs` `SpawnCount` / `DeathCount`, `flights.rs` `DeathCount`). Configurable (`DropcountMeaning::{ResetOn1, ResetOn0}`, default `ResetOn1`) until probe cell T-e answers. | e |
| `MCU_ModifierSetVal` (ParamIndex 0, Data0 0) | Resets its target Counter's count to 0. For a `Dropcount 0` counter that has already fired: configurable `SetValRefires::{Yes, No}`, default `Yes` (it counts again and can fire again at Counter, which `THsaa COUNT` relies on), until probe cell T-e E6 answers. Tests of the N ≥ 2 path run under both. | e |
| Several pulses into one `MCU_Counter` in one tick | Configurable: `SameTickCounts::{Each, One}`, default `Each`. The budget tests run under both; with the census spaced 0.05 s apart (§3.3) the result must be the same. | r |
| Same-tick delivery | Configurable: `DepthFirst` (the default: each pulse and every switch it causes is finished before the next pulse of the tick) and `Batched` (every pulse of a tick is delivered against the states at the start of the tick, and switches take effect at its end). The overshoot tests need `Batched`: under `DepthFirst` the first request shuts the gates before the second is delivered. | q |
| `MCU_TR_ComplexTrigger` (added in step 12) | Emits "entered" once per scripted track entering the radius. | d |
| OnEvent Type 13 | A scripted "kill" event on a plane entity emits once. (Type 4 never fires in the game: flight 1G.) | g |
| `MCU_Delete`, commands, waypoints | Recorded, no effect, except a scripted "arrive at WP" event that fires the waypoint's Targets. | — |

Inputs: scripted tracks (coalition, list of (t, x, z)), forced pulses (node name, t), kill events, and a fixed seed. Output: a log of (t, node name) pulses and state snapshots. Deterministic for a given seed.

**Acceptance (tests):**
- `walker_self_test_exclusive_activation_grants_one`: on the unmodified `TemplateExamples/Exclusive_Activation_6plan.Group` (13 counters with `Dropcount 1`, 4 with `Dropcount 0`; it runs under the default `ResetOn1`).
  - Plans are found with `bombers::extract_exclusive_plans` (only to identify each plan's group and plane entities; the walker runs on the original tree, whose gate links `extract_exclusive_plans` would strip). Each plan's trigger zones are `bombers::inspect_plan(plan).suggested_triggers`.
  - Stimulus: one scripted NATO track and one DPRK track, each passing through every plan's trigger checkzones in turn, starting after Mission Begin.
  - A **grant** (the word here is about the Exclusive Activation file, not about P14's budget) is a pulse of an `MCU_Activate` or `MCU_Spawner` whose Objects contain that plan's plane entities.
  - Expect grants for exactly one plan.
- `walker_self_test_fighter_pack_nodegates_grants_one`: a Fighter Pack built by `pack.rs` grants one per gate.
- `walker_random_is_seeded_and_rolled_each_time`: 40 pulses into Random 50 give 12–28 hits, the same count for the same seed.
- `walker_checkzone_track`: a Closer 1 zone fires when a track enters after the pulse and not before; a Closer 0 zone fires when the last track leaves.
- `walker_every_deactivated_out_has_a_rearm_or_is_single_use`: a static rule check over any tree; used by steps 3–7 on every generated pack.

### Step 2. Library and `place_air_sortie` — M. Branch `claude/p14-place`

**Files:** `src/airtask/library.rs`, `src/airtask/place.rs`.

**Depends on:** step 1.

**Acceptance (tests):**
- `library_builtins_load_with_side_and_roles`: all six load; side and role detection match the §5 table.
- `inspect_sortie_finds_the_parts_by_wiring`: for each of the six, the ids it returns are the MCUs that Template Builder names `Zone IN`, `MISSION BEGIN`, `Zone Out`, `Zone Out ReActivate`, `PULSE OUT`, `MISSION END`, `DELETE DELAY` and every `Mission Complete n`; and the navigation lead's attack timer is 37 for HVAR, CloseSupport and ArmedRecon, **39 for AirAlert**, and the branch whose AttackArea command holds entity 7 for Yak and Il-10 (third review, T1).
- `inspect_sortie_does_not_read_names`: with every MCU of the HVAR copy renamed to `X`, it returns the same ids.
- `inspect_sortie_accepts_each_tested_shape`: table-driven over Activate-mode S1–S4 and S7 (§3.2, decision 0017). S1–S4 are the six built-ins, S3 also with its two flights swapped in file order. Generated cases cover Formation off/on and RTB on every lead or on one of two leads (S7). `inspect_sortie_refuses_spawn_mode` generates single-plane Spawn templates with Repeat off/on and checks the Activate-mode refusal; S5/S6 are not accepted.
- `inspect_sortie_refusals`: table-driven, one row per refusal of §3.2, each with its message: no Zone Out; two Closer 0 zones; a vehicle among the units; no lead with a waypoint; two path waypoints on a lead; no AttackArea on the navigation lead; a time-on-target step between the attack order and Mission Complete; a BingoFuel → Force Complete event hook; an order started by a report hook. Each template is written by `generate_template` in the test, so the test follows what Template Builder can write.
- `inspect_sortie_accepts_nothing_untested`: the accepted-shape table in `library.rs` and the rows of `inspect_sortie_accepts_each_tested_shape` have the same length (U21).
- `inspect_sortie_escort_first_picks_the_second_lead`: Armed Recon with its two flights swapped in file order: the navigation lead is the strike lead.
- `library_leads_are_entities_with_empty_targets`: AirAlert, HVAR, CloseSupport = [7]; Armed Recon = [7, 11]; Yak = [7, 11]; Il-10 = [7, 15].
- `library_speed_alt_attack_time_per_file` (table-driven over all six, order AirAlert / HVAR / CloseSupport / ArmedRecon / Yak / Il-10): WP speed 660 / 660 / 520 / 520 / 500 / 390, WP altitude 3050 / 3050 / 1500 / 900 / 3050 / 1500, T_attack 1080 / 600 / 900 / 1200 / 300 / 600.
- For each of the six, placed at target (100000, 150000) with heading 45°, spawn distance 30 km, RTB (80000, 120000) (`place_all_builtins_*`):
  - `…_lead_at_spawn`: the navigation lead is at `target − 30 km·(cos 45°, sin 45°)` ± 1 m;
  - `…_bearing`: before snapping, the navigation lead → its WP bearing is 45° ± 0.01°;
  - `…_offsets_rotate_rigidly`: for every node with XPos that is not snapped (not a path waypoint, not an AttackArea), the new offset from the lead equals R(θ)·(old offset) within 0.01 m; YPos, XOri and ZOri are unchanged;
  - `…_plane_yori`: each plane's `YOri` == old + θ (mod 360);
  - `…_waypoints_and_attack_areas_on_target`: every non-RTB `MCU_Waypoint` and every ground and air AttackArea is on the target;
  - `…_rtb_per_lead`: one `RTB n` per lead at the RTB point with Priority 2; the Armed Recon escort's `RTB` uses its plane's YPos; `DELAYED END ORDERS.Time = 60`;
  - `…_links_resolve`: `assert_links_resolve` passes.
- `place_rejects_two_path_waypoints_per_lead` (D26 message) and `place_rejects_template_without_waypoint`, both on synthesized templates.
- `place_low_speed_uses_model_spec_and_warns`: a synthesized template with `Speed = 100` gets `suggested_waypoint_speed_kmh` and a warning.
- `place_low_speed_with_unknown_models_is_refused`: `Speed = 100` and a model `model_spec` does not know: refused, and the message names the model (D39).
- `place_rtb_per_lead_keeps_and_adds`: a template with RTB on one of two leads (shape S7): the lead that had an RTB keeps its waypoint, moved to the RTB point; the other lead gets a new one; both are targets of the template's one RTB timer (third review, T5).

### Step 3. Sortie shell, budget and assembly — M–L. Branch `claude/p14-shell`

**Files:** `src/airtask/shell.rs`, `src/airtask/budget.rs`, `src/airtask/assemble.rs`; decision 0017 also updates `library.rs` refusals and `probe.rs` flight 2.

**Depends on:** steps 1, 1b and 2, and the **flight 0 results** (gate G0, §10). Strategy enums: `ZoneOutImpl`, `LossesImpl`, `BudgetImpl`, `CensusImpl`. Owns `AT s *`, the side's `Translator Mission Begin` (`emit_side_logic`), the per-entry copy pointer (§3.7) and `BUD s *` (§3.3). `budget::build_side` reads the gates, flags and events that `assemble_side` registered in `SideLogic`; its own tests fill them with stand-in relays.

**Acceptance, structural (table-driven over all six built-ins, each placed and wrapped as copy `SN01`/`SD01`):**
- `shell_keeps_zone_in_and_strips_start`: Zone IN is retained under its template name. `ENABLE / PULSE IN`, `COOLDOWN`, `Zone In ReActivate`, both `Self Deactivate` and every `Mission Complete n` are gone. `Translator Mission Begin` has `Enabled 0` and no Targets. `assert_links_resolve` passes, so no link to a deleted MCU is left.
- `shell_end_timer_only_from_done_hub`: the end timer (`MISSION END`) has exactly one inbound link: `DONE HUB`.
- `shell_done_hub_inputs`: `DONE HUB` receives exactly `HARD STOP`, `COLD`, `EXTEND`, `LOSSES` and `Zone Out`, and targets `DONE HUB OFF`, which deactivates `DONE HUB` only.
- `shell_delete_timer_releases_the_budget`: `SNjj DELETE DELAY` targets `SNjj DELETE` (Objects = every plane entity) and `SNjj LANDED`, which deactivates `FLYING`; the shell delay is registered in `SideLogic.budget_events` (D38, U22).
- `shell_one_on_station_per_sortie`: exactly one `ON STATION`, `Time` = T_attack per file (1080 / 600 / 900 / 1200 / 300 / 600), targeted only by `ON STATION IN` (a latch), which is targeted only by the navigation lead's attack timer (for AirAlert, timer 39, not 37; for Yak and Il-10, the branch of entity 7, not the other lead's).
- `shell_on_station_with_the_escort_first`: Armed Recon with its two flights swapped in file order: `ON STATION IN` is fed by the strike lead's on-target timer (second review, R4-4).
- `shell_wp1_pulses_zone_in`: the navigation lead's `WP 1` targets Zone IN, and only Zone IN targets Zone Out's arming pair. Both zones are at the target with radius 35 km, Closer 1 / 0 and their unchanged enemy coalition filters. Zone IN is not initialized off and no Deactivate in the emitted copy targets it.
- `shell_losses_counts_every_plane`: `LOSSES` Counter = 4 / 4 / 4 / 4 / 4 / 8, and the number of `OnEvent Type 13 → LOSSES` equals the plane count (the Armed Recon escort included).
- `shell_end_of_station_pair`: `ON STATION` targets exactly `SNjj HOT` and `SNjj COLD`; `HOT` targets `EXTEND`; `COLD` and `EXTEND` target `DONE HUB`; both relays are registered in `SideLogic.hot_consumers` / `cold_consumers` under the copy's area. The copy contains exactly two check zones: Zone IN and Zone Out.
- `shell_timing_values`: HVAR at clock distance: `HARD STOP.Time = 2100` (300 + 600 + 600 + 600); Armed Recon at clock distance: `2700`; `EXTEND.Time = 600`.
- `shell_start_is_the_gate`: `ENii COPY r` targets `SNjj START` directly and there is no `GATE` MCU; `START` is registered in `SideLogic.gates` and `SideLogic.budget_events`; `START` targets the bring-up timer, `HARD STOP`, `STARTED ON` and `STARTED OFF`; `STARTED ON` activates `FLYING`. The shell writes no link to a `BUD`, `EN` or `ZN` MCU: those come through `SideLogic.links`.
- `shell_adds_19_mcus_per_copy`: HVAR removes 8 template MCUs and adds 19 plus one `RTB 1` waypoint, for 43 total (42 without Zone IN). `shell_keeps_template_rtb_timer`: generated S7 removes 7 and adds 18 plus the missing lead's RTB waypoint, for 47 total (46 without Zone IN).
- `shell_replaces_the_template_delete_chain`: a built-in with a chain and a regenerated Activate template without one each end with exactly one `SNjj DELETE` and no dangling link.
- `shell_does_not_reuse_removed_delete_chain_indexes`: a template whose Delete chain owns its highest indexes, wrapped with a stale allocator, leaves every shell-added index outside the removed chain's range.
- `shell_writes_only_the_losses_hooks`: every entity has exactly one type 13 into LOSSES, and no other event or report hook, across all six built-ins.
- `inspect_sortie_refuses_spawn_mode`: generated single-plane Spawn templates, Repeat off and on, receive the Activate-mode refusal. The step 2 repeat acceptance test becomes `inspect_sortie_refuses_on_killed_repeat_events`.
- `shell_orphan_sweep_keeps_everything_reachable`: for each built-in, every order timer, command, waypoint, `Activate Units`, `AFTER BRING UP`, the end chain and the Zone Out arming pair are still there.
- `shell_refuses_a_second_link_into_the_end_timer`.
- Variant: `shell_zone_out_watchdog_structure`; the 4/2/0 losses latch is removed.
- `assemble_copy_pointer_structure`: per entry (including a `Zone`-only entry), `ENii COPY 1..runs` each target a distinct `SNjj START`; copy r's `STARTED OFF` deactivates `COPY r` and its `STARTED ON` activates `COPY r+1`; the last copy's `START` targets `ENii EXHAUSTED`; `COPY r` for r ≥ 2 is in `SideLogic.init_off`; `SideCopies` returns the `COPY` and `EXHAUSTED` ids.
- `assemble_emit_side_logic`: `Translator Mission Begin` targets exactly `SideLogic.mission_begin` plus `AT s INIT` (0.5 s), and `AT s INIT OFF` lists exactly `SideLogic.init_off`; each `SNjj HOT` / `COLD` is registered under its area; each `SNjj START`, `FLYING` and shell `DELETE DELAY` is registered for the budget.
- `assemble_applies_links`: for each pair in `SideLogic.links`, the MCU `from` has `to` in its Targets after `emit_side_logic`, wherever in the side's trees `from` lives; a pair whose `from` does not exist is an error, not a silent skip.
- `assemble_exhausted_is_one_deactivate_per_entry`: `ENii EXHAUSTED` is an `MCU_Deactivate`; the last copy's `START` targets it; for a `Both` entry its target list holds the clock's three relays and the zone's three.

**Acceptance, budget structure** (N = 2, 3 sorties):
- `budget_structure`: the side has the 16 MCUs of §3.3 and no other `BUD` MCU; there is no `EVENT` relay and no `GATE`. `SideLogic.links` holds, for every `START`, every delete timer and `AGAIN`, a link to `MARK` and a link to `CHECK`. `SHUT` lists exactly the 3 `START` relays and `CHECK`; `OPEN` lists exactly the 3 `START` relays. `CENSUS` targets exactly the 3 `FLYING` timers, whose `Time` values are 0.05, 0.10 and 0.15. Each `FLYING` targets `COUNT`. `COUNT` has `Counter = 2`, `Dropcount 1`. `END.Time = 0.30`. `END` re-arms `CHECK`. `AGAIN DELAY.Time = 0.05`. `SideLogic.init_off` holds the 3 `FLYING` timers and `AGAIN`.
- `budget_mcu_count_does_not_depend_on_n`: N = 2 and N = 6 give the same MCU count (16). N = 1 gives 14: no `COUNT`, no `ZERO`, and every `FLYING` targets `STAY SHUT` (`budget_n1_has_no_counter`).
- Variants: `budget_single_slot_structure` (`BudgetImpl::SingleSlot` forces N = 1 on both sides, whatever the plan says) and `budget_census_same_tick_structure` (every `FLYING.Time = 0`, `END.Time = 0.2`).

**Acceptance, walker scenarios** (each also run with `ReverseTargets` and with the three re-trigger modes):
- `shell_empty_target_stays_on_station`: no enemy, HVAR DONE from COLD at WP 1 + 0.5 + 600 s, never at arrival.
- `shell_enemy_present_at_arrival_then_leaves`: enemy already inside, leaves at WP 1 + 180 s; Zone IN fires at arrival, Zone Out ends the sortie when it leaves.
- `shell_enemy_arrives_after_then_leaves`: enemy inside from WP 1 + 120 to +300 s; Zone IN waits, then Zone Out ends the sortie at departure (plus game reaction time).
- `shell_hot_empty_target_extends_once`: no enemy, HOT; DONE from EXTEND after one 600 s extension.
- `shell_done_fires_once_when_on_station_end_and_zone_out_coincide`: `DONE HUB` passes one pulse and the end timer starts once.
- `shell_losses_all_planes_triggers_done`: walker kills every aircraft through event 13, including Armed Recon escorts; DONE HUB passes once.
- `shell_deletion_frees_the_budget`: DONE at t, Delete at t + 60.6 s, FLYING remains on until deletion, and a later request starts.
- `shell_cold_area_ends_at_on_station_end`: `DONE HUB` at T_attack after the attack timer fires (for HVAR that is WP 1 arrival + 0.5 s + 600 s); the delete timer fires 60.6 s later, and only then is `FLYING` off.
- `shell_hot_area_extends_once_then_ends`: `DONE HUB` 600 s after the end of on-station time, even if the area is still HOT; the later hard stop finds the hub closed.
- `shell_late_flight_ends_at_hard_stop` (WP 1 never reached): `DONE HUB` at `START` + T_final.
- `shell_airalert_on_station_starts_once`: AirAlert, WP 1 reached: timer 39 is pulsed twice (from WP 1 and from 37), and `ON STATION` starts exactly once under all three re-trigger modes.
- `budget_grants_2_refuses_3rd_then_grants_after_deletion`: requests for 3 copies 2 s apart; then one copy's `DONE HUB`; the 3rd request 30 s after that is **still dropped** (the aircraft exist); the 3rd request 5 s after that copy's deletion starts (U22).
- `budget_two_deletions_in_one_tick_reopen` (review finding C1): budget 2, both flying, both delete timers in one tick: the gates are open `T_count` later, and two more requests 2 s apart both start.
- `budget_two_deletions_50_ms_apart_reopen`: the same with the second deletion 0.05 s after the first.
- `budget_deletion_during_recount_runs_a_second_recount`: a deletion 0.2 s after a start: `CHECK` passes twice, and the gates end open.
- `budget_request_during_recount_is_dropped`.
- `budget_never_locks_when_nothing_flies`: 200 seeded random orders of requests, ends and deletions (budget 1, 2 and 3; gaps from 0 to 1 s, in steps of 0.01 s; each deletion 60.6 s after its end), under `DepthFirst` and `Batched` delivery. After the last copy has been deleted and 5 s have passed, every gate is open. **The number of copies whose aircraft exist never passes N**, except after requests in one tick under `Batched`, and then by no more than the number of such requests less one.
- `budget_requests_in_one_tick_overshoot_then_heal` (`Batched` delivery): budget 1, two requests in one tick: both start; the recount keeps the gates shut; **after one of the two is deleted the gates are still shut**; after both are deleted a new request starts. Under `DepthFirst` only the first starts. This pins the one documented overshoot (§3.3), so a change in behaviour is noticed.
- `budget_same_copy_started_twice_in_one_tick` (`Batched` delivery, third review T10): the clock's `GO` and the zone's `FIRE` of one `Both` entry in the same tick: the copy's `START` passes two pulses, the side has one flag more, and `DONE HUB` passes once. This pins the accepted behaviour.
- `budget_single_slot_grants_1` (variant).
- `assemble_side_numbers_copies_and_uses_spawn_rule`: one copy per run per eligible entry, jj unique per side, `Zone`/`Both` copies at 30 km ± 1 m, `Clock` copies at v_wp·M ± 1 m.
- `walker_every_deactivated_out_has_a_rearm_or_is_single_use` passes on the wrapped copies and on the budget.

**Flight 2 was flown and is frozen (decision 0018).** Its four files under `handoff/p14-probes/flight2/` remain the replay evidence. The `write_p14_probe_2` writer is replaced by `write_p14_probe_2b`; the shared writer refuses `Flight::Two`. The flight 2 builder still uses the current shell for structural checks, without regenerating the flown files. `Flight::Two`, `RUN_SHEET_2` and cues keep it in the common probe checks. The retired `write_p14_sortie_probe` is removed.
- **T-s1**: placed HVAR starts at 60 s, target K14 + (50, 20) km. **T-s2**: placed AirAlert starts at 90 s, target K14 + (50, 60) km. Both use Activate mode, a 300 s nominal transit, 600 s margin and 600 s extension; each stand-in area switches HOT on / COLD off at 600 s. Subtitles and breadcrumbs observe START, DONE HUB, EXTEND and the issuing `SNjj DELETE DELAY` pulse (the Delete command is a sink; it executes in that same tick).
- **T-l**: `T-l1` is a single independent F-80 at K14 + (-25, +50) km, (80934, 319477), air start at 60 s, heading east (90°), altitude 150 m, Fuel 0, `AiRTBDecision = 0`. Its cruise waypoint is 10 km farther east, (80934, 329477), altitude 150 m, speed 732 km/h from `model_spec`. `assets/combined_terrain.bin` marks both points as land. The start is 55.902 km from K14 and at least 21.577 km from every recorded airfield; the nearest is K-49 Yangsu-ri, (102114, 315358) in `TemplateExamples/K49 DPRK_AB.Group`. The check covered the 39 `AIRFIELDS` markers in `References/landscape_Korea_FullScene.Group`, the three standalone reference airfields and all 25 positioned `Airfield` blocks in `TemplateExamples/`, `References/` and `src/testdata/`. The cruise leg stays at least 15 km from every field, increases the nearest-field clearance to 25.363 km and leads away from K14 (65 km at the waypoint). The nearest other flight 2 aircraft starts over 22 km away. Event 13 feeds a Counter 1 / Dropcount 0 LOSSES; events 2, 0 and 5 have separate observations. No Damage command is present. **T-l2 is omitted:** the WMAP file is a land/water/open mask (`watermap.rs`), and the repository contains no measured elevation for a chosen ridge point.
- Every aircraft has its own entity-Objects close Delete at 2760 s, following flight 1P. Budget cells close at 220 s and 280 s. The flight's final cue is at 2760 s; end normally after it.
- cell **T-k**, t = 180 s, built with `budget.rs` (N = 2, 7 stand-in copies, so `T_count` = 0.5 s): requests at t, t + 2 s, t + 4 s and t + 6 s; the two copies that started are deleted in the same tick at t + 30 s; a 5th and a 6th request at t + 32 s and t + 34 s; a 7th at t + 36 s. Every request is more than `T_count` after the one before, so the result does not depend on how the game delivers two pulses in one tick.
- cell **T-k1** (N = 1, t = 240 s, 5 stand-ins): two requests in the same tick at t; a third at t + 5 s; **one** of the copies that started is deleted at t + 20 s; a fourth request at t + 22 s; every other copy that started is deleted at t + 30 s; a fifth request at t + 32 s.

Check:
- arrival time against §4.3 (±20 %), that the AttackArea is on the target, the RTB turn, deletion about 60 s after `DONE HUB`, and that "DONE HUB" shows once per sortie;
- AirAlert's two AttackAreas (D28);
- what the aircraft do during the extension (risk v);
- T-l1 receives no Damage command: record whether the fuel-zero crash raises 13, 2, 0 or 5, and whether LOSSES fires once;
- T-k: the requests at t and t + 2 s start; those at t + 4 s and t + 6 s are dropped; after the two deletions, the 5th and 6th start; the 7th is dropped.
- T-k1: 1 or 2 start at t (2 is the documented overshoot; the result says how the game delivers two pulses in one tick); the third is dropped. If 2 started, the fourth is dropped too, because one copy still exists. The fifth starts.
- **load (D36):** the ignored tests `write_p14_load_probe` and `write_p14_probe_2b` share a plain helper that writes `Flight::Load` (`P14_Load_Probe`) through the common probe writer, once even when both tests run together. It contains the acceptance plan scaled to 24 copies per side (240 idle aircraft, all six built-ins), now with 48 check zones per side. It lives in `probe.rs`. The user notes the mission load time and the server's tick delay with it loaded and idle, and both go into `handoff/P14-probe-results.md`.

The original structural/walker checks remain `flight2_sorties_use_the_shell_lifecycle`, `flight2_budget_requests_follow_the_plan`, `flight2_no_damage_loss_uses_a_once_counter`, `flight2_loss_starts_over_land_away_from_airfields` and `flight2_load_has_24_idle_copies_per_side`. `flown_probes_regenerate_byte_for_byte_except_flight2` compares the 44 files of flights 0–1P byte for byte and explicitly excludes flight 2 under decision 0018. `flight2_frozen_files_parse_and_links_resolve` checks the frozen traced Group, English locale, spawn list and trace map.

**Needs the user in game: flight 2B (automated, 30 min).** `Flight::TwoB`, `generate_probe_2b`, `RUN_SHEET_2B`, cues and the ignored `write_p14_probe_2b` produce `P14_Probe_2B` plain/traced files and `target/p14/P14_Probe_2B_schedule.md`. Four independent NATO sortie cells have targets at K14 + (140, -90), (140, 0), (230, -90), (230, 0) km, all on land, at least 90 km apart and 140 km from K14.
- T-s1: HVAR START 60 s, empty target, HOT at 600 s; 600 s on station and one 600 s extension. T-s2: AirAlert START 90 s, empty COLD target; 1080 s on station.
- T-s3: HVAR START 120 s; a two-plane DPRK Yak-9P flight activates at 30 s and is deleted at 586 s. T-s4: HVAR START 150 s; the same enemy pair activates at 556 s and is deleted at 736 s. Those schedules use flight 2's measured transits from START (HVAR 286 s, AirAlert 279 s), predicting WP 1 at 346, 369, 406, 436 s.
- Enemy pairs have country 503, `AiRTBDecision = 0`, an 8000 m air start and an eight-waypoint orbit 18–22 km from the target and the coincident attack point. The orbit has a 2 km radius, is centred 20 km east of the target, and turns 45° at every waypoint; every straight leg stays at least 18 km from the attack point and inside 30 km of the target. They receive the shipped Behaviour command after 0.1 s with `Engageable = 0`; its retained `Filter = 8` is undocumented in the repository and remains unverified. The Plane property is also 0. No fight is needed.
- Breadcrumbs cover Zone IN, Zone Out, LOSSES, COLD, EXTEND, HARD STOP, START, WP 1, every attack timer, ON STATION, DONE HUB, DELETE DELAY and the issuing enemy activation/deletion timers. All cells close at 1800 s with one Delete per written aircraft entity and a final cue.
- New checks: `flight2b_sorties_and_breadcrumbs_match_the_plan`, `flight2b_enemies_are_isolated_and_scheduled`, `flight2b_walks_all_four_outcomes`. The common loader-shape, locale, run-sheet and link checks include 2B.

### Step 4. Threshold cell — M. Branch `claude/p14-threshold`

**Files:** `src/airtask/threshold.rs`.

**Depends on:** steps 1 and 1b, and the **flight 1A results** (gate G1, §10). Strategy enum: `PresenceCheckImpl` (CHECK and the re-check), removed by decision 0025. v1 builds the one-player path only (§3.5); the counted path is step 12. **As built (2026-10-18), §3.5 and its acceptance tests changed (decisions 0023 to 0025): the list below is the original draft; the built tests are in `src/airtask/threshold.rs` and listed in `handoff/sessions/2026-10-18-p14-step4.md`.** The builder takes a `SideLogic` whose `hot_consumers`, `cold_consumers` and `pass_subscribers` are already filled (it runs after the front ends, §3.0); its tests fill them with stand-in relays.

**Acceptance:**
- `threshold_structure`: `THNaa ENTRY ZONE` (Closer 1, `Zone = 12000`, coalition of the other side) targets `ARM` only; `ARM` targets `ENTRY OFF` and `DWELL`; `ENTRY OFF` deactivates `ENTRY ZONE` and `ARM`; `DWELL.Time = 120`; `INNER` (`Zone = 6000`); `INNER CLOSE.Time = 2`, and it targets `INNER OFF` and `INNER CLOSE DELAY` (`Time = 0.05`), which targets `INNER CLOSE OUT`; `HOLD.Time = 240`. There is no `CHECK`, `PASS` or `FAIL` MCU. `DWELL` and `HOLD` each target exactly `CHECK ARM`, `INNER ON`, `INNER PULSE` and `INNER CLOSE`. `INNER` targets `PASS OFF`, `HOT ON`, `COLD OFF DELAY` (0.05 s), `HOLD` and exactly the area's `SideLogic.pass_subscribers`. `INNER CLOSE OUT` targets `COLD ON`, `HOT OFF DELAY` (0.05 s) and `REARM`. `REARM` re-activates `ARM` and `ENTRY ZONE` and re-pulses `ENTRY ZONE`. `HOT ON` / `HOT OFF` list exactly the area's `hot_consumers`, and `COLD ON` / `COLD OFF` its `cold_consumers`. `SideLogic.init_off` contains the HOT set, and `SideLogic.mission_begin` contains `ENTRY ZONE`.
- `threshold_hold_does_not_switch_hot_off`: `HOLD` targets only the four CHECK nodes (review finding C5).
- `threshold_has_23_mcus_and_2_check_zones` (U13).
- `threshold_inner_closer0_race_structure` (`PresenceCheckImpl::Closer0Race` variant): `INNER` is Closer 0, and `PASS` comes from the 2 s race timer only if `INNER` has not fired.
- `threshold_emits_no_complex_trigger` (D1).
- `threshold_timing_follows_radius`: R = 16 km gives D and H of 160 / 320.
- Walker scenarios:
  - `threshold_loiter_passes`: a track enters at t = 0 and stays inside r_in: PASS between 120.1 and 122.6 s (DWELL + INNER PULSE + ordering).
  - `threshold_fly_through_does_not_pass`.
  - `threshold_hot_and_cold_are_never_both_off`: over a pass, a failed re-check and a second pass, at every walker sample at least one of a consumer's two relays is active, and both are active for at most 0.06 s at each change.
  - `threshold_pass_and_timeout_in_one_tick_is_a_pass` (`Batched` delivery and `DepthFirst`, both target orders; third review T9): the track's first sample inside r_in is in the tick in which `INNER CLOSE` fires: HOT goes on, COLD goes off 0.05 s later, `REARM` does not start, and exactly one `HOLD` is running.
  - `threshold_stays_hot_through_a_successful_recheck`: a track stays inside r_in for 600 s: a consumer pulse at any 0.01 s step after the first PASS reaches the HOT relay.
  - `threshold_pass_leave_pass_again`: PASS; the player leaves; the re-check fails (COLD, ENTRY re-armed); the player re-enters and loiters: a second PASS after a full dwell.
  - `threshold_second_plane_after_close_does_not_pass`: a second plane entering r_in after `INNER CLOSE` gives no PASS until a new dwell.
  - `threshold_repeat_firing_entry_does_not_restart_dwell` (walker `Repeat` switch on).
  - `threshold_closer0_race_loiter_passes` (`Closer0Race` variant).

### Step 5. Clock front end — M–L. Branch `claude/p14-clock`

**Files:** `src/airtask/clock.rs`.

**Depends on:** steps 3 and 4. Built with `RandomImpl::{InMission, Seeded}`; the default comes from the flight 0 result f. **As built (2026-10-18), decision 0029 B changed the inside of the clock and removed `Seeded`: the list below is the original draft; the built tests are in `src/airtask/clock.rs` and listed in `handoff/sessions/2026-10-18-p14-step5.md`.**

**Walker test plan, pinned in code** (`clock_test_plan()`): the lifecycle scenarios below need more grants than the defaults allow, so they use `runs = 6` on every entry, budget 6, and a scripted `DONE HUB` for each copy 60 s after its `START`. Scenarios that state other values say so.

**Acceptance (tests):**
- `clock_structure_three_entries` (NATO, 3 entries on 3 areas, weights 5, 5, 5): `CLK N FIRST.Time = 600`, `CLK N PERIOD.Time = 1200` with the loop back to `TICK`, `CLK N QUIET ROLL Random = 80`, 3 `JITTER k DELAY` with 0 / 180 / 360, and `EN0i RANDOM` with Random 33 / 50 / 100 in shuffled order (seeded), at `Time` 0.5 / 1.0 / 1.5 s in waterfall order, `Wait for Output` at 2.0 s and `ATTEMPT END.Time = 2.5`.
- `clock_entry_relays`: each entry has `AVAILABLE`, `LAST`, `EXCLUDE`, `HOT` and `COLD` (registered in `SideLogic.hot_consumers` / `cold_consumers`) and `GO`; it has no `MISS` timer; `GO` targets every `ENii COPY r` from `SideCopies` (built by `assemble_side`, not by the clock).
- `clock_mcu_counts`: the three-entry plan has 12 + 12 + 11 entry MCUs, and one `ENii EXHAUSTED` per entry from `assemble_side`; the per-side count is pinned as a number when the step is built and stated in the stop report (U13).
- `clock_rearm_edges`: `DRAW` activates `ATTEMPT END OUT`; every `GO` targets `CLK s WIN`, which deactivates `ATTEMPT END OUT` and `REROLL`; every copy's `START` targets its entry's `LAST SET`, and `GO` does not; `LAST SET`'s Deactivate lists every `LAST` except its own; `SLOT OPEN` targets `LAST CLEAR` (`Time` 0.2), which deactivates every `LAST`, and `DRAW DELAY` stays at 0.3 s so the clear comes before the `LAST SET` that the slot's own winner causes.
- `clock_last_copy_disables_random_and_go`: `SideLogic.links` holds `ENii EXHAUSTED` → `ENii AVAILABLE`, `ENii RANDOM` and `ENii GO`.
- `clock_slot_window_formula`: `CLK N SLOT CLOSE.Time = min(300, (a + 10)·A)`, with A = L + 0.5.
- `clock_ignores_zone_only_and_ineligible_entries`: `Zone`-only, disabled, orphaned and date-invalid entries have no `EN RANDOM`.
- `schedule_disabled_emits_no_clk`.
- `clock_generate_time_choices_are_seeded_and_deterministic` (`Seeded` variant): same seed, same unrolled slots; a slot tries its list in order.
- Walker scenarios (seeded, `clock_test_plan()` unless stated):
  - (i) `clock_all_hot_one_start_per_slot_no_back_to_back`: 4 entries, all areas HOT, 12 slots: one `START` per non-quiet slot, and no entry starts in two slots in a row.
  - (ii) `clock_only_one_area_hot_alternates_with_quiet`: only entry 2's area HOT, 10 slots: entry 2 starts, reached by reroll, in every other non-quiet slot; the slot after each start is quiet because of no-repeat.
  - (iii) `clock_no_area_hot_sends_no_request`.
  - (iv) `clock_budget_full_slot_is_quiet_and_sets_no_last`: budget 1 and one sortie flying: the winner's request is dropped, no reroll follows, and the same entry can win the next slot.
  - (v) `clock_entry_never_wins_after_runs_starts` (`runs = 3`).
  - (vi) `clock_early_miss_then_later_winner_one_request_per_slot`: a COLD entry at waterfall position 1 followed by a winner: exactly one request in the slot.
  - (vii) `clock_win_then_later_miss_rerolls`: an entry starts in slot 1; in slot 3 it is drawn while its area is COLD: its area is excluded and another HOT entry wins.
  - (viii) `clock_sixteen_entries_one_passing_area_found`: 16 entries on 8 areas, one HOT area: the slot starts an entry on that area.
  - (ix) `clock_quiet_rate_and_jitter_spread`: 200 **independent** one-slot runs (a fresh walker and a new seed each): quiet rate within 20 % ± 7 %, and each jitter step used.
  - (x) `clock_both_entry_exhausted_by_zone_never_wins_a_slot`: a `Both` entry used up through the zone; the next clock slot still starts another HOT entry.
  - (xi) `clock_nobody_won_quiet_rate_is_small`: 200 **independent** one-slot runs, each starting from the same preset state (3 equal-weight entries on 3 areas, #3's `LAST` on, only #1's area HOT): #1 starts in at least 97 % of the runs whose quiet roll passed (expected about 99.5 % with the +10 window, §3.7).
  - (xii) `clock_attempt_in_flight_at_slot_close_can_still_win`: a winner drawn in the last attempt starts after `SLOT CLOSE` and before `slot window + A` (review finding C6).
  - (xiii) `clock_pending_random_of_an_exhausted_entry_does_not_end_the_slot` (walker `RunsOn`): DRAW starts an entry's RANDOM timer; the zone starts the entry's last copy 0.1 s later; the RANDOM output finds `GO` off, and the slot still starts another HOT entry (second review, R4-6).
  - (xiv) `clock_zone_start_inside_the_first_200_ms_has_its_last_cleared`: pins the accepted exception of §3.7.
  - `clock_seeded_slot_rerolls_down_its_list` (`Seeded` variant).
- `clock::validate_schedule(plan, ctx) -> Vec<AirIssue>` holds the §3.7 schedule checks; step 7's `validate` calls it and adds nothing to them. Tests: `clock_validate_schedule_period_shorter_than_jitter_plus_window_plus_attempt_is_an_error` (144 entries, period 671 s, jitter span 360 s: an error, because 671 ≤ 360 + 300 + 73 + 10) and `clock_validate_schedule_slot_window_cap_warns`.

### Step 6. Zone front end — M. Branch `claude/p14-zone`

**Files:** `src/airtask/zone.rs`.

**Depends on:** steps 3 and 4.

**Acceptance (tests):**
- `zone_structure`: an entry with mode Zone: `ZNii CHANCE Random = 50`, `ZNii COOLDOWN.Time = 900`. `ZNii ARMED` is a target of the area's `THsaa INNER` and targets `ARMED ARM`, which activates `MISS WAIT OUT` only. `ZNii CHANCE` targets `FIRE ARM` (which activates `WAITING` and `DENY WAIT OUT`), `FIRE OFF` and `FIRE`; `FIRE.Time = 0.05`. `FIRE` targets every `ENii COPY r` and `DENY WAIT` (`Time = 1`). Every copy's `START` targets `ZNii GRANTED`, which deactivates `DENY WAIT OUT`. `DENY WAIT OUT` and every copy's `DONE HUB` target `ZNii WAITING`, which targets `ZNii COOLDOWN` and its own Deactivate. `ZNii WAITING` starts OFF.
- `zone_has_17_mcus_per_entry` (U13; the shared `ENii EXHAUSTED` is counted by `assemble_side`).
- `zone_stagger_per_area`: three zone entries on one area, 9 copies on the side (`T_count` = 0.6): their `CHANCE.Time` values are 0.1, 0.9 and 1.7, and their `MISS WAIT.Time` values 1.0, 1.8 and 2.6.
- `zone_last_copy_deactivates_rearm_armed_and_fire`: `SideLogic.links` holds `ENii EXHAUSTED` → `ZNii REARM`, `ZNii ARMED` and `ZNii FIRE`; `ZNii FIRE` targets every `ENii COPY r`; `ZNii ARMED` is registered in `SideLogic.pass_subscribers` under its area.
- `zone_ignores_clock_only_entries`: `Clock` entries have no `ZN*` MCUs.
- `zone_debug_subtitles_only_when_enabled`: with `debug_subtitles = true`, `ZNii ARMED`, `ZNii FIRE`, `ZNii MISS WAIT OUT` (chance miss), `ZNii DENY WAIT OUT` (refused), `ZNii COOLDOWN` start and `ZNii REARM` each drive their own subtitle; with `false`, the zone emits no subtitle.
- `both_entry_copies_use_zone_spawn_distance`: lead at 30 km ± 1 m.
- `zone_seeded_chance_sequence_structure` (`Seeded` variant).
- Walker scenarios:
  - `zone_budget_full_refused_rearmed_after_cooldown_then_started`: PASS while the budget is full → `CHANCE` at 0.10 s → `FIRE` at 0.15 s → `DENY WAIT OUT` at 1.15 s → re-armed after 900 s → PASS → started.
  - `zone_clock_launch_before_fire_does_not_strand_the_zone` (second review, R4-2; budget 1, 4 copies on the side, the entry second on its area): PASS at 0; the clock starts the entry's first copy at 0.01 s; that copy is deleted at 0.55 s (a scripted deletion), and the recount shuts the gates until 0.90 s; the zone's `CHANCE` fires at 0.65 s and its `FIRE` at 0.70 s, and the request is dropped; `DENY WAIT OUT` fires at 1.70 s, `COOLDOWN` starts, and the zone re-arms. Run under both target orders.
  - `zone_clock_launch_after_fire_arm_leaves_a_way_back`: a clock start of the same entry between `FIRE ARM` and `DENY WAIT OUT` cancels the refusal; the cooldown starts at that copy's `DONE HUB`.
  - `zone_runs_1_both_entry_used_by_the_clock_starts_no_copy` (R4-6): also with `FIRE` already running when the clock starts the last copy (walker `RunsOn`): `FIRE` gives its output, no `START` passes a pulse, and `REARM` stays off.
  - `zone_three_entries_on_one_area_fill_the_budget`: budget 2, three zone entries on one area, chance 100 %: the first two start and the third is refused (review finding C13).
  - `zone_granted_request_does_not_start_the_cooldown`: a started copy cancels `DENY WAIT OUT`; the cooldown starts at its `DONE HUB`.
  - `zone_no_request_after_runs_starts`.
  - `zone_fire_then_later_chance_miss_reaches_cooldown`: a FIRE, its `DONE HUB`, re-arm, then a failed chance roll: `COOLDOWN` starts and the zone re-arms.
  - `zone_clock_done_does_not_start_idle_zone_cooldown`.
  - **Busy-area check of the clock's no-repeat rule (decision 0029, added 2026-10-18):** one area, a player loitering for 3 h, zone and clock both on it; 200 paired runs (same seed) of `LAST SET` as built (B) and as the two-switch rule (B′), for 2 and 4 entries on the area and budget 1 and 2. Report launches on the area, slots lost and B′'s repeats per 3-hour mission. If B and B′ differ by less than half a launch per mission in every case, B stays; otherwise the user chooses.

### Step 7. Export integration — S–M. Branch `claude/p14-export`

**Files:** `src/airtask/mod.rs` (`build_air_packs`, `preview`, `validate`), `src/frontlines.rs` (`FrontOptions.air_packs`, stamping, `inspect_base_map` recognizer, `ImportedBaseMap.air_tasking_found`), plus the one `FrontOptions` line in `ui.rs` (§7). `airtask/mod.rs` also gets the body of `totals`.

**Depends on:** steps 2–6 and the Probe 2 results.

`build_air_packs` runs each side's builders in the §3.0 build order (assemble → clock → zone → threshold → budget → `emit_side_logic`) over one `SideLogic`. `validate` calls `clock::validate_schedule` for the schedule checks and sets `rail_only = true` on the "no enabled valid entry" issue only (§6.1).

**Test plan, pinned in code** (`export_test_plan()`): one NATO HVAR `Clock` entry on NATO objective 1, one DPRK Il-10 `Both` entry on arrow 1, the default settings, `AirPlan.seed = 7`, date 1950-09-15, the `base_map.Group` fixture inputs.

**Acceptance (tests):**
- `existing_outputs_match_baseline` still passes.
- `generate_front_with_empty_air_packs_matches_baseline`.
- `generate_front_with_disabled_air_plan_matches_baseline`: a plan whose entries are all disabled, orphaned or date-invalid gives an empty `build_air_packs` and baseline output.
- `export_plan_has_air_tasking_groups`: the output contains the `Air Tasking` group with the §3.0 sub-groups; `assert_links_resolve` passes; all indexes are unique; the walker re-arm rule passes.
- `export_is_deterministic`: the same plan (same `AirPlan.seed`) twice gives byte-identical output.
- `inspect_export_keeps_armies_and_flags_air`: `inspect_base_map` on that output gives the same armies and fighters as the baseline and `air_tasking_found == true`.
- `inspect_baseline_has_no_air_tasking`: on the baseline base map, `air_tasking_found == false`, and armies and fighters are unchanged.
- `load_then_generate_has_one_air_tasking_group`: inspect, regenerate with the plan: exactly one `Air Tasking` group.
- `validate_clean_plan_has_no_issues`.
- `validate_flags_orphaned_target`, `validate_flags_out_of_date_sortie` (Il-10 is fine in 1950; the La-11 template is synthesized in the test by changing the `Script` / `Model` of a built-in copy to `la11`), `validate_flags_d30_path_conflict`.
- `validate_orphaned_only_pool_issue_is_rail_only`: an orphaned-only pool gives exactly one Error, with `rail_only == true`.
- `totals_match_the_generated_pack`: for `export_test_plan()` and `acceptance_test_plan()`, `totals` equals a count made by walking the `build_air_packs` output (copies, `Plane` blocks, `MCU_*` blocks, `MCU_CheckZone` blocks). The acceptance plan's totals are pinned as numbers in the test, so a rise is seen (U13).
- `validate_warns_above_totals_limits`: a plan of 30 entries × 3 runs gives one Warning per limit it passes, and no Error.
- `validate_warns_on_locale_id_conflict`: two user templates with different text under one locale id give one Warning that names both files and the id (D41).
- `validate_d30_is_an_error_when_ai_counts`: the D30 case with `strategies.ai_counts = true` gives an Error.
- `arrow_side_matches_generate_front`: for the test arrows, `point_north_of_front` on `ctx.front` and on `dense_base` agree.
- An ignored test `write_p14_acceptance_mission` writes the step 11 mission (§8 step 11) with `debug_subtitles = true`, from a plan pinned in code, `acceptance_test_plan()`: 2 DPRK + 2 NATO objectives at fixed points; NATO: AirAlert `Clock` on objective 1, HVAR `Clock` on objective 2, CloseSupport `Zone` on objective 1, ArmedRecon `Both` on objective 2; DPRK: Yak `Zone` on objective 1, Il-10 `Both` on objective 2; one NATO entry's objective placed so that its spawn → target → RTB path passes within R of a DPRK area (D30); `AirPlan.seed = 7`; default settings otherwise.
- `acceptance_mission_has_zone_entries_and_debug_subtitles`: the written mission has, per side, at least one `Zone` and one `Both` entry's `ZN*` cell, the clock and zone debug subtitles named in step 11, and `validate(acceptance_test_plan())` has exactly one D30 warning and no errors.

### Step 8. Air Tasking rail tab — M. Branch `claude/p14-ui-tab`

**Files:** `src/ui.rs`, `src/shell.rs`, `src/help.rs`, `src/ui_tests.rs`, `USER_MANUAL.md` (stub section only).

**Depends on:** step 7. 8a and 8b run as one step (§10).

**Acceptance, 8a:** the §6.1 arms are in place (the compiler checks the exhaustive ones), the §6.3 "Extend" edits are made, and the §6.3 "New (step 8a)" tests pass. Every guard test passes.

**Acceptance, 8b:** the §6.3 "New (step 8b)" tests pass, "Generate File" and the readiness checks are wired to the real `validate` / `build_air_packs`, and every guard test still passes. `cargo test` count rises by at least the new tests of 8a and 8b.

**Live check (ask the user that the screen is free):** `tools/ui-live/drive.ps1` screenshots of the tab at 1400 × 1000.

### Step 9. Map dock Air tab — M. Branch `claude/p14-ui-map`

**Files:** `src/ui.rs`, `src/shell.rs` (only for the D35 label, if needed), `src/ui_tests.rs`, and the removal of the `// P14` dead-code allows.

**Depends on:** steps 7 and 8b.

**Acceptance (tests):** the dock tabs fit (`map_dock_tabs_fit_a_three_digit_reference_count`), `map_dock_tabs_all_render` includes "Air", and the §6.3 "New (step 9)" tests pass. No `// P14` `allow(dead_code)` remains except the `trace.rs` / `missionlog.rs` ones, which step 9T removes; warnings do not rise.

### Step 9T. Trace and replay in the UI (rest of P10) — M. Branch `claude/p14-replay-ui`

**Depends on:** step 0T, the flight 0 results (carrier, objective style, tick rate), step 9.

- **Trace build.** One header checkbox "Trace build", beside Generate, on every tab. When on, Generate calls `trace::instrument` with `decision_points()` (plus the P14 names on the Air Tasking and Map tabs) and writes the `.trace.json` sidecar next to the `.Group`. Off by default and not remembered between runs, so normal output never changes. The status bar says "Trace build: N breadcrumbs" after Generate.
- **Replay view.** A "Replay…" header button opens a panel: pick the `.Group`, its `.trace.json` (found automatically when it sits next to the `.Group`) and the log folder. It shows the `report_markdown` sections as tables: Dead links, Unobserved successors (with the note that these are observations), Never fired, Never spawned, and a Timeline filter by group.
- **Map overlay.** On the Map tab, a "Replay" layer draws spawn positions (from spawn records) and kill positions, coloured by side, with a legend chip. Off unless a replay is loaded.
- Readiness: Replay needs a `.Group` and at least one log file; a missing sidecar is an orange note (the report then has no breadcrumb sections).

**Acceptance (tests):**
- `trace_checkbox_off_output_matches_baseline` and `trace_checkbox_on_writes_sidecar` (ui_tests: Generate on the Template tab both ways).
- `replay_panel_shows_dead_links_and_unobserved_successors` (ui_tests, synthetic fixtures from step 0T).
- `map_replay_layer_draws_spawns` (counts drawn markers against the replay's spawn count).
- Every new control has an AccessKit label, 28 px height and no missing glyphs: `replay_panel_widgets_are_28_px` (with a replay loaded, disabled controls included) and the glyph guard with `trace.rs` and `missionlog.rs` in its source list (§6.3).

### Step 10. Docs and manual — S. Branch `claude/p14-docs`

**Files:** `USER_MANUAL.md` (`## Air Tasking`, a Map dock "Air" paragraph, and a `## Trace builds and replay` section: the server text-log setting confirmed in step 0T, what a breadcrumb is, the sidecar, the CLI, the Replay panel), `docs/src-guide.md`, `docs/ui-redesign/README.md` §4 / §6.4, `HANDOFF.md` (log row + P14 status), `TemplateExamples/Historical1950/README.md` (fix step 3: the ground AttackArea sits on the spawn point).

**Depends on:** steps 8, 9 and 9T.

**Acceptance (tests):**
- The help test passes.
- `manual_air_tasking_names_every_approximation`: the `## Air Tasking` section contains each of these phrases: "one player", "waterfall", "no back-to-back", "one extension", "last presence check", "AI may count", "runs", "head count", "two requests in the same tick", "zone re-arm", "UNVERIFIED", "Zone Out", "kill events", "Random %", "totals".
- `no_ctrl_1_6_left`: no "Ctrl 1–6" in `src/`, `USER_MANUAL.md` or `docs/`, except the README §9 phase history; the plain map-key "1–6" is still present in `help.rs` and the manual.
- `no_p14_dead_code_allows_left` (grep): no `// P14` dead-code allow is left, except the ones marked `// P14 step 12`.

### Step 11. Full in-game acceptance — user, solo

- Generate the acceptance mission **as a trace build** (step 9T) with `write_p14_acceptance_mission` (step 7), keep the server's text log on, and after the flight run Replay on it. Each check below is read from the replay report first (exact times; the SLOT OPEN, START, DONE HUB and EXTEND breadcrumbs), and from the debug subtitles and video only where the log cannot show it. The mission is: a base map with 2 DPRK + 2 NATO objectives and the pinned `acceptance_test_plan()` (all six built-ins; per side at least one `Zone` and one `Both` entry; one NATO entry that raises the D30 warning), with debug subtitles for slot open, winner, `START`, the end-of-station result, the `DONE HUB` cause, and the zone's ARMED, FIRE, chance miss, refusal, COOLDOWN start and REARM.
- **One tester flies it alone (U14), in two sessions of about 45 min:** one as DPRK (this triggers the NATO sorties) and one as NATO (this triggers the DPRK sorties).
- Check, each against the replay report or a subtitle:
  - slots at first + k·period + jitter step, each ± 5 s (10 / 30 min, + 0 / 3 / 6 min);
  - "winner" subtitles only for areas where the player loitered at least D (no sortie over an empty area);
  - never more than 2 copies per side whose aircraft exist: between a side's second `START` and the next deletion on that side there is no third `START` (U22);
  - **two sorties on one objective that end together** (leave the 35 km zone while both are there): both `DONE HUB`s show, both are deleted about 60 s later in the same tick, and a later slot or zone trigger still starts a sortie (review finding C1);
  - each sortie reaches its target, attacks, RTBs and is deleted about 60 s after its `DONE HUB`;
  - zone entries fire on arrival (chance permitting) and respect the cooldown;
  - **extension:** the player loiters inside r_in of a sortie's target when its on-station time ends: "EXTEND" shows once, and `DONE HUB` follows 10 min later even if the player stays;
  - **Zone Out:** after a sortie reaches its target, the player leaves beyond 35 km: the `DONE HUB` cause is "Zone Out";
  - **AI and the other side:** with the D30 orange warning showing for one entry, no sortie is triggered by the other side's AI (a winner subtitle for that area without the player present is a failure). If flight 0 showed that AI counts, this check is replaced by the D30 error check of step 0d;
  - **totals (D36):** the mission's load time and the server's tick delay are noted next to the totals `validate` reported;
  - **standalone file (risk l):** import the Air Tasking tab's standalone file into a mission that already has groups; check in the editor that indexes do not collide and that links survive, then fly one slot.
- Record the results in `handoff/P14-probe-results.md`.

### Step 12. Multi-player phase — after step 11 (U16)

**Starts only when the breadcrumbs, the replay and every step 11 check are confirmed working.** It needs at least two players. It is planned here and scoped in detail when it starts.

| Part | What | Needs |
|---|---|---|
| 12a | **Complex Trigger reference (user, in the mission editor).** In a blank Korea mission, place one `MCU_TR_ComplexTrigger` with radius 10000. Tick every filter the editor offers (countries, object types, names, any player-only flag). Link each event slot to its own dummy timer, named after the event. Save it as a group, `TemplateExamples/Reference/ComplexTrigger_reference.Group`. Claude then adds the round-trip test `complex_trigger_reference_round_trips` (`parse_group_file` → `serialize_group`) | The user; `TemplateExamples/Reference/` does not exist yet |
| 12b | **Probe flight 2.** File `P14_Probe_2.Group`, written by `write_p14_probe_2` from the 12a reference. Cell **T-d** (Complex Trigger: R 10 km, planes, DPRK countries; each event slot → its own subtitle and its own Counter(2) readout; a DPRK AI pair passes through from 01:00; a DPRK player passes, then circles, from 10:00; Deactivate at 14:00 and Activate at 14:01 with the player inside; a NATO player shoots the DPRK player down inside from 20:00). Cell **T-b** (Closer 0 with two NATO players: both enter, one leaves, then the other; variant: one is shot down inside) | 12a; two or three players; about 45 min |
| 12c | **Threshold for N ≥ 2** (§3.9): `ThresholdImpl::ComplexTrigger`, `CounterResetImpl::{SetVal, Ring}`, the walker's Complex Trigger rule, and the tests `threshold_complex_trigger_structure`, `threshold_counter_ring_structure`, `threshold_n2_fewer_than_n_in_window_does_not_fire`, `threshold_n2_count_resets_at_w`, `threshold_n2_fires_again_after_failed_dwell` | 12b confirms d |
| 12d | **Zone Out strategy.** If T-b shows ANY-out, switch the default to `ZoneOutImpl::Watchdog` (allowed only if flight 0 gave p = "restart") | 12b |
| 12e | **UI.** The THRESHOLD section lets N go above 1 | 12c |
| 12f | **Acceptance with two or more players:** the step 11 mission with N = 2 | 12c–12e |

Confirmed if (12b): T-d "entered" fires once per plane that enters, and a country filter excludes the other side. T-b is ALL-out if the zone fires only after the second player leaves, and fires when the last player dies inside.

---

## 9. Risk register

The repo does not prove any of these in game. The shipped files only show what the authors assumed (`handoff/R3-template-feature-gaps.md` ≈115-122).

| ID | Behaviour | P14 assumes | Repo evidence | Status | Fallback |
|---|---|---|---|---|---|
| a | CheckZone `PlaneCoalitions` fires for AI | Only players should start sorties | `USER_MANUAL.md` ≈75 "until a player enters"; Fighter Pack Zone IN `[2]` 16 km | UNVERIFIED (flights 0 and 1A) | The D30 validator and the dwell. If AI counts, the D30 warning becomes an error and the user decides (§10 item 15). No player-only filter is known for a check zone |
| b | Closer 0 with several players | Fires when ALL have left | `USER_MANUAL.md` ≈75, ≈113 | UNVERIFIED. **It cannot be tested solo: step 12 (T-b).** v1 uses the same Zone Out as every shipped template | Watchdog (it needs p = "restart") plus the hard stop |
| c | Activate/pulse while a player is already inside fires at once | Dwell CHECK and the presence re-check | `USER_MANUAL.md` ≈111; COOLDOWN re-pulse (`flights.rs` `build_flight_group`) | UNVERIFIED (flight 1A) | Closer 0 + race-timer negative logic, only if z is confirmed. If c and z both fail, the build stops at step 4 |
| d | Complex Trigger filters and events | Per-object "entered alive", country filter | None: 0 hits in samples; only the drop-list in `template.rs` | UNVERIFIED. **Step 12 only**; it needs the user's 12a file and a two-player flight | v1 does not use it |
| e | Counter reset (Dropcount, ModifierSetVal) | `Dropcount 1` = reset after firing (the codebase convention, §3.2 note); ModifierSetVal clears a partial count (the budget's `COUNT`, §3.3). Step 12 only: a fired `Dropcount 0` counter fires again after a reset (T-e E6) | `template.rs` `DeathCount` / `Reset Counter`, `flights.rs` `DeathCount`, commit `7437c51` | UNVERIFIED (flight 0) | `BudgetImpl::SingleSlot` (budget 1, no counter). Counter ring in step 12 |
| f | Timer `Random %` rolled on each trigger | Quiet, jitter, waterfall, zone chance | `flights.rs` ≈1010-1013, `recon.rs` ≈1246-1256 | UNVERIFIED (the Fighter Pack already depends on it) | `RandomImpl::Seeded` |
| g | Kill events per aircraft | Type 13 once per plane; no 4/2/0 latch and no event-7 abort (decisions 0011/0015/0017) | `handoff/P14-probe-results.md`, flights 1G/1P | VERIFIED for gun/Damage loss and fuel-bail-out; crash without damage remains for flight 2 T-l | On-station end, Zone Out and hard stop still end the sortie |
| h | Deactivating a waiting checkzone cancels it | INNER CLOSE, INNER OFF | `USER_MANUAL.md` ≈203, `flights.rs` ≈905-915 | UNVERIFIED | Gate a downstream relay instead |
| j | Relay gating; running-timer cancel | D11 relays | recon/flights closers | UNVERIFIED | Explicit clear; OUT relays already cover the cancel case |
| k | The head count in game | §3.3 | none | UNVERIFIED (Probe 2, cell T-k) | `BudgetImpl::SingleSlot` |
| l | Editor re-indexes a standalone group on import | The standalone Air Tasking file imports cleanly | R4 P13 "first check" | UNVERIFIED | Offer only the base-map path |
| m | `rotate_tree` and MCU `YOri`; AttackArea overridden by a later order; Force Complete Priority 2 cancels an AttackArea | Placement and cleanup | R3 in-game checks | UNVERIFIED | Probe 2 (step 3) |
| n | All `model_spec` cruise speeds | D and H use 732 km/h | No citation (`model_spec.rs` ≈1-21; the `f80c10` row has an empty notes field) | Unsourced | If the player speed measured in cell T-n is off by more than 10 %, set `timing::F80_CRUISE_OVERRIDE_KMH` to the measured value, with a comment citing `handoff/P14-probe-results.md`. `model_spec` is not changed (D40). `Manual timing` stays a per-plan override |
| o | A pulsed Closer 1 zone fires again while a plane stays inside | Not relied on | Templates self-deactivate Zone IN | UNVERIFIED | None needed: ENTRY OFF plus the ARM latch (§3.5) |
| p | A running timer re-triggered | Not relied on; the one template timer that is re-triggered (AirAlert 39) reaches generated logic only through the `ON STATION IN` latch, and the budget's `CHECK` latch keeps a recount from being re-triggered | AirAlert `WP 1` → [37, 39], 37 → 39 | UNVERIFIED (flight 0, J5) | A once-gate in front of any timer the walker shows can be re-triggered |
| q | Same-tick delivery: a latch hit twice in one tick; Activate before a pending fan-out pulse | A latch passes once | none | UNVERIFIED (flight 0, J4) | A once-gate made of `Counter = 1`, `Dropcount 0`, if E8 is confirmed. Otherwise doubled pulses are accepted: they cannot corrupt the head count (§3.3) |
| t1 | A Mission Objective MCU writes an `AType:8` line to the text log **each** time it fires | Trace builds (step 0T) | None in the repo; community log documentation | UNVERIFIED (cell T-t T1, T2) | `TraceCarrier::Spawn` (spawn line per firing) |
| t2 | `AType:8` identifies the objective by its index (`OBJID`) or only by position | Sidecar keeps both; match by index, else by position ± 1 m | None | UNVERIFIED (T-t) | Position match only |
| t3 | A trace objective shows nothing to players and does not affect the mission result | Silent `ObjectiveStyle` (T4 picks it) | Airfield files carry objectives (`K14 AFB.Group` ≈21535) but say nothing about display | UNVERIFIED (T-t T4, the players' notes) | `Spawn` carrier |
| t4 | `T:` counts ticks at 50 per second | Timeline in seconds | Community documentation only | UNVERIFIED | The replay fits the rate from the cue breadcrumbs and uses the fit |
| t5 | Two breadcrumbs in one tick both log | "Unobserved successors" and counts | None | UNVERIFIED (T-t T3) | Report same-tick firings as "at least one" |
| t6 | The log records everything the video shows (no dropped lines; the last file written on a normal mission end) | Replay is the primary evidence in 0d and step 11 | None | UNVERIFIED (T-t T6) | Video and subtitles stay the evidence; the replay is a cross-check |
| r | Several pulses into one counter in one tick are counted one by one | Not relied on: the census spaces its pulses 0.05 s apart | none | UNVERIFIED (flight 0, E7) | `CensusImpl::Spaced` is the default |
| u | Spawn-mode entity association after respawn | Air Tasking accepts Activate-mode S1–S4/S7 only | Flight 1L V2; decisions 0013/0017 | Spawn/Repeat templates refused; no Spawn cell in flight 2 | "Air Tasking needs an Activate-mode template" plus the regeneration guidance |
| v | What a flight does after its AttackArea `Time` has run out | The extension adds time only | none | UNVERIFIED (Probe 2) | The shell adds `SNjj RENEW`: the attack order is given again when the extension starts, and `DONE HUB OFF` switches it off (§3.4) |
| w | Mission load time and server tick delay with many inactive copies | The D36 limits | none | UNVERIFIED (Probe 2 load check, step 11) | Lower the limits; a smaller Auto-fill (§10 item 23) |
| z | An empty `Closer 0` zone fires at once when pulsed | Only the `Closer0Race` fallback | none | UNVERIFIED (flight 0, T-z) | None. If c and z both fail, the build stops at step 4 |

---

## 10. Notes for the build

**Who does what (U19).** Astra (GPT-6 Astra, through the Codex broker) writes the code, one step per delegation. Fable scopes each step, writes the delegation prompt, reviews the diff, re-runs the build and the tests, and commits. The `codex-delegation` skill holds the rules. Fable writes code only with the user's permission for that task.

**Before any code:**
- Read `HANDOFF.md`, `CLAUDE.md`, `.cursorrules` and `docs/ui-redesign/README.md`.
- Confirm that the working tree is clean and that `main` has no merge in progress. Do not touch or stash anyone else's uncommitted edits.
- Confirm the model: every delegation passes `model` explicitly, and the job's `output.log` must name `gpt-6-astra` (`HANDOFF.md` §2).
- Create the integration branch `claude/p14-air-tasking` from `main` (`a34a00e` or later; it must contain the upstream commit `c1d88c3`, U15).

**How one step runs (same-host delegation).** Claude and Codex share this machine's disk, so there is no GitHub round trip:
1. Fable checks that the tree is clean and creates `codex/p14-<step>` from `claude/p14-air-tasking`.
2. Fable delegates with `codex_start` (`reasoning_effort: "max"`, `sandbox: "workspace-write"`, `max_idle_seconds: 1200`). The prompt names this file and the step, lists the files in scope and the files not to touch, quotes the step's acceptance tests, and orders `cargo build`, `cargo clippy` and `cargo test --offline` before Astra reports. Astra edits files only; it makes no git writes.
3. Fable reads the whole diff, re-runs the three commands itself, and checks the step's test names by grep.
4. If it passes, Fable commits on the step branch and merges it into `claude/p14-air-tasking` with `--no-ff`. If not, Fable sends the delta back with `codex_resume`.
5. Fable stops and reports (below).

**Order.** The steps run one after another. There is no parallel fan-out.

| # | Step | Files it may change | Gate before it starts |
|---|---|---|---|
| 1 | **1a** skeleton | `main.rs`, `trace.rs` and `missionlog.rs` stubs, `airtask/mod.rs` (§5 exact source), every `airtask/*.rs` stub, `in_theatre.rs` and `baseline_tests.rs` stubs, `MapAirPack` in `frontlines.rs`, the visibility-only edits and `timer_random` in `template.rs`, `recon.rs`, `placement.rs` | — |
| 2 | **1** core helpers and baselines | `placement.rs` (`rotate_tree`), `weapon_range.rs`, `airtask/timing.rs`, `in_theatre.rs`, `baseline_tests.rs`, `src/testdata/p14_baseline/`, the baseline addition to `ui_tests.rs` | — |
| 3 | **1b** walker | `airtask/testkit.rs` | — |
| 4 | **0T** trace and replay core | `trace.rs`, `missionlog.rs`, `src/testdata/missionlog/` | — |
| 5 | **0b** probe generators | `airtask/probe.rs`, `locale.rs` (probe strings only) | — |
| — | **Flight 0** (user, 11 min) and its 0d results | `handoff/P14-probe-results.md`; the trace defaults in `trace.rs` | Tell the user the file is ready, with the server text-log setting |
| 6 | **2** library and placement | `airtask/library.rs`, `airtask/place.rs` | None. It may run while the user flies flight 0 |
| — | **Flights 1A and 1B** (user, 26 and up to 30 min) and their 0d results | `handoff/P14-probe-results.md` | Flight 0 results (they choose the trace carrier) |
| 7 | **3** shell, budget, assembly | `airtask/shell.rs`, `airtask/budget.rs`, `airtask/assemble.rs` | **G0: flight 0 results** |
| — | **Probe 2** (user, solo, about 40 min) | `handoff/P14-probe-results.md` | Step 3 |
| 8 | **4** threshold | `airtask/threshold.rs` | **G1: flight 1A results** |
| 9 | **5** clock | `airtask/clock.rs` | — |
| 10 | **6** zone | `airtask/zone.rs` | — |
| 11 | **7** export | `airtask/mod.rs`, `frontlines.rs`, one line in `ui.rs` | Probe 2 results |
| 12 | **8** rail tab (8a and 8b as one step) | `ui.rs`, `shell.rs`, `help.rs`, `ui_tests.rs`, `USER_MANUAL.md` (stub section) | — |
| 13 | **9** Map dock tab | `ui.rs`, `shell.rs`, `ui_tests.rs` | — |
| 14 | **9T** trace and replay UI | `ui.rs`, `ui_tests.rs` | — |
| 15 | **10** docs | the documents named in step 10 | — |
| — | **11** acceptance (user, solo) | `handoff/P14-probe-results.md` | — |
| — | **12** multi-player phase | scoped when it starts | Step 11 confirmed (U16) |

- A change to a type in `airtask/mod.rs` after step 1a is its own small commit by Fable, before the step that needs it.
- Where a step's text in §8 says "wave", "parallel with" or "through the coordinator", read it as this serial order. Those words come from the earlier fan-out plan.

**Why the gates (§10 item 16).** Flight 0 takes 11 minutes and answers whether relays, counters, latches and random timers behave as the design assumes. Step 3 is the first step that depends on those answers. Flight 1A answers the check-zone questions that step 4 depends on. Waiting for them costs little and removes the risk of building on a wrong assumption.

**Verification expectations:**
- Every acceptance line in §8 is a named test; Fable checks coverage by grepping the names.
- Run the walker's re-arm rule and `assert_links_resolve` over every generated air pack in steps 3–7.
- Keep the baseline fixture test green from step 1 onward.
- Record the test and warning counts in the `HANDOFF.md` log row at each stop.
- After step 7, record the totals (D36) of `export_test_plan()` and `acceptance_test_plan()` in the stop report.

**Decided by the user in round 4** (2026-09-27; these were open in earlier drafts):
- Item 1, the extension condition: the area's state (U17, D32).
- Item 3, user templates from disk: they stay in v1 (U18, D39).
- Item 6, budget races: replaced by the head count (U20, D25).
- Item 11, the threshold for two or more players: step 12 (U16).

**For the user to confirm (additions and deviations; implement as written, and list them in the first stop report).** The numbers are kept from earlier drafts, because the text refers to them.

2. **When the extension is decided (D31).** At the end of on-station time only. The hard stop is final.
4. **Run cap on the clock (D12).** Each entry runs at most `runs` times (default 3, range 1–6) across both front ends, so a pool of E entries gives at most E × runs sorties per mission. The manual states it, and the Runs tooltip shows it.
5. **No-repeat can leave a slot quiet** when only the last winner's area is busy (§3.7).
7. **Zone re-arm residual (§3.8).** For a `Both` entry, a clock copy ending first can re-arm the zone while the zone's own copy is flying.
8. **MiG-15 date (D34).** `mig15bis` is valid from 1 Nov 1950 as the stand-in for the MiG-15.
9. **Debug subtitles** exist only in the step 11 test mission, not in the UI.
10. **"Refs" dock label (D35)** if five tabs do not fit.
12. **A clock slot can go quiet with a passing entry available (U6, §3.7).** U6 says to reroll until an entry passes. Attempts in which the waterfall picks nobody (possible once the last waterfall entry is excluded as LAST or by an area miss) are random and not bounded, while the slot window is finite (`min(300 s, (a + 10)·A)`). In the worst simple case (3 entries, the only passing one first in the waterfall) about 0.5 % of slots go quiet; with many entries, where the 300 s cap binds, more can. The walker test `clock_nobody_won_quiet_rate_is_small` pins the rate; the manual states it.
13. **P10 is split (U12).** The replay core (parser, timeline, never fired, unobserved successors, dead links, never spawned, Markdown report, CLI) is step 0T, before the probe flights. Its UI (Trace build checkbox, Replay panel, Map spawn/kill overlay) is step 9T. R4 P10's "templates that never spawned" is reported as counts per entity name, because copies share names and the log carries no group path; a name is tied to a group only when it is unique in the file.
14. **Trace builds use Mission Objective MCUs as log breadcrumbs,** with a spawned-object fallback. Both are UNVERIFIED until cell T-t; if neither logs, tracing falls back to debug subtitles (§8 step 0T).
15. **If check zones count AI aircraft (result a).** No player-only filter is known for a check zone. The build then refuses an entry whose area lies within R of another side's sortie path (the D30 warning becomes an error). The user decides after flight 0 whether that is enough.
16. **Gates G0 and G1.** Step 3 waits for the flight 0 results and step 4 for the flight 1A results. Earlier drafts built steps 3–6 before any result.
17. **Decided (U22): the budget is released when the aircraft are deleted** (D38). It is a hard limit on the flights whose aircraft exist. A sortie that is flying home holds its place for about 60 s, so a replacement starts about a minute later than it would otherwise.
18. **Totals limits (D36).** The warning limits (96 aircraft, 1500 MCUs, 40 check zones added by Air Tasking) are Claude's estimate. Set them from the Probe 2 load check.
19. **The one way over budget (§3.3).** Requests that pass open gates in the same tick all start. It needs unrelated triggers in one tick (the clock and a zone, or zones on different areas), and it heals at the next recount.
24. **One copy can be started twice in one tick (§3.3, third review T10).** It needs the clock and the zone to pick the same `Both` entry in the same 0.02 s tick. The copy then gets its orders twice. It is accepted and stated in the manual; no MCU is added for it.
25. **Decided (U21): tested template shapes only** (§3.2, D39). A template with event or report hooks, with a time-on-target step, or with units that are not planes is refused with a message. A shape is added by adding its test.
20. **Zone stagger (§3.8).** The second zone entry on an area fires `T_count + 0.2` s after the first, the third twice that.
21. **Hard stop is 10 min later than in earlier drafts** (`T_final` includes the extension time), because it is now final.
22. **Decided (U24), provisional: one check zone per area in place of two.** Detect inside r_in, then check again one inner-zone transit time later. It saves about 6 MCUs and one check zone per (side, area), but it changes U3 (the dwell comes from the transit of the inner zone, not of the whole zone). Applied in §3.5 and §4.2 when step 4 is scoped; the two-zone design stays in this file as the fallback.
    - **Tests.** No extra flight. (1) Flight 1A cell T-c "C2 fired again" already tests the one thing the design needs from the game: a check zone that is deactivated, re-activated and pulsed while the player is inside fires again. (2) Step 4 walker tests: `threshold_one_zone_straight_pass_fails` (a track through the centre at cruise gives FAIL), `threshold_one_zone_loiter_passes` (a track orbiting inside r_in gives PASS and HOT), `threshold_one_zone_edge_orbit_fails` (an orbit between r_in and R never goes HOT). (3) Step 11 adds a check: fly straight through an area without loitering; no winner subtitle for that area.
    - **Revisit (go back to two zones) if:** C2 does not fire again in flight 1A; or in step 11 a straight pass makes an area HOT, or a loiter of at least D inside r_in does not.
23. **Not applied, needs the user: a smaller Auto-fill.** "Auto-fill from objectives" adds one entry per date-valid sortie for each target. Adding one entry per target instead (rotating through the side's sorties) would cut the copies, and so the aircraft and MCUs, to a quarter on the NATO side.

26. **To explore, not scheduled (user, 2026-10-08): the gate-switch view.** The design is already a circuit of IL-2's gate primitives, named in §3.0 (relay, latch, pair, cancel, once-gate, counter threshold, set-value reset). Explore making that explicit as a small cell library, `airtask/gates.rs`: one builder per idiom (`relay`, `latch`, `once_gate`, `threshold(n)`, `reset`, `pair`), each with its MCU count and start state pinned by a unit test, so the shell, budget, threshold, clock and zone builders stamp the same cells instead of each writing its own.
    - **What it might buy.** Fewer MCUs where a builder hand-rolled a heavier idiom; one place for each probe fallback (the J4 once-gate swap would then apply to every latch at once); a walker that knows the cells and checks the re-arm rule per cell; §3 diagrams that read as cells.
    - **Rules.** Output stays byte-identical where the behaviour is the same. The counts per copy, side, area and entry never go up. No MCU types beyond the probed ones. It is presentation of the same logic, not a new design.
    - **Limits to keep in mind.** The game has no true AND (a `Counter = 2` means "two pulses before the reset", order-free and timing-bound), no compare and no edge detection; state is only active or inactive plus a counter's running value. Cells must say which of these they rely on.
    - **When.** After step 3 lands: the shell and the budget are the first two builders that share idioms, so that is the first point where the repetition can be measured. Its own small step by Fable, off Astra's serial path, with a count table before and after.

**Stop and report.** After each step, list what changed, the test count and the warnings. Say what needs the user in game. Do not start the next step until the user has seen the list if the user asked for that at session start.

**Nothing is pushed or merged to `main` without the user.** No tags, since a `v*` tag triggers the release autobuild. Never `git add -A`. `.gitattributes * -text` stays.

---

## 11. Revision log

- **Round 1 review (2026-09-24).** Re-arm edges added to every cancel relay (§3.0, §3.5, §3.7, §3.8). Lead rule fixed (empty Targets). Threshold entry self-deactivates behind an ARM latch. Budget level switches use SETTLE timers. Clock attempts are fixed length, exclude whole areas, and end the slot on a win. Stop probe moved to the end of on-station with a fresh inner check (D31, D32). One ON STATION per sortie. Zone cooldown gated by WAITING. Walker made its own step (1b) with a semantics table. Skeleton commit, dead-code allows, `MapAirPack` and the `FrontOptions` line given owners. Named tests for every acceptance line; probe "Confirmed if" column; T-k moved to Probe 2 with a 6th request; new cells C5, J4, J5 and risks o, p, q. Shortcut edits limited to "Ctrl 1–6". Code anchors re-based on `bf1a356` and named by function. `in_theatre` seeding uses `mig15bis` as the stand-in and has no La-9 row.
- **Round 2 review (2026-09-24).** Builder `pub(crate)` edits and `timer_random` moved into the 1a skeleton, so the 0b probe compiles in wave 1b; probe AI flights come from the HVAR / Yak files through `probe_flight`. Step 0 split into flight 1 (all cells but T-d) and flight 2 (T-d, after 0a) with its own ignored writer and tests. Run sheets added (K14 origin, offsets, arm / close times, player per cell, cue subtitles; constants pinned by `probe_run_sheet_matches_the_table`). T-a split into T-a1 (AI triggering) and T-n (player-measured F-80C cruise, the basis of W and D). Every count is read through threshold counters with their own subtitles (C5, F1, T-d). Walker Counter rule corrected (`Dropcount 1` = reset, the codebase convention; configurable until T-e), `MCU_TR_MissionBegin` rule added, Exclusive self-test defined. §5 now gives the exact skeleton source (derives, `Default`s, `DPRK`/`NATO` index constants, `AirPreview` / `AirIssue` / `AirDrawCounts` fields, strategy enum variants) and a `baseline_tests.rs` stub. `AirPlan.seed` added (the Map has no stored seed). The `base_map_ui.Group` fixture and `map_generate_without_air_matches_baseline` moved to step 1 with explicit inputs and terrain off. §10 item 11 states the N = 1 default and the 0a unlock. Anchors re-based on `4c90de6`.
- **Round 3 verification (2026-09-24).** Probe: C4 now tests risk h (activated and pulsed at 11:30 while N1 is outside, deactivated at 11:45, cue at 12:00); every "exactly once" criterion reads a `Counter(2)` readout (E1, E2, E6, J4, J5, each T-g type 4), probe subtitles `Duration = 1` and cues 20; T-g's G1 has its own centre 30 km east with an icon; new cell E6 (a fired `Dropcount 0` counter re-fires after `ModifierSetVal`) and walker switch `SetValRefires`; ZA4 belongs to T-n for arming and closing. Skeleton: `pub use` of `builtin_library`, `load_user_sortie`, `f80_transit_s`, `derived_timing` (signature stubs in 1a), new `DerivedTiming`, `SideLogic` (cross-module wiring, build order, owner of `AT s *` = step 3), `PresenceCheckImpl` (risk c), `AirIssue.rail_only`. Copy pointer and `ENii EXHAUSTED` moved to `assemble.rs`. RANDOM k at 0.5·k s, matching `build_randomizer`. Slot window `(a + 10)·A` and §10 item 12 (quiet slot with a passing entry). Map readiness no longer blocks on an orphaned-only pool. `SNjj ON STATION IN` latch (AirAlert timer 39 is pulsed twice). Step 5 schedule checks moved to `clock::validate_schedule`; step 8 split into 8a / 8b; serial fallback when local merges are declined; Load tests moved to step 9; step 11 plan pinned as `acceptance_test_plan()` with Zone / Both entries, a D30 case and zone debug subtitles; the h/j, p, q fallbacks stated as post-probe code changes outside the strategy enums.
- **Round 3 follow-up (2026-09-24, Claude).** Added `CLK s LAST CLEAR` (0.2 s after SLOT OPEN): without it nothing turned an entry's `LAST` off except another entry's win, so an entry whose area was the only HOT one stayed excluded for good after its first win, contradicting `clock_only_one_area_hot_alternates_with_quiet`.
- **Trace and replay (2026-09-25, Claude, user request U12).** New step 0T (trace builds with Mission Objective breadcrumbs and a `.trace.json` sidecar; `missionlog.rs` parser, replay and `replay` CLI), built in wave 1b before probe flight 1; flight 1 flies the traced probe. New cell T-t (04:30–05:00) tests the breadcrumbs (t1–t6 in §9). Step 9T adds the UI (Trace build checkbox, Replay panel, Map overlay) before step 10; step 11 reads its checks from the replay report. Scope note: the offline replay is not the excluded live log reader. §10 items 13–14 added.
- **Trace and replay review (2026-09-25).** One reviewer, 9 findings, all applied: T-t owned by 0b and excluded from `instrument` (no double breadcrumbs), `trace::breadcrumb` / `TraceMap::push` stubs; entity sources traced through `OnEvents`, edges walk follows OnEvents with a visited set; step 9's allow rule excludes `trace.rs` / `missionlog.rs`; `trace_off_changes_nothing` deferred and relaxed for 9T; T5 fires twice and no longer expects a removal line; `expected_s` and a slope + offset tick fit; T4 4 s apart, T-t to 05:30, F2 moved to 05:30; trace grid from `geo.rs` / `placement.rs` bounds; never-spawned counted per name.
- **Round 4 (2026-09-27): Fable + Astra review, and user decisions U13–U20.** The review is `handoff/P14-review-fable-astra.md` (finding numbers C1–C14, N1–N4 below are its numbers).
  - **Budget (C1, C13, U20).** The level counter is replaced by a head count: one flying flag per copy, the copy's `START` relay as its gate, a recount at every launch and end, no running total. Requests need no answer, so REQ, GRANTED, DENIED and the per-copy timeout are gone. 1 MCU per copy (was 6 + 2N) and 16 per side (14 at N = 1). New fallback `BudgetImpl::SingleSlot`; new `CensusImpl`; new probe cell E7.
  - **Extension and shell (U17, N2).** The extension reads the area's state through a HOT / COLD pair; the stop-probe cell and its check zone are gone. The hard stop is final and 10 min later. `DONE HUB` releases the budget and the zone cooldown (D38); the `DONE` relay is gone. The shell adds 16 MCUs per copy (was about 38).
  - **Threshold (C5, U16).** v1 builds the one-player path only. HOT stays on during a re-check. HOT and COLD are two sets switched make-before-break. The Complex Trigger path, the counter window, cell T-d, cell T-b and editor task 0a move to the new step 12; their designs are kept in §3.9.
  - **Clock (C6, D37).** The MISS race is replaced by the HOT / COLD pair; attempts are L + 0.5 s. The period check adds one attempt length. `LAST SET` comes from `START`, so a refused entry is not excluded from the next slot.
  - **Zone (C13).** Entries on one area fire staggered. Refusal is a per-entry race timer cancelled by the copy's `START`.
  - **User templates (C8, U18).** They stay in v1. `library::inspect_sortie` finds every part of a template by its wiring, so Activate-mode (`MISSION BEGIN`) and Spawn-mode (`SPAWN UNITS`) templates are both accepted. The navigation lead is the first lead with a waypoint. An orphan sweep removes what the cut leaves unreachable, a repeat-mode respawn loop included; a template with a second link into the end timer is refused. Locale ids keep the existing first-wins merge, with a warning (D41).
  - **Solo testing (U14, N1).** Step 0 is three solo flights: 0 (automated cells and the trace test, 11 min), 1A (NATO, 26 min), 1B (DPRK, up to 30 min). Probe 2 and step 11 are solo. New cells T-z, E7, E8. Probe 2 flies HVAR, AirAlert and a Spawn-mode sortie, and has a load check.
  - **Fallbacks (C7).** §8 step 0d lists, for each probe result, the strategy to set and whether the build stops. Two results stop the build: J2 (before step 3), and c with z (at step 4).
  - **Replay (C9).** Edges follow pulse links only. "First dead link" is split into "Unobserved successors" (an observation) and "Dead links" (unconditional paths only).
  - **Output safety (C10).** The measured cruise speed is a P14 constant (D40); `model_spec` is not changed. The Template baseline has an F-80C seat.
  - **Tests (C11, C12).** Probability tests use independent one-slot runs. Lifecycle tests use `clock_test_plan()`. DONE timing names `DONE HUB` or `Trigger Delete`. T-t closes at 05:30 everywhere. New UI guards for a filled pool, the dock and the Replay panel; the glyph guard reads the new files. One MCU-count test per builder (U13).
  - **Totals (C14, U13).** `airtask::totals`, the limits of D36, a `validate` warning and a line in the UI.
  - **Small fixes (C2, C3).** `trace.rs` uses `i32`. Runs: default 3, range 1–6.
  - **Upstream (C4, U15).** `c1d88c3` is merged (`main` @ `a34a00e`); 469 tests pass. It removes the 50 % ceiling auto altitude and the Fighter Pack reinforcement timer.
  - **Builder (U19).** Astra builds, step by step in serial order, with gates G0 (flight 0 before step 3) and G1 (flight 1A before step 4). §10 is rewritten.
  - **Second review:** Astra reviewed the round 4 designs the same day. See the next entry.
- **Round 4, second review (2026-09-27, Astra; findings R4-1 to R4-8).** Astra re-read the revised plan. It found the head count, the HOT / COLD sets, the clock timing and every stated MCU count sound, and confirmed its ten round 3 findings fixed or partly fixed. Its new findings, all applied:
  - **R4-1.** A counter once-gate cannot be re-armed, so it replaces single-use latches only. If J4 and E8 both fail, the build stops before step 3.
  - **R4-2.** The zone arms `WAITING` and `DENY WAIT OUT` when the chance roll succeeds, and sends its request 0.05 s later. A clock launch of the same entry can no longer strand the zone.
  - **R4-3.** The J4 and E8 results are read with the E7 result or the trace log. New probe cells E9 (the census spacing) and "C2 again" (a check zone used a second time).
  - **R4-4.** The shell uses the navigation lead throughout, finds the on-target node whether it is a timer or a waypoint, reuses a template's own RTB waypoints, and keeps event hooks that are not kill events.
  - **R4-5.** The budget counts sorties, not aircraft: up to 2N flights' aircraft can exist for about a minute after an end. The walker gets a `Batched` same-tick mode so the overshoot can be tested.
  - **R4-6.** Exhaustion also switches off `ENii GO`, `ZNii ARMED` and `ZNii FIRE`.
  - **R4-7.** If the extension needs a new order, `SNjj HOT` gives it when the extension starts.
  - **R4-8.** Test timings corrected (on-target delay, Probe 2 cell T-k and new T-k1, zone refusal time, the `LAST` exception).
  - **MCU savings Astra proposed, applied (U13).** `START` is the copy's gate (16 per copy, was 17). The budget has no `EVENT` relay and no separate latch Deactivate (16 per side, was 18), and no counter at N = 1 (14). The threshold's CHECK, PASS and FAIL are events, not relays (23 per area, was 26).
  - **Not re-reviewed:** the text of this second-review pass itself. Fable reads each step's section again when it writes that step's delegation prompt.
- **Round 4, third review (2026-09-27, Fable and Astra; findings T1 to T15) and user decisions U21 to U23.** The user asked for one more review before the commit. It found the head count, `START` as the gate, the zone handshake, the repeat-mode sweep and every MCU count sound, and fifteen further points. All fifteen are applied:
  - **User templates: tested shapes only (U21; T1 to T5).** `inspect_sortie` accepts shapes S1 to S7 and refuses the rest, each refusal with its own message and test. A lead's attack timer is found through the Objects of its command, so Yak and Il-10 (two leads, one waypoint) are no longer ambiguous. Templates with event or report hooks, or with a step between the attack order and Mission Complete, are refused. A lead with no RTB of its own gets one.
  - **Hard limit on aircraft (U22; T11).** A copy leaves the head count when its aircraft are deleted: the delete timer switches the flag off through `SNjj LANDED` and starts the recount. The shell adds 17 MCUs per copy (was 16).
  - **`SideLogic.links` (T6).** A builder that needs a link out of another builder's MCU pushes a pair; `emit_side_logic` applies it. `ENii EXHAUSTED` becomes one Deactivate per entry, shared by both front ends: 12 MCUs per clock entry (was 13) and 17 per zone entry (was 18).
  - **Skeleton (T7).** `MapAirPack` derives `Debug` and `Clone`; `mapnet::is_rtb_waypoint` becomes `pub(crate)`; stubs name unused arguments with an underscore; the step 12 enum variants keep a marked dead-code allow.
  - **Trace (T8).** `TraceSelect` gets `event_types` and `random_timers`; `TraceEntry` gets `event_type`.
  - **Threshold (T9).** The timeout switches the inner zone off first and counts 0.05 s later, so a pass and a timeout in one tick give a pass only. `ENTRY OFF` also closes the `ARM` latch. Still 23 MCUs per area.
  - **One copy started twice in one tick (T10).** Accepted and documented; a walker test pins it.
  - **Probes (T12).** New cells T-t T7 (one breadcrumb fired twice in a tick) and "Z1 again"; a 0d row for E9; the rule for reading J4 and E8 uses T7.
  - **Tests (T13).** Probe 2 cell T-k spaces its requests beyond the recount time; T-k1 deletes one overshooting copy first; zone timings and names corrected.
  - **Extension fallback (T14).** If the probe calls for a renewed order, it goes through `SNjj RENEW`, which `DONE HUB OFF` switches off.
  - **Stale text (T15).** Step 0b names its MCUs; risk v and D5 corrected.
  - **No further full review (U23).** These last changes are not reviewed by a second model. Fable re-reads each step's section when it scopes that step for Astra.
- **Gate-switch view (2026-10-08, user).** §10 item 26 added: explore a cell library of the gate idioms (relay, latch, once-gate, threshold, reset, pair) shared by every builder, scoped after step 3. Not scheduled; no design, count or step changed.
- **2026-10-18, step 4 built.** §3.5 rewritten as built: one country-filtered Complex Trigger per area for the entry and a pulsed check zone for the presence check, 19 MCUs (decisions 0023, 0024, 0025; flight 2J showed a switched-on Complex Trigger misses a plane already inside). U24 timing in `timing.rs`. `PresenceCheckImpl` removed.
- **2026-10-18, step 5 built.** §3.7 opens with the clock as built (decision 0029 B): quiet roll and jitter as J random timers into a latch, `ATTEMPT END` cancelled directly, a 2-MCU entry randomizer, one-Activate `LAST SET`, `GO` lists copy pointers last run first; 11 MCUs per entry and 17 + J + a per side (55 on the three-entry test plan, from 76). `RandomImpl` and the `Seeded` variants removed (flight 0 confirmed f). Step 6 adds a busy-area check of the `LAST SET` rule (B against B′).
