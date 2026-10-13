# IL-2 loader and mission-log facts

Facts about the game that are not obvious from this repo's code. Each one cost at least one flight attempt to learn (2026-09-28 to 2026-10-07). Add to this list when a new one is found.

## Loading a mission or group

- **The loader rejects the whole file, with an empty error box,** if a quoted value holds `;` `=` `{` `}` `'` or any non-ASCII character. This applies to the editor and to `MissionResaver.exe`. Guard test: `probe_strings_are_loader_safe`.
- **Blocks must match the shipped shapes:** small LC ids, objective `LCName` and `LCDesc`, the shipped key order, no Group `XPos`. Guard test: `probe_blocks_match_shipped_shapes`.
- **`MissionResaver.exe` reproduces editor load errors without the editor.** It is in `C:\Program Files\IL2Series\game\bin\resaver`. Use it to test any generated file.
- **Build probes with `tools/p14/build_probe_mission.py`.** It resaves and checks the result, and deletes old outputs first, because the resaver keeps stale language files. `--spawns <file>` adds the air-start spawn points a probe lists in its `.spawns.json`; `--no-resave` stops after writing the `.Mission`, for a sandbox that cannot run the resaver.
- **`K14 AFB_mp.Group` has no `Planes` list.** Pass `--spawn-planes` to the builder to add spawn planes.

## Language tables

- **A client or server can show a stale language table** from an earlier build with the same mission name. Give every build a new mission file name, or clear the old `.eng` and other language files on both server and client.
- **A new subtitle replaces the one on screen.** Long cues get cut off by the next one.

## The text mission log

- Turn it on with `mission_text_log` under `[KEY = system]` in `startup.cfg`. Logs are written to `data\logs\txt\`, a new file about every 33 s on a dedicated server and about every 11 s when the game client hosts.
- **Log clock:** tick = 3 + 50 per second of mission time.
- **`OBJID` is not the MCU index.** Match objectives by position, which is exact.
- A Mission Objective with Success 1 **ends a dogfight round** about 8 s later. Success 0 logs `AType:8` and the round continues.
- Spawned-and-deleted vehicles write no log line.
- One MCU firing twice in the same tick logs one line.

## Mission logic

- **AI aircraft trigger check zones.** The coalition filter still works. See `docs/decisions/0008-ai-aircraft-trigger-check-zones.md`.
- An empty "Closer 0" zone does not fire on Activate plus a pulse.
- A pulse into an inactive relay is dropped, not replayed when the relay is activated.
- Deactivating a running timer cancels it. Re-triggering a running timer restarts it.
- Inputs to an inactive counter are not counted; the count survives Deactivate and Activate.

- **An aircraft that dies raises OnEvent 13, then 0, then 2; it does not raise 4.** Seen for a gun kill and for a Damage command (flight 1G, 2026-10-12). 13 and 0 come within 0.1 s of the kill, 2 some seconds later when the wreck hits the ground, and not always.
- **`MCU_Delete` raises no event** on the aircraft it removes; neither does the template's Deactivate then Delete chain.
- **A Delete may leave a live aircraft behind** when another aircraft in its Objects list is already dead. Seen once (flight 1G, G1a); the cause is not confirmed.
- A plane's `Country` can be changed on a copied flight, and the log reports the new country.

Full evidence for the logic rows: `handoff/P14-probe-results.md`.

## The Complex Trigger

Read from a block saved by the editor on 2026-10-12 (game `261002`): `TemplateExamples/Reference/ComplexTrigger_reference.Group`, with a screenshot of its properties next to it. Copy the block shape from that file; do not write it from memory.

- The block is `MCU_TR_ComplexTrigger`. The editor writes `Enabled = 1;` **twice**.
- Filters are plain lines after the 21 `EventsFilter…` keys: `Country = 601;`, `ObjectScript = "luascripts\worldobjects\planes\f51d.txt";`, `ObjectName = "…";`. The sample has one of each; the shape for two or more is not known. A trigger with only the `Country` line, and one with no filter line at all, were written by the probe generator and **load in the resaver** (2026-10-12); neither shape was saved from the editor.
- Outputs are an `OnEvents` block, not `Targets`. Event types seen: Object Entered = 58, Object Left = 60. The other 19 type numbers are not known.
- One key is misspelled in the game and must be written that way: `EventsFilterDropedBombs`.
- The editor offers five countries by name: Neutral Country, USSR, PRC, DPRK and USA (601). A mission built on the dogfight test Options block also carries 602 and 603 on the NATO side (log header `CNTRS:…601:2,602:2,603:2`), but the editor shows no name for them, so a trigger cannot pick them from its list.

## Other blocks read from the editor or shipped files

- **`MCU_CMD_Damage`** (saved from the editor 2026-10-12, `TemplateExamples/Reference/CommandDamage_reference.Group`): after the common keys it has `Damage = 2;` and `Type = 1;` for Damage Type "Damage" with Damage Level "Complete". The levels are Minor, Major and Complete; the other radio choice is Repair. The numbers for those were not read.
- **`MCU_CMD_AttackTarget`** (`TemplateExamples/CorrectedFlightfromTemplate.Group`): `AttackGroup = 1;` `Priority = 1;`, Objects = the attacker's entity, Targets = the target's entity.
- **A dogfight spawn point** is an `Airfield` block with the `fakefield` model, an `MCU_TR_Entity` and a `Planes` list. An entry with `StartType = 0` is an air start at `Altitude`. Seen at K14 (`YOri = 0`): the plane spawned heading north at about 460 km/h indicated. The game's dogfight test mission has such entries for 12 aircraft on both sides, the F-80C and MiG-15bis among them.
- No shipped file and no other template in this repo contains a Damage command or a Complex Trigger; the two reference files are the only examples.

## Found on 2026-10-13 (flight 1H, game 1.005)

- **An `MCU_Delete` must list entities.** With `Plane` block indexes in `Objects` it loads, the resaver accepts it, and in the game it removes nothing. Every file saved from the editor lists entities.
- **`MCU_CMD_Damage` acts on the whole formation** of the aircraft it names. To kill one aircraft, make it a flight of its own.
- **A deactivated aircraft still exists:** it is in the end-of-mission list, and a Damage command does not reach it.
- **The end-of-mission list (`AType:12` after `AType:7`) holds live aircraft only.** A dead aircraft is absent whether or not a Delete named it.
- **`AType:28`** is written once per aircraft when its flight is activated.
- **Since game update 1.005 (`261013`), `MissionResaver.exe` crashes at once** (exit `0xC0000005`, no output) on every mission tried, the game's own `_test_dogfight_IL-3.Mission` included. Build with `--no-resave`: the game hosts a plain `.Mission` with its `.eng` beside it, and the server list shows the title from the `.eng`. A mission resaved under 1.004 also still loads.
- **The launcher's update download stalls on the `Cdn` source.** Settings (the gear), source `S3`, then pause and resume: it runs at 8 to 27 MB/s for a few minutes and stalls again; pause and resume once more. Do not press pause once it says "Unpacking".
- **The game's main menu goes to a hangar fly-by after about a minute idle** and hides the menu. One click on empty space brings it back.
- **A Delete may list a dead aircraft** (flight 1J): the live aircraft in the same list are removed, in either order.
- **Leaving a client-hosted server:** on the results screen, press Leave Server and confirm within about 15 s. On 2026-10-13 a leave confirmed as the next load began left the game on "0 % please wait" for minutes.
- **A Delete acts on the aircraft it names** (flight 1K): naming a formation's lead leaves the wingman, and the other way round.
- **A damaged AI aircraft can stop taking commands** (flights 1G, 1K): neither a Delete nor a level 2 Damage command reached it. Likely tied to `AiRTBDecision = 1`; see the flight 1K results before relying on a Delete for an aircraft that may have been hit.
- **`MCU_CMD_Damage` levels:** `Damage = 2` kills; 1 and 0 do not kill an F-80C. After level 1 both aircraft drop their stores within about a minute (`AType:26`) and fly home separately.
- **The log session name is when the server started loading,** not when the mission clock started: the gap was between 10 and 90 s on 2026-10-13. Read the clock from the log's last `T:` before leaving.

