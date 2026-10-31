# 2026-10-30 — P14 step 11 item 4: the standalone file imported in the editor (risk l): run sheet

**Status: done 2026-10-30 by Claude. The editor re-indexes the import cleanly, and one clock slot was flown from the saved mission. No fallback is needed** (risk l, `handoff/P14-air-tasking.md` §9). Branch `claude/goofy-lederberg-944d08`, cut from `main` at `20d3d18`. The user approved the plan ("Yes, build it", 2026-10-30) and confirmed the screen was free.

**Why this check.** The Air Tasking tab's Generate File writes a standalone `.Group`: one `Air Tasking` group with indexes from 1 (`airtask::build_air_group`). Any mission it is imported into also has indexes from 1. The design relied on the Mission Editor re-indexing the import, and nobody had checked that (risk l). The fallback was to offer only the base-map path.

## Files

- **Code (test code only; Generate output unchanged):**
  - `src/ui_tests.rs`:
    - `a3_inputs` is split out of `build_a3` (A3's byte check still passes);
    - `build_standalone_l` takes A3's inputs, with "Include in base map" off. Map Generate writes the base map, and the Air Tasking tab's Generate File (Ctrl G) writes the standalone file;
    - the ignored writer `write_p14_standalone_l`;
    - the check `standalone_l_build_matches_the_frozen_files_and_a3`. It checks:
      - the build against the frozen files, byte for byte;
      - that the base map has no Air Tasking group;
      - that the standalone file is indexed 1 to 478, and every one of those indexes is also used by the base map;
      - that, under renumbering, the standalone file is **A3's flown Air Tasking group**. A3's 78 breadcrumbs sit at the map root and are moved into its group first.
  - `src/airtask/export_acceptance_flights.rs`: `standalone_l_editor_import_renumbers_and_keeps_every_link` reads the editor-saved mission (below).
  - `src/airtask/testkit.rs`:
    - `assert_same_up_to_indexes`: is tree b tree a with its indexes renumbered? It checks the same blocks, properties and links, and compares numbers as numbers. `Renumbering` sets whether language ids are compared by their text, and whether Group names may be cut as the editor cuts them;
    - `assert_indexes_unique_and_links_resolve` and `read_locale_file`, split out of `assert_frozen_files_resolve`, which now uses them.
  - `src/harvest.rs`: a `#[cfg(test)]` `parse_mission_body` that calls the existing `.Mission` parser.
  - `src/airtask/mod.rs`: `testkit` is `pub(crate)` (still `#[cfg(test)]`), so `ui_tests` can use it.
- **Generate:** `cargo test --offline write_p14_standalone_l -- --ignored` writes these to `target/p14/`:
  - `P14_Standalone_L_Base.Group` (478,508 bytes), six language files (392 bytes each) and `.trace.json` (41 bytes, empty: the base map has no breadcrumbs);
  - `P14_Standalone_L_Air.Group` (189,741 bytes, 478 indexes) and `.trace.json` (35,262 bytes, 78 breadcrumbs). There are no language files: every `LCName` and `LCDesc` in the group is 0.
- **Frozen:** `handoff/p14-probes/standalone-l/`: the files above, plus the editor's saved `P14_Standalone_L_20261030.Mission` (671,342 bytes) and its `.eng` (5,436 bytes).
- **Base mission** (no resaver):
  ```
  python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe handoff/p14-probes/standalone-l/P14_Standalone_L_Base.Group --no-resave --name P14_Standalone_L_Base_20261030 --title "P14 standalone L base (Map air start, no Air Tasking)" --out "C:\Machine Intelligence\P14 Flight L\mission"
  ```
  This writes `P14_Standalone_L_Base_20261030.Mission` (480,381 bytes) and `.eng`.

## Steps, as done

1. **Mission Editor:** File › Open `C:\Machine Intelligence\P14 Flight L\mission\P14_Standalone_L_Base_20261030.Mission` (about 1.5 min to load).
2. **File › Import From File…** `P14_Standalone_L_Air.Group`, from the same folder. The import showed no error box and no question. The mission tree showed "Korea Base Map 1951-04-22 (3)" and **"Air Tasking (2251)"**: the editor gave the group a new index (1 in the file), and the base map's group too (1,126 in the file). So the renumbering happens before the save, on load or on import. Link lines ran from the breadcrumbs at (5,000; 5,000) to the logic at A, where the file put them.
3. **File › Save As** `P14_Standalone_L_20261030`. The editor wrote `.Mission`, six language files, `.msnbin` and an empty `.list`. The mission kept the base map's title, "P14 standalone L base (Map air start, no Air Tasking)", which is how it shows in the server list.
4. All nine files were copied into `C:\Program Files\IL2Series\game\data\Multiplayer\Dogfight\` (approved for this check).
5. **Flight:** hosted from the client, alone in the rotation, Tacview on.
   - DPRK side, "P14 A3 DPRK air start", Yak-9P.
   - LCtrl+R, then throttle to 65 % (about 470–500 km/h indicated), heading 088.
   - Left orbit (LAlt+Z held in 90 s chunks) from abeam A.
   - Left the server after `SN04 Zone IN`.
   - The mouse did not move in flight.

## What the saved mission shows

From `standalone_l_editor_import_renumbers_and_keeps_every_link` and the frozen files:

| Check | Result |
|---|---|
| Indexes | 1,608 blocks: base 1,130 + standalone 478. **None is used twice.** The editor renumbered both parts: the base map 3 to 2,249, Air Tasking 2,251 to 2,995 (group index 2,251, from 1) |
| Blocks and order | Each part is its source file block for block, in order. No block was added or lost |
| Links | Every `Targets`, `Objects`, `LinkTrId`, `MisObjID`, `TarId` and `CmdId` points at the renumbered block. Nothing points outside its own part, and nothing dangles |
| Positions and properties | Unchanged. The editor writes `90` for `90.000` |
| Group names | **The six sortie copies' names are cut to 31 characters** ("SN01 US F-80C HVAR Strike on Ar"). Every other name is unchanged |
| Language ids | The base map's texts keep the builder's +100 offset. The 78 breadcrumb objectives had 0. Each now has its own `LCName` / `LCDesc` with the text `<no name>` |
| Trace sidecar | Each of the 78 entries finds exactly one saved objective at its position, the renumbered breadcrumb. So the standalone file's own `.trace.json` works on the imported mission |

## Flight results (flown 2026-10-30)

Session `2026-10-30_18-25-28`, logs `src/testdata/missionlog/real_53/` (35 files), Tacview `handoff/tacview/P14_Standalone_L.zip.acmi`. The raw `.acmi` is in `C:\Machine Intelligence\P14 Flight L\tracks\`. One run.

| | Flown |
|---|---|
| Load | No loader error. **The host loaded the editor's `.msnbin`** (`MFile:Multiplayer/Dogfight/P14_Standalone_L_20261030.msnbin`). The DPRK spawn list offered "P14 A3 DPRK air start" |
| Spawn | S = 86.78 s, Yak-9P on **`COUNTRY:501`** at (187,500; 258,037), 1,998 m |
| A goes HOT | `THN01 INNER` **265.88** (S + 179.1; A3: S + 180.9). `ZN01 CHANCE` 265.98, `ZN01 FIRE` and **`SN01 START` 266.02**: +0.10 and +0.14, as in A3. Four F-80C (602) activated at (170,352; 302,667), A3's spawn point |
| A stays HOT | `THN01 INNER` at 265.88, 425.88, 585.88, 745.88 and 905.88, **160.00 s apart** |
| **The clock slot** | `CLK N SLOT OPEN` **600.10**; `EN02 GO` and **`SN04 START` 600.90**; `BUD N COUNT` 601.14. These are A3's times to the tick. Four F-51D (602) activated at (161,619; 312,743) |
| Transit | `SN01 Zone IN` 432.68 (s + 166.66; A3: + 166.48); `SN04 Zone IN` **914.80** (s + 313.90; A3: + 314.04) |
| Leave | Leave Server at 979.08. The player logged as destroyed (`AType:3 AID:-1`), as in A3. The results screen was closed with `CloseMainWindow` |
| Replay | `replay --group handoff/p14-probes/standalone-l/P14_Standalone_L_Air.Group --trace ….trace.json --logs src/testdata/missionlog/real_53`: **no dead links**; every breadcrumb matched by position. Never fired: the sorties' own Mission Begin breadcrumbs (A3's never logged either), `LOSSES`, SN02, SN03, SN05 and SN06, and the timers still running at the leave |

**Not predicted, and not caused by the import:** at 563.88 SN01's fourth F-80C ("Red 14") was destroyed with no attacker (`AType:3 AID:-1`) at 896 m, about 11 km from A. The pilot bailed out at 570.14, and the aircraft hit the ground at 578.14. **The Tacview shows a mid-air collision with Red 13**, its neighbour on the left of the Wedge (Safe density): closest 5.2 m at 563.4 s, both attitudes jumping at 563.2 s. It came in the steady descending right turn the flight flew over A, which had no ground target. Details are in `docs/il2-loader-and-log-facts.md` (2026-10-30, standalone-file check L). Losses of HVAR F-80s are the subject of another session.

## Choices, and why

- **A3's inputs, not a new plan.** Same seed, same objective, same air start, same two entries. This made it possible to prove the standalone file equals A3's flown group under renumbering, so the pilot steps and timings were already known. The user chose this over a clock-only file.
- **One slot.** The orbit keeps A HOT, so slot 1 at 600.1 s launches the clock entry (no quiet slots, one jitter step). The flight ended once that copy's `Zone IN` logged. Nothing else in A3 depends on how the group got into the mission.
- **The editor's own files were flown,** the `.msnbin` included, as a user would put them on a server. The game chose the `.msnbin` over the `.Mission`.

## Checks (2026-10-30, branch `claude/goofy-lederberg-944d08`)

- `cargo test --offline`: **1048 passed, 0 failed, 37 ignored**. That is A3's 1,046 / 36 plus the two new checks and the ignored writer. The frozen-file checks (A0 to A3, flights 2 to 2L) still pass after the testkit split.
- Warnings: **81 build, 306 clippy** (unchanged). None is in the new code.
- No product code changed: Generate writes the same files (`existing_outputs_match_baseline` passes).
- `STATE.md` is left to the coordinating session.
- **Code review** (two axes, Standards and Spec). Fixed:
  - `parse_mission_body` now calls `harvest`'s own `.Mission` parser instead of a weaker copy;
  - the unique-index and link check, and the language-file read, are one testkit helper each; `assert_frozen_files_resolve` uses them;
  - `assert_same_up_to_indexes` takes a `Renumbering` struct, not two loose arguments;
  - clearer names (`map_indexes`, `compare_blocks`, `plain_properties`, `expected_name`);
  - the assertion about shared indexes says what fails;
  - a source language id missing from its table now fails, where it could pass silently before;
  - a cut name is compared by characters, not bytes;
  - file-read panics name the file;
  - the saved mission's top level must be exactly the two groups;
  - the breadcrumb counts come from the trace files, not a hard-coded 78;
  - risk row l's fallback column now says none is needed;
  - the facts doc says when the editor renumbers (before the save), and that the icon-title note was read from the screen only.
  - The `a3_inputs` split is its own commit, apart from the new check.
  - Kept:
    - no decision record: no decision was made, only a risk answered;
    - no test reads `real_53`, as for A3's `real_51`. The run sheet and the facts doc quote it.

## Follow-ups (not done here)

- Front-line icons with `LCName = 0` show the mission title on the briefing map (`docs/il2-loader-and-log-facts.md`, this date). That is a Map / `build_probe_mission.py` matter, not Air Tasking.
- `handoff/P14-probe-results.md` has no section for A3 or this check yet; the coordinating session may add both.
- Next free log folder: `src/testdata/missionlog/real_54`.
