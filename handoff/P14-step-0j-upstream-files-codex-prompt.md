<task>
Produce three mission group files with the upstream author's own code, unmodified, so that they can be flown in a test. This worktree is a checkout of the upstream repository at commit `a31ad0a` (branch `trial/upstream-strike-files`). It is a throwaway: nothing here is merged anywhere. The only change you make is one added, ignored test that writes the files.

Read first: `USER_MANUAL.md` (search for "Missions" and "Strike"), then in `src/template.rs`: `MissionPackage`, `apply_mission_package`, `strike_orders`, `cap_orders`, `STRIKE_ABORTS`, `CAP_ABORTS`, `TemplateOptions`, `TemplateSeat`, `FlightRole`, `generate_template`, `builtin_plane_catalog`, and the existing tests `strike_attacks_for_two_minutes_then_rtb` and the CAP test near it (they show how a mission package is applied to seats). Also `src/serialize.rs` (`serialize_group`) and how the app writes a `.Group` and its `.eng` text file when the user presses Generate in Template Builder (find the function the UI calls and use the same one, so the files are what the app would write).

What to write: one `#[ignore]` test (add it to the tests module of `src/template.rs`) named `write_upstream_mission_package_files`. It builds each group below exactly the way the Template Builder UI would after the user picks the aircraft and presses **+ Mission ▾**, with every option left at the value the UI leaves it at, and writes the files to `target/upstream-mission-files/`.

| File stem | Seats | Package | Notes |
|---|---|---|---|
| `Upstream_Strike_NATO_F80C_4ship` | four F-80C (`f80c10`), country 601: one lead and three wingmen that follow it | `MissionPackage::Strike` | set whatever the UI sets after a strike is applied (for example `zone_out_enabled = false` when `applied.disable_zone_out`) |
| `Upstream_Strike_DPRK_Il10_4ship` | four Il-10 (`il10`), the DPRK country the catalog gives them: one lead and three wingmen | `MissionPackage::Strike` | the same |
| `Upstream_CAP_NATO_F86A_4ship` | four F-86A-5 (`f86a5`), country 601: one lead and three wingmen | `MissionPackage::Cap` | whatever the UI sets after a CAP is applied |

For each, write `<stem>.Group` and, if the app writes one for a generated group, `<stem>.eng`, with the same bytes and line endings the app writes.

Then report, for each file, read from the written file (not from the code):
1. Every MCU in the group's logic: `Name`, block type, `Index`, `Targets`, `Objects`, and `Time` for timers. In particular the bring-up chain (what Mission Begin leads to, the check zones, the timer that activates or spawns the units), each order's nodes in chain order (Formation, each waypoint, the AttackArea, the report, the home waypoint, Mission Complete), and the clean-up chain (`MISSION END` onward, with the `Objects` of `Deactivate Units` and `Trigger Delete`).
2. Every `OnEvent` and `OnReport` on each plane entity: type number, the app's name for it, and the `Name` of the node it targets.
3. Per plane: `Name`, `Index`, entity index, `Country`, `StartType`, `YPos`, `Fuel`, `AiRTBDecision`, `DeleteAfterDeath`, `DeleteAfterLand`, `PayloadId`, `NumberInFormation`.
4. Positions: where the group's origin is, and the offsets of the initial point, the target (the AttackArea) and the home waypoint from it, in metres north and east.
5. Which single node a test harness should pulse to start the group the way a player entering the zone would (the node the Zone In check zone leads to), and what else Zone In pulses that such a harness would skip.

Acceptance:
- `cargo test --offline write_upstream_mission_package_files -- --ignored` passes and writes the files. If the offline build fails for a missing crate, stop and report the crate name; do not fetch anything.
- `git status --short` shows only `src/template.rs` modified (the added test); the written files are under `target/` and untracked.
- No other test is changed and no non-test code is changed.

Files in scope: `src/template.rs` (the one added test only).
</task>

<git_protocol>
Same-host mode. Edit files in the working tree only. No git command that writes. `git status` and `git diff` are allowed.
</git_protocol>

<verification_loop>
Run the one ignored test. You need not run the whole suite, cargo clippy or cargo fmt. Do not run the game, the editor or MissionResaver.
</verification_loop>

<structured_output_contract>
Return: (1) summary, three lines at most; (2) the test's result and the files written with their sizes; (3) the five numbered reports above, one block per file, as compact tables; (4) anything about how the UI builds these groups that the test could not reproduce exactly, stated plainly.
</structured_output_contract>
