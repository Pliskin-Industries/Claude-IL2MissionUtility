<task>
Build P14 step 1b (the dry-run walker) in this repository (Rust 2024 binary crate, egui, nom). Steps 1a and 1 are already on this branch.

Read these first, in this order:
1. `handoff/P14-air-tasking.md`: §8 "General rules for every step"; §8 "Step 1b. Dry-run walker" (the whole section: the rule table, inputs and output, and the acceptance list); §3.0 "Conventions"; §9 risks b, c, e, f, j, o, p, q (each walker switch names the risk it stands in for).
2. `CLAUDE.md` and `.cursorrules`.
3. `src/airtask/testkit.rs` (the existing `assert_links_resolve` stays), `src/ast.rs` (`Il2Entity`), `src/bombers.rs` (`extract_exclusive_plans`, `inspect_plan`), `src/pack.rs`, and the comment above `silence_clone_starts` in `src/recon.rs`.

What to build, all in `src/airtask/testkit.rs` and all test-only (`#[cfg(test)]` or used only from tests):
- An event-queue simulator over an `Il2Entity` tree that follows the rule table of step 1b exactly, row by row, except the `MCU_TR_ComplexTrigger` row (that is step 12; do not build it).
- Every configurable behaviour in the table is a field of one config struct with the table's default: timer re-trigger `Restart | Ignore | Twice`; deactivate a running timer `RunsOn | Cancels`; `DropcountMeaning::{ResetOn1, ResetOn0}`; `SetValRefires::{Yes, No}`; `SameTickCounts::{Each, One}`; same-tick delivery `DepthFirst | Batched`; `ReverseTargets` on/off; check zone `Once | Repeat`.
- Inputs: scripted tracks (coalition, list of (t, x, z) samples, 1 s apart), forced pulses (node name, t), kill events on plane entities, "arrive at WP" events, and a fixed RNG seed. Output: a log of (t, node name) pulses and state snapshots (active flags, counter counts). The same inputs and seed give the same output.
- Time is a fixed-point or integer tick (the design uses 0.05 s steps); do not compare floats for event order.
- A static rule check `walker_every_deactivated_out_has_a_rearm_or_is_single_use` usable on any tree (steps 3–7 call it on every generated pack), as a public-in-crate function plus its test.

Acceptance. Each line is a named test or a command result:
- `walker_self_test_exclusive_activation_grants_one`, defined exactly as in step 1b (unmodified `TemplateExamples/Exclusive_Activation_6plan.Group`, one NATO and one DPRK track through every plan's suggested trigger zones, grants for exactly one plan).
- `walker_self_test_fighter_pack_nodegates_grants_one`: a Fighter Pack built by `pack.rs` grants one per gate.
- `walker_random_is_seeded_and_rolled_each_time`: 40 pulses into Random 50 give 12–28 hits, the same count for the same seed.
- `walker_checkzone_track`: a Closer 1 zone fires when a track enters after the pulse and not before; a Closer 0 zone fires when the last track leaves.
- `walker_every_deactivated_out_has_a_rearm_or_is_single_use`.
- The two existing `assert_links_resolve_*` tests still pass.
- `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline`: at least 487 passed (482 now plus the five named tests), 0 failed, 5 ignored.

Files in scope: `src/airtask/testkit.rs` only.

Do not touch: every other file. In particular `src/airtask/mod.rs` and the other `src/airtask/*.rs`, `src/ui.rs`, `src/bombers.rs`, `src/pack.rs`, `src/template.rs`, `src/recon.rs`, `src/ast.rs`, `src/testdata/`, `TemplateExamples/`, `handoff/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`. No change to any Generate output. If the walker needs a change outside `testkit.rs` (for example a private item in `bombers.rs` or `pack.rs`), stop and report instead of making it.
</task>

<git_protocol>
Same-host mode. The orchestrator has already checked out the branch `codex/p14-1b`. Edit files in the working tree only. Do not run any git command that writes (no add, commit, branch, checkout, stash, reset, restore). `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Do not stop after a partial change. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Before you report, run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline`, and read the warning counts and the test totals from their output. If a count is above its limit or a test fails, fix it inside `testkit.rs` and run all three again. Do not silence a warning in existing code.
</verification_loop>

<action_safety>
Stay inside `src/airtask/testkit.rs`. No refactor, rename, reformat or cleanup of existing code. Do not run `cargo fmt` on the whole crate; format only the code you add.
</action_safety>

<missing_context_gating>
Do not guess repository facts or game behaviour. Read the code. Where the rule table and the code disagree, follow the code for names and signatures of existing items, follow the plan for walker semantics, and list the difference in the report.
</missing_context_gating>

<structured_output_contract>
Return, in this order:
1. Summary of the change (5 lines at most).
2. The three command results: build warnings, clippy warnings, tests passed / failed / ignored.
3. The walker's public-in-crate API (types and function signatures).
4. For each rule-table row: how it is implemented, in one line, and its config switch if any.
5. For each self-test: the stimulus used and the grants or firings seen.
6. Each choice you made where the plan gave no exact shape, differences between the plan and the code, and residual risks.
</structured_output_contract>
