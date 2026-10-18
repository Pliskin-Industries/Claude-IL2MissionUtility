# Delegation prompt for flight 2I (the two Pairs built-ins as shipped under decisions 0020 A and 0022 A2)

One job, in the worktree `distracted-taussig-b9259d` on `claude/p14-flight-2i` (cut from `claude/pairs-loose-0020` at `b0d640e`, plus this prompt and the design note, committed). Sent with `codex_start`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1200. Model: the config default.

---

<task>
Build probe flight 2I: a no-pilot probe that flies the F-51 ArmedRecon and the Yak airfield raid built-ins exactly as the app now ships and places them (decisions 0020 A and 0022 A2: Pairs at Loose, a flight flying a Cover order at Safe), with no change after placement, over empty targets. Probe code only. Product code, placement, the templates and every flown file stay as they are.

Read first, in this order:
1. `handoff/sessions/2026-10-18-flight-2i.md` (the design; this prompt makes it exact).
2. `docs/decisions/0020-…`, `0022-f51-armed-recon-at-loose.md`; `handoff/P14-probe-results.md`, sections "Flight 2G" and "Flight 2H".
3. `CLAUDE.md`, `.cursorrules`.
4. Code: `src/airtask/probe.rs`. Flight 2G is the model for the variant (`EmptyTargetVariant::BuiltInChange` with `PlacedProbeChange::AsShipped`, `builtin_change_schedule`); flight 2H for a recent flight's tables, tests, schedule and the S3 F-51 (`TWO_H_*`, `flight2h_…` tests); flight 2F for target reuse within one mission (`TWO_F_TARGET_A`, `TWO_F_TARGET_B`, the reuse test). Also `src/template.rs` (`set_air_formation_density`, the constants), `place.rs`, `library.rs`, `testkit.rs`.

What to build, in `src/airtask/probe.rs`:

A. **`Flight::TwoI`**, stem `P14_Probe_2I`, not frozen and not yet regenerating flown files, with a run sheet, cues, `generate_probe_2i()`, and an ignored writer `write_p14_probe_2i` that writes the files and `target/p14/P14_Probe_2I_schedule.md`. Add it to `Flight::ALL` and every exhaustive match. Every earlier flight's output stays exactly as it is now (48 flown files byte-identical).

B. **Six cells**, each a built-in through `build_empty_target_probe` with `EmptyTargetVariant::BuiltInChange { change: PlacedProbeChange::AsShipped, … }` (own stand-in area, COLD; no HOT switch, no enemy, no spawn):

| Cell | Prefix | Built-in | Shape, attack s | Transit s (named constants) | Target km (x, z) | START s |
|---|---|---|---|---|---|---|
| IF1 | SN01 | `1950_US_F51_ArmedRecon_2plus2.Group` | S3, 1200 | 318.5 (`TWO_H_F51_TRANSIT_S`) | (230, −90) | 60 |
| IF2 | SN02 | the same | S3, 1200 | 318.5 | (140, −90) | 90 |
| IY1 | SN03 | `1950_DPRK_Yak_AirfieldRaid_2plus2.Group` | S4, 300 | 264.1 (`TWO_F_YAK_TRANSIT_S`) | A (230, 0) | 120 |
| IY2 | SN04 | the same | S4, 300 | 264.1 | B (140, 0) | 150 |
| IY3 | SN05 | the same | S4, 300 | 264.1 | A | 800 |
| IY4 | SN06 | the same | S4, 300 | 264.1 | B | 830 |

Reuse `TWO_F_TARGET_A`, `TWO_F_TARGET_B` and 2H's (230, −90) target; name the (140, −90) target (flight 2E's EC target). 24 aircraft written, at most 16 alive at once.

- **The placed formations are the shipped rule**, and tests check them: in each F-51 cell the Red lead's Formation command (`Objects` = the Red lead entity) has `FormationType` 19, `FormationDensity` 2, and the Blue lead's (the lead that flies the Cover order) 19, 1; in each Yak cell both commands 19, 2. Each cell's sortie equals the same file placed and wrapped by the current placement with nothing changed after it.
- **`empty_target_source`'s checks**: S3 / 1200 and S4 / 300 on the parsed source (placed copies inspect as S7; do not require S3/S4 there). If anything refuses, stop and report the exact message.
- **Paths and reuse**, tested with the placed copies' own lead Plane and `WP 1` positions: all four targets over land; different-target paths at least 45 km apart (C1–C2 about 46.7 km for the F-51, A–B 48.3 km for the Yak, the two lines 90 km apart); A and B reused only after the earlier cell's predicted `DELETE DELAY` (IY1 745.2 before IY3 800; IY2 775.2 before IY4 830); at most 16 aircraft alive at any predicted moment.

C. **Timing.** The host leaves at **28:00** (1,680 s). Walker test (Restart, scripted `WP 1` arrivals at the predicted times): each copy's `ON STATION` starts once at its predicted time and its `DONE HUB` and `DELETE DELAY` fire at the predicted times (IF1 1579.0 / 1639.6, IF2 1609.0 / 1669.6, IY1 684.6 / 745.2, IY2 714.6 / 775.2, IY3 1364.6 / 1425.2, IY4 1394.6 / 1455.2; follow the walker where it differs and report it); every attack has ended before 1,680 s. Safety close at **2,100 s** with one `MCU_Delete` per aircraft entity and a final cue.

D. **Cues** (subtitles):
- 0 s: "Flight 2I: spectate, press on the sky, then LCtrl+R to record. Leave the server at 28:00"
- at each START: "Flight 2I: IF1 starts (F-51 as shipped)", "Flight 2I: IF2 starts (F-51 as shipped)", "Flight 2I: IY1 starts (Yak as shipped)", "Flight 2I: IY2 starts (Yak as shipped)", "Flight 2I: IY3 starts (Yak as shipped)", "Flight 2I: IY4 starts (Yak as shipped)"
- 1,650 s: "Flight 2I: leave the server at 28:00"
- 2,100 s: "Flight 2I closed. Every aircraft has a Delete. End the mission normally"

E. **Breadcrumbs** as flights 2F to 2H (`observe_sortie_probe`). F. **No spawn** (`spawns()` empty).

G. **Schedule**: per cell the built-in, prefix, target, START, predicted `WP 1` (say which measured transit; note that 2H's Loose F-51 Red pair arrived 4–9 s late), `ON STATION`, `DONE HUB`, `DELETE DELAY`, target and spawn (X, Z); aircraft alive per interval; the Formation commands as written (`append_probe_formations`). Reuse the 2G/2H schedule helpers; earlier flights' schedules unchanged.

H. **Tests** (`flight2i_…`), each stating its own literals: the table; the placed formations as in B; only-as-shipped (equal to plain placement); sources equal the parsed files with S3/1200 and S4/300; side logic and icons (NATO for IF, DPRK for IY); paths, land, reuse and the 16-aircraft maximum; the walker run; the cues in the shared tables; the existing tests unchanged and passing (the flown-file test still compares 48 files).

Acceptance:
- Baseline first: `cargo build --offline` / `cargo clippy --offline` warnings (expected 69 / 251), `cargo test --offline` (expected 700 / 0 / 27). After: no new warnings; 0 failed; passed = baseline + new tests; ignored = baseline + 1.
- `cargo test --offline write_p14_probe_2i -- --ignored` writes the Group, traced Group, `.eng`, `.trace.json`, `.spawns.json` and schedule, and touches nothing under `handoff/`.
- Every file under `handoff/p14-probes/` unchanged.
- `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_2I_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_2I_check --title "P14 probe 2I check" --out target/p14/missions/P14_Probe_2I_check --no-resave` exits 0 (use the installed Python 3 if `python` is not on PATH), every `Index` unique, every reference resolved. Never without `--no-resave`. Nothing copied into the game folder.

Files in scope: `src/airtask/probe.rs` only. Do not touch anything else. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `claude/p14-flight-2i` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test, the ignored writer, the byte comparison and the mission builder check, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now: const tables with named fields, shared helpers, named constants, no flag arguments, failures that name the cell. No refactor outside the change; extract the smallest helper that keeps every earlier flight byte-identical.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings, tests, byte comparison, mission builder). 3. Files changed. 4. Tests added or changed. 5. The run sheet per cell (prefix, target, START, predicted WP 1, ON STATION, DONE HUB, DELETE DELAY, target and spawn X/Z), most aircraft alive. 6. Each cell's Formation nodes as written (index, type, density, Objects, who targets them). 7. Choices and residual risks.
</structured_output_contract>
