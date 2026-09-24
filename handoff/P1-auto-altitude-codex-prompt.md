# P1 auto-altitude: Codex delegation prompt (send verbatim)

Send with `codex_start`: cwd = this folder, `reasoning_effort: max`,
`max_idle_seconds: 1200`. The first attempt (job `20260924024246-e38e95ca`)
failed at spawn because `codex.exe` was not installed. It made no changes.

---

<task>
Implement "Auto altitude: 50% of service ceiling" for aircraft in Template Builder mode of this Rust/egui app (IL-2 Mission Utility). Read docs/src-guide.md and .cursorrules first; obey them (logic stays out of egui; ui.rs calls logic modules through public APIs).

Background (verified by the orchestrator):
- src/model_spec.rs `pub fn ceiling_m(script) -> f32` returns the service ceiling in metres (fallback 8000 for unknown/ground).
- src/template.rs `TemplateSeat` has `altitude: f32` and `start_type: i32`; `PlaneStart::stored_for_altitude(stored, altitude)` must be re-applied whenever altitude changes (altitude > 0 means airstart). `append_seat` (≈line 1340) makes a new seat inherit the last plane's altitude; `replace_seat_unit` (≈line 1395) swaps a model and keeps altitude.
- Every plane in assets/Models.Group has `YPos = 0.000`, so today a newly added plane defaults to a ground start (altitude 0).
- src/ui.rs: Template "Add unit" button ≈line 3322 calls `append_seat`; model swap ≈line 2222 calls `replace_seat_unit` then clamps altitude to the ceiling; per-seat Altitude slider ≈line 2500 (`egui::Slider` 0..=ceiling); `load_template` path ≈line 3870; app state struct has `tpl_*` fields (≈line 425) initialised ≈line 1534; `reset_template_builder` ≈line 3824.

Required behaviour (acceptance criteria):
1. Logic in src/model_spec.rs: `pub const AUTO_ALTITUDE_FRACTION: f32 = 0.5;` and `pub fn auto_altitude_m(script: &str) -> f32` = `(ceiling_m(script) * AUTO_ALTITUDE_FRACTION).round()`. Examples: mig15bis → 7500, f86a5 → 7620, b29 → 5334, unknown script → 4000.
2. Logic in src/template.rs: `pub fn apply_auto_altitude(seat: &mut TemplateSeat)` — no-op for non-air units; for air units sets `altitude = model_spec::auto_altitude_m(&seat.unit.script)` and re-derives `start_type` via `PlaneStart::stored_for_altitude`. Plus `pub fn apply_auto_altitude_all(seats: &mut [TemplateSeat])` applying it to every seat. Do NOT change the existing behaviour/signatures of `append_seat`, `replace_seat_unit`, `copy_seat_attributes`, `TemplateSeat::new`, or `load_template` (existing tests pin them).
3. UI state in src/ui.rs: new field `tpl_auto_altitude: bool`, default `true`, NOT reset by `reset_template_builder` (it is a user preference for the session).
4. UI checkbox in the Template "Add unit" row (next to "Add unit" / "Copy attributes to all"), label "Auto altitude (50% ceiling)", hover text explaining: new planes and model swaps start at half the aircraft's service ceiling; turning it on re-heights every plane; turn it off to add ground-start (runway/parked) planes. When the checkbox transitions false→true, call `apply_auto_altitude_all(&mut self.tpl_seats)`. Transition true→false changes nothing.
5. When `tpl_auto_altitude` is true: after "Add unit" appends an air seat, call `apply_auto_altitude` on the new seat; after a model swap on an air seat (the `change_unit` branch), call `apply_auto_altitude` on that seat (instead of only clamping). When false, existing behaviour is byte-for-byte unchanged.
6. Per-seat button next to the existing per-seat Altitude slider (air seats only), label "50% ceiling", hover text showing the computed value (e.g. "Set to 7500 m (half of 15000 m service ceiling)"); clicking calls `apply_auto_altitude` on that seat. Works regardless of the checkbox.
7. Loading a template file (`load_template` path) never rewrites loaded altitudes, even with auto on. Manual slider edits after auto are kept (auto is event-driven, not continuously enforced).
8. Unit tests (in the existing `#[cfg(test)]` modules): model_spec — auto_altitude_m values above; template — apply_auto_altitude on an air seat sets 50% and start_type == PlaneStart::Air.as_i32(); on a ground/vehicle seat is a no-op (altitude stays 0); apply_auto_altitude_all on a mixed seat list; a ground-start plane (altitude 0, StartType Running) becomes airborne with StartType Air. Build seats the way existing template tests do (look for existing helpers in template.rs tests, e.g. bundled_catalog / builtin_plane_catalog).
9. Docs: in USER_MANUAL.md "Per seat" bullet (≈line 72), replace "New planes copy the last plane’s height and start." with one or two plain-English sentences describing Auto altitude (default on, 50% of service ceiling for new planes and model swaps, the "50% ceiling" button, turn off for ground starts; with it off new planes copy the last plane’s height and start). In docs/src-guide.md add `auto_altitude_m` / `AUTO_ALTITUDE_FRACTION` to the model_spec.rs entry and `apply_auto_altitude(_all)` to the template.rs entry. Update the `//!` Public API header comments of model_spec.rs and template.rs likewise.
</task>

<files_in_scope>
src/model_spec.rs, src/template.rs, src/ui.rs, USER_MANUAL.md, docs/src-guide.md. Nothing else.
</files_in_scope>

<do_not_touch>
Cargo.toml, Cargo.lock, build.rs, assets/**, TemplateExamples/**, References/**, MapHelper/**, .gitattributes, .gitignore, any file under "HANDOFF" or "Claude*" names. Do not change Fighter Pack (flights.rs) altitude logic, serialization, the parser, or any generated .Group output for templates where the user did not use the new feature. No unrelated refactors, renames, reformatting (do not run cargo fmt over whole files), or dependency changes.
</do_not_touch>

<git_protocol>
1. If the primary clone (C:\Claude\IL2MissionUtility\Claude IL2Mission Utility) has a dirty tree (`git status --short` non-empty), STOP and report. Never discard or reset it.
2. You cannot write .git in the primary clone. `git clone --no-hardlinks "C:\Claude\IL2MissionUtility\Claude IL2Mission Utility" <a temp dir you can write>\auto-altitude` and work ONLY in that clone. `git checkout -b codex/auto-altitude`. There is no GitHub remote; leave origin as-is. Do NOT fetch, pull, or push.
3. Line endings: the repo has `.gitattributes` `* -text`; files must keep their existing line endings byte-for-byte (most are LF). Do not convert files.
4. Run `cargo test --offline` in the clone (no network; all crates are in the local cargo cache). Baseline is 371 passing tests. If tests fail, fix within scope or stop and report; do NOT commit failing work. Also run `cargo build --offline` and make sure there are no new warnings in the files you touched.
5. `git add` only in-scope paths, commit on codex/auto-altitude with a clear message. Never commit to main.
</git_protocol>

<completeness_contract>
Resolve the task fully. Check every call site that adds or swaps a Template seat in ui.rs so the toggle is honoured consistently.
</completeness_contract>

<verification_loop>
Before finalizing, re-read your diff against acceptance criteria 1–9 and confirm each; re-run cargo test --offline after the last edit.
</verification_loop>

<missing_context_gating>
Do not guess repo facts; read the code. If something in the spec conflicts with the code, choose the lowest-risk interpretation, keep going, and list it under residual risks.
</missing_context_gating>

<structured_output_contract>
Return: 1) absolute path of the temp clone, branch name, commit SHA(s); 2) summary of the change per acceptance criterion (1–9, one line each); 3) files touched with rough line counts; 4) exact `cargo test --offline` result line; 5) residual risks / spec conflicts.
</structured_output_contract>