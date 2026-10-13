<task>
Change the service ceilings of the aircraft in `src/model_spec.rs` to documented figures. The user ruled on 2026-10-13: all twelve aircraft take the best documented figure, with each source noted in the code.

Read first: `docs/historical-reference/aircraft-service-ceilings.md` (the research: section 1 is the summary table, section 3 has one entry per aircraft), `CLAUDE.md`, `.cursorrules`, then `src/model_spec.rs` and every use of `ceiling_m` in `src/` (`template.rs`, `flights.rs`, `ui.rs` and tests).

New values, in metres (the document's "Recommended" column; the B-29 is unchanged):

| Script | Aircraft | Now | New | Source to cite in the comment |
|---|---|---|---|---|
| `b29`, `simpleb29` | B-29 | 10668 | 10668 | USAF Characteristics Summary, 9 March 1949: combat ceiling, 35,000 ft at 100,000 lb |
| `c47b` | C-47B | 6096 | 8047 | Francillon 1988 via secondary: 26,400 ft at 26,000 lb, two-speed blower working |
| `f51d` | F-51D | 11278 | 12680 | Wright Field Memo Report TSCEP5E-1908, 15 June 1945: 41,600 ft at 9,760 lb |
| `f80c10` | F-80C-10 | 13716 | 13030 | Knaack 1978 (official USAF history), p. 11: 42,750 ft |
| `f84e` | F-84E | 12192 | 13716 | Knaack 1978, p. 50: 45,000 ft |
| `f86a5` | F-86A-5 | 15240 | 14630 | secondary (Wagner 1963 via Baugher): 48,000 ft |
| `il10` | Il-10 | 6950 | 7250 | secondary: practical ceiling at 6,300 kg |
| `la11` | La-11 | 10150 | 10250 | secondary (Shavrov): practical ceiling at 3,730 kg |
| `li2t` | Li-2T | 6800 | 5600 | secondary: passenger Li-2, M-62IR |
| `mig15bis` | MiG-15bis | 15000 | 15500 | secondary (Gordon 2001; Arsenyev and Krylov 1999): practical ceiling |
| `tu2` | Tu-2 | 10500 | 9500 | secondary (Shavrov): Tu-2S, ASh-82FN, 10,360 kg |
| `yak9p` | Yak-9P | 10600 | 10500 | secondary: 1947 all-metal Yak-9P, 3,550 kg |

What to write.
1. The thirteen rows in `SPECS`. Above the aircraft rows, a short comment that names `docs/historical-reference/aircraft-service-ceilings.md` as the source of the ceilings and says the game's own handbook figures are listed there for comparison. On each aircraft row, a trailing or preceding comment with the source in a few words and the original figure in its original unit where it was feet. Mark the seven secondary-sourced rows as such. Do not put any of this in the `notes` field: that field is shown in the UI and must stay as it is.
2. Do not change cruise speeds, labels, classes or any other row.
3. Tests. Update the assertions that pin the old figures (`model_spec.rs` near lines 407 to 413; `template.rs` near line 6585, "IL-10 capped at its ceiling") to the new figures. Add one test in `model_spec.rs` that checks all thirteen ceilings against literals, so that a later edit to one row is seen. Write the failing assertions first, then the data.
4. Find out and report whether any ceiling reaches a generated file. `ceiling_m` is believed to be used only by the UI's altitude slider, by `copy_seat_attributes` and by `apply_plane_start` (`AIR_START_ALTITUDE_M.min(ceiling)`). List every use you find in `src/`, including `flights.rs`, and say for each whether it can change what Generate writes for a given set of inputs.

Hard rule: no baseline or fixture file may be re-recorded or edited (`fighter.Group`, `base_map.Group`, `exclusive.Group`, anything under `src/testdata/`, `TemplateExamples/`, `handoff/p14-probes/`). If a baseline test fails because of the new ceilings, stop and report which test and why; do not touch the baseline.

Acceptance:
- `cargo test --offline`: at least 546 passed (545 plus the new test), 0 failed, 11 ignored. `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- `cargo test --offline write_p14_probe_ -- --ignored` passes and every `target/p14/P14_Probe_*` file is byte-for-byte identical to before (copy them to a baseline folder under `target/` before you edit).
- `git diff --stat` shows only `src/model_spec.rs` and `src/template.rs` (and another `src/*.rs` only if a test there pinned an old figure; name it).

Files in scope: `src/model_spec.rs`; tests in other `src/*.rs` files only where they pin an old ceiling figure.
Do not touch: `docs/`, `handoff/`, `USER_MANUAL.md`, `STATE.md`, `TemplateExamples/`, `src/testdata/`, `tools/`.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/model-spec-ceilings` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers and the comparison before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<structured_output_contract>
Return: (1) summary, four lines at most; (2) command results and the byte comparison; (3) the thirteen rows as written; (4) the list of every `ceiling_m` use with "can change Generate output: yes / no" and why; (5) anything in `USER_MANUAL.md` or the docs that states an old ceiling figure and will now be stale (report only, do not edit).
</structured_output_contract>
