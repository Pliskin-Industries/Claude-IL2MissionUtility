# 2026-10-28: a used sortie copy goes quiet (decision 0038)

Branch `claude/p14-shell-quiet-a24002`, from `94d2046` (the A1 branch). Not committed, not pushed. `STATE.md` is left to the session that owns it; the facts for it are here.

## What changed

- `src/airtask/shell.rs` (`wrap_sortie`): `DONE HUB OFF` also targets `HARD STOP`, `ON STATION`, `EXTEND`, `LOSSES`, `Zone IN`, `Zone Out`, `PULSE OUT` and every timer that targets an `MCU_CMD_AttackArea`. That last rule is wider than `SortieShape.attack_timers` (each lead's last attack timer), which misses AirAlert's timer 37. The code review found it. The Watchdog variant puts its two timers and two OUT gates on the same list and loses `WATCHDOG OFF`.
- Tests, `shell.rs`:
  - New: `shell_used_copy_is_quiet_after_done_hub`. It replays A1's two orders of events in `builtin(5)` (the Il-10 copy A1 flew) and `builtin(0)` (AirAlert), with a late `WP 1` at +0.15 s and +60.6 s. Before the fix it failed with `LOSSES`, `AttackArea 1`/`5`, `Zone IN`, `PULSE OUT`, `EXTEND` and `HARD STOP` firing after `DONE HUB`. With only `attack_timers` on the list, it failed on AirAlert's timer 37.
  - New: `shell_done_hub_off_stops_every_attack_timer` (AirAlert's 37 and 39).
  - New: `flown_configs()`, the walker with the flown J1 `Cancels`.
  - Updated: `shell_done_hub_inputs` (the new target list, with each template's attack timers written out) and `shell_wp1_pulses_zone_in` (the only Deactivate on Zone IN is `DONE HUB OFF`).
  - Updated: `shell_zone_out_watchdog_structure`. It checks the Watchdog's four targets, that `WATCHDOG OFF` is gone, the 47-MCU count (43 + 4), and that neither timer fires after `DONE HUB` under `Cancels`.
  - Updated: the enemy tests (no late `COLD`) and the HOT tests (no late `HARD STOP`). These four now use `flown_configs()`.
- Tests, `probe.rs`:
  - 2F, 2H and 2I are frozen. The byte check drops from 52 to 40 files. The new `shell_frozen_files_parse_and_links_resolve` parses them and checks their links.
  - The 2F, 2G and 2I walker tests now expect no late `HARD STOP`: PD/PS/PL, GC/G3 and IY1/IY2 used to log one after their `DONE HUB`.
  - `flight2e_and_2g_change_only_the_flown_pairs_density_bytes` applies the 0038 target list to the frozen copy first, so its byte check still isolates the density change.
- Tests, `export_acceptance_flights.rs`: A0 and A1 are frozen. `acceptance_flights_regenerate_byte_for_byte` is replaced by `frozen_acceptance_files_parse_and_links_resolve`, which checks:
  - parse, links, unique indexes, property links, locale ids, trace entries and edges, through `testkit::assert_frozen_files_resolve`, which the probe check now shares (a link value of 0 means unlinked, as on A0's map objects);
  - each flown `DONE HUB OFF` still closes only its hub;
  - A1's spawn list.
- Records: `docs/decisions/0038-used-sortie-copy-goes-quiet.md`, its index row, and plan §3.2 and the step 3 test list in `handoff/P14-air-tasking.md`.

## Generate output

Regenerating A1 with the new code (`write_p14_acceptance_a1`, into `target/p14`) and diffing it against the frozen file shows:
- Only the 18 `DONE HUB OFF` blocks change: 1 → 10 targets in `SD01` to `SD06` (Il-10, two flights) and `SN01` to `SN03` (AirAlert), and 1 → 9 in `SN04` to `SN12`.
- 2,978 MCUs before and after.
- `.eng` and `.spawns.json` are identical.
- The trace sidecar's entries are identical; only its `edges` list grows.

## Test run (2026-10-28)

- `cargo test --offline`: 982 passed, 0 failed, 32 ignored (after the code-review fixes).
- `cargo build`: 81 warnings. `cargo clippy`: 306 warnings. Both are the same as `main`.

## Still open

- **What fires `WP 1` after `DONE HUB` is not known.** It is either `Force Complete` at `DONE HUB` + 0.15 s or the Delete at + 60.6 s; `WP 1` has no breadcrumb. With 0038 either one is harmless.
- **The next acceptance flight checks 0038 in the game:** after a copy's `DONE HUB`, nothing from that copy may log except `DELETE DELAY`.
- **For `STATE.md`:**
  - decision numbers 0001 to 0038 are taken, so the next free one is 0039;
  - the flown-probe byte count is now 40;
  - A0 and A1 are frozen and no longer byte-checked.
