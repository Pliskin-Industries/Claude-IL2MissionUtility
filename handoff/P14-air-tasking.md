# P14: Air Tasking (historical air sorties against Map objectives)

Status 2026-09-25. **Fully planned, nothing built.** Revised after review rounds 1 and 2 and the round 3 verification, then extended with trace builds and the P10 replay core before the probe flights (U12, step 0T). §11 lists the changes. This file is the whole brief for a separate Opus "ultracode" implementation session. That session reads this file and the repo, and nothing else from the planning conversation.

Repo root: `C:\Claude\IL2MissionUtility\Claude IL2Mission Utility`. All paths below are relative to it.

**Code anchors.** Every anchor is given by item or function name first. Line numbers are approximate, for `main` @ `bf1a356` (after the `claude/ui-polish` merge). `main` kept moving while this was revised (now `4c90de6`, after the `claude/terrain-apply` merge `7b9a2d0`), so find each anchor by its name and use the number only as a starting point.

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
  - An air budget allows at most N active sorties per side.
  - A player-threshold trigger.
  - A stop probe with one engaged extension (§3.4).
- Front end A, the **mission clock** (primary). Slots come from a timer chain started at Mission Begin. Each slot draws from a weighted pool, with jitter, a quiet chance, no back-to-back repeat, and a reroll when the threshold check fails.
- Front end B, the **check zone**. A threshold zone at the target, with a chance to fire, a cooldown, a repeat limit, and drop-and-re-arm when the budget is full.
- One pool and one budget shared by both front ends.
- A 7th rail tab **Air Tasking** and a 5th Map dock tab **Air**.
- Date filtering from a new in-theatre table keyed on `model_spec` ids.
- Built-in library of the six files, plus user templates added from disk (a Claude addition, §10 list).
- Export inside Map's Generate Base Map, and as a standalone file from the Air Tasking tab.
- Step 0: generated in-game MCU probe missions that settle the risk register (flight 1 now; flight 2, the Complex Trigger cell, after the user's editor task 0a).
- Step 0T, built **before** the probe flights (U12): **trace builds** (breadcrumb MCUs that write a line to the server's text mission log when a chosen MCU fires, plus a `.trace.json` sidecar) and the **replay core** of R4 P10 (parse `missionReport*.txt`, timeline, fired / never fired, first dead link, templates that never spawned; a Markdown report from a CLI). The probe flights test both. The replay UI (Trace checkbox, report view, Map overlay) is step 9T, before the step 11 acceptance flight.

**v1 explicitly excludes:**
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
| U1 | Player threshold is **Option A**: in-mission only, built from Complex Trigger + Counter + window W + dwell check. There is no external helper, because it cannot run alongside the server. | user, 2026-09-24 |
| U2 | The budget is **not** scaled by player population. It is a fixed setting per side. | user, 2026-09-24 |
| U3 | Window W and dwell time come from the time an **F-80 takes to fly through the zone at cruising speed**. | user, 2026-09-24 |
| U4 | Threshold N is decided later. It must be a setting with a sensible default (see D1). | user, 2026-09-24 |
| U5 | Slot timing is a **timer chain counted from Mission Begin**, not `MCU_DateTime`. | user, 2026-09-24 |
| U6 | If a clock slot's chosen entry fails the threshold, reroll among the other pool entries whose areas pass. If none pass, the slot stays quiet. | user, 2026-09-24 |
| U7 | "Hard stop while players still engaged: if N players are still within the inner zone at the stop, allow ONE extension; after that delete regardless." How the build approximates this is in §3.4 (D31, D32). | user, 2026-09-24 |
| U8 | If a zone fires while the budget is full, drop the trigger and re-arm after the cooldown. | user, 2026-09-24 |
| U9 | UI: a rail tab plus a Map dock tab. The user originally asked for "an air force tab like the Army tab". | user, 2026-09-24 |
| U10 | v1 uses air starts. Takeoffs from airfields are a later phase. | user, 2026-09-24 |
| U11 | Implementation happens in a separate Opus ultracode session. Nothing is pushed or merged without the user. | user, 2026-09-24 |
| U12 | Build **trace builds** (breadcrumbs in the server's text mission log) and the **P10 replay** core **before** the probe flights, and use the flights to test that they work too (step 0T, cell T-t). | user, 2026-09-25 |

### 2.2 Agreed architecture

| # | Decision | Source |
|---|---|---|
| A1 | Shared back end (§3.1–3.6) with two front ends (§3.7, §3.8). | Claude default, confirmed by user |
| A2 | Front end A (clock) is primary. Front end B (zone) is secondary. Pool entries are tagged `Clock`, `Zone` or `Both`. | Claude default, confirmed by user |
| A3 | The sortie shell guarantees DONE exactly once, on the first of: mission complete (end of on-station time, after at most one extension), all aircraft killed, Zone Out, or hard stop. DONE then deletes what is left and reopens the budget. | Claude default, confirmed by user |
| A4 | The budget gate follows the Fighter Pack NodeGates / Exclusive Activation pattern, generalised to N. This is safe because DONE is guaranteed. | Claude default, confirmed by user |
| A5 | Role is detected from template orders: an air AttackArea = patrol / air alert, a ground AttackArea = strike / CAS, Cover = escort. | Claude default, confirmed by user |
| A6 | Existing Generate output stays byte-identical when the feature is unused. | Claude default, confirmed by user (also CLAUDE.md) |

### 2.3 Claude defaults (settings, changeable)

| # | Decision | Default | Rationale |
|---|---|---|---|
| D1 | Threshold N | **1** (range 1–8) | Never leaves the sky empty for a lone player. When N = 1 the threshold uses a plain CheckZone path with no Complex Trigger (§3.5), so the default does not depend on the least-known MCU (risk d). N ≥ 2 stays locked until the user has built the 0a editor reference and probe flight 2 (T-d) confirms the Complex Trigger (§10 item 11). |
| D2 | Threshold radius R | **12 km**, one cell per (side, area), shared by both front ends | Inside the 10–16 km band for front end B. One cell per area halves the MCU count. |
| D3 | Inner (dwell) zone radius | R / 2 | Someone flying straight through has left the inner zone by the time the dwell ends. |
| D4 | W, dwell D, hold H | W = D = 2R / v_cruise rounded up to 10 s; H = 2W | U3. Formulas and numbers are in §4. |
| D5 | Budget per side | **2** (range 1–6) | About 8 AI aircraft per side at most. Keeps server load modest. |
| D6 | Clock: first slot, period, jitter, quiet chance | first slot 10 min; period **20 min**; 3 jitter steps of 3 min (0 / 3 / 6 min); quiet chance 20 % | A new F-80 flight was launched every 20 min [S9 p.101], reference line 26 area. The first slot at 10 min gives players time to reach the front. |
| D7 | Spawn distance | Clock: arrive in **M = 5 min** at the template's waypoint speed. Zone: **30 km** from the target. | Clock sorties must already be on the way. Zone sorties must be seen arriving (original proposal). |
| D8 | Hard stop | arrival time + on-station time + **10 min**; one extension of **10 min** | A backstop for a late or stuck flight. The normal end is the stop probe at the end of on-station time (D31). |
| D9 | "Mission complete" | One timer `SNjj ON STATION` per sortie, equal to T_attack, started when the first lead's AttackArea timer fires (on WP 1) | The shipped chain ends the sortie about 1 s after WP 1 (§3.1). The AttackArea completion report (type 2) is UNVERIFIED in game. |
| D10 | DONE latch | Self-deactivating relay, not a Counter | Counter semantics are UNVERIFIED (risk e). "Self Deactivate" is an existing pattern. Same-tick double pulses are risk q (probe J4). |
| D11 | Cancel pattern | Every cancellable delay is a timer followed by a 0 s `… OUT` relay. Cancelling means deactivating the OUT relay. The node that starts the next cycle re-activates the OUT relay first (§3.0). | Whether deactivating a *running* timer cancels it is UNVERIFIED. Deactivating a relay that has not yet been pulsed is the pattern `Close_Remaining_Output(s)` already relies on (`recon.rs` `build_randomizer`). If probe J1 shows a running timer is cancelled, the OUT relays can be dropped later; keep them in v1. |
| D12 | Re-running a sortie | **One stamped copy per run**, with a copy pointer per entry. Max runs per entry: **3**, for both front ends. | A linked flight is forced to Activate mode and cannot respawn (R3 F1, `template.rs` ≈2572-2577). The cap also limits the clock (§10 list). |
| D13 | RTB | Hand-insert one `RTB n` waypoint per lead at `mapfighters::rtb_ao_point` (`mapfighters.rs` ≈73). Set `DELAYED END ORDERS` to 60 s, as the generator does for RTB groups. | None of the files has RTB. Regenerating through `load_template` is lossy (R3 F2). |
| D14 | Approach heading | Arrow target: tail → tip. Objective target: from the side's rear point (`rtb_ao_point`) toward the target. Each entry can override it. | Sorties come from the friendly side. |
| D15 | Weights | 1–10, default 5 (equal). Entry order is shuffled with the plan seed (`AirPlan.seed`, §5). | The shuffle spreads the waterfall bias described in §3.7. |
| D16 | New entry mode | `Clock` | A2 |
| D17 | Clock slot and budget full | The slot is quiet, with no reroll. | The budget is per side, so rerolling cannot help. |
| D18 | Zone chance and cooldown | 50 %, 15 min | Mid values. Both are settings. |
| D19 | Built-in library | `include_str!` of the six files. User templates via "Add template…" (Ctrl O on the tab). | The release exe is self-contained. The six files contain no `LCName` / `LCDesc`, so they need no locale sidecar (checked with grep). |
| D20 | Date data | New module `src/in_theatre.rs`, keyed on `model_spec` ids and seeded from the reference with a citation per row. R3 F5 and R4 P11 reuse it later. | F5 requires one table, not two. |
| D21 | Output | Map's Generate Base Map includes the air pack when the plan has at least one enabled, valid entry and "Include in base map" is ticked (it ticks itself when the first entry is added). The Air Tasking tab's "Generate File" writes the air pack alone. | Keeps the `every_tab_has_a_primary_button_with_its_shortcut` rule (`ui_tests.rs` ≈293), and lets the user regenerate air tasking without a new base map. |
| D22 | Loading a base map that contains Air Tasking | Recognise it by group name and skip it, so it is not imported as an army. Show a warning. Do not rebuild the plan. | See §7. |
| D23 | Names in new `.Group` groups | Use DPRK / NATO. | Only legacy groups keep "Eastern". `coalition_is_eastern` accepts names starting "DPRK" (`frontlines.rs` ≈1878-1889). |
| D24 | How each front end uses the threshold | Clock reads the area's HOT state. Zone reacts to the area's PASS event. | One cell serves both front ends (§3.5). |
| D25 | Budget races | Accepted: (1) two requests in the same stagger slot within 0.05 s can overshoot the budget by one; (2) an UP and a DOWN of the same side in the **same MCU tick** leave two levels active, and nothing resyncs them, so the side stays over or under budget for the rest of the mission. | A full lock needs per-requester routing. Both races need two timer-driven events to land in one tick, and requests come from jittered slots or cooldown-gated zones. The walker has a case that shows outcome (2), and the manual states it (§10 list). |
| D26 | User templates with more than one path waypoint per lead | Rejected in v1 with a clear message. | `park_path_waypoints` fills waypoints in index order (`mapnet.rs` ≈892). One target per lead is well defined. |
| D27 | Tab labels and shortcuts | Rail: "Air Tasking", Ctrl 7. Dock: "Air". No plain-7 Map tool; the Map tool keys stay 1–6. | Adding a plain 7 would panic `MAP_TOOLS[i]` (§6.1). |
| D28 | AirAlert's two AttackAreas firing together | Left as the file has them. Flagged for the in-game check. | Changing a curated template is out of scope. |
| D29 | Timeline display length | 3 h (display only). The clock loops until the mission ends. | Typical server rotation. |
| D30 | AI setting off the other side's threshold | Orange warning when a sortie's spawn → target → RTB path passes within R of an area that triggers the *other* side. | Known weakness of Option A (risk a). |
| D31 | When the extension is tested | At the **end of on-station time** (the normal end), and at the hard stop if that comes first (a late flight). One `SNjj STOP PROBE` latch per sortie serves both, so there is at most one extension. | With the probe only at the hard stop, on-station always ends the sortie 10 min earlier and the extension could never fire. |
| D32 | Extension condition | The area is HOT (N entries seen, presence confirmed within H) **and** a fresh inner-zone check at the probe finds at least one enemy plane inside r_in now. | A CheckZone answers "at least one plane inside"; nothing in the mission can count N planes inside a zone. This is the closest in-mission test of U7 (§10 list). |
| D33 | Clock draw attempts | Fixed length A = L + 1.5 s. A failed area excludes every entry on that area at once. A win ends the slot at once. | Overlapping attempts could give two winners in one slot (§3.7). |
| D34 | MiG-15 date | `mig15bis` stands in for the MiG-15 family from 1950-11-01 [S9 p.241]. The row notes that the bis variant itself appeared in Nov 1951 [S9 p.434]. La-9 has no `model_spec` id and gets no row. | `model_spec` has no plain MiG-15. The reference already uses `mig15bis` as the stand-in (reference ≈902). |
| D35 | Dock label fallback | If five dock tabs do not fit 288 px, "References" becomes "Refs" when the count has 3 digits. | One fallback, chosen now, so the fit test has a fixed expectation (§6.2). |

---

## 3. Architecture

### 3.0 Conventions

**Names.** Every generated MCU name uses these prefixes. `s` is the side letter, `D` or `N`, and numbers are two digits, 1-based, per side. Tests assert on these exact strings.

| Prefix | Meaning | Example |
|---|---|---|
| `AT s` | Per-side housekeeping | `AT N INIT`, `AT N INIT OFF` |
| `BUD s` | Budget | `BUD N L1 ON`, `BUD N DEC 2` |
| `CLK s` | Clock | `CLK N TICK`, `CLK N ATTEMPT END` |
| `TH s aa` → `THNaa` | Threshold cell for area aa | `THN02 COUNT` |
| `ENii` / `EDii` | Pool entry ii | `EN05 OUT`, `EN05 COPY 2` |
| `ZNii` / `ZDii` | Zone trigger of entry ii | `ZN05 ARMED` |
| `SNjj` / `SDjj` | Placed sortie copy jj | `SN07 START`, `SN07 DONE HUB` |

- **Relay** = `MCU_Timer` with `Time = 0` and `Random = 100`, used as a gate: it passes a pulse only while it is active. This is the Fighter Pack gate-cell idiom (`pack.rs` ≈54, all timers Time 0).
- **Set ON / set OFF** = one `MCU_Activate` / `MCU_Deactivate` whose MCU links list every relay in a named set. Use the same link field as `Close_Remaining_Output(s)` / `CloseInput` in `recon.rs` / `flights.rs` `build_randomizer`. The implementer checks whether that field is Targets or Objects and uses the same one.
- **Latch** = a relay that also targets a Deactivate of itself: the first pulse passes, later ones are dropped.
- **Start states.** At Mission Begin + 0.5 s, `AT s INIT` → `AT s INIT OFF` (a Deactivate) switches off every relay that must start inactive. Everything else starts active. No reliance on an `Enabled` flag on timers.
- **Cancel pattern (D11).** `X` (timer, T s) → `X OUT` (relay) → consequences. Cancel = deactivate `X OUT`.
- **Re-arm rule.** Every `X OUT` that a success path deactivates is re-activated by the node that starts the next cycle, **before** `X` can fire again. The diagrams mark each re-arm edge with ⟲. A cell with a cancel but no ⟲ edge is used once per copy (the sortie shell) and says so. A test checks the rule (step 1b, `walker_every_deactivated_out_has_a_rearm_or_is_single_use`).
- **Emit order.** When one node pulses several relays of a set (for example `BUD s DEC` → every `DEC k`), its Targets list them in ascending k. The walker uses this order; the game may not (risk q).
- **Cross-module wiring (`SideLogic`, §5).** Builders never edit an MCU another builder owns by name. Each side has one `SideLogic` value that every builder appends to: `mission_begin` (ids to pulse from `Translator Mission Begin`), `init_off` (relays `AT s INIT OFF` switches off), `hot_consumers` (area → `ENii HOT` / `SNjj HOT` relays for `THsaa HOT ON` / `HOT OFF`) and `pass_subscribers` (area → `ZNii ARMED` relays that `THsaa PASS` targets). **Build order per side** (step 7, `build_air_packs`): `assemble_side` (place, wrap, budget, copy pointer; registers `SNjj HOT`) → clock (registers `ENii HOT`) → zone (registers `ZNii ARMED`) → threshold (reads `hot_consumers` and `pass_subscribers`, so its lists are complete) → `assemble::emit_side_logic`, which creates `Translator Mission Begin`, `AT s INIT` (0.5 s) and `AT s INIT OFF` from the collected lists. **Owner of `AT s *` and the side's `Translator Mission Begin`: step 3 (`assemble.rs`).**

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

The sortie shell MCUs `SNjj *` live inside the copy's own `Logic` group. Budget request relays `SNjj BUD INC k` live in the side's Logic group, in a subgroup `Budget`.

### 3.1 `place_air_sortie` (move + rotate a whole template)

What the templates are today (verified from the files and `template.rs`):
- The lead plane sits on `ORIGIN = (40000, 40000)` (`template.rs` ≈75-76). `WP 1` is 4 km north, at `(44000, alt, 40000)` (≈94, ≈2938-2958). So the lead → WP 1 bearing is 0°, where heading 0 = +X = north and 90 = +Z = east (`placement.rs` ≈224-232).
- **The ground AttackArea sits on the spawn point, not on WP 1** (`template.rs` ≈2815-2821). README step 3 is wrong about this.
- The air AttackArea sits at a canvas slot.
- Zone IN (16 km, Closer 1) and Zone Out (35 km, Closer 0) are centred on the spawn. Zone IN's Targets are its Self Deactivate, `Zone Out ReActivate`, `PULSE OUT` and the bring-up (HVAR: `[19, 20, 16, 25]`). Nothing else arms Zone Out.
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

Steps, in this order:
1. **Find the leads.** A lead is a `Plane` whose linked `MCU_TR_Entity` has **empty Targets**, meaning it does not target another plane's entity. A wingman's entity targets its lead's entity (`template.rs` ≈2564-2570 sets the follower's targets to `[lead_eid]`). Expected: HVAR leads = [7]; Il-10 = [7, 15]; Armed Recon = [7, 11]. The first lead is the one first in file order. Pivot = first lead XZ. Template bearing β = `heading_toward(first lead, its first non-RTB MCU_Waypoint)`; a waypoint belongs to a lead when its Objects contain that lead's entity.
   - Reject a template in which no lead has a waypoint, or in which a lead has more than one non-RTB waypoint (D26). A lead with no waypoint (an escort) is allowed.
2. **Rotate the whole tree** by `θ = p.heading_deg − β` about the pivot. Add a new `placement::rotate_tree(root, pivot, θ)`:
   - Rotate every node that has `XPos`/`ZPos`: planes, entities, every MCU, waypoints, commands, checkzones, icons. Use the formula from `apply_group_heading` (`placement.rs` ≈116-134): `nx = px + dx·cos − dz·sin`, `nz = pz + dx·sin + dz·cos`.
   - Add θ to `YOri` only on the nodes `apply_group_heading` already turns (Model nodes and linked entities, walk at ≈178-202). MCU `YOri` is left alone; whether it is ever read is UNVERIFIED (risk m).
   - Never touch `XOri`/`ZOri` or `YPos`.
   - Reuse `set_coord` / `add_yori` (`placement.rs` ≈204-222; made `pub(crate)` in the 1a skeleton), so decimals are written the same way.
3. **Translate.** `spawn = target − d·(cos h, sin h)`, then `placement::move_anchor_to(root, lead, spawn)` (`placement.rs` ≈101). This keeps YPos, so file altitudes survive (`ast.rs` ≈221-227).
4. **Snap the waypoint.** `mapnet::park_path_waypoints(root, &vec![target; n_path_wps])` (`mapnet.rs` ≈892). It sets XZ only, so WP altitudes (3050 / 1500 / 900 m) are kept. A shared WP 1 (Il-10, Yak) is fine.
5. **Snap AttackAreas.**
   - Ground: `weapon_range::snap_ground_attack_areas(root, x, z)` (`weapon_range.rs` ≈274). **Required**, because it currently sits on the spawn point.
   - Air: new `weapon_range::snap_air_attack_areas`. Same code, with the predicate `AttackAir == 1`.
   - Cover and Formation have no world meaning, so they only rotate and translate.
6. **Insert RTB (D13).** For each lead: a new `MCU_Waypoint` `RTB n` at `p.rtb`, `Area 1000`, `Priority 2`, `Speed` = WP speed, Objects = [lead entity]. `YPos` = that lead's WP 1 altitude; a lead with no waypoint (the Armed Recon escort) uses its own plane's `YPos`. Copy the property set of the generator's RTB waypoints (`template.rs` ≈2743-2787).
   - Add a timer `RTB DELAY` (0.5 s) → all `RTB n`.
   - Add `MISSION END ORDERS` → `RTB DELAY`.
   - Set `DELAYED END ORDERS.Time = 60`.
   - `mapnet::is_rtb_waypoint` (`mapnet.rs` ≈328) skips names starting "RTB", so later waypoint parking leaves these alone.
7. **Return** the spawn point, leads, planes, WP `Speed` (km/h, read from the file) and on-station time T_attack = the largest `MCU_CMD_AttackArea` `Time` whose Objects contain any lead.
   - If a user template has `Speed ≤ 150` (R3 F7's 100 km/h default), use `model_spec::suggested_waypoint_speed_kmh` (`model_spec.rs` ≈352-369) instead and push a warning.

Index merge: each placed copy goes through `duplicate::duplicate_template(copy, &mut next_id)` (`duplicate.rs` ≈101) before it is stamped.

### 3.2 Sortie shell (START in, DONE out exactly once)

New `src/airtask/shell.rs`: `wrap_sortie(root, prefix: &str, spec: &ShellSpec, next_id: &mut i32) -> ShellIds`. It is written generically so R3 F4 can later call it with the prefix `EXT`. `ShellSpec` carries T_attack, T_hard, T_ext, the target, r_in, the watched coalition, the `ZoneOutImpl`, `LossesImpl` and `PresenceCheckImpl` strategies, and whether the copy has an area HOT relay.

**Stripped from the copy.** `recon::silence_clone_starts` (`recon.rs` ≈1681, make it `pub(crate)`) sets `Enabled=0` and empties the Targets of `Translator Mission Begin`. IL-2 still runs Mission Begin when `Enabled=0` (comment at `recon.rs` ≈1680).

Then delete these MCUs and every link that points at them:
- `ENABLE / PULSE IN`
- `Zone IN`
- both `Self Deactivate`
- `Zone In ReActivate`
- `COOLDOWN`
- every `Mission Complete n` timer (the single `SNjj ON STATION` replaces them, D9)

**Kept and rewired:**
- `Zone Out` is re-centred on the target (radius unchanged, 35 km) and its Targets are replaced with `[SNjj DONE HUB]`.
- `Zone Out ReActivate` and `PULSE OUT` are kept as its arming pair. **New edge:** the first lead's `WP 1` → `Zone Out ReActivate`, `PULSE OUT`. (Zone IN used to fire them; it is deleted.)
- **ON STATION insertion, found by edge, never by name.** The first lead's "on-target" timer is the `MCU_Timer` whose Targets contained `Mission Complete 1` (the lowest-numbered Mission Complete that belongs to the first lead). That edge is replaced by → `SNjj ON STATION IN` (latch) → `SNjj ON STATION`. Other leads' order timers keep their AttackArea commands and simply lose their Mission Complete target. Only the first lead's timer feeds ON STATION, so a shared WP 1 cannot send two same-tick pulses into the latches (risk q). That timer itself can be pulsed twice: in AirAlert, `WP 1` targets both 37 and 39, and 37 (0.5 s) also targets 39, so 39 is pulsed at WP 1 and again 0.5 s later. The `ON STATION IN` latch passes only the first of those, so `ON STATION` starts exactly once per sortie under every re-trigger behaviour (risk p). The template's own edges are left as they are (D28).

```
 SNjj GRANTED (from budget §3.3)
   └─► SNjj START (relay)
         ├─► MISSION BEGIN (template bring-up timer, 0.5 s) ─► Activate Units, AFTER BRING UP ─► Formation n ─► Goto WP 1 ─► WP 1
         └─► SNjj HARD STOP (timer, T_hard)

 WP 1 (first lead's) ─► AttackArea n timers (unchanged) ─► AttackArea cmd (on target)
                    └─► Zone Out ReActivate, PULSE OUT ─► Zone Out                      (new edge)
 first lead's on-target timer ─► SNjj ON STATION IN (latch) ─► SNjj ON STATION (timer, T_attack)   (replaces → Mission Complete 1)

 Stop probe (D31, D32; used once per copy, so no re-arm):
   SNjj ON STATION ─► SNjj STOP PROBE
   SNjj HARD STOP  ─► SNjj STOP PROBE
   SNjj STOP PROBE (latch) ─┬─► SNjj HOT (area gate, §3.5) ─► SNjj STOP CHECK (relay)
                            └─► SNjj STOP WAIT (1 s) ─► SNjj STOP WAIT OUT ─────────────────────────────►┐ (area not HOT)
   SNjj STOP CHECK ─► Deactivate SNjj STOP WAIT OUT                                                        │
                   ├► SNjj STOP ZONE ON (Activate SNjj STOP ZONE) ─► (0.1 s) SNjj STOP PULSE ─► SNjj STOP ZONE  │
                   └► SNjj STOP CLOSE (2 s) ─► SNjj STOP CLOSE OUT ──────────────────────────────────────►│ (nobody inside now)
   SNjj STOP ZONE (CheckZone Closer 1, r_in, at target, watched coalition)                                  │
                   ─► Deactivate SNjj STOP CLOSE OUT, SNjj STOP ZONE OFF, SNjj EXTEND (timer T_ext) ────────►│ (one extension)
                                                                                                           │
 Other DONE causes:                                                                                        │
   each Plane entity OnEvent Type 4 ─► SNjj LOSSES (Counter = planes, Dropcount 0 = fire once, stay) ─────►│
   Zone Out (Closer 0, 35 km, at target; armed on WP 1 arrival) ──────────────────────────────────────────►│
                                                                                                           ▼
 SNjj DONE HUB (latch; active from mission start; used once)
   └─► MISSION END ─► MISSION END ORDERS (0.05 s) ─► Force Complete - High, RTB DELAY ─► RTB n
                   └► DELAYED END ORDERS (60 s) ─► Deactivate Units, DELETE DELAY (0.5 s) ─► Trigger Delete
                                                                                      └────► SNjj DONE (relay)
 SNjj DONE ─► BUD s DEC (§3.3), ZNii WAITING (if the entry has a zone trigger, §3.8)
```

Notes:
- The single extension falls out of the wiring. `SNjj STOP PROBE` is a latch fed by both ON STATION and HARD STOP, so only the first of them probes, and `EXTEND` goes straight to `DONE HUB`. A flight on time probes at the end of on-station; the hard stop (10 min later) then finds the latch closed. A flight late by more than the margin probes at the hard stop instead.
- If the area has no HOT relay for this copy (should not happen; every copy has a target area), `STOP PROBE` → `STOP WAIT OUT` → DONE.
- A copy is used for one run only (D12). `DONE HUB` never needs re-arming.
- Kill events that arrive after `Trigger Delete` (if Delete raises them, risk g) reach a closed hub and do nothing.
- **Dropcount convention.** The codebase uses `Dropcount = 1` for a counter that resets after firing and can fire again (`template.rs` `SpawnCount` `drop = i32::from(repeat)`, `DeathCount` 1 in repeat mode; `flights.rs` `DeathCount` 1), and `Dropcount = 0` for one that fires once and stays. `LOSSES` (used once per copy) and `THsaa COUNT` (§3.5, reset explicitly by `ZERO`) both want "fire once and stay", so both are `Dropcount 0`. `THsaa COUNT` also needs a **fired** `Dropcount 0` counter to fire again after `ZERO` (`ModifierSetVal`) resets it. Nothing in the codebase does that today (the `template.rs` reset only clears a partial count of a `Dropcount 1` `DeathCount`), so probe cell T-e E6 tests exactly that case (risk e). If E6 fails, `CounterResetImpl::Ring` (fresh counters) is the fallback.
- `SNjj LOSSES` needs `attach_event(entity, 4, losses_idx)` (`template.rs` ≈3865) on each plane's `MCU_TR_Entity`. The escort lead's planes count too. Fallback for risk g is `LossesImpl::Latch` (§8 table).
- `MISSION END` is no longer reachable from anywhere except `DONE HUB`. A test asserts this.

### 3.3 Air budget gate (at most N active per side)

This is a one-hot up/down counter built from relays, so it does not depend on Counter semantics. Level k (0..N) = number of active sorties.
- Level-k set = `{ SNjj BUD INC k, SNjj BUD INC k OUT for every sortie jj of side s } ∪ { BUD s DEC k }`.
- Level 0 has no `DEC 0`. Level N has no `INC N`: a request at level N gets no answer, so it times out and is **denied**.
- **Level switch.** Each UP or DOWN switches the old level OFF at once and the new level ON 0.1 s later (a SETTLE timer). The immediate OFF blocks competing requests. The delayed ON means the new level's `DEC` / `INC` relays cannot receive a pulse from the fan-out that caused the switch, so one DONE moves the level by exactly one even if the engine delivers the fan-out after an Activate in the same tick (risk q, probe J3). A request that fires during the 0.1 s gap finds no level and is denied; that is an undershoot, never an overshoot.

```
 AT s INIT OFF (Mission Begin + 0.5 s) switches OFF: level sets 1..N (INC k, INC k OUT, DEC k)       -> level 0 active

 SNjj REQ (relay)
   ├─► SNjj BUD INC k (timer, Time = 0.10 + 0.05·(jj mod 8) s)  for k = 0..N-1 ─► SNjj BUD INC k OUT (relay)
   └─► SNjj REQ TIMEOUT (2 s) ─► SNjj REQ TIMEOUT OUT ─► SNjj DENIED (relay) ─► front-end hook
 SNjj BUD INC k OUT ─► BUD s UP k (relay) ─► BUD s L{k} OFF (Deactivate level-k set)
                   │                     └► BUD s UP k SETTLE (timer 0.1 s) ─► BUD s L{k+1} ON (Activate level-k+1 set)
                   └► SNjj GRANTED (relay) ─► Deactivate SNjj REQ TIMEOUT OUT, SNjj START
 SNjj DONE ─► BUD s DEC (relay) ─► BUD s DEC k (relay, k = 1..N in ascending order; only the current level's is active) ─► BUD s DOWN k
 BUD s DOWN k ─► BUD s L{k} OFF
              └► BUD s DOWN k SETTLE (timer 0.1 s) ─► BUD s L{k-1} ON
```

- Only the active level's `INC k` starts when `REQ` pulses all of them.
- The stagger means that, of two near-simultaneous requests, the first to fire moves the level. Moving the level deactivates the other request's `INC k OUT` before it fires, so the second one times out and is denied (D25 (1)).
- `REQ TIMEOUT OUT` is used once per copy, so it has no re-arm.
- Front-end hooks:
  - Clock `DENIED` → nothing, the slot stays quiet (D17).
  - Zone `DENIED` → `ZNii WAITING` → `ZNii COOLDOWN` (U8, §3.8).
- Before this, the codebase only had 1-of-N gates (`bombers.rs` ≈243-368, `pack.rs` ≈54). This is the N-slot generalisation of that pattern.

### 3.4 Stop probe and extension (U7)

- `T_hard = t_arrive + T_attack + margin` with margin 10 min (D8). `T_ext = 10 min`.
- The probe runs at the end of on-station time, or at the hard stop if that comes first (D31). It extends once when both hold (D32):
  1. the target area is HOT (`SNjj HOT` relay in the area's HOT set, §3.5), meaning N entries were seen within a window and presence was confirmed within the last H seconds; and
  2. a fresh check of the inner zone r_in at the probe finds at least one enemy plane inside **now** (`SNjj STOP ZONE`, the same activate + pulse + 2 s close race as the threshold cell's INNER check).
- **Deviation from U7, stated for the user (§10).** U7 asks for "N players still within the inner zone". Condition 2 can only confirm one plane inside. Condition 1 adds "N arrived recently". For N = 1 (the default) the two together are exactly U7. For N ≥ 2 the build extends when N arrived within the window and at least one is still in the inner zone.
- The UI labels it "Extend once if players are still over the target when the sortie would end."

### 3.5 Player-threshold cell (per side × area)

- An **area** is a target point: an objective, or an attack-arrow tip.
- The cell for side s watches the **other** side's planes. For example, `THNaa` triggers NATO sorties and watches DPRK-coalition planes, `PlaneCoalitions [1]`, the same as the US templates' zones.
- Consumers read its state through per-consumer **HOT relays**:
  - `ENii HOT` for each clock entry on this area;
  - `SNjj HOT` for each sortie copy on this area (stop probe).

  The set `THsaa HOT` = all of these. Each consumer's builder creates its own HOT relay and registers it in `SideLogic.hot_consumers` (§3.0); the threshold builder runs after them and lists exactly the registered relays.

**Entry, path N ≥ 2 (Complex Trigger).** Blocked on risk d until the reference block exists.

```
 THsaa ENTRY (MCU_TR_ComplexTrigger: radius R, planes only, other side's countries, event "entered alive")
   └─► THsaa COUNT (MCU_Counter: Counter = N, Dropcount 0 = fire once, stay until ZERO) ─► THsaa ARM
 Mission Begin ─► THsaa WINDOW (timer W) ─► THsaa ZERO (MCU_ModifierSetVal, ParamIndex 0, Data0 0; target COUNT)
                                        └─► THsaa WINDOW                      (loop; the loop contains a timer, so P9-safe)
```

**Entry, path N = 1 (default, CheckZone only).**

```
 THsaa ENTRY ZONE (MCU_CheckZone Closer 1, radius R, other coalition), pulsed at Mission Begin
   ├─► THsaa ENTRY OFF (Deactivate THsaa ENTRY ZONE)          <- fires once per arming, like the templates' Zone IN
   └─► THsaa ARM
```

**Common dwell check, HOT state and re-arm:**

```
 THsaa ARM (latch) ─► THsaa DWELL (timer D) ─► THsaa CHECK       <- the latch stops repeat entries restarting DWELL
 THsaa CHECK (relay)
   ├─► Activate THsaa INNER CLOSE OUT                                                        ⟲ re-arm, first
   ├─► THsaa INNER ON (Activate THsaa INNER)
   ├─► THsaa INNER PULSE (0.1 s) ─► THsaa INNER (MCU_CheckZone Closer 1, radius R/2, other coalition)
   └─► THsaa INNER CLOSE (2 s) ─► THsaa INNER CLOSE OUT ─► THsaa INNER OFF (Deactivate THsaa INNER), THsaa CHECK FAIL OUT
 THsaa INNER ─► THsaa PASS (relay)
   ├─► Deactivate THsaa INNER CLOSE OUT
   ├─► THsaa INNER OFF                                         <- a later passer-by cannot reach PASS without a new CHECK
   ├─► THsaa HOT ON  (Activate set THsaa HOT)
   ├─► THsaa HOLD (timer H) ─► THsaa HOT OFF (Deactivate set THsaa HOT), THsaa CHECK   (presence re-check)
   └─► ZNii ARMED for every zone entry on this area  (§3.8; from SideLogic.pass_subscribers)
 THsaa CHECK FAIL OUT (relay) ─► THsaa REARM (timer 1 s)
   ├─► Activate THsaa ARM                                                                    ⟲ re-arm
   ├─► N = 1:  THsaa ENTRY ON (Activate THsaa ENTRY ZONE) ─► (0.1 s) THsaa ENTRY PULSE ─► THsaa ENTRY ZONE   ⟲ re-arm
   └─► N ≥ 2:  THsaa ZERO                                       <- a fresh count after a failed dwell
 AT s INIT OFF switches OFF: set THsaa HOT                     (the threshold builder appends it to SideLogic.init_off)
```

- Cycle: entry → ARM closes → DWELL → CHECK. A pass keeps HOT and re-checks every H; a fail (at the first CHECK or at a HOLD re-check) re-opens ARM and re-arms the entry.
- `INNER CLOSE` deactivates the inner zone on a fail, and PASS deactivates it on a pass. Without that, a pulsed zone keeps watching and a later passer-by would pass without a dwell. Whether deactivating a waiting checkzone stops it is risk h.
- ENTRY ZONE deactivates itself when it fires, as the templates' Zone IN does. Whether a pulsed Closer 1 zone fires repeatedly while a plane stays inside is risk o (probe C5); the ARM latch makes the cell correct either way.
- The HOT relays turn off for about 0.1–2 s at each `HOLD` re-check. That gap is harmless.
- Known weaknesses (accepted in the design):
  - The window is a fixed bucket, not a true sliding window.
  - A re-entry at the zone edge counts twice.
  - A player who leaves still counts until the next reset.
  - AI may count (risk a, D30).
  - With N ≥ 2, keeping an area HOT only needs one plane present after the first pass.

### 3.6 Weighted re-armable randomizer

New `airtask::weighted_random_pct(weights: &[u32]) -> Vec<u8>` in `airtask/mod.rs` (step 1): `p_i = round(100 · w_i / Σ_{j≥i} w_j)`, with the last entry at 100. The existing `recon::random_pct` (`recon.rs` ≈463) is equal-odds only.

The structure copies the re-armable `flights::build_randomizer` (`flights.rs` ≈918-1004):
- `Randomizer:INPUT` → `CloseInput`, `Wait for Output`, and every Random timer. Random k (k = 1..n, 1-based) has `Time = 0.5·k` s, the same times as `build_randomizer`'s `0.5·(i+1)` with 0-based i, so the last output comes at 0.5·n = L − 0.5 s.
- `Wait for Output` (fires at L = 0.5·(n+1) s) → `ReOpen Outputs` and re-opens INPUT, a clean 0.5 s after the last output, never in the same tick. A DRAW that arrives before L is dropped, so attempts must be at least L long (§3.7).

Differences: names carry the `CLK s` prefix, and exclusion acts on `ENii RANDOM` (not on the Outs), so `ReOpen Outputs` does not undo it.

### 3.7 Front end A: mission clock (per side)

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
   ├─► CLK s ATTEMPT ARM (Activate CLK s ATTEMPT END OUT and every ENxx MISS OUT)                ⟲ re-arm, first
   ├─► CLK s Randomizer:INPUT ─► ENii RANDOM p% (0.5·k s, k = waterfall position 1..n) ─► ENii OUT ─► (closer: later ENxx OUT)
   └─► CLK s ATTEMPT END (timer A = L + 1.5 s) ─► CLK s ATTEMPT END OUT ─► CLK s REROLL
 ENii OUT ─► ENii HOT (area gate) ─► ENii GO
          └► ENii MISS (1 s) ─► ENii MISS OUT ─► CLK s AREA aa OUT (Deactivate ENxx RANDOM for every entry on area aa)
 CLK s REROLL (relay) ─► CLK s DRAW                                      (only while the slot window is open)
 ENii GO ─► Deactivate ENii MISS OUT, CLK s ATTEMPT END OUT, CLK s REROLL     (the slot ends at once)
         ├► ENii LAST SET: Deactivate every ENxx LAST except ENii; Activate ENii LAST
         └► every ENii COPY r ─► SNjj REQ     (copy pointer, built by assemble.rs, below)
 ENii EXHAUSTED (from the copy pointer) ─► Deactivate ENii AVAILABLE, ENii RANDOM              (clock adds these targets)

 Copy pointer per pool entry (built by assemble_side, step 3; shared by both front ends):
 ENii COPY r (one-hot relays, r = 1..runs; only r = 1 starts ON) ─► SNjj REQ     (copy r of this entry)
 SNjj GRANTED (copy r of entry ii) ─► Deactivate ENii COPY r, Activate ENii COPY r+1
                                   └► (last copy) ENii EXHAUSTED (relay)        (clock and zone each add their own targets)
```

- **Attempts are fixed length and do not overlap (D33).** Every attempt ends at `ATTEMPT END` (A = L + 1.5 s after DRAW): the last `OUT` comes by L − 0.5 (RANDOM k at 0.5·k s), its `MISS OUT` by L + 0.5, and the randomizer INPUT re-opens at L. Only `ATTEMPT END` starts the next DRAW, so there is never more than one attempt in flight. A win cancels `ATTEMPT END OUT` and switches `REROLL` off, so the slot sends exactly one REQ.
- **A failed area is excluded once.** `MISS OUT` deactivates the RANDOM relay of every entry on that area (`CLK s AREA aa OUT`), because they share one HOT state. So at most (distinct areas) + 1 attempts end with a winner or an exclusion. **Nobody-won attempts are not bounded:** once the last waterfall entry is excluded (as LAST, or by an area miss), an attempt can end with no OUT at all, and each such attempt is a random event. Example: 3 equal-weight entries on 3 areas, #3 excluded as LAST, only #1's area HOT: #1 wins about 1/3 of attempts, so a window of 6 attempts leaves about 9 % of such slots quiet although #1 passes.
- **Slot window** = `min(300 s, (a + 10)·A)`, where a = number of distinct areas among the side's clock-eligible entries, L = `0.5·(n+1)` s for n clock-eligible entries, A = L + 1.5 s. After it closes, `REROLL` is off and the slot is quiet (U6). The +10 is sized for the example above: 13 attempts leave about 0.5 % of those slots quiet ((2/3)^13). With many entries the 300 s cap can bind, and the quiet rate with a passing entry rises. This deviation from U6 is §10 item 12.
- `validate` gives an **error** when `period ≤ (J − 1)·jitter_step + slot window + 10 s` (one slot's SLOT CLOSE would cut into the next slot), and a **warning** when `(a + 10)·A > 300 s` (some areas may go untried, and nobody-won attempts may use up the window). Both checks are `clock::validate_schedule` (step 5), which `validate` calls (step 7).
- **Distribution caveat.** When entry k is excluded, the waterfall passes k's share to the entries after k. The shuffled order (D15) spreads that bias. If the last entry is excluded, "nobody won" is possible; that attempt excludes nothing and `ATTEMPT END` rerolls, so a slot can close quiet while a passing entry exists (see "Slot window"). Document this in the manual.
- **No repeat.** The last winner is excluded for one slot only. `NO REPEAT` (0.1 s) excludes it, then `LAST CLEAR` (0.2 s) switches every `LAST` off, before `DRAW` (0.3 s) can produce a new winner whose `LAST SET` turns its own `LAST` back on. A quiet slot therefore sets no `LAST`, and the entry is eligible again in the slot after. If its area is the only HOT one, the slots alternate win / quiet (§10 list; walker test `clock_only_one_area_hot_alternates_with_quiet`).
- **Exhaustion.** The last copy's GRANTED (from either front end) pulses `ENii EXHAUSTED`, which switches off `ENii RANDOM` as well as `AVAILABLE`, so a used-up entry cannot win a later slot and waste it.
- **Copy pointer ownership.** `ENii COPY r`, the GRANTED → advance edges and `ENii EXHAUSTED` are built by `assemble_side` (step 3) for every pool entry that has copies, `Zone`-only entries included, and their ids are returned in `SideCopies`. The clock (step 5) adds `GO → every COPY r` and `EXHAUSTED → Deactivate AVAILABLE, RANDOM`; the zone (step 6) adds `FIRE → every COPY r` and `EXHAUSTED → Deactivate ZNii REARM`. Neither front end creates or names the other's MCUs.
- **Starting states** (`SideLogic.init_off`): `ENii LAST`, `CLK s REROLL` (clock); `ENii COPY r` for r ≥ 2 (assemble).
- `QUIET ROLL`, `JITTER` and `RANDOM` rely on the Timer `Random` % being rolled each time the timer is triggered (risk f). The builder takes `RandomImpl::{InMission, Seeded}`:
  - `InMission` (primary) is the diagram above.
  - `Seeded` (fallback if f fails) makes every choice at Generate time from the plan seed (`AirPlan.seed`). The clock is unrolled into S = ceil(6 h / period) slots. Each slot gets a fixed jitter step, a fixed quiet flag, and an ordered entry list (a weighted sample without replacement over the clock-eligible entries). In the mission, a slot tries its list in order through the same `ENii HOT` gates: the first HOT, available, not-LAST entry wins, and "reroll" means the next entry in the list (U6). After slot S the clock stops. The zone chance becomes a fixed per-zone sequence of 8 fire/skip outcomes, stepped by a one-hot pointer at each arming and cycled.

### 3.8 Front end B: check zone (per entry tagged Zone or Both)

```
 THsaa PASS ─► ZNii ARMED (relay; ON = armed)
 ZNii ARMED ─► Activate ZNii MISS WAIT OUT                                                    ⟲ re-arm, first
            ├► ZNii DISARM (Deactivate ZNii ARMED)
            ├► ZNii CHANCE (0.1 s, Random = chance%) ─► ZNii FIRE (relay)
            └► ZNii MISS WAIT (1 s) ─► ZNii MISS WAIT OUT ─► ZNii COOLDOWN
 ZNii FIRE ─► Deactivate ZNii MISS WAIT OUT
           ├► ZNii WAITING ON (Activate ZNii WAITING)
           └► every ENii COPY r (the copy pointer built by assemble_side, §3.7) ─► SNjj REQ
 ENii EXHAUSTED (last copy granted, §3.7) ─► Deactivate ZNii REARM                            (zone adds this target)
 SNjj DENIED, SNjj DONE (every copy of entry ii) ─► ZNii WAITING (relay, starts OFF)
                                                     ├─► ZNii WAITING OFF (Deactivate ZNii WAITING)
                                                     └─► ZNii COOLDOWN
 ZNii COOLDOWN (timer C) ─► ZNii REARM (relay; OFF after the last copy is granted) ─► ZNii ARM (Activate ZNii ARMED)
```

- The threshold cell's `PASS` keeps firing every H seconds while players stay. So after the cooldown, a still-busy area offers the trigger again.
- `ZNii WAITING` lets only a DENIED or DONE that follows a zone FIRE start the cooldown. A clock copy's DONE no longer restarts the cooldown of an idle zone.
- **Residual edge case, accepted and documented (§10 list):** for a `Both` entry, if the zone has fired and a clock-launched copy of the same entry is denied or ends first, the zone starts its cooldown early and can re-arm while its own copy is still flying.
- Zone sorties use the zone spawn distance (30 km, D7). Copies are shared by both front ends, so each copy is placed once. Rule: copies of a `Zone` or `Both` entry are placed with the **zone** distance, because that is the tighter case. Copies of a `Clock` entry use the clock distance.
- "Max repeats" = the entry's run count (D12).
- Starting states (`SideLogic.init_off`, switched off by `AT s INIT OFF`): `ZNii WAITING`.
- The zone builder registers each `ZNii ARMED` in `SideLogic.pass_subscribers` for its area; the threshold builder (run after it) makes `THsaa PASS` target them.

---

## 4. Timing formulas

### 4.1 Speed source

- `v_cruise` = F-80C-10 cruise **732 km/h = 203.3 m/s**, from `model_spec::spec_for("f80c10").cruise_kmh` (`model_spec.rs` ≈156, `spec_for` ≈302).
- **UNVERIFIED / unsourced.** Every `notes` field in that table is empty. The reference gives F-80 altitudes only ("cruise above 4,570 m", "CAP orbit 3,050 m" [S9 p.81, p.53], reference §2.2) and no speeds (§9 Gaps).
- Implement one helper, `airtask::f80_transit_s(radius_m) -> f64` in `timing.rs`, that reads the table at run time, and `airtask::derived_timing(&ThresholdSpec) -> DerivedTiming` on top of it (W, D, H, r_in with the Manual timing overrides applied; the §4.4 UI and every builder read this one function). Both are re-exported from `airtask/mod.rs` (§5). A later correction to the cruise value then updates W and D (risk n).
- Probe cell **T-n** (§8 step 0) measures the value: a NATO player flies an F-80C at the in-game cruise setting over a timed 24 km leg. The measured speed, not an AI flying a commanded speed, decides whether 732 km/h stays.

### 4.2 Threshold timing (U3, D4)

```
transit(R) = 2R / v_cruise           (straight through the centre)
W = D = ceil_to_10s(transit(R))      window and dwell
H = 2W                               HOT hold before the presence re-check
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
T_hard = t_arrive + T_attack + 600 s            T_ext = 600 s
```

| Template | v_wp (file) | clock d (M = 5) | zone t_arrive (30 km) |
|---|---|---|---|
| F-80 (both) | 660 km/h | 55.0 km | 164 s |
| F-51 (both) | 520 km/h | 43.3 km | 208 s |
| Yak-9P | 500 km/h | 41.7 km | 216 s |
| Il-10 | 390 km/h | 32.5 km | 277 s |

Examples:
- F-80 HVAR, clock: T_attack 600 s → T_hard = 300 + 600 + 600 = 1500 s (25 min).
- F-51 Armed Recon, clock: T_attack 1200 s → 2100 s (35 min).

Take `T_attack` from each file (§3.1 step 7). Do not hard-code it.

Clock slot times: first slot 600 s, then every 1200 s, plus a jitter of 0 / 180 / 360 s.

### 4.4 In the UI

- The Threshold section shows R (editable) and the derived W, D, H and r_in as read-only values, with the label "From F-80C cruise 732 km/h (model_spec)".
- A "Manual timing" checkbox makes W, D and H editable. They are stored as `Option<f64>` overrides, and `None` means derived.
- The Schedule section shows M, the zone spawn distance, T_ext and the hard-stop margin.
- Each pool row shows its computed T_hard in a tooltip.

---

## 5. Data model

New module tree `src/airtask/` (the step 1 skeleton adds `mod airtask;` and `mod in_theatre;` to `main.rs`). The UI only calls into it (CLAUDE.md rule).

| File | Contents | Owner step |
|---|---|---|
| `airtask/mod.rs` | Types below (exact source), `weighted_random_pct`, the `pub use` lines the UI needs, stubs of `build_air_packs`, `preview`, `validate` (bodies in step 7) | 1a (types frozen), 7 (bodies) |
| `airtask/library.rs` | Built-in six (`include_str!("../../TemplateExamples/Historical1950/…")`), `builtin_library()`, `load_user_sortie(path)`, role and side detection. The 1a skeleton writes the two `pub fn` signatures with stub bodies (`vec![]`, `Err(..)`) so `mod.rs` can re-export them | 1a (signatures), 2 (bodies) |
| `airtask/place.rs` | `place_air_sortie` (§3.1) | 2 |
| `airtask/shell.rs` | `wrap_sortie` (§3.2); under `#[cfg(test)]`, the ignored Probe 2 writer `write_p14_sortie_probe` (step 3) | 3 |
| `airtask/budget.rs` | §3.3 | 3 |
| `airtask/assemble.rs` | `assemble_side`: per side, stamps one copy per run of each eligible entry (jj numbering, spawn distance rule of §3.8), calls place + wrap + budget, builds each entry's copy pointer (`ENii COPY r`, GRANTED advance, `ENii EXHAUSTED`, §3.7), registers `SNjj HOT` in `SideLogic`, and returns `SideCopies` (copy ids per entry, REQ/GRANTED/DENIED/DONE/HOT ids, `COPY r` ids and the `EXHAUSTED` id per entry). Also `emit_side_logic` (§3.0): the side's `Translator Mission Begin`, `AT s INIT`, `AT s INIT OFF` | 3 |
| `airtask/threshold.rs` | §3.5 | 4 |
| `airtask/clock.rs` | §3.7 | 5 |
| `airtask/zone.rs` | §3.8 | 6 |
| `airtask/timing.rs` | §4 helpers: `f80_transit_s(radius_m)`, `derived_timing(&ThresholdSpec) -> DerivedTiming`. The 1a skeleton writes both `pub fn` signatures with stub bodies (`0.0`, `DerivedTiming::default()`) so `mod.rs` can re-export them | 1a (signatures), 1 (bodies) |
| `airtask/testkit.rs` (`#[cfg(test)]`) | `assert_links_resolve` (skeleton) and the dry-run walker (step 1b) | 1 skeleton, 1b |
| `airtask/probe.rs` (`#[cfg(test)]`) | Step 0 probe mission (§8) | 0b |
| `src/in_theatre.rs` | D20 table | 1 |
| `src/frontlines.rs` | `pub struct MapAirPack { pub root: Il2Entity }` next to `MapFighterPack` / `MapShipPack` / `MapGroundPack` (≈317-331). Added in the step 1 skeleton so the frozen API compiles; the stamping and `FrontOptions` field come in step 7. | 1 skeleton, 7 |

**Exact skeleton source of `airtask/mod.rs`** (the step 1a commit writes this; later steps only add bodies, and a type change goes through the coordinator, §10). Every type derives `Debug`, `Clone` and `PartialEq`, because the Undo fingerprints use `format!("{:?}", …)` (`map_fingerprint`, `ui.rs` ≈5535) and tests compare plans for equality. Field-less enums are also `Copy` and `Eq`. No type derives `Hash` (the plan holds `f64`s); the fingerprint is the `{:?}` string.

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
    pub runs: u8, pub heading_override: Option<f64>, pub enabled: bool,
}
impl PoolEntry {                                             // no Default: an entry always has a target
    pub fn new(sortie_id: String, target: TargetRef) -> Self {
        Self { sortie_id, target, weight: 5, mode: TriggerMode::Clock, runs: 3, heading_override: None, enabled: true }
    }
}                                                            // weight 1..=10 (D15), runs 1..=6 (D12)

#[derive(Debug, Clone, PartialEq)]
pub struct AirBudget { pub per_side: [u8; 2] }               // [DPRK, NATO], each 1..=6
impl Default for AirBudget { fn default() -> Self { Self { per_side: [2, 2] } } }      // D5

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdSpec {
    pub players_n: u8, pub radius_m: f64, pub inner_ratio: f64,
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

// Strategy enums (§8 step 0 table). The Default is the primary design, except ThresholdImpl (ZoneOnly until d is confirmed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum ThresholdImpl    { #[default] ZoneOnly, ComplexTrigger }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum ZoneOutImpl      { #[default] CheckZone, Watchdog }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum LossesImpl       { #[default] Type4, Latch }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum CounterResetImpl { #[default] SetVal, Ring }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum RandomImpl       { #[default] InMission, Seeded }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum BudgetImpl       { #[default] OneHot, Queue }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum PresenceCheckImpl { #[default] PulseInside, Closer0Race }   // risk c

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Strategies {
    pub threshold: ThresholdImpl, pub zone_out: ZoneOutImpl, pub losses: LossesImpl,
    pub counter_reset: CounterResetImpl, pub random: RandomImpl, pub budget: BudgetImpl,
    pub presence: PresenceCheckImpl,
}

/// Cross-module wiring for one side (§3.0). Every builder appends; nothing edits another builder's MCU by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SideLogic {
    pub(crate) mission_begin: Vec<i32>,                        // pulsed by the side's Translator Mission Begin
    pub(crate) init_off: Vec<i32>,                             // switched off by AT s INIT OFF (Mission Begin + 0.5 s)
    pub(crate) hot_consumers: BTreeMap<usize, Vec<i32>>,       // area index → ENii HOT / SNjj HOT relays (THsaa HOT ON / OFF)
    pub(crate) pass_subscribers: BTreeMap<usize, Vec<i32>>,    // area index → ZNii ARMED relays (THsaa PASS targets)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AirPlan {
    pub library: Vec<AirSortie>,                             // Default is empty; the UI fills it with the built-ins (step 2)
    pub pool: Vec<PoolEntry>, pub budget: AirBudget, pub threshold: ThresholdSpec,
    pub schedule: [Schedule; 2],                             // [DPRK, NATO]
    pub zone: ZoneSpec, pub stop: StopSpec, pub include_in_base_map: bool,
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
```

Only fields may be added to `AirPreview` and `AirIssue` in step 7, through the coordinator. `AirDrawCounts` lives here so that `ui.rs` (`self.air_drawn`) and the `air_dock_draws_preview` test compare against `preview(..).counts()`.

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
- `set_coord` / `add_yori` (`placement.rs` ≈204-222).

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
  - BUDGET: DPRK / NATO max active.
  - THRESHOLD: N, R, derived W/D/H/r_in, Manual timing. Under `ThresholdImpl::ZoneOnly`, N is clamped to 1 with the explanation "N above 1 needs the Complex Trigger, not yet confirmed in game".
  - SCHEDULE DPRK / SCHEDULE NATO: enabled, first slot, period, jitter steps × step, quiet %, arrive in M min, and the side's **slot timeline strip** (the same draw helper as the dock tab, §6.2).
  - ZONE TRIGGERS: chance %, cooldown, spawn distance.
  - HARD STOP: margin, extension.
  - OUTPUT: "Include in base map".

**Primary:** "Generate File" writes the air pack alone. Readiness is `airtask::validate` as `Check`s:
- a front is loaded;
- at least one enabled entry that is valid and not orphaned;
- budget ≥ 1 on each side that has entries;
- every template placed without error;
- each enabled side's period is longer than its jitter span plus slot window (§3.7).

The D30 AI-trigger check and the slot-window cap warning are orange warnings, not blockers.

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
- `interactive_widgets_are_at_least_28_px_tall` (≈1235) and `every_ui_glyph_is_in_a_loaded_font` (≈1661): run automatically over the new tab.

**New (step 8a, wave 2: need only the frozen types and stubs):**
- `ctrl_7_opens_air_tasking`.
- `plain_7_on_map_does_not_panic`.
- `air_tasking_undo_restores_weight_change` (a pool entry built with `PoolEntry::new`; edit a weight, Ctrl Z, pool equal to before).
- `air_tasking_threshold_n_clamped_under_zone_only`.
- `air_plan_first_entry_ticks_include_in_base_map`.

**New (step 8b, wave 4: need step 2's library and step 7's `validate` / `build_air_packs` bodies):**
- `air_tasking_tab_shows_builtin_library` (six names present).
- `air_tasking_builtin_remove_disabled`.
- `air_tasking_generate_disabled_until_validate_passes` (empty pool, then an orphaned-only pool, then budget 0 on a side with entries: the button is disabled each time and the matching Check text is shown).
- `air_tasking_generate_file_writes_air_pack` (uses `dialog::answer`; parses the file and asserts a single top-level `Air Tasking` group, both side Logic groups, `assert_links_resolve`, and indexes starting at 1; `assert_group_file` alone only checks that it parses).

**New (step 9):**
- `air_dock_auto_fill_adds_entries_for_objectives`, `air_dock_auto_fill_includes_arrows`, `air_dock_auto_fill_twice_is_idempotent`.
- `air_dock_draws_preview` (asserts `air_drawn` equals the `airtask::preview` counts).
- `map_undo_restores_air_pool_after_auto_fill`, `map_undo_restores_air_entry_after_remove`.
- `map_readiness_includes_air_check_when_included`.
- `map_readiness_orphaned_only_air_pool_warns_but_allows_generate` (Include ticked, every entry orphaned: the orange note shows, Map's Generate is enabled, and the output equals the baseline).
- `map_generate_with_air_includes_air_tasking_group`.
- `map_generate_with_include_unticked_matches_baseline` (same inputs as the step 1 fixture, plus a pool entry with "Include in base map" unticked).
- `load_base_map_with_air_tasking_is_not_an_army`, `load_base_map_keeps_air_library_and_settings` (drive `load_base_map` in `ui.rs`; need step 7's `inspect_base_map` recognizer).

**Already there from step 1 (wave 1b), not new here:** `write_p14_baseline_ui` (ignored) and `map_generate_without_air_matches_baseline` (§7). The step 8 and step 9 agents keep them green.

---

## 7. Export integration

- `FrontOptions` (`frontlines.rs` ≈367-392) gets `pub air_packs: Vec<MapAirPack>`, and its `Default` gets `Vec::new()`. The test constructions use `..FrontOptions::default()`. Only the literal in `generate_front_file` (`ui.rs` ≈11289) lists every field with no `..Default`, so it needs the new line. **Step 7 adds that one line (`air_packs: Vec::new(),`) in the same commit as the field, through the coordinator** (§10), because step 8 owns `ui.rs` at the time. It is the only `ui.rs` line step 7 touches.
- **Stamping.** In `generate_front` (`frontlines.rs` ≈1116), the packs are stamped near ≈1450-1473. Stamp the air packs **after the ground packs and before `aoi_border_group`**, with the same `stamp_rooted_packs(iter, &mut next_id)` (≈2200-2210). When the list is empty the loop does nothing and `next_id` does not change, so the output is byte-identical.
- **`generate_front_file`** (`ui.rs` ≈11257, step 9). After the ground packs: `let air_packs = if self.air_plan.include_in_base_map { airtask::build_air_packs(&self.air_plan, &ctx)? } else { vec![] };`. The built-in templates need no sidecar (D19). Add user-template paths to the `paths` passed to `merge_template_sidecars`.
- **Standalone file** (Air Tasking tab): `airtask::build_air_packs`, then a root holding only the `Air Tasking` group with indexes from 1, then `serialize_group`, the save dialog and `write_sidecars`. Other tabs' standalone files already rely on the editor re-indexing when groups are imported into a mission. Whether it does that is UNVERIFIED (risk l, R4 P13 "first check"); step 11 checks it.
- **Load.** `inspect_base_map` (`frontlines.rs` ≈1543-1638) must recognise a group whose name starts with `Air Tasking` **before** `group_has_units` (≈1912). Otherwise the sorties come back as an `ImportedArmy`.
  - Add `ImportedBaseMap.air_tasking_found: bool`.
  - The UI shows the orange warning: "Loaded map had Air Tasking; Generate will replace it with the current plan (removed if the plan is empty)."
- **Determinism.** Given the same plan (which includes `AirPlan.seed`) and context, `build_air_packs` returns identical trees. Every random choice made at Generate time (the entry shuffle, and all `RandomImpl::Seeded` choices) uses `AirPlan.seed` (§5 "Seed"), never `placement_seed()` directly.
- **Byte-identical guarantee.** Before any P14 code, step 1 records baseline fixtures from unmodified `main`:
  - Directory `src/testdata/p14_baseline/`, committed. Files: `template.Group`, `recon.Group`, `fighter.Group`, `exclusive.Group`, `airfield_mp.Group`, `base_map.Group`, `base_map_ui.Group`, plus each file's locale sidecars where the generator writes them.
  - Test module `src/baseline_tests.rs` (`#[cfg(test)] mod baseline_tests;` in `main.rs`). Each fixture's inputs are written as code in that module: fixed seat lists, a fixed seed, the templates under `TemplateExamples/` it loads, and for `base_map.Group` a `FrontOptions` with fighters, ships, ground packs, armies and attack arrows. The first six call the same `generate_*` function each tab's primary button calls.
  - `base_map_ui.Group` is produced through the UI harness via `generate_front_file`, so it covers the `ui.rs` path. `Harness` and `dialog::answer` are private to `src/ui_tests.rs`, so this fixture's writer and test live there, and **step 1 owns that small `ui_tests.rs` addition in wave 1b**, before the step 8 agent takes the file:
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
- The test count must not drop below the recorded baseline. Re-record it at step 1 (a grep of `#[test]` gave 467 at `bf1a356`).
- **Warnings.** This is a binary crate with no `lib.rs` (`main.rs` header), so new code is `dead_code` until `ui.rs` calls it. The step 1 skeleton therefore puts a temporary `#![allow(dead_code)] // P14: removed in step 9` at the top of `airtask/mod.rs` (it covers the submodules) and `in_theatre.rs`, and an item-level `#[allow(dead_code)] // P14` on each new helper outside those modules (`rotate_tree`, `snap_air_attack_areas`, `timer_random`, `MapAirPack`, and the `FrontOptions.air_packs` field if needed). `trace.rs` gets a module-level `#![allow(dead_code)] // P14: removed in step 9T` (its only non-test caller is the 9T UI); `missionlog.rs` is reachable through the CLI and needs an allow only on items the CLI does not use, removed in 9T. Test-only modules (`testkit`, `probe`, `baseline_tests`) are `#[cfg(test)]` and need none. With those in place, warnings must not rise above the recorded baseline at any step. Step 9 removes every `// P14` allow once the code is reachable from `main` (9T removes the `trace.rs` / `missionlog.rs` ones); step 10's acceptance greps that none remain.
- The baseline-fixture test (§7) must stay green.
- **Every acceptance line is a named test.** The names are given below. Walker scenarios live next to the builder they test.
- Stop after each step and list the changes (CLAUDE.md phase rhythm).
- Never `git add -A`. `.gitattributes * -text` stays.

### Step 0. In-game MCU probe — S (code) + two user flights. Branch `claude/p14-probe`

Step 0 has two probe files. **Flight 1** (`P14_MCU_Probe.Group`) holds every cell except T-d and needs nothing from the user before it is built. **Flight 2** (`P14_MCU_Probe_CTrigger.Group`) holds only T-d, the Complex Trigger cell, and is built from the user's 0a reference file. Recommendation: fly flight 1 as soon as it exists, and fly flight 2 as a short second session (about 30 min) after 0a, so the rest of step 0 is not blocked on the editor task.

**0a. Complex Trigger reference (user, in the mission editor).** This blocks threshold path N ≥ 2 and flight 2 only. `TemplateExamples/Reference/` does not exist yet.
- In a blank Korea mission, place one `MCU_TR_ComplexTrigger` with radius 10000.
- Tick every filter the editor offers (countries, object types, names, any player-only flag).
- Link each event slot to its own dummy timer, named after the event.
- Save it as a group, `TemplateExamples/Reference/ComplexTrigger_reference.Group`.
- Claude then adds a `parse_group_file` → `serialize_group` round-trip test for it (`src/parser.rs` ≈226, `src/serialize.rs` ≈48): `complex_trigger_reference_round_trips`, `#[ignore = "needs 0a reference"]` until the file exists.

**0b. Probe generators.** `airtask/probe.rs` (`#[cfg(test)]`) with `generate_mcu_probe() -> Il2Entity` (flight 1) and `generate_ctrigger_probe(reference: &Il2Entity) -> Il2Entity` (flight 2), plus their locale strings (`src/locale.rs` ≈89).
- **Depends on:** the step 1a skeleton only. The builders it needs (`mcu`, `timer`, `timer_random`, `counter`, `checkzone`, `modifier_set_val`, `attach_event`, `silence_clone_starts`) are made `pub(crate)` in that commit (§5 "Builders").
- Ignored writers: `cargo test --offline write_p14_probe -- --ignored` writes `target/p14/P14_MCU_Probe.Group` + `.eng`. `write_p14_probe_ctrigger` writes `target/p14/P14_MCU_Probe_CTrigger.Group` + `.eng`; until the 0a file exists it fails with the message "needs TemplateExamples/Reference/ComplexTrigger_reference.Group (step 0a)".
- It touches no existing Generate path.
- Every observation is an `MCU_TR_Subtitle` with `Coalitions [0,1,2]` and a unique `LCText`, using the block shape from `TemplateExamples/BomberMissions/B29Mission.Group` ≈758-785. A subtitle MCU may fire several times; the video shows how often.
- **Readouts.** IL-2 counters show nothing in game, so every count is read through threshold counters, each driving its own subtitle. Readout counters are `Dropcount 0` (fire once). **No pass/fail criterion counts repeats of one subtitle:** "exactly once" is read as "`X` shows and `X ≥2` never shows", with a `Counter(2)` readout on that output (E1, E2, E6, J4, J5, each T-g plane's type 4, C5). They depend on counters working (risk e), so every counted event also drives a per-event subtitle that the user can count on the video if T-e shows odd counter behaviour.
- **Subtitle duration.** Per-event and readout subtitles use `Duration = 1` (s), below the shortest pulse spacing in the probe (2 s, T-e), so two firings of the same subtitle show as two separate appearances. Cue subtitles use `Duration = 20`, as in the B29 block, so a player can read them. A same-tick double firing still shows as one appearance; only the `≥2` readout can see it.
- **AI flights** come from the built-in templates through a test-only helper `probe_flight(src, keep_planes, start, exit, next_id)`:
  1. `parse_group_file(include_str!("../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group"))` (the Yak 2+2 file for T-d);
  2. keep the first `keep_planes` planes in file order (the lead first); delete the others with their `MCU_TR_Entity`s, and drop their indexes from every Targets and Objects list;
  3. `duplicate::duplicate_template(copy, &mut next_id)`;
  4. `placement::move_anchor_to(root, lead_xz, start)`, then `mapnet::park_path_waypoints(root, &[exit])` and `weapon_range::snap_ground_attack_areas(root, exit.0, exit.1)`, so the flight does not turn back to attack its spawn point (§3.1). WP 1 `Speed` stays as in the file. Every probe flight flies north (start → exit on a bearing of 0°, the templates' own bearing), so no rotation is needed; `rotate_tree` is not available to 0b;
  5. `recon::silence_clone_starts`, then delete the §3.2 strip list (`ENABLE / PULSE IN`, `Zone IN`, both `Self Deactivate`, `Zone In ReActivate`, `COOLDOWN`, every `Mission Complete n`) and every link to them;
  6. a probe timer at the cell's start time pulses the template's `MISSION BEGIN` bring-up timer. A cell's close deletes the flight with its own `MCU_Delete` on the flight's plane entities.
- **Cue subtitles and timing.** Every cell is armed, cued and closed by timers started directly by the probe's `Translator Mission Begin`, at fixed mission times. A step that depends on a player being in place starts at a fixed cue time: the cue subtitle tells the player to be there, and the cell's timers run from that same time. No zone is used to detect the player, so the run sheet does not rely on risks a or c. At its close time, every checkzone of the cell is deactivated, so a player crossing a finished cell later changes nothing.
- All positions and times are constants in `probe.rs` (`K14_ORIGIN`, `RUN_SHEET`, `CUES`), and the tests pin them to the tables below.

**Run sheet, flight 1** (`P14_MCU_Probe.Group`). Origin = the `K-14_Kimpo_AF` airfield in `TemplateExamples/K14 AFB_mp.Group` (X 105934, Z 269477; the file's only airfield). Offsets are km north (+X), km east (+Z). Players: **N1**, **N2** = NATO from K14 (N2 in an F-80C); **D1** = DPRK from a DPRK spawn the user adds at the probe's `P14 DPRK SPAWN` icon, K14 + (170, 0). Times are mm:ss from Mission Begin. Everyone should be airborne by 04:00.

| Cell | Centre (N, E km) | Armed | Closed | Player | Cues (subtitle at the time shown) |
|---|---|---|---|---|---|
| T-a1 | (0, +60); AI start (−30, +60), exit (+30, +60) | 00:05 | 06:00 (AI deleted) | none. Players stay 20 km or more away until 06:00 (K14 is 60 km away) | — |
| T-e | automated, at (0, +5) | 01:00 | 02:30 | none | 01:00 "T-e start" |
| T-f | automated, at (0, +5) | 02:30 | 09:00 | none | 02:30 "T-f F1 start"; 05:30 "T-f F2 start" |
| T-t | automated, at (0, +5); breadcrumbs on the trace grid (step 0T) | 04:30 | 05:30 | none; all players watch for objective messages or map markers | 04:30 "T-t start: note any objective message or map marker in the next 60 s" |
| T-j | automated, at (0, +5) | 09:00 | 10:00 | none | 09:00 "T-j start" |
| T-n (+ ZA4) | START (+30, +30), END (+54, +30); ZA4 at (+42, +30). ZA4 belongs to this cell for arming and closing; only its result is read under T-a1 | 10:00 | 22:00 | N2 | 10:00 "T-n: N2 fly north through START then END, level at 3050 m, F-80C cruise setting" |
| T-c | C0–C3, C5 at (+40, −20); C4 at (+40, −8) | 10:00 | 14:00 | N1 | 10:00 "T-c: N1 orbit inside C (within 5 km) until told"; 12:00 "T-c: N1 fly east into C4 now" (after C4's 11:30 Activate + pulse and 11:45 Deactivate); 14:00 "T-c done" |
| T-g | G2, G3: (+130, +50); G1: (+130, +80), with `MCU_Icon` "T-g G1" on the DPRK map | 12:00 | 30:00 | D1 | 12:00 "T-g: D1 shoot down both G1 planes at the T-g G1 icon; leave the other flights alone" |
| T-b | (+100, 0) | 14:00 | 46:00 | N1, N2; D1 for the variant | 14:00 "T-b: N1 N2 hold 15 km south of B"; 22:00 "T-b: N1 N2 enter B now"; 24:00 "T-b: N1 leave B, 15 km south"; 27:00 "T-b: N2 leave B, 15 km south"; 30:00 "T-b: D1 fly to B, hold 15 km north"; 36:00 "T-b variant: N2 enter B and stay, N1 stay out, D1 shoot N2 down inside B" |

**Run sheet, flight 2** (`P14_MCU_Probe_CTrigger.Group`, after 0a). Same origin and spawns; players N1 and D1.

| Cell | Centre (N, E km) | Armed | Closed | Player | Cues |
|---|---|---|---|---|---|
| T-d | (+120, 0); DPRK AI pair start (+90, 0), exit (+150, 0) | 01:00 | 30:00 | D1, N1 | 00:00 "T-d: D1 hold near spawn until 09:00"; 10:00 "T-d: D1 pass through T-d, then circle inside"; 14:00 (Deactivate, Activate at 14:01, with D1 inside); 20:00 "T-d: N1 shoot D1 down inside T-d" |

**0c. User flights.** Before flight 1: turn on the server's text mission log (step 0T "Server setting") and note the time the mission starts. Flight 1 uses the **traced** file `P14_MCU_Probe_traced.Group` (step 0T); flight 2 uses its plain file. Import each probe into a blank Korea mission with `K14 AFB_mp.Group` (the NATO spawn) and a DPRK spawn at the `P14 DPRK SPAWN` icon. Run it on the dedicated server and record it (video or Tacview). End the mission normally, not by killing the server, so the last log file is written; then copy that session's `missionReport*.txt` files into `target/p14/logs/flight1/` (or `flight2/`). Flight 1: 2 NATO players and 1 DPRK player, about 46 min. Flight 2 (after 0a): 1 NATO and 1 DPRK player, about 30 min.

**0d. Results.** Run `replay` on the flight's logs first (step 0T "0d addition"); it gives exact times for everything that logged. Write `handoff/P14-probe-results.md`, one row per cell, observed (video, cross-checked with the replay report) vs the "Confirmed if" column. Then pick the primary design or the fallback for each risk (§9). This is a mechanical comparison. Risk d stays open (and `ThresholdImpl::ZoneOnly` stays the default) until flight 2.

**Cells:**

| Cell | Tests | Setup and procedure | Observe | Confirmed if |
|---|---|---|---|---|
| T-a1 | AI vs player (a) | ZA1: Closer 1, `[2]`, 12 km, pulsed at 00:05. ZA2 is the same with `[1]` (control). At 00:10 the 4-plane HVAR flight (`probe_flight`, all 4 planes) starts 30 km south of the centre and flies north through it. ZA4 (a copy of ZA1, on the T-n leg) belongs to cell T-n: armed and pulsed at 10:00, deactivated at T-n's 22:00 close, and N2 flies through it. Only its result is read here, so T-a1's 06:00 close does not touch it. Each zone's output → its own subtitle. | Does ZA1 fire for the AI? Does ZA4 fire for the player? | a confirmed (only players count) if ZA1 stays silent until 06:00 and ZA4 fires for N2. ZA2 stays silent either way. |
| T-n | F-80C cruise speed (n) | START and END: Closer 1, `[2]`, radius 2 km, 24 km apart on a north line, pulsed at 10:00, each self-deactivating; `MCU_Icon` markers "T-n START" / "T-n END" on the NATO map. N2 flies level at 3050 m (the templates' F-80 altitude) at the F-80C cruise setting from the game's aircraft specifications. | Time between the "T-n START" and "T-n END" subtitles on the video; speed = 24 km / Δt. Both zones fire 2 km before their point, so the offset cancels. | n confirmed if the measured speed is within ±10 % of 732 km/h. Otherwise risk n's fallback (§9) sets the cruise to the measured value. |
| T-b | Closer 0 with several players (b) | ZB-in (Closer 1, `[2]`, 10 km) → 1 s → activate and pulse ZB-out (Closer 0). Timed by the cues: N1 and N2 enter at 22:00; N1 leaves at 24:00; N2 leaves at 27:00. Variant at 36:00: ZB-in and ZB-out re-armed, N2 enters and stays, N1 stays out, D1 shoots N2 down inside. | ANY-out or ALL-out? | Confirmed (ALL-out) if ZB-out fires only after N2 leaves (27:00, not 24:00), and fires when N2 dies inside in the variant. |
| T-c | Activation while a player is inside (c, h, o) | C0 (sentinel: Closer 1, 5 km, active, pulsed at 11:00; shows N1 really is inside) and C1–C4 (Closer 1, 5 km), deactivated at start. N1 orbits inside from the 10:00 cue. At 11:00: C1 Activate only; C2 Activate + pulse 0.1 s; C3 pulse while deactivated. **C4 (risk h):** at 11:30, while N1 still orbits C (C4's edge is 2–12 km from N1), C4 Activate, then pulse 0.1 s later, so it is active and waiting; at 11:45 C4 Deactivate; at 12:00 the cue "fly east into C4" sends N1 into it, after the Deactivate. **C5:** Closer 1, pulsed once at 10:30. Its output → "C5 fired" (per event), and into Counter(2) → "C5 fired ≥2" and Counter(10) → "C5 fired ≥10". | Which fire, and after how long; how often C5 fires | c confirmed if C2 fires within 1 s. h confirmed (deactivating a waiting zone cancels it) if C4 never fires, including after N1 enters it at 12:00. o = "fires once" if "C5 fired" shows once and "C5 fired ≥2" never shows; "repeats" otherwise. |
| T-d | Complex Trigger (d). **Flight 2 only.** | Built from the 0a property set: R 10 km, planes, DPRK countries. Each event slot → its own subtitle "T-d <event>" (per event) and its own Counter(2) → "T-d <event> ≥2". DPRK AI pair (Yak file, 2 planes) passes through from 01:00; D1 passes, then circles, from the 10:00 cue; Deactivate at 14:00 + Activate at 14:01 with D1 inside; N1 shoots D1 down inside from the 20:00 cue. | Event per object? AI vs player? Re-emit on re-activate? Death = "left"? Record every filter option. | Confirmed if "entered" fires once per plane that enters, and a country filter excludes the other side. |
| T-e | Counter (e) | Automated loop of 2 s pulses from 01:00. E1 Counter 3 / Dropcount 0, 6 pulses; E2 Counter 3 / Dropcount 1, 6 pulses; E3 2 pulses → Deactivate → Activate → 1 pulse; E4 2 pulses → `ModifierSetVal` → 1 pulse; E5 2 pulses while deactivated → Activate → 1 pulse; **E6** Counter 3 / Dropcount 0, 3 pulses (fires) → `ModifierSetVal` → 3 pulses. Each counter's output → its own subtitle ("E1 fired", …), and E1, E2 and E6 outputs also → Counter(2) readouts "E1 fired ≥2", "E2 fired ≥2", "E6 fired ≥2". The pulse times are constants, so the video time gives the pulse number. | When each fires | Confirmed if E1 fires on the 3rd pulse and "E1 fired ≥2" never shows; E2 fires on the 3rd pulse and "E2 fired ≥2" shows at the 6th (Dropcount 1 resets, the codebase convention); E4 does **not** fire on its 3rd pulse (the reset worked); and "E6 fired ≥2" shows at E6's 6th pulse (SetVal re-enables a fired Dropcount 0 counter, which `THsaa COUNT` needs, §3.2 note). |
| T-f | Timer Random % (f) | F1: Random 50, pulsed 40 times at 3 s from 02:30. Each hit → "F1 hit" (per event), and into Counter(12) → "F1 reached 12" and Counter(29) → "F1 reached 29". F2: 4-output waterfall re-run 40 times at 5 s from 05:30 (ends 08:45, before T-j at 09:00); each output k → "F2 out k". | Hits | Confirmed if "F1 reached 12" shows and "F1 reached 29" does not (12–28 hits), and each "F2 out k" shows at least once. |
| T-g | Kill events (g) | Three 2-ship AI F-80C flights (`probe_flight`, 2 planes) start at 12:00, each 10 km south of its own centre and attacking ground there: G2 and G3 at (+130, +50), G1 at (+130, +80), 30 km east, marked by an `MCU_Icon` "T-g G1" on the DPRK map so D1 can tell it apart. Each plane has OnEvents 0, 2, 4, 5, 13 → its own subtitle per type ("G1a ev4", …), and its type 4 also → a Counter(2) readout ("G1a ev4 ≥2", …). G1 is shot down by D1. G2: probe `MCU_Delete` at 22:00. G3: the template's `Deactivate Units` at 22:00, then its `Trigger Delete` 0.5 s later (the `template.rs` ≈2704-2720 chain). G1 is deleted at the 30:00 close if still alive. | Which types fire, how many per plane, whether Delete fires them | Confirmed if, for each G1 plane shot down, "…ev4" shows and "…ev4 ≥2" never shows, and no G2 or G3 "ev4" subtitle shows. |
| T-j | Timer gating (j, p, q) | Automated from 09:00. J1: Deactivate a timer while it is running (Time 10 s) at 5 s. J2: pulse a deactivated relay, then Activate 1 s later (does the pulse replay?). J3: Activate + pulse in the same tick. **J4:** two pulses in the same tick (one timer with two targets into one relay) into a latch; its output → "J4 out" (per event) and a Counter(2) readout "J4 out ≥2". **J5:** a 10 s timer re-triggered at 5 s; output → "J5 out" (per event) and a Counter(2) readout "J5 out ≥2". | Each outcome | j confirmed if J2 does not replay. J1 "cancels" or "runs on" (either is handled). q confirmed if "J4 out" shows and "J4 out ≥2" never shows. p = "twice" if "J5 out ≥2" shows; otherwise "restart" (the one "J5 out" comes 15 s after the first trigger) or "ignore" (10 s after). J3 is recorded; §3.3 is safe either way. |

T-k (budget) moved to Probe 2 in step 3, so it is built from the real `budget.rs`.

**Step 0 acceptance (tests):**
- `probe_parses_and_links_resolve`: the flight 1 file parses back with `parse_group_file`, and `assert_links_resolve` passes.
- `probe_subtitles_are_unique`: every `MCU_TR_Subtitle` in the flight 1 file has a unique `LCText`.
- `probe_cells_match_the_table`: for each flight 1 cell (every cell except T-d, and including T-t: T1–T5 breadcrumbs at the T-t times, T4's six variants 4 s apart, T5's two firings), the key MCUs exist with the table's properties. For example: T-a1's flight has 4 planes, and ZA1 / ZA2 are Closer 1 with radius 12000 and coalitions `[2]` / `[1]`; ZA4 (checked under T-n, the cell that arms and closes it) is Closer 1, radius 12000, `[2]`; T-n's START and END are 24 km ± 1 m apart with radius 2000; C4 is activated at 11:30, pulsed at 11:30.1 and deactivated at 11:45, and the "fly east into C4" cue is at 12:00; C5 has one pulse and readout counters 2 and 10, each driving its own subtitle; T-e E1 `Counter = 3`, `Dropcount 0`, E2 `Dropcount 1`, 6 pulses each; E6 `Counter = 3`, `Dropcount 0`, 3 pulses, a `ModifierSetVal`, 3 pulses; T-f F1 `Random = 50`, a 40-pulse 3 s loop, readout counters 12 and 29 (`Dropcount 0`) and a per-hit subtitle; T-g has 6 planes with OnEvents 0, 2, 4, 5, 13, G1's WP 1 and AttackArea at its own centre 30 km ± 1 m east of G2's / G3's, and a "T-g G1" `MCU_Icon` at G1's centre; T-j J1 timer `Time = 10` with its Deactivate at 5 s. Every `Counter(2)` readout named in 0b "Readouts" exists with `Dropcount 0` and drives its own subtitle; every per-event and readout subtitle has `Duration = 1`, and every cue subtitle `Duration = 20`. It skips T-d.
- `probe_run_sheet_matches_the_table`: every cell centre is at `K14_ORIGIN` + its offset (± 1 m); every cue subtitle is fired by a timer from Mission Begin at the run-sheet time; every checkzone of a cell is deactivated at the cell's close time, using the same cell mapping as `probe_cells_match_the_table` (ZA4 belongs to T-n, so it is deactivated at 22:00, not at T-a1's 06:00).
- `probe_ctrigger_cells_match_the_table` and `probe_ctrigger_parses_and_links_resolve`, both `#[ignore = "needs 0a reference"]` until the 0a file exists: the Complex Trigger carries the 0a property set with radius 10000 and DPRK countries; each event slot drives its own subtitle and its own Counter(2) readout; the T-d times are as in the flight 2 run sheet.
- The ignored test `write_p14_probe` writes flight 1, and the user has the file. `write_p14_probe_ctrigger` writes flight 2 once 0a exists.

**Needs the user in game:** 0a (editor), 0L (an existing log, optional), the server's text-log setting, 0c (two flights, flight 1 with the traced file) and 0d.

**Order.** Flight 1 needs step 0T **and** 0b merged: the traced probe is written only then. `probe_run_sheet_matches_the_table` also pins T-t's arm (04:30) and close (05:00) times and its cue.

**Which later steps depend on which result:**

| Result | Used by | Primary if confirmed | Fallback if not |
|---|---|---|---|
| a | Step 4, D30 | Rely on the validator warning only | Also make the dwell mandatory, and switch to a player filter if T-d shows one |
| b | Step 3 (Zone Out cause) | `ZoneOutImpl::CheckZone`, as §3.2 | `ZoneOutImpl::Watchdog`: a timer loop re-pulses a Closer 1 zone, and each hit restarts a watchdog timer. Watchdog expiry = nobody inside → DONE HUB |
| c | Steps 3 (stop check), 4 (CHECK and HOLD re-check) | `PresenceCheckImpl::PulseInside`: pulse-while-inside fires at once | `PresenceCheckImpl::Closer0Race`: negative logic, pulse the inner zone as Closer 0 plus a 2 s race timer. If Closer 0 has not fired, someone is inside. Needs b = ALL |
| d (flight 2, after 0a) | Step 4, N ≥ 2 path | `ThresholdImpl::ComplexTrigger` | `ThresholdImpl::ZoneOnly`: N is limited to 1 in the UI with an explanation. The Complex Trigger path is not emitted |
| e | Step 4 (ZERO) | `CounterResetImpl::SetVal` | `CounterResetImpl::Ring`: K counters; a timer every W/K deactivates the oldest and activates a fresh one. If a deactivated counter still takes input, gate its input with a relay |
| f | Steps 5, 6 | `RandomImpl::InMission` | `RandomImpl::Seeded` (§3.7) |
| g | Step 3 (LOSSES) | `LossesImpl::Type4` | `LossesImpl::Latch`: per-plane latch relay fed by types 4, 2 and 0 → LOSSES. The hard stop is the backstop |
| h / j | Steps 3–6 (all relays) | D11 as written | If J2 replays, add an explicit clear. If J1 cancels, the OUT relays may be removed later (not in v1) |
| o | Step 4 | ENTRY ZONE self-deactivates; the ARM latch covers both outcomes | None needed |
| p | Steps 3–6 | Generated timers are never re-triggered while running by design (latches, fixed-length attempts). The templates' own order timers can be (AirAlert's timer 39 is pulsed by WP 1 and by 37), so `SNjj ON STATION IN` latches the one edge from them into generated logic (§3.2) | If J5 = "twice", add a latch in front of any timer the walker shows can be re-triggered |
| q | Steps 3, 5 | Latches and SETTLE timers as written | If J4 outputs twice: `DONE HUB` → 0.05 s → `DONE GATE` (latch), and the same for `STOP PROBE` and `THsaa ARM` |
| t1–t6 (T-t) | Step 0T defaults, step 9T, step 11 | `TraceCarrier::Objective` with the silent, logged `ObjectiveStyle` from T4; the tick rate fitted from the cue breadcrumbs | `Spawn` carrier if objectives log once only or not at all, or show on screen; debug subtitles only if neither carrier logs (the replay still reports spawns, kills and never-spawned groups) |
| k | Step 3 | `BudgetImpl::OneHot` (§3.3) | `BudgetImpl::Queue`: serialise requests through a single 1 s relay queue per side |

Every builder whose fallback is a strategy enum (`ThresholdImpl`, `ZoneOutImpl`, `LossesImpl`, `CounterResetImpl`, `RandomImpl`, `BudgetImpl`, `PresenceCheckImpl`) takes it from `AirPlan.strategies`. Steps 3–6 are built **before** the results arrive and do not wait for them; the switch is a one-line default change. **Each variant of these seven enums, primary and fallback, gets its own structural test and at least one walker scenario** (names in the steps below). The h/j (explicit clear), p (latch in front of a re-triggered timer) and q (`DONE GATE` latches) fallbacks are **not** strategy variants: they are small local code changes made after the probe results, only if a result calls for them. The walker already runs every scenario under `ReverseTargets` and all three re-trigger modes, so the cases they guard are exercised before then.

### Step 0T. Trace builds and replay core (P10) — M. Branch `claude/p14-trace`

**Why (U12).** Subtitles show what fired only to whoever is watching, with no file and no "never fired". The server's text mission log records spawns, kills, takeoffs and landings anyway. A Mission Objective MCU writes an `AType:8` line when it fires, so a small objective hung on any MCU turns that MCU's firing into a log line (a **breadcrumb**). The app knows the whole MCU graph, so it can report what fired, what never fired, and where each chain stopped. It is built **before** probe flight 1 so the flight produces a real log, and cell T-t (below) tests whether it works.

Everything in this step is **UNVERIFIED until flight 1** (risks t1–t6, §9): the log line format, what `AType:8` carries, and whether an objective shows anything to players. Build against the community-documented format, keep the parser tolerant, and treat the flight 1 log as the real fixture.

**0L. A real log first (user, optional but preferred).** If the dedicated server already writes text logs, the user copies one set of `missionReport*.txt` files from a past session into `src/testdata/missionlog/real_1/` (trimmed by the implementing agent to under 200 kB). If none exist, build against the documented format with a synthetic fixture, and add the flight 1 log as `real_1` in 0d.

**Server setting (user, before flight 1).** Turn text logging on for the dedicated server. The community-documented setting is `mission_text_log = 1` in the server's `startup.cfg` (`[KEY = system]`); the files land in `data\logs\text\`. The exact key and folder are **UNVERIFIED**: the implementing agent confirms them from current IL-2 documentation and writes them into the step's stop report and the manual.

**Files:** new `src/trace.rs` (instrumentation + sidecar) and `src/missionlog.rs` (parser, replay, report, CLI). `main.rs` gets `mod trace; mod missionlog;` and one CLI line next to the existing `heightprobe::run_cli` hook (`main.rs` ≈44-47): `if let Some(code) = missionlog::run_cli(&args) { std::process::exit(code); }`. Both are general tools, not P14-only, so they sit at the top level of `src/`, not under `airtask/`. The `mod` lines, the CLI line and stubs with the signatures below are part of the **1a skeleton** (§10), so 0T, 0b and step 1 build in parallel.

**0T-1. Trace instrumentation** (`trace.rs`):
- `pub struct TraceSelect { pub block_types: Vec<String>, pub name_prefixes: Vec<String>, pub indexes: Vec<i64> }`. An MCU is traced if any rule matches. `TraceSelect::decision_points()` = `MCU_CheckZone`, `MCU_Counter`, every `MCU_Timer` with `Random < 100`, `MCU_TR_MissionBegin`, `MCU_Spawner`, plus every name prefix the caller adds (P14 passes its START / DONE / GRANTED / DENIED / SLOT OPEN / FIRE / EXTEND … names).
- `pub fn instrument(root: &mut Il2Entity, sel: &TraceSelect, next_id: &mut i64, carrier: TraceCarrier, map: &mut TraceMap)`. For each traced source it adds exactly one breadcrumb, registered in `map` (numbering continues from the entries already there). An MCU source gets the breadcrumb's index appended to its `Targets`, so the breadcrumb fires whenever the MCU outputs. An **entity source** (a plane's or vehicle's `MCU_TR_Entity` whose events are wired through `OnEvents` / `OnReports`, as in T-g) gets a new `OnEvent` with the same `Type` and the breadcrumb as `TarId`, once per event type the selection names. It changes nothing else.
- `pub fn breadcrumb(carrier: TraceCarrier, map: &mut TraceMap, next_id: &mut i64, source: TraceSource) -> Vec<Il2Entity>` builds one breadcrumb (the blocks to add: one objective, or Spawner + object + Delete) and registers it. `instrument` uses it, and so do hand-built probes such as T-t. `TraceMap::push(entry)` registers a breadcrumb built elsewhere. Both signatures are in the 1a stubs.
- `pub enum TraceCarrier { Objective(ObjectiveStyle), Spawn }`:
  - `Objective(style)` (primary): an `MCU_TR_MissionObjective` with `TaskType 0`, `IconType 0`, and the `Coalition` / `Success` / `LCName` choice given by `ObjectiveStyle` (default = the variant cell T-t shows is silent to players and logged; until then `Coalition 0, Success 1, no LCName`).
  - `Spawn` (fallback, risk t1 or t3): an `MCU_Spawner` that spawns a small vehicle named `TRACE <n>`, then an `MCU_Delete` of it after 1 s. Use a `Vehicle` with a linked `MCU_TR_Entity`, copied from a sample such as the WillysMB `"NOICON"` in `TemplateExamples/K14 AFB_mp.Group` ≈1519 (Index 117, entity ≈1564), set to a neutral country and not engageable. Nothing in IL-2 is truly hidden, so every spawn-carrier object appears at one fixed dry-land point `TRACE_SPAWN_POINT` far from the front (checked with the water map), and the log line is matched by **name**, not position. The spawn writes the log's spawn line; a deleted object may write nothing. It costs one short-lived object per firing, so use it only if objectives fail.
- **Breadcrumb IDs.** Objective breadcrumb n sits on a **trace grid**: (`TRACE_X0` + 10·(n mod 3000) m, `TRACE_Z0` + 10·(n div 3000) m), with `TRACE_X0` = `TRACE_Z0` = 5 000. That is inside the whole-map bounds of `geo.rs` ≈23-24 (0 to 499 200) and outside the placement area of `placement.rs` ≈30-32 (40 000 to 470 000), so it never meets the front, placed units or the parking grid that starts at `placement::MAP_MIN` (≈43). Rows are 30 km long. Pin the constants and both bound checks in a test. The log line is matched by the objective's index if `AType:8`'s `OBJID` turns out to be the MCU index, otherwise by position (± 1 m). Which one works is risk t2; the sidecar holds both.
- `pub struct TraceMap { pub entries: Vec<TraceEntry>, pub edges: Vec<(i64, i64)> }`, `TraceEntry { breadcrumb_index, source_index, source_name, source_type, group_path, pos: (f64, f64), carrier, spawn_name: Option<String>, expected_s: Option<f64> }`, serialised as `<mission>.trace.json` next to the `.Group` with `write_trace_sidecar(path, &TraceMap)`. `expected_s` is the mission time at which the source is known to fire, filled for timers started directly by Mission Begin (the probe's cue timers: their `Time`); the replay fits the tick rate from these. `edges`: for each traced source, the traced sources reachable downstream through untraced ones. The walk follows `Targets`, entity `OnEvents` / `OnReports` `TarId`s and waypoint `Targets`, and keeps a visited set, because timer loops exist (T-e, T-f, the clock). The replay uses it for "first dead link".
- **Byte-identical.** Nothing calls `instrument` unless trace is requested. With trace off, every Generate path is unchanged (the step 1 baseline test covers it). With trace on, the only changes are added MCUs with fresh indexes and appended `Targets` entries.

**0T-2. Log parser and replay** (`missionlog.rs`):
- `pub fn parse_log_dir(dir) -> Result<MissionLog, String>` and `parse_log_files(&[PathBuf])`. A session's files are `missionReport(<date>)[n].txt`; sort by the `[n]` suffix and concatenate. One line = one record: `T:<ticks> AType:<n> key:value …`. Parse into `Record { t_ticks, atype, fields: Vec<(String, String)> }` with typed views for the types the replay uses: 0 (mission start), 3 (kill), 5 (takeoff), 6 (landing), 8 (mission objective), 10 (player plane), 12 (object spawn: id, type, name, position), 16 (removed), 20 / 21 (player join / leave). **Unknown types and unknown fields are kept and skipped, never fatal**; the report counts them.
- **Time.** `T` is documented as game ticks (50 per second). **UNVERIFIED** (risk t4): in the traced probe every cue timer carries a breadcrumb with `expected_s` set, so the replay fits a line `T = offset + rate·s` from those (log `T:0` is not necessarily Mission Begin), reports both values, and uses the fit when it has one.
- `pub fn replay(log: &MissionLog, group: &Il2Entity, trace: Option<&TraceMap>) -> Replay`:
  - **Timeline:** breadcrumb firings (by source MCU name) merged with spawns, kills, takeoffs, landings and removals, in time order.
  - **Fired / never fired:** per traced MCU, its firing times and count.
  - **First dead link:** a traced MCU that never fired while an upstream traced MCU (from `edges`) did fire. These are listed first; everything downstream of a dead link is grouped under it rather than listed again.
  - **Never spawned:** entities in the `.Group` (planes, vehicles, ships, trains) whose `Name` never appears in a spawn record. Log lines carry no group path and names repeat across copies, so it reports counts **per name** ("`F-80C`: 6 spawned, 8 in the mission") and attributes a name to a group path only when that name is unique in the `.Group`. Whether the spawn line's name field is the mission's `Name` for AI planes is UNVERIFIED (risk t6); flight 1 settles it.
- `pub fn report_markdown(&Replay) -> String`: header (log files, first and last `T`, fitted tick rate, unknown-record count), then First dead links, Never fired, Never spawned, Timeline.
- **CLI:** `IL2MissionUtility.exe replay --group <file.Group> [--trace <file.trace.json>] --logs <dir | files…> [--out <report.md>]`. It prints the report (or writes it with `--out`) and returns exit code 0, or 2 on bad arguments or unreadable files. Follow `heightprobe::run_cli`'s style (it returns `None` when the first argument is not its own).

**Probe hook-up (with 0b).** The ignored writer `write_p14_probe` writes both `P14_MCU_Probe.Group` (plain) and `P14_MCU_Probe_traced.Group` + `.trace.json`. The traced file is `instrument(probe, &TraceSelect { indexes: <the source of every per-event, readout and cue subtitle, except T-t's>, block_types: vec![], name_prefixes: vec![] }, next_id, Objective(default), &mut map)`, where `map` already holds cell T-t's own breadcrumbs (registered by `probe.rs` when it builds T-t). T-g's per-plane sources are entities, so they get `OnEvent` breadcrumbs. **Flight 1 flies the traced file.** Every subtitle the video shows should then have a matching log line.

**Cell T-t (flight 1, automated): does tracing work?** Centre (0, +5), armed 04:30, closed 05:30, in the gap between T-f's F1 (last pulse 04:27) and F2 (moved to start at 05:30 for this cell). Cue at 04:30: "T-t start: note any objective message or map marker in the next 60 s". **Owner:** the 0b agent builds T-t in `probe.rs` (and its one `LCName` string in `locale.rs`), using `trace::breadcrumb` from the 1a stubs. Each T-t breadcrumb is registered in the probe's `TraceMap` with `TraceMap::push`, so the replay can name it; T-t's timers are **excluded** from the `instrument` selection, so no T-t source gets a second breadcrumb. Each breadcrumb's source timer also drives its own per-event subtitle, so the video and the log can be compared line by line.

| Sub | Setup | Confirmed if (from the replay report) |
|---|---|---|
| T1 | One objective breadcrumb pulsed once at 04:35 | exactly one `AType:8` line for it (t1) |
| T2 | One breadcrumb pulsed 5 times, 2 s apart, from 04:37 | 5 lines (t1 "every firing"); 1 line = "once only", then the `Spawn` carrier becomes the default |
| T3 | Two breadcrumbs pulsed in the same tick at 04:47 | both logged (t5) |
| T4 | Six objective variants, 4 s apart from 04:50 (a message stays on screen for a few seconds, so each variant needs its own window): `Coalition` 0 / 1 / 2 × `Success` 0 / 1, all `IconType 0`, one with an `LCName` | which variants log, and which show anything to players on the video (t3). The default `ObjectiveStyle` becomes the silent variant that logs |
| T5 | One `Spawn`-carrier breadcrumb fired twice, at 05:15 and 05:20 | two spawn lines named `TRACE <n>` (fallback carrier works and re-spawns on each firing). A deleted object may write no line at all, so any removal line is recorded as information only |
| T6 | The whole traced probe | every per-event subtitle seen on the video has a log line within ± 2 s of its video time, and no breadcrumb logs without its subtitle (t6). The cue breadcrumbs give the tick-rate fit (t4) |

**0T acceptance (tests):**
- `trace_off_changes_nothing` (added by the 0T agent after step 1's fixtures have merged, like the traced-probe test): every baseline fixture is unchanged, and `instrument(` appears only in `trace.rs`, `probe.rs` and tests. From step 9T the grep also allows the one call behind the trace flag in `ui.rs`, and `trace_checkbox_off_output_matches_baseline` covers the UI side.
- `trace_instrument_adds_exactly_one_breadcrumb_per_source`: on `Exclusive_Activation_6plan.Group` with `decision_points()`, the breadcrumb count equals the number of matching MCUs; each traced MCU's `Targets` gains exactly its breadcrumb; no other property of any existing block changes (compare the serialised trees with the breadcrumbs removed).
- `trace_breadcrumbs_are_unique_and_inside_the_map`: unique indexes and positions on the trace grid, inside the `geo.rs` bounds and outside the `placement.rs` area.
- `trace_entity_source_gets_an_onevent`: tracing a plane entity's type-4 event adds one `OnEvent { Type 4, TarId = breadcrumb }` and nothing else.
- `trace_edges_follow_onevents_and_stop_on_loops`: an OnEvents link is followed, and a timer loop terminates.
- `trace_sidecar_round_trips` and `trace_edges_skip_untraced_mcus` (a traced → untraced timer → traced chain gives one edge).
- `trace_spawn_carrier_deletes_its_object`: the `Spawn` carrier has a Spawner, a neutral vehicle named `TRACE <n>` with its linked entity at `TRACE_SPAWN_POINT`, and a 1 s Delete.
- `missionlog_parses_synthetic_fixture` (`src/testdata/missionlog/synthetic/`: at least one record of each typed kind, one unknown `AType`, one unknown field, two files `[0]` and `[1]`): record counts per type, correct order across files, unknown kept and counted.
- `missionlog_real_fixture_parses` (`#[ignore = "needs a real log (0L or flight 1)"]` until `real_1` exists): no errors; unknown-record count reported.
- `replay_first_dead_link`: a synthetic log in which A fires, B (downstream of A) never fires, C (downstream of B) never fires reports B as the first dead link and groups C under it.
- `replay_never_spawned_counts_per_group` and `replay_tick_rate_fit` (breadcrumbs at known times give 50 ticks/s ± 1 %).
- `replay_cli_bad_args_exit_2` and `replay_cli_writes_report` (run `run_cli` in-process on the synthetic fixture with `--out` to a temp file).
- `probe_traced_every_subtitle_source_has_a_breadcrumb` (runs after 0b is merged): in the traced probe, every per-event, readout and cue subtitle's source has **exactly one** breadcrumb, T-t's sources have none from `instrument`, every cue breadcrumb has `expected_s`, and the sidecar has an entry for each T-t breadcrumb.

**0d addition.** After flight 1: add the log as `real_1` (trimmed), un-ignore `missionlog_real_fixture_parses`, run `replay` on the traced probe, and put the report next to `handoff/P14-probe-results.md` as `handoff/P14-probe-replay.md`. Record t1–t6 in the results and set the `TraceCarrier` / `ObjectiveStyle` defaults and the tick rate from them. If objectives do not log at all (t1 false), switch the default to `Spawn`; if neither carrier logs, trace builds fall back to debug subtitles only and the replay still reports spawns, kills and never-spawned groups.

### Step 1. Skeleton, core helpers and baseline — S–M. Branch `claude/p14-core`

**First commit: the skeleton.** It is merged into `claude/p14-air-tasking` before any other wave-1 agent starts, and the others branch from it (if the user declined local merges, §10 "Merges", the steps run serially on one branch instead):
- `main.rs`: `mod airtask;`, `mod in_theatre;`, `mod trace;`, `mod missionlog;`, the `missionlog::run_cli` line (step 0T), `#[cfg(test)] mod baseline_tests;`.
- `airtask/mod.rs` exactly as the §5 source: every type with its derives and `Default`s (including `DerivedTiming` and `SideLogic`), the `DPRK` / `NATO` index constants, `weighted_random_pct` (real body), the three stub functions, the strategy enums, every `mod` line and the two `pub use` lines.
- An empty stub file for each submodule (a doc comment only), so each later agent only fills its own file. Two exceptions carry `pub fn` signatures with stub bodies, so the `pub use` lines compile and the step 8 agent can call them in wave 2: `library.rs` (`builtin_library() -> Vec<AirSortie>` returning `vec![]`, `load_user_sortie(path: &Path) -> Result<AirSortie, String>` returning `Err`) and `timing.rs` (`f80_transit_s(radius_m: f64) -> f64` returning `0.0`, `derived_timing(t: &ThresholdSpec) -> DerivedTiming` returning the default).
- `src/baseline_tests.rs` stub (a doc comment only), so `cargo test` compiles after the skeleton commit.
- `in_theatre.rs` stub (a doc comment and the dead-code allow).
- `testkit.rs` with `assert_links_resolve(root)` (every Targets / Objects index resolves to a node in the tree) and a walker placeholder.
- `MapAirPack` in `frontlines.rs`.
- The visibility-only changes of §5 "Builders": `pub(crate)` on `mcu`, `timer`, `counter`, `checkzone`, `modifier_set_val`, `attach_event` (`template.rs`), `clone_named`, `synthesize_mcu`, `silence_clone_starts` (`recon.rs`) and `set_coord` / `add_yori` (`placement.rs`), plus the new `timer_random` next to `timer`. None of this changes output; the step 1 baseline fixtures are still recorded from `main` code, because nothing on a Generate path calls these differently.
- The dead-code allows (§8 general rules).

**Rest of step 1. Files:** `src/placement.rs` (`rotate_tree`), `src/weapon_range.rs` (`snap_air_attack_areas`), `src/airtask/timing.rs`, `src/in_theatre.rs`, `src/baseline_tests.rs`, `src/testdata/p14_baseline/`, and a small addition to `src/ui_tests.rs`: `write_p14_baseline_ui` (ignored) and `map_generate_without_air_matches_baseline` (§7). Step 1 owns `ui_tests.rs` only in wave 1b, before the step 8 agent takes it.

**Depends on:** nothing. **Runs in parallel with:** Steps 0b and 1b (after the skeleton commit).

**Acceptance (tests):**
- `existing_outputs_match_baseline`: the fixtures are generated from unmodified `main` code by `write_p14_baseline` and committed, and the test passes.
- `map_generate_without_air_matches_baseline` (`ui_tests.rs`): `base_map_ui.Group` is written by `write_p14_baseline_ui` from the §7 inputs and committed, and the test passes. Active from step 1, so the `ui.rs` Map export path is guarded before any air pack goes through it.
- `rotate_tree_quarter_turn`: by 90° about (40000, 40000), WP 1 moves from (44000, h, 40000) to (40000, h, 44000). YPos is unchanged. Plane `YOri` gets +90. MCU `YOri` is unchanged.
- `snap_air_attack_areas_moves_only_air`: only nodes with `AttackAir = 1` move. The ground AttackArea is untouched.
- `weighted_random_pct_values`: `(&[5,5,5]) == [33,50,100]` and `(&[1,3]) == [25,100]`.
- `f80_transit_and_derived_timing`: `f80_transit_s(12000)` is within 0.1 s of 118.0. `derived_timing` gives W for 10 / 12 / 16 km of 100 / 120 / 160, H = 2W and r_in = R/2, and applies the `Option` overrides.
- `in_theatre_la11_invalid_in_1950`: `la11` on 1950-08-01 is invalid and cites [S9 p.686].
- `in_theatre_mig15bis_from_1950_11_01`: invalid on 1950-10-31, valid on 1950-11-01, and the citation says "stand-in".
- `in_theatre_all_builtins_valid_on_1950_07_01`: every model in the six built-ins is valid on 1950-07-01.
- `in_theatre_every_row_is_cited_or_unsourced`: each row has a citation or `sourced = false`, and every `model_spec` plane id has a row.
- The test count is re-recorded, and warnings do not rise.

### Step 1b. Dry-run walker — M. Branch `claude/p14-walker`

**Files:** `src/airtask/testkit.rs` only (test-only code).

**Depends on:** the step 1 skeleton. **Parallel with:** step 1 rest and step 0b. Steps 3–7 use it.

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
| `MCU_TR_ComplexTrigger` | Emits "entered" once per scripted track entering the radius. | d |
| OnEvent Type 4 | A scripted "kill" event on a plane entity emits once. | g |
| `MCU_Delete`, commands, waypoints | Recorded, no effect, except a scripted "arrive at WP" event that fires the waypoint's Targets. | — |

Inputs: scripted tracks (coalition, list of (t, x, z)), forced pulses (node name, t), kill events, and a fixed seed. Output: a log of (t, node name) pulses and state snapshots. Deterministic for a given seed.

**Acceptance (tests):**
- `walker_self_test_exclusive_activation_grants_one`: on the unmodified `TemplateExamples/Exclusive_Activation_6plan.Group` (13 counters with `Dropcount 1`, 4 with `Dropcount 0`; it runs under the default `ResetOn1`).
  - Plans are found with `bombers::extract_exclusive_plans` (only to identify each plan's group and plane entities; the walker runs on the original tree, whose gate links `extract_exclusive_plans` would strip). Each plan's trigger zones are `bombers::inspect_plan(plan).suggested_triggers`.
  - Stimulus: one scripted NATO track and one DPRK track, each passing through every plan's trigger checkzones in turn, starting after Mission Begin.
  - A **grant** is a pulse of an `MCU_Activate` or `MCU_Spawner` whose Objects contain that plan's plane entities.
  - Expect grants for exactly one plan.
- `walker_self_test_fighter_pack_nodegates_grants_one`: a Fighter Pack built by `pack.rs` grants one per gate.
- `walker_random_is_seeded_and_rolled_each_time`: 40 pulses into Random 50 give 12–28 hits, the same count for the same seed.
- `walker_checkzone_track`: a Closer 1 zone fires when a track enters after the pulse and not before; a Closer 0 zone fires when the last track leaves.
- `walker_every_deactivated_out_has_a_rearm_or_is_single_use`: a static rule check over any tree; used by steps 3–7 on every generated pack.

### Step 2. Library and `place_air_sortie` — M. Branch `claude/p14-place`

**Files:** `src/airtask/library.rs`, `src/airtask/place.rs`.

**Depends on:** step 1. **Parallel with:** steps 3 and 4.

**Acceptance (tests):**
- `library_builtins_load_with_side_and_roles`: all six load; side and role detection match the §5 table.
- `library_leads_are_entities_with_empty_targets`: AirAlert, HVAR, CloseSupport = [7]; Armed Recon = [7, 11]; Yak = [7, 11]; Il-10 = [7, 15].
- `library_speed_alt_attack_time_per_file` (table-driven over all six, order AirAlert / HVAR / CloseSupport / ArmedRecon / Yak / Il-10): WP speed 660 / 660 / 520 / 520 / 500 / 390, WP altitude 3050 / 3050 / 1500 / 900 / 3050 / 1500, T_attack 1080 / 600 / 900 / 1200 / 300 / 600.
- For each of the six, placed at target (100000, 150000) with heading 45°, spawn distance 30 km, RTB (80000, 120000) (`place_all_builtins_*`):
  - `…_lead_at_spawn`: the first lead is at `target − 30 km·(cos 45°, sin 45°)` ± 1 m;
  - `…_bearing`: before snapping, the first lead → its WP bearing is 45° ± 0.01°;
  - `…_offsets_rotate_rigidly`: for every node with XPos that is not snapped (not a path waypoint, not an AttackArea), the new offset from the lead equals R(θ)·(old offset) within 0.01 m; YPos, XOri and ZOri are unchanged;
  - `…_plane_yori`: each plane's `YOri` == old + θ (mod 360);
  - `…_waypoints_and_attack_areas_on_target`: every non-RTB `MCU_Waypoint` and every ground and air AttackArea is on the target;
  - `…_rtb_per_lead`: one `RTB n` per lead at the RTB point with Priority 2; the Armed Recon escort's `RTB` uses its plane's YPos; `DELAYED END ORDERS.Time = 60`;
  - `…_links_resolve`: `assert_links_resolve` passes.
- `place_rejects_two_path_waypoints_per_lead` (D26 message) and `place_rejects_template_without_waypoint`, both on synthesized templates.
- `place_low_speed_uses_model_spec_and_warns`: a synthesized template with `Speed = 100` gets `suggested_waypoint_speed_kmh` and a warning.

### Step 3. Sortie shell, budget and assembly — M–L. Branch `claude/p14-shell`

**Files:** `src/airtask/shell.rs`, `src/airtask/budget.rs`, `src/airtask/assemble.rs`.

**Depends on:** step 1 and step 1b; step 2 for the tests that place real templates (merge step 2 first, or run those tests after it lands). **Parallel with:** steps 2 and 4. Strategy enums: `ZoneOutImpl`, `LossesImpl`, `BudgetImpl`, `PresenceCheckImpl` (stop check). Owns `AT s *`, the side's `Translator Mission Begin` (`emit_side_logic`) and the per-entry copy pointer (§3.0, §3.7).

**Acceptance, structural (table-driven over all six built-ins, each placed and wrapped as copy `SN01`/`SD01`):**
- `shell_strips_zone_in_and_start` : `Zone IN`, `ENABLE / PULSE IN`, `COOLDOWN`, both `Self Deactivate`, `Zone In ReActivate` and every `Mission Complete n` are gone. `Translator Mission Begin` has `Enabled 0` and no Targets.
- `shell_mission_end_only_from_done_hub`: `MISSION END` has exactly one inbound link: `DONE HUB`.
- `shell_done_hub_inputs`: `DONE HUB` receives exactly `STOP WAIT OUT`, `STOP CLOSE OUT`, `EXTEND`, `LOSSES` and `Zone Out`, and targets a Deactivate of itself.
- `shell_one_on_station_per_sortie`: exactly one `ON STATION`, `Time` = T_attack per file (1080 / 600 / 900 / 1200 / 300 / 600), targeted only by `ON STATION IN` (a latch), which is targeted only by the first lead's on-target timer (found by edge; for AirAlert, timer 39, not 37).
- `shell_wp1_arms_zone_out`: the first lead's `WP 1` targets `Zone Out ReActivate` and `PULSE OUT`. `Zone Out` is centred on the target.
- `shell_losses_counts_every_plane`: `LOSSES` Counter = 4 / 4 / 4 / 4 / 4 / 8, and the number of `OnEvent Type 4 → LOSSES` equals the plane count (the Armed Recon escort included).
- `shell_stop_probe_wiring`: `ON STATION` and `HARD STOP` both target `STOP PROBE`; `STOP PROBE` is a latch; `STOP ZONE` has radius r_in and is at the target.
- `shell_timing_values`: HVAR at clock distance: `HARD STOP.Time = 1500`; Armed Recon at clock distance: `2100`; `EXTEND.Time = 600`.
- `shell_delete_delay_targets_delete_and_done`.
- Variants: `shell_zone_out_watchdog_structure`, `shell_losses_latch_counts_each_plane_once_across_types_4_2_0`, `shell_stop_check_closer0_race_structure` (`PresenceCheckImpl::Closer0Race`: `STOP ZONE` is Closer 0, and a 2 s race timer reaches `EXTEND` only if the Closer 0 zone has not fired).
- `assemble_copy_pointer_structure`: per entry (including a `Zone`-only entry), `ENii COPY 1..runs` each target a distinct `SNjj REQ`; copy r's `GRANTED` deactivates `COPY r` and activates `COPY r+1`; the last copy's `GRANTED` targets `ENii EXHAUSTED`; `COPY r` for r ≥ 2 is in `SideLogic.init_off`; `SideCopies` returns the `COPY` and `EXHAUSTED` ids.
- `assemble_emit_side_logic`: `Translator Mission Begin` targets exactly `SideLogic.mission_begin` plus `AT s INIT` (0.5 s), and `AT s INIT OFF` lists exactly `SideLogic.init_off`; each `SNjj HOT` is registered in `SideLogic.hot_consumers` under its area.

**Acceptance, budget structure** (N = 2, 3 sorties): `budget_level_sets`: `BUD N L1 ON` lists the 3 `INC 1` and 3 `INC 1 OUT` relays plus `BUD N DEC 1`; there is no `INC 2`; `SideLogic.init_off` holds every level ≥ 1 relay (so `AT N INIT OFF` from `emit_side_logic` lists them); the `INC` stagger values are as in §3.3; each `UP k` / `DOWN k` has a 0.1 s SETTLE timer before its `L ON`; `BUD N DEC` lists `DEC 1`, `DEC 2` in ascending order. Variant: `budget_queue_structure`.

**Acceptance, walker scenarios** (each also run with `ReverseTargets` and with the three re-trigger modes):
- `shell_done_fires_once_when_mission_complete_and_zone_out_coincide`: `DONE` and `BUD DEC` each pulse once.
- `shell_losses_all_planes_triggers_done`.
- `shell_on_station_without_hot_ends_at_t_arrive_plus_t_attack_plus_1`.
- `shell_on_station_with_player_inside_extends_once_then_deletes`: DONE at on-station end + 600 s even if the player is still there; the later hard stop does nothing.
- `shell_hot_but_nobody_in_inner_zone_does_not_extend`.
- `shell_late_flight_probes_at_hard_stop` (WP 1 never reached).
- `shell_airalert_on_station_starts_once`: AirAlert, WP 1 reached: timer 39 is pulsed twice (from WP 1 and from 37), and `ON STATION` starts exactly once under all three re-trigger modes.
- `shell_closer0_race_extends_when_player_inside` (`Closer0Race` variant).
- `budget_grants_2_denies_1_then_grants_after_done`: START on 3 sorties, DONE on 1, START on the 3rd: grants 2, denies 1, then grants 1.
- `budget_level_falls_by_exactly_one_per_done`: after one DONE at level 2, a request is granted and the next one is **denied**.
- `budget_same_tick_up_and_down_is_the_documented_d25_case`: shows D25 (2), so a change in behaviour is noticed.
- `budget_queue_grants_2_of_4` (variant).
- `assemble_side_numbers_copies_and_uses_spawn_rule`: one copy per run per eligible entry, jj unique per side, `Zone`/`Both` copies at 30 km ± 1 m, `Clock` copies at v_wp·M ± 1 m.
- `walker_every_deactivated_out_has_a_rearm_or_is_single_use` passes on the wrapped copies.

**Needs the user in game: Probe 2.** The ignored test `write_p14_sortie_probe` (in `shell.rs` under `#[cfg(test)]`; `probe.rs` belongs to 0b) writes: one placed HVAR sortie started by a timer at 60 s, with a "DONE" subtitle; and cell **T-k**, built with `budget.rs` (N = 2, 4 requesters on timers at t, t, t+0.02 s and t+5 s, then DONE for one of them at t+30 s, then a 5th request at t+31 s and a 6th at t+32 s). Check:
- arrival time against §4.3 (±20 %), that the AttackArea is on the target, the RTB turn, deletion 60 s after RTB, and that "DONE" fires once;
- AirAlert's two AttackAreas (D28);
- T-k: exactly 2 granted at first; the 5th granted after the DONE; the 6th denied.

### Step 4. Threshold cell — M. Branch `claude/p14-threshold`

**Files:** `src/airtask/threshold.rs`.

**Depends on:** step 1 and step 1b; 0a and probe flight 2 for the N ≥ 2 path (build `ZoneOnly` first). **Parallel with:** steps 2 and 3. Strategy enums: `ThresholdImpl`, `CounterResetImpl`, `PresenceCheckImpl` (CHECK and HOLD re-check). The builder takes a `SideLogic` whose `hot_consumers` and `pass_subscribers` are already filled (it runs last, §3.0); its tests fill them with stand-in relays.

**Acceptance.** Step 4 is done when the `ZoneOnly` tests pass. The `ComplexTrigger` structural test is `#[ignore = "needs 0a reference"]` until the 0a file exists; enabling it is part of the commit that lands 0a.
- `threshold_zone_only_structure`: `THNaa ENTRY ZONE` (Closer 1, `Zone = 12000`, coalition of the other side) targets `ENTRY OFF` and `ARM`; `ARM` is a latch; `DWELL.Time = 120`; `INNER` (`Zone = 6000`); `INNER CLOSE.Time = 2`; `HOLD.Time = 240`. `CHECK` targets the Activate of `INNER CLOSE OUT`. `PASS` targets `INNER CLOSE OUT` Deactivate, `INNER OFF`, `HOT ON`, `HOLD` and exactly the area's `SideLogic.pass_subscribers`. `REARM` activates `ARM` and re-arms and re-pulses `ENTRY ZONE`. `HOT ON` / `HOT OFF` list exactly the area's `SideLogic.hot_consumers`. `SideLogic.init_off` contains the HOT set, and `SideLogic.mission_begin` contains `ENTRY ZONE`.
- `threshold_inner_closer0_race_structure` (`PresenceCheckImpl::Closer0Race` variant): `INNER` is Closer 0, and `PASS` comes from the 2 s race timer only if `INNER` has not fired.
- `threshold_n1_emits_no_complex_trigger` (D1).
- `threshold_complex_trigger_structure` (ignored until 0a): `ENTRY` is a copy of the 0a reference property set with radius and filters set; `COUNT` has Counter 3; there is a `WINDOW → ZERO`, `WINDOW → WINDOW` loop at 120 s; `REARM` targets `ZERO`.
- `threshold_counter_ring_structure` (variant).
- `threshold_timing_follows_radius`: R = 16 km gives W, D and H of 160 / 160 / 320.
- Walker scenarios:
  - `threshold_loiter_passes`: a track enters at t = 0 and stays inside r_in: PASS between 120.1 and 122.6 s (DWELL + INNER PULSE + ordering).
  - `threshold_fly_through_does_not_pass`.
  - `threshold_pass_leave_pass_again`: PASS; the player leaves; the HOLD re-check fails (HOT OFF, ENTRY re-armed); the player re-enters and loiters: a second PASS after a full dwell.
  - `threshold_second_plane_after_close_does_not_pass`: a second plane entering r_in after `INNER CLOSE` gives no PASS until a new dwell.
  - `threshold_hold_recheck_turns_hot_off_when_player_left`.
  - `threshold_repeat_firing_entry_does_not_restart_dwell` (walker `Repeat` switch on).
  - `threshold_closer0_race_loiter_passes` (`Closer0Race` variant).
  - `threshold_n2_fewer_than_n_in_window_does_not_fire`, `threshold_n2_count_resets_at_w` and `threshold_n2_fires_again_after_failed_dwell` (COUNT fired, the dwell fails, `ZERO`, N more entries: a second ARM; run under both `SetValRefires` settings: `CounterResetImpl::SetVal` passes under `Yes`, and `Ring` passes under both) (ignored until 0a, like the structure test).

### Step 5. Clock front end — M–L. Branch `claude/p14-clock`

**Files:** `src/airtask/clock.rs`.

**Depends on:** steps 3 and 4. **Does not wait for f**: it is built with `RandomImpl::{InMission, Seeded}`. **Parallel with:** step 6.

**Acceptance (tests):**
- `clock_structure_three_entries` (NATO, 3 entries on 3 areas, weights 5, 5, 5): `CLK N FIRST.Time = 600`, `CLK N PERIOD.Time = 1200` with the loop back to `TICK`, `CLK N QUIET ROLL Random = 80`, 3 `JITTER k DELAY` with 0 / 180 / 360, and `EN0i RANDOM` with Random 33 / 50 / 100 in shuffled order (seeded), at `Time` 0.5 / 1.0 / 1.5 s in waterfall order, and `Wait for Output` at 2.0 s.
- `clock_entry_relays`: each entry has `AVAILABLE`, `LAST`, `EXCLUDE`, `MISS` + `MISS OUT`, `HOT` (registered in `SideLogic.hot_consumers`), `GO`; `GO` targets every `ENii COPY r` from `SideCopies` (built by `assemble_side`, not by the clock).
- `clock_rearm_edges`: `DRAW` activates `ATTEMPT END OUT` and every `MISS OUT`; `GO` deactivates `ATTEMPT END OUT` and `REROLL`; `LAST SET`'s Deactivate lists every `LAST` except its own; `SLOT OPEN` targets `LAST CLEAR` (`Time` 0.2), which deactivates every `LAST`, and `DRAW DELAY` stays at 0.3 s so the clear always comes before any new `LAST SET`.
- `clock_last_copy_disables_random`: `ENii EXHAUSTED` (from `SideCopies`) targets a Deactivate of `ENii AVAILABLE` and `ENii RANDOM`.
- `clock_slot_window_formula`: `CLK N SLOT CLOSE.Time = min(300, (a + 10)·A)`.
- `clock_ignores_zone_only_and_ineligible_entries`: `Zone`-only, disabled, orphaned and date-invalid entries have no `EN RANDOM`.
- `schedule_disabled_emits_no_clk`.
- `clock_generate_time_choices_are_seeded_and_deterministic` (`Seeded` variant): same seed, same unrolled slots; a slot tries its list in order.
- Walker scenarios (seeded):
  - (i) `clock_all_hot_one_grant_per_slot_no_back_to_back`: all areas HOT: one grant per non-quiet slot over 20 slots, and no entry wins two slots in a row.
  - (ii) `clock_only_one_area_hot_alternates_with_quiet`: only entry 2's area HOT: entry 2 wins, reached by reroll, in every other non-quiet slot; the slot after each win is quiet because of no-repeat.
  - (iii) `clock_no_area_hot_sends_no_req`.
  - (iv) `clock_budget_full_denied_no_reroll`.
  - (v) `clock_entry_never_wins_after_runs_grants`.
  - (vi) `clock_early_miss_then_later_winner_one_req_per_slot`: a miss at waterfall position 1 followed by a winner: exactly one REQ in the slot.
  - (vii) `clock_win_then_later_miss_rerolls`: an entry wins slot 1; in slot 3 it is drawn while its area is not HOT: its area is excluded and another HOT entry wins.
  - (viii) `clock_sixteen_entries_one_passing_area_found`: 16 entries on 8 areas, one HOT area: the slot grants an entry on that area.
  - (ix) `clock_quiet_rate_and_jitter_spread`: 200 slots: quiet rate within 20 % ± 7 %, and each jitter step used.
  - (x) `clock_both_entry_exhausted_by_zone_never_wins_a_slot`: a `Both` entry used up through the zone; the next clock slot still grants another HOT entry.
  - (xi) `clock_nobody_won_quiet_rate_is_small`: 3 equal-weight entries on 3 areas, #3 the last winner (excluded), only #1's area HOT, 200 slots: #1 wins at least 97 % of the non-quiet-roll slots (expected about 99.5 % with the +10 window, §3.7).
  - `clock_seeded_slot_rerolls_down_its_list` (`Seeded` variant).
- `clock::validate_schedule(plan, ctx) -> Vec<AirIssue>` holds the §3.7 schedule checks; step 7's `validate` calls it and adds nothing to them. Tests: `clock_validate_schedule_period_shorter_than_jitter_plus_window_is_an_error` and `clock_validate_schedule_slot_window_cap_warns`.

### Step 6. Zone front end — M. Branch `claude/p14-zone`

**Files:** `src/airtask/zone.rs`.

**Depends on:** steps 3 and 4. **Parallel with:** step 5.

**Acceptance (tests):**
- `zone_structure`: an entry with mode Zone: `ZNii CHANCE Random = 50`, `ZNii COOLDOWN.Time = 900`. `ZNii ARMED` is the target of `THsaa PASS` and activates `ZNii MISS WAIT OUT`. `ZNii FIRE` activates `ZNii WAITING`. `SNjj DENIED` and `SNjj DONE` target `ZNii WAITING`, which targets `ZNii COOLDOWN` and its own Deactivate. `ZNii WAITING` starts OFF.
- `zone_last_copy_deactivates_rearm`: `ENii EXHAUSTED` (from `SideCopies`) targets a Deactivate of `ZNii REARM`; `ZNii FIRE` targets every `ENii COPY r`; `ZNii ARMED` is registered in `SideLogic.pass_subscribers` under its area.
- `zone_ignores_clock_only_entries`: `Clock` entries have no `ZN*` MCUs.
- `zone_debug_subtitles_only_when_enabled`: with `debug_subtitles = true`, `ZNii ARMED`, `ZNii FIRE`, `ZNii MISS WAIT OUT` (chance miss), `ZNii COOLDOWN` start and `ZNii REARM` each drive their own subtitle; with `false`, the zone emits no subtitle.
- `both_entry_copies_use_zone_spawn_distance`: lead at 30 km ± 1 m.
- `zone_seeded_chance_sequence_structure` (`Seeded` variant).
- Walker scenarios:
  - `zone_budget_full_denied_rearmed_after_cooldown_then_granted`: PASS while the budget is full → DENIED → re-armed after 900 s → PASS → granted.
  - `zone_no_req_after_runs_grants`.
  - `zone_fire_then_later_chance_miss_reaches_cooldown`: a FIRE, its DONE, re-arm, then a failed chance roll: `COOLDOWN` starts and the zone re-arms.
  - `zone_clock_done_does_not_start_idle_zone_cooldown`.

### Step 7. Export integration — S–M. Branch `claude/p14-export`

**Files:** `src/airtask/mod.rs` (`build_air_packs`, `preview`, `validate`), `src/frontlines.rs` (`FrontOptions.air_packs`, stamping, `inspect_base_map` recognizer, `ImportedBaseMap.air_tasking_found`), plus the one coordinator line in `ui.rs` (§7).

**Depends on:** steps 2–6. **Parallel with:** step 8a (the UI skeleton, already running since wave 2).

`build_air_packs` runs each side's builders in the §3.0 build order (assemble → clock → zone → threshold → `emit_side_logic`) over one `SideLogic`. `validate` calls `clock::validate_schedule` for the schedule checks and sets `rail_only = true` on the "no enabled valid entry" issue only (§6.1).

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
- `arrow_side_matches_generate_front`: for the test arrows, `point_north_of_front` on `ctx.front` and on `dense_base` agree.
- An ignored test `write_p14_acceptance_mission` writes the step 11 mission (§8 step 11) with `debug_subtitles = true`, from a plan pinned in code, `acceptance_test_plan()`: 2 DPRK + 2 NATO objectives at fixed points; NATO: AirAlert `Clock` on objective 1, HVAR `Clock` on objective 2, CloseSupport `Zone` on objective 1, ArmedRecon `Both` on objective 2; DPRK: Yak `Zone` on objective 1, Il-10 `Both` on objective 2; one NATO entry's objective placed so that its spawn → target → RTB path passes within R of a DPRK area (D30); `AirPlan.seed = 7`; default settings otherwise.
- `acceptance_mission_has_zone_entries_and_debug_subtitles`: the written mission has, per side, at least one `Zone` and one `Both` entry's `ZN*` cell, the clock and zone debug subtitles named in step 11, and `validate(acceptance_test_plan())` has exactly one D30 warning and no errors.

### Step 8. Air Tasking rail tab — M. Branch `claude/p14-ui-tab`

**Files:** `src/ui.rs`, `src/shell.rs`, `src/help.rs`, `src/ui_tests.rs`, `USER_MANUAL.md` (stub section only).

**Depends on:** step 1 types (8a); steps 2 and 7 bodies (8b). **Runs:** the step 8 agent starts in wave 2 and keeps `ui.rs` through waves 3 and 4. **Not** parallel with step 9 (same files).

**Acceptance, 8a (wave 2 into wave 3; stop and report when done):** the §6.1 arms are in place (the compiler checks the exhaustive ones), the §6.3 "Extend" edits are made, and the §6.3 "New (step 8a)" tests pass. Every guard test passes.

**Acceptance, 8b (wave 4, after step 7 merges and before step 9; stop and report when done):** the §6.3 "New (step 8b)" tests pass, "Generate File" and the readiness checks are wired to the real `validate` / `build_air_packs`, and every guard test still passes. `cargo test` count rises by at least the new tests of 8a and 8b.

**Live check (ask the user that the screen is free):** `tools/ui-live/drive.ps1` screenshots of the tab at 1400 × 1000.

### Step 9. Map dock Air tab — M. Branch `claude/p14-ui-map`

**Files:** `src/ui.rs`, `src/shell.rs` (only for the D35 label, if needed), `src/ui_tests.rs`, and the removal of the `// P14` dead-code allows.

**Depends on:** steps 7 and 8b.

**Acceptance (tests):** the dock tabs fit (`map_dock_tabs_fit_a_three_digit_reference_count`), `map_dock_tabs_all_render` includes "Air", and the §6.3 "New (step 9)" tests pass. No `// P14` `allow(dead_code)` remains except the `trace.rs` / `missionlog.rs` ones, which step 9T removes; warnings do not rise.

### Step 9T. Trace and replay in the UI (rest of P10) — M. Branch `claude/p14-replay-ui`

**Depends on:** step 0T, flight 1 results (carrier, objective style, tick rate), step 9. Same agent as steps 8–9 (`ui.rs` has one owner).

- **Trace build.** One header checkbox "Trace build", beside Generate, on every tab. When on, Generate calls `trace::instrument` with `decision_points()` (plus the P14 names on the Air Tasking and Map tabs) and writes the `.trace.json` sidecar next to the `.Group`. Off by default and not remembered between runs, so normal output never changes. The status bar says "Trace build: N breadcrumbs" after Generate.
- **Replay view.** A "Replay…" header button opens a panel: pick the `.Group`, its `.trace.json` (found automatically when it sits next to the `.Group`) and the log folder. It shows the `report_markdown` sections as tables: First dead links, Never fired, Never spawned, and a Timeline filter by group.
- **Map overlay.** On the Map tab, a "Replay" layer draws spawn positions (from spawn records) and kill positions, coloured by side, with a legend chip. Off unless a replay is loaded.
- Readiness: Replay needs a `.Group` and at least one log file; a missing sidecar is an orange note (the report then has no breadcrumb sections).

**Acceptance (tests):**
- `trace_checkbox_off_output_matches_baseline` and `trace_checkbox_on_writes_sidecar` (ui_tests: Generate on the Template tab both ways).
- `replay_panel_shows_first_dead_link` (ui_tests, synthetic fixture from step 0T).
- `map_replay_layer_draws_spawns` (counts drawn markers against the replay's spawn count).
- Every new control has an AccessKit label, 28 px height and no missing glyphs (the existing guard tests cover it once the controls exist).

### Step 10. Docs and manual — S. Branch `claude/p14-docs`

**Files:** `USER_MANUAL.md` (`## Air Tasking`, a Map dock "Air" paragraph, and a `## Trace builds and replay` section: the server text-log setting confirmed in step 0T, what a breadcrumb is, the sidecar, the CLI, the Replay panel), `docs/src-guide.md`, `docs/ui-redesign/README.md` §4 / §6.4, `HANDOFF.md` (log row + P14 status), `TemplateExamples/Historical1950/README.md` (fix step 3: the ground AttackArea sits on the spawn point).

**Depends on:** steps 8, 9 and 9T.

**Acceptance (tests):**
- The help test passes.
- `manual_air_tasking_names_every_approximation`: the `## Air Tasking` section contains each of these phrases: "tumbling window", "waterfall", "no back-to-back", "one extension", "at least one plane", "AI may count", "runs", "budget race", "zone re-arm", "UNVERIFIED", "Complex Trigger", "Zone Out", "kill events", "Random %".
- `no_ctrl_1_6_left`: no "Ctrl 1–6" in `src/`, `USER_MANUAL.md` or `docs/`, except the README §9 phase history; the plain map-key "1–6" is still present in `help.rs` and the manual.
- `no_p14_dead_code_allows_left` (grep).

### Step 11. Full in-game acceptance — user

- Generate the acceptance mission **as a trace build** (step 9T) with `write_p14_acceptance_mission` (step 7), keep the server's text log on, and after the flight run Replay on it. Each check below is read from the replay report first (exact times; the SLOT OPEN, GRANTED, DONE and EXTEND breadcrumbs), and from the debug subtitles and video only where the log cannot show it. The mission is: a base map with 2 DPRK + 2 NATO objectives and the pinned `acceptance_test_plan()` (all six built-ins; per side at least one `Zone` and one `Both` entry; one NATO entry that raises the D30 warning), with debug subtitles for slot open, winner, DENIED, GRANTED, the stop probe result, the DONE cause, and the zone's ARMED, FIRE, chance miss, COOLDOWN start and REARM. Fly a 90 min server session.
- Check, each against a subtitle:
  - slots at first + k·period + jitter step, each ± 5 s (10 / 30 / 50 / 70 min, + 0 / 3 / 6 min);
  - "winner" subtitles only for areas where a player loitered at least D (no sortie over an empty area);
  - never more than 2 GRANTED without a DONE in between, per side;
  - each sortie reaches its target, attacks, RTBs and is deleted;
  - zone entries fire on arrival (chance permitting) and respect the cooldown;
  - **extension:** one player loiters inside r_in of a sortie's target when its on-station time ends: "EXTEND" shows once, and DONE follows 10 min later even if the player stays;
  - **Zone Out:** after a sortie reaches its target, all players leave beyond 35 km: the DONE cause is "Zone Out";
  - **AI and the other side:** with the D30 orange warning showing for one entry, no sortie is triggered by the other side's AI (a winner subtitle for that area without a player present is a failure);
  - **standalone file (risk l):** import the Air Tasking tab's standalone file into a mission that already has groups; check in the editor that indexes do not collide and that links survive, then fly one slot.
- Record the results in `handoff/P14-probe-results.md`.

---

## 9. Risk register

The repo does not prove any of these in game. The shipped files only show what the authors assumed (`handoff/R3-template-feature-gaps.md` ≈115-122).

| ID | Behaviour | P14 assumes | Repo evidence | Status | Fallback |
|---|---|---|---|---|---|
| a | CheckZone `PlaneCoalitions` fires for AI | Only players should start sorties | `USER_MANUAL.md` ≈75 "until a player enters"; Fighter Pack Zone IN `[2]` 16 km | UNVERIFIED | D30 validator, dwell, and a Complex Trigger player filter if one exists |
| b | Closer 0 with several players | Fires when ALL have left | `USER_MANUAL.md` ≈75, ≈113 | UNVERIFIED | Watchdog (§8 table) plus the hard stop |
| c | Activate/pulse while a player is already inside fires at once | Dwell CHECK, HOLD re-check, stop check | `USER_MANUAL.md` ≈111; COOLDOWN re-pulse (`flights.rs` ≈451-452) | UNVERIFIED | Closer 0 + race-timer negative logic |
| d | Complex Trigger filters and events | Per-object "entered alive", country filter | None: 0 hits in samples; only the drop-list at `template.rs` ≈5210 | UNVERIFIED, **blocking for N ≥ 2**; needs the user's 0a file and probe flight 2 | N limited to 1 (`ZoneOnly`) |
| e | Counter reset (Dropcount, ModifierSetVal) | `Dropcount 1` = reset after firing (the codebase convention, §3.2 note); ZERO resets COUNT, and a fired `Dropcount 0` COUNT fires again after ZERO (T-e E6) | `template.rs` ≈2638-2639, ≈2650-2672, `flights.rs` ≈398, commit `7437c51` | UNVERIFIED | Counter ring |
| f | Timer `Random %` rolled on each trigger | Quiet, jitter, waterfall, zone chance | `flights.rs` ≈1010-1013, `recon.rs` ≈1246-1256 | UNVERIFIED (the Fighter Pack already depends on it) | `RandomImpl::Seeded` |
| g | Kill events per aircraft | Type 4 once per plane; none on Delete | `template.rs` ≈559-583; Fighter Pack Type 4 → DeathCount | UNVERIFIED | Per-plane latch on 4 / 2 / 0; the hard stop |
| h | Deactivating a waiting checkzone cancels it | INNER CLOSE, INNER OFF | `USER_MANUAL.md` ≈203, `flights.rs` ≈905-915 | UNVERIFIED | Gate a downstream relay instead |
| j | Relay gating; running-timer cancel | D11 relays | recon/flights closers | UNVERIFIED | Explicit clear; OUT relays already cover the cancel case |
| k | Budget one-hot counter races | D25 | none | UNVERIFIED | Per-side request queue |
| l | Editor re-indexes a standalone group on import | The standalone Air Tasking file imports cleanly | R4 P13 "first check" | UNVERIFIED | Offer only the base-map path |
| m | `rotate_tree` and MCU `YOri`; AttackArea overridden by a later order; Force Complete Priority 2 cancels an AttackArea | Placement and cleanup | R3 in-game checks | UNVERIFIED | Probe 2 (step 3) |
| n | All `model_spec` cruise speeds | W, D and H use 732 km/h | No citation (`model_spec.rs` ≈1-21; the `f80c10` row has an empty notes field) | Unsourced | If the player speed measured in cell T-n is off by more than 10 %, set `model_spec` `f80c10` `cruise_kmh` to the measured value, with a note citing `handoff/P14-probe-results.md` (or to a sourced figure if one is found later), so the derived W and D follow it. `Manual timing` stays a per-plan override |
| o | A pulsed Closer 1 zone fires again while a plane stays inside | Not relied on | Templates self-deactivate Zone IN | UNVERIFIED | None needed: ENTRY OFF plus the ARM latch (§3.5) |
| p | A running timer re-triggered | Not relied on; the one template timer that is re-triggered (AirAlert 39) reaches generated logic only through the `ON STATION IN` latch | AirAlert `WP 1` → [37, 39], 37 → 39 | UNVERIFIED | Latch in front of any timer the walker shows can be re-triggered |
| q | Same-tick delivery: a latch hit twice in one tick; Activate before a pending fan-out pulse | Latch passes once; SETTLE timers avoid the fan-out case | none | UNVERIFIED | `DONE GATE` style short-timer latch (§8 table) |
| t1 | A Mission Objective MCU writes an `AType:8` line to the text log **each** time it fires | Trace builds (step 0T) | None in the repo; community log documentation | UNVERIFIED (cell T-t T1, T2) | `TraceCarrier::Spawn` (spawn line per firing) |
| t2 | `AType:8` identifies the objective by its index (`OBJID`) or only by position | Sidecar keeps both; match by index, else by position ± 1 m | None | UNVERIFIED (T-t) | Position match only |
| t3 | A trace objective shows nothing to players and does not affect the mission result | Silent `ObjectiveStyle` (T4 picks it) | Airfield files carry objectives (`K14 AFB.Group` ≈21535) but say nothing about display | UNVERIFIED (T-t T4, the players' notes) | `Spawn` carrier |
| t4 | `T:` counts ticks at 50 per second | Timeline in seconds | Community documentation only | UNVERIFIED | The replay fits the rate from the cue breadcrumbs and uses the fit |
| t5 | Two breadcrumbs in one tick both log | "First dead link" and counts | None | UNVERIFIED (T-t T3) | Report same-tick firings as "at least one" |
| t6 | The log records everything the video shows (no dropped lines; the last file written on a normal mission end) | Replay is the primary evidence in 0d and step 11 | None | UNVERIFIED (T-t T6) | Video and subtitles stay the evidence; the replay is a cross-check |

---

## 10. Notes for the implementing ultracode session

**Before any code:**
- Read `HANDOFF.md`, `CLAUDE.md`, `.cursorrules` and `docs/ui-redesign/README.md`.
- Confirm the working tree state and that `main` has no merge in progress. Do not touch or stash anyone else's uncommitted edits. Work in git worktrees under `.claude/worktrees/`.
- Create `claude/p14-air-tasking` from `main`.
- **Merges (U11). Ask the user, before wave 1a,** whether step branches may be merged locally into `claude/p14-air-tasking` during the session. Merging to `main` and any push always need the user.
  - **Yes:** run the fan-out plan below.
  - **No (or no answer):** do not fan out. Run the steps serially as commits on the one branch `claude/p14-air-tasking`, with no merges: 1a, 1, 1b, 0T, 0b, 2, 3, 4, 5, 6, 7, 8 (8a and 8b together), 9, 9T, 10. Ownership, tests and the stop-and-report rhythm are unchanged; the coordinator commits of the table are ordinary commits in that order.

**Fan-out plan:**

| Wave | Parallel agents (one worktree each) | Files they own | Conflicts to avoid |
|---|---|---|---|
| 1a | Step 1 skeleton commit (one agent, serial) | `main.rs` (including `mod trace; mod missionlog;` and the `missionlog::run_cli` line), `trace.rs` and `missionlog.rs` stubs with the step 0T signatures, `airtask/mod.rs` (§5 exact source), every `airtask/*.rs` stub, `testkit.rs` (`assert_links_resolve`), `in_theatre.rs` and `baseline_tests.rs` stubs, `MapAirPack` in `frontlines.rs`, and the visibility-only `pub(crate)` edits + `timer_random` in `template.rs`, `recon.rs`, `placement.rs` | Merged into the integration branch before 1b starts |
| 1b | Step 1 rest; Step 0b probe code; Step 1b walker; **Step 0T trace + replay** | Step 1: `placement.rs` (`rotate_tree`), `weapon_range.rs`, `timing.rs`, `in_theatre.rs`, `baseline_tests.rs`, fixtures, and the baseline addition to `ui_tests.rs`. 0b: `probe.rs`. 1b: `testkit.rs`. 0T: `trace.rs`, `missionlog.rs`, `src/testdata/missionlog/` | Nobody edits `airtask/mod.rs`; its types are frozen. 0b only calls the builders made `pub(crate)` in 1a. 0b's writer calls `trace::instrument` through the 1a stub; the test `probe_traced_every_subtitle_source_has_a_breadcrumb` is added by the 0T agent once both have merged. Step 1's `ui_tests.rs` addition lands before wave 2, when the step 8 agent takes the file |
| 2 | Steps 2, 3, 4, and step 8a (the step 8 agent starts here and keeps `ui.rs` until step 9 ends) | `place.rs` + `library.rs` / `shell.rs` + `budget.rs` + `assemble.rs` / `threshold.rs` / `ui.rs` + `shell.rs` + `help.rs` + `ui_tests.rs` | A needed type change in `mod.rs` goes through a single coordinator commit. Steps 3 and 4 meet only through `SideLogic` (§3.0) |
| 3 | Steps 5 and 6; step 8a continues if not done | `clock.rs` / `zone.rs` | Both call the assemble (`SideCopies`, copy pointer, `SideLogic`), budget and threshold APIs only; neither creates or names the other's MCUs |
| 4 | Step 7, then step 8b, then step 9 (same agent as step 8), then step 9T (same agent), then step 10 | `frontlines.rs`, `airtask/mod.rs`; `ui.rs`; docs | `ui.rs` has one owner at a time. Step 7's one `FrontOptions` line in `ui.rs` goes in through the coordinator, and the step 8 agent merges it before 8b |

Step 0c/0d (probe flight 1) can happen as soon as step 0T and 0b are merged (the start of wave 2), and at any time during waves 2–3 after that. **Tell the user when the traced probe file is ready**, with the server text-log setting (U12). Flight 2 (T-d) happens whenever the user has done 0a; it does not block any wave. When results arrive, change the strategy-enum defaults in one small commit.

**Verification expectations:**
- Every acceptance line in §8 is a named test; the orchestrator checks coverage by grepping the names.
- Run the walker's re-arm rule and `assert_links_resolve` over every generated air pack in steps 3–7.
- Keep the baseline fixture test green from step 1 onward.
- Record the test and warning counts in the `HANDOFF.md` log row at each stop.

**For the user to confirm (additions and deviations; implement as written, and list them in the first stop report):**
1. **U7 extension condition (D32).** The build cannot count N players inside the inner zone. It extends when the area is HOT (N arrivals recently) and at least one enemy plane is inside r_in at that moment. For N = 1 this is exactly U7.
2. **When the extension is tested (D31).** At the end of on-station time, not only at the hard stop, because otherwise it could never fire.
3. **User templates from disk.** Added by Claude; the design named only the six built-ins.
4. **Run cap on the clock (D12).** Each entry runs at most `runs` times (default 3) across both front ends, so a pool of E entries gives at most E × runs sorties per mission. The manual states it, and the Runs tooltip shows it.
5. **No-repeat can leave a slot quiet** when only the last winner's area is busy (§3.7).
6. **Budget race (D25 (2)).** An UP and a DOWN in the same tick leave the budget wrong for the rest of the mission. Rare; documented.
7. **Zone re-arm residual (§3.8).** For a `Both` entry, a clock copy ending first can re-arm the zone while the zone's own copy is flying.
8. **MiG-15 date (D34).** `mig15bis` is valid from 1 Nov 1950 as the stand-in for the MiG-15.
9. **Debug subtitles** exist only in the step 11 test mission, not in the UI.
10. **"Refs" dock label (D35)** if five tabs do not fit.
11. **Threshold ships as CheckZone-only (N = 1) until the user unlocks N ≥ 2.** U1 asks for Option A built from Complex Trigger + Counter + window + dwell. v1 defaults to `ThresholdImpl::ZoneOnly`: a plain CheckZone entry plus the dwell check, and the UI caps N at 1. The Complex Trigger + Counter path for N ≥ 2 is built and tested but not emitted until two things happen: **the user completes 0a** (builds `TemplateExamples/Reference/ComplexTrigger_reference.Group` in the mission editor, §8 step 0) and **probe flight 2 (cell T-d) confirms it** in game. Until then no threshold above one player is available.
12. **A clock slot can go quiet with a passing entry available (U6, §3.7).** U6 says to reroll until an entry passes. Attempts in which the waterfall picks nobody (possible once the last waterfall entry is excluded as LAST or by an area miss) are random and not bounded, while the slot window is finite (`min(300 s, (a + 10)·A)`). In the worst simple case (3 entries, the only passing one first in the waterfall) about 0.5 % of slots go quiet; with many entries, where the 300 s cap binds, more can. The walker test `clock_nobody_won_quiet_rate_is_small` pins the rate; the manual states it.
13. **P10 is split (U12).** The replay core (parser, timeline, never fired, first dead link, never spawned, Markdown report, CLI) is step 0T, before the probe flights. Its UI (Trace build checkbox, Replay panel, Map spawn/kill overlay) is step 9T. R4 P10's "templates that never spawned" is matched per group path by entity name, with counts, because copies share names.
14. **Trace builds use Mission Objective MCUs as log breadcrumbs,** with a spawned-object fallback. Both are UNVERIFIED until cell T-t; if neither logs, tracing falls back to debug subtitles (§8 step 0T).

**Stop and report.** After each step, list what changed, the test count and the warnings. Say what needs the user in game. Do not start the next wave until the user has seen the list if the user asked for that at session start.

**Nothing is pushed or merged to `main` without the user.** No tags, since a `v*` tag triggers the release autobuild. Never `git add -A`: the parent folder is 37 GB. `.gitattributes * -text` stays.

---

## 11. Revision log

- **Round 1 review (2026-09-24).** Re-arm edges added to every cancel relay (§3.0, §3.5, §3.7, §3.8). Lead rule fixed (empty Targets). Threshold entry self-deactivates behind an ARM latch. Budget level switches use SETTLE timers. Clock attempts are fixed length, exclude whole areas, and end the slot on a win. Stop probe moved to the end of on-station with a fresh inner check (D31, D32). One ON STATION per sortie. Zone cooldown gated by WAITING. Walker made its own step (1b) with a semantics table. Skeleton commit, dead-code allows, `MapAirPack` and the `FrontOptions` line given owners. Named tests for every acceptance line; probe "Confirmed if" column; T-k moved to Probe 2 with a 6th request; new cells C5, J4, J5 and risks o, p, q. Shortcut edits limited to "Ctrl 1–6". Code anchors re-based on `bf1a356` and named by function. `in_theatre` seeding uses `mig15bis` as the stand-in and has no La-9 row.
- **Round 2 review (2026-09-24).** Builder `pub(crate)` edits and `timer_random` moved into the 1a skeleton, so the 0b probe compiles in wave 1b; probe AI flights come from the HVAR / Yak files through `probe_flight`. Step 0 split into flight 1 (all cells but T-d) and flight 2 (T-d, after 0a) with its own ignored writer and tests. Run sheets added (K14 origin, offsets, arm / close times, player per cell, cue subtitles; constants pinned by `probe_run_sheet_matches_the_table`). T-a split into T-a1 (AI triggering) and T-n (player-measured F-80C cruise, the basis of W and D). Every count is read through threshold counters with their own subtitles (C5, F1, T-d). Walker Counter rule corrected (`Dropcount 1` = reset, the codebase convention; configurable until T-e), `MCU_TR_MissionBegin` rule added, Exclusive self-test defined. §5 now gives the exact skeleton source (derives, `Default`s, `DPRK`/`NATO` index constants, `AirPreview` / `AirIssue` / `AirDrawCounts` fields, strategy enum variants) and a `baseline_tests.rs` stub. `AirPlan.seed` added (the Map has no stored seed). The `base_map_ui.Group` fixture and `map_generate_without_air_matches_baseline` moved to step 1 with explicit inputs and terrain off. §10 item 11 states the N = 1 default and the 0a unlock. Anchors re-based on `4c90de6`.
- **Round 3 verification (2026-09-24).** Probe: C4 now tests risk h (activated and pulsed at 11:30 while N1 is outside, deactivated at 11:45, cue at 12:00); every "exactly once" criterion reads a `Counter(2)` readout (E1, E2, E6, J4, J5, each T-g type 4), probe subtitles `Duration = 1` and cues 20; T-g's G1 has its own centre 30 km east with an icon; new cell E6 (a fired `Dropcount 0` counter re-fires after `ModifierSetVal`) and walker switch `SetValRefires`; ZA4 belongs to T-n for arming and closing. Skeleton: `pub use` of `builtin_library`, `load_user_sortie`, `f80_transit_s`, `derived_timing` (signature stubs in 1a), new `DerivedTiming`, `SideLogic` (cross-module wiring, build order, owner of `AT s *` = step 3), `PresenceCheckImpl` (risk c), `AirIssue.rail_only`. Copy pointer and `ENii EXHAUSTED` moved to `assemble.rs`. RANDOM k at 0.5·k s, matching `build_randomizer`. Slot window `(a + 10)·A` and §10 item 12 (quiet slot with a passing entry). Map readiness no longer blocks on an orphaned-only pool. `SNjj ON STATION IN` latch (AirAlert timer 39 is pulsed twice). Step 5 schedule checks moved to `clock::validate_schedule`; step 8 split into 8a / 8b; serial fallback when local merges are declined; Load tests moved to step 9; step 11 plan pinned as `acceptance_test_plan()` with Zone / Both entries, a D30 case and zone debug subtitles; the h/j, p, q fallbacks stated as post-probe code changes outside the strategy enums.
- **Round 3 follow-up (2026-09-24, Claude).** Added `CLK s LAST CLEAR` (0.2 s after SLOT OPEN): without it nothing turned an entry's `LAST` off except another entry's win, so an entry whose area was the only HOT one stayed excluded for good after its first win, contradicting `clock_only_one_area_hot_alternates_with_quiet`.
- **Trace and replay (2026-09-25, Claude, user request U12).** New step 0T (trace builds with Mission Objective breadcrumbs and a `.trace.json` sidecar; `missionlog.rs` parser, replay and `replay` CLI), built in wave 1b before probe flight 1; flight 1 flies the traced probe. New cell T-t (04:30–05:00) tests the breadcrumbs (t1–t6 in §9). Step 9T adds the UI (Trace build checkbox, Replay panel, Map overlay) before step 10; step 11 reads its checks from the replay report. Scope note: the offline replay is not the excluded live log reader. §10 items 13–14 added.
- **Trace and replay review (2026-09-25).** One reviewer, 9 findings, all applied: T-t owned by 0b and excluded from `instrument` (no double breadcrumbs), `trace::breadcrumb` / `TraceMap::push` stubs; entity sources traced through `OnEvents`, edges walk follows OnEvents with a visited set; step 9's allow rule excludes `trace.rs` / `missionlog.rs`; `trace_off_changes_nothing` deferred and relaxed for 9T; T5 fires twice and no longer expects a removal line; `expected_s` and a slope + offset tick fit; T4 4 s apart, T-t to 05:30, F2 moved to 05:30; trace grid from `geo.rs` / `placement.rs` bounds; never-spawned counted per name.
