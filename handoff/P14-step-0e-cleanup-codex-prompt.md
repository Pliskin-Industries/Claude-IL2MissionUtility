<task>
Clean up the P14 step 0e code in this repository. This is a quality pass with **no change in behaviour**: every generated file must stay byte-for-byte the same. The findings come from a standards review of your step 0e commits.

Read first: `CLAUDE.md`, `.cursorrules`, `docs/il2-loader-and-log-facts.md`, then all of `tools/p14/build_probe_mission.py` and `src/airtask/probe.rs`.

Before you edit, make the baseline you will compare against, in a folder outside the files in scope (for example `target/p14-cleanup-baseline/`):
1. `cargo test --offline write_p14_probe_ -- --ignored`, then copy every `target/p14/P14_Probe_*` file (`.Group`, `.eng`, `.trace.json`, `.spawns.json`; five flights, plain and traced).
2. Build each mission with `--no-resave` and copy the `.Mission` and `.eng`:
   - flight 0: `--probe target/p14/P14_Probe_0_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10,f86a5`
   - flight 1A and 1G: `--probe target/p14/P14_Probe_<F>_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10`
   - flight 1N and 1D: `--probe target/p14/P14_Probe_<F>_traced.Group --spawns target/p14/P14_Probe_<F>_traced.spawns.json`
   each with `--game "C:\Program Files\IL2Series\game" --name BASE_<F> --title "base <F>" --out target/p14-cleanup-baseline/missions/<F>`.

Findings to fix.

A. `tools/p14/build_probe_mission.py`
1. **Fail loudly with a clear message.** `shipped_blocks`, `block_value`, `set_block_value` and the bare `next(...)` calls raise a raw `StopIteration` / `IndexError` traceback if the game's test mission changes shape (an unclosed block, a missing key, an airfield with no `Planes`). Each must exit through `SystemExit` with a message that names the file, the block kind and the key, like the messages `spawn_airfields` already gives for bad JSON.
2. **One airfield lookup, one parsing style.** `spawn_airfields` and the older `planes_block` both find the test mission's airfield for a country and pick `Plane` entries by script and start type, one by line and indent, the other by regex. Put the shared steps in helpers that both use (read the test mission once; find the airfield block for a country; list its `Plane` entries; pick the entry for a script and a `StartType`). Keep `planes_block`'s output text exactly as it is.
3. **Small duplications.** The indent / prefix computation repeated in `block_value` and `set_block_value`; the `"loader-unsafe strings:\n  "` exit written twice.
4. **`spawn_airfields` does five jobs** (read, validate, look up, edit, renumber). Split it so that validating one entry and building one airfield are functions of their own.
5. Named constants for `StartType` 0 and 1 (air start, ground start) instead of bare `"0"` / `"1"`.

B. `src/airtask/probe.rs`
1. **Named constants** for the country codes and event types now written as bare numbers in builders and tests: 601 (USA, NATO players), 602 (NATO AI), 501 (USSR, DPRK-side players), 503 (DPRK, DPRK-side AI); Complex Trigger events 58 (Object Entered) and 60 (Object Left); entity events 0, 2, 4, 5, 13 where they are used by name (`G_EVENT_TYPES` may keep its literal list, with a comment naming each). Use one representation, not `i32` in builders and `&str` in assertions, unless a comparison against a parsed property needs the string, in which case derive it from the constant.
2. **No flag arguments.** `build_n(&mut p, 2000.0, true)` and `build_y(&mut p, true)` cannot be read at the call site. Replace the bools with a small enum or a small struct with named fields (for example a `Side` enum for the Y cell that carries prefix, player country, AI country and coalition; and a named variant or struct for T-n against T-n2).
3. **`ai_country`** reads like a getter and silently changes only nodes whose `Country` is `"601"`. Rename it to say that it sets the country of every plane in a copied flight, document that the built-in template's planes are USA, and assert that it changed the number of planes the flight has.
4. In `build_y`, the loop tuple `(suffix, country, …)` shadows the outer `country`. Give the two different names.
5. **Parse each reference file once.** Taking one block out of a reference `.Group` is written three times (`reference_mcu`, in `probe_blocks_match_shipped_shapes`, inline in `probe_cells_match_the_table`, the last re-parsing on each loop pass). One helper, used by all three.
6. The wingman-letter expression `if i == 0 { 'a' } else { 'b' }` appears three times: one helper.
7. `spawn_json` builds JSON by hand. Keep it (no new dependency, and `trace.rs` is out of scope), but make it safe: assert that every `name` and `script` is ASCII without `"`, `\` or control characters before writing, and say so in a comment.
8. Add a comment where the Complex Trigger's `ObjectScript`, `ObjectName` and (for the unfiltered trigger) `Country` lines are removed: these two shapes were not saved from the editor; they load in MissionResaver and worked in flights 1N and 1D on 2026-10-12 (`handoff/P14-probe-results.md`).

Do not change what any builder writes: names, indexes, order, times, positions, locale ids and texts all stay.

Acceptance:
- After the change, the writers' files and the five `--no-resave` mission builds are **byte-for-byte identical** to the baseline (compare all of them and report the comparison, file by file or as a count of identical files with zero differing).
- `cargo test --offline`: 544 passed, 0 failed, 10 ignored. `cargo build --offline`: at most 69 warnings. `cargo clippy --offline`: at most 251 warnings.
- Each failure path of finding A1 gives a clear message: show it with a throwaway copy of the test mission that has (a) an airfield with no `Planes` block, (b) an unclosed block, (c) a missing `Country` key; report the three messages. Do not add the throwaway files to the repo.

Files in scope: `tools/p14/build_probe_mission.py`, `src/airtask/probe.rs`.
Do not touch: every other file, including `src/trace.rs`, `src/locale.rs`, `handoff/`, `docs/`, `TemplateExamples/`, `STATE.md`, and anything under `C:\Program Files\IL2Series`.
</task>

<git_protocol>
Same-host mode. The orchestrator has checked out the branch `codex/p14-0e-cleanup` in this worktree. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Fix every finding. If a finding cannot be fixed without changing output, leave that part as it is and say exactly why in the report.
</completeness_contract>

<verification_loop>
Run the three cargo commands, the ignored writers and the five `--no-resave` builds, and the comparisons, before you report. Do not run `cargo fmt` on the whole crate, MissionResaver, the game or the editor.
</verification_loop>

<action_safety>
Stay inside the two files. No renames or reformatting of code this task does not name.
</action_safety>

<structured_output_contract>
Return: (1) one line per finding (A1 to A5, B1 to B8): fixed, or not fixed and why; (2) command results and the byte-for-byte comparison; (3) the three failure messages for A1; (4) anything else you noticed in the two files that you did not change.
</structured_output_contract>
