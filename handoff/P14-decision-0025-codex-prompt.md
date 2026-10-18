# Delegation prompt for decision 0025 (remove TriggerReactivate; walker trigger is Silent; freeze flight 2J)

One job, in the worktree `distracted-taussig-b9259d` on `codex/p14-4` (head after this prompt is committed). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Carry out decision 0025 (accepted by the user 2026-10-18): the threshold cell has one presence check, the pulsed check zone (`ZonePulse`); the walker models a Complex Trigger as flight 2J flew it; flight 2J is frozen.

Read first, in this order:
1. `docs/decisions/0025-remove-trigger-reactivate.md`, `0024-…`, `0023-…`.
2. `handoff/P14-probe-results.md`, section "Flight 2J" (what the game did), and `docs/il2-loader-and-log-facts.md`, section "Found on 2026-10-18 (flight 2J…)".
3. `handoff/sessions/2026-10-18-p14-step4.md` ("Fallback", "Built and reviewed", the 2J sections).
4. `CLAUDE.md`, `.cursorrules`.
5. Code: `src/airtask/mod.rs` (`PresenceCheckImpl`, `Strategies`), `src/airtask/threshold.rs`, `src/airtask/testkit.rs` (`TriggerActivation`, `check_trigger`, `switch`), `src/airtask/probe.rs` (`Flight::TwoJ`, `is_frozen`, `regenerates_flown_files`, `TWO_J_*`, `ThresholdProbeCell`, `add_threshold_probe_side`, `threshold_probe_walk`, the `flight2j_…` tests, and how frozen flights 2E and 2G are tested against their frozen files, e.g. `flown_formation_probe`).

What to change:

A. **`src/airtask/mod.rs`:** delete `PresenceCheckImpl` and `Strategies::presence`. Nothing else in `mod.rs` changes. (The plan's rule that a type change is the orchestrator's own commit is waived for this job, because the type cannot be removed in a commit that still compiles without its users; say in your report exactly what you changed here.)

B. **`src/airtask/threshold.rs`:** `build_side` builds only the `ZonePulse` cell (19 MCUs: the 17 shared roles plus `INNER` and `INNER PULSE`; no `SEEN`), with every behaviour, name, edge, timing, validation and output byte it has today for `ZonePulse`. Remove the variant branching rather than leaving a one-armed switch. Update the module and function doc comments (the cell is: country-filtered Complex Trigger for the entry, pulsed coalition check zone at r_in for the presence check; cite decisions 0023 and 0024 and flight 2J). Tests: keep every named acceptance test that still applies, now on the single cell: `threshold_structure` (the ZonePulse edges and keys; fold in what `threshold_zone_pulse_structure` checked and remove the duplicate), `threshold_countries`, `threshold_hold_does_not_switch_hot_off`, one count test renamed `threshold_has_19_mcus_one_trigger_one_check_zone`, `threshold_timing_follows_radius`, `threshold_refuses_more_than_one_player`, `threshold_refuses_counted`, `threshold_refuses_invalid_input`, `threshold_links_resolve_and_rearm_rule`, `threshold_empty_areas_and_one_based_registry_keys`, and the walker scenarios `threshold_one_zone_loiter_passes` (PASS at entry + 60.1 s with the walker's sampling; state the window), `threshold_one_zone_straight_pass_fails`, `threshold_one_zone_edge_orbit_fails`, `threshold_ai_country_does_not_count`, `threshold_hot_and_cold_are_never_both_off`, `threshold_pass_and_timeout_in_one_tick_is_a_pass`, `threshold_timeout_then_pass_in_one_tick_is_one_outcome` (rebuild the race on INNER: the late pass is a stand-in pulse into `INNER`'s output path or an `INNER` fire in the same tick, CLOSE first; same expectations as now: DepthFirst clean FAIL, Batched PASS only, never both, never a pair fully off; if the check zone cannot be made to fire after CLOSE in one tick in the walker, say so and keep the strongest honest version), `threshold_stays_hot_through_a_successful_recheck`, `threshold_pass_leave_pass_again`, `threshold_second_plane_after_close_does_not_pass`, `threshold_two_planes_pass_once_per_check`, and `threshold_zone_pulse_ai_passes_a_recheck` (rename to `threshold_enemy_ai_passes_a_recheck`; it documents 0024's accepted weakness). Remove the tests that only exist for `TriggerReactivate` or the old walker hypothesis: `threshold_has_18_mcus_one_trigger_no_check_zone`, `threshold_silent_trigger_never_passes`, `threshold_batched_two_planes_exposes_latch_hypothesis`, `threshold_presence_before_init_off_exposes_startup_gap`; for the startup case add `threshold_plane_inside_at_load_waits_for_a_dwell` (a player-country plane inside r_in from tick 0: ARM, then the first check one dwell later; HOT and COLD never both off).

C. **`src/airtask/testkit.rs`:** remove `TriggerActivation` and `WalkerConfig::trigger_activation`. The Complex Trigger model is the flown behaviour: switching a trigger on (and mission load) takes every track already inside as known and reports nothing for it; afterwards each crossing reports Entered / Left once per track; switching off forgets nothing it would report later (a track that crossed while it was off is not reported on switch-on: flight 2J KB) — choose the simplest model consistent with KA, KB and KC and state it in the doc comment, citing flight 2J, and note that one switch-on in 2J (THD01 at 212.00 s) reported an entry the trigger had not yet reported, which the model does not reproduce. Update the walker's own trigger tests to the single model.

D. **`src/airtask/probe.rs`:** freeze flight 2J: `Flight::TwoJ.is_frozen()` is true (add a comment: decision 0025 removed the `TriggerReactivate` cell that KD and KE flew), `regenerates_flown_files()` false for it, and the flown-file test's expected count goes back from 56 to 52 with its comment updated. Every file under `handoff/p14-probes/` stays byte-identical (2J's frozen files are the flown record). The 2J generator must still build: KD, KE and KF all use the remaining builder; `ThresholdProbeCell::threshold` keeps only the prefix. Rework the `flight2j_…` tests so that each either pins the generator as it now is (ZonePulse cells; walker predictions with the single trigger model, which now match 2J's flown KA, KB, KC and THD02 outcomes), or pins the frozen flown files (as 2E and 2G do), for example that the frozen traced Group holds `THD01 SEEN` and three `MCU_TR_ComplexTrigger` cells plus KA to KC's triggers. Do not keep any `TriggerReactivate` wiring in `src/`. Earlier flights' output and tests stay exactly as they are.

Acceptance:
- Baseline first: `cargo build --offline` / `cargo clippy --offline` warnings (expected 68 / 250), `cargo test --offline` (expected 749 passed / 0 failed / 29 ignored). After: no new warnings; 0 failed; state the new passed / ignored counts and which tests were removed, renamed and added.
- `grep -rn "TriggerReactivate\|PresenceCheckImpl\|TriggerActivation\|SEEN" src/` finds nothing except, if you keep it, the frozen-file assertion that the flown 2J file holds `THD01 SEEN`.
- `flown_probes_regenerate_byte_for_byte_except_flight2` passes with 52 files; every file under `handoff/p14-probes/` unchanged (`git status` shows none of them).
- For the same inputs, `threshold::build_side` output is byte-identical to what `PresenceCheckImpl::ZonePulse` produced before this change (show how you checked, e.g. a test that compares against a serialized copy taken before the edit, or a one-off comparison you then remove).

Files in scope: `src/airtask/mod.rs` (A only), `src/airtask/threshold.rs`, `src/airtask/testkit.rs`, `src/airtask/probe.rs`. Do not touch anything else. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. The branch `codex/p14-4` is checked out in this worktree and its working tree is clean when you start. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken a test to make it pass.
</completeness_contract>

<verification_loop>
Record the baselines before editing. Then run build, clippy, test, the grep and the byte comparison, and read the counts. Fix and re-run until every line holds. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Write in the style the code has now. No refactor outside the change.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Where this prompt and the code disagree, follow the code and report each difference.
</missing_context_gating>

<structured_output_contract>
Return: 1. Summary (five lines). 2. Command results (baseline and final warnings and tests, grep, byte comparison, the build_side byte-identity check). 3. Files changed. 4. Tests removed, renamed, added. 5. The walker's trigger model as documented. 6. Choices and residual risks.
</structured_output_contract>
