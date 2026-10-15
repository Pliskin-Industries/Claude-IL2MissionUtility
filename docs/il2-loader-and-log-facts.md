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
- **Write AI aircraft with `AiRTBDecision = 0` if the mission must stay in control of them** (flight 1L). With 1, a damaged pair splits: one aircraft stops answering Delete, Deactivate and Damage, flies to the nearest friendly airfield and lands; the game removes it a few minutes after landing.
- **Entity events seen** (flight 1L, all types 0 to 13 listened for): a kill raises 12 and 13, then 0, 1 and 3; fuel starvation raises 7, then 12 and 3, then 13 at the bail-out and 2 at the crash; level 1 damage raises 3, then 9 when the stores are dropped; a landing raises 5. **Type 7 also fired three seconds after activation on aircraft with full tanks.** Type 4 has never fired.
- **Log records:** `AType:18` bail-out; `AType:31` an aircraft meets the ground (crash or landing); `AType:27` with a crash; `AType:26` stores dropped.
- **A Damage command aimed at a unit's entity reaches a spawned copy.** A deactivated wave of a spawner stays in the end-of-mission list under the same name as the live wave.
- **Probe groups made by `generate_template` keep the app's key order in their `Plane` blocks** (`Index`, `LinkTrId`, `Name`), not the shipped order the other probe blocks use, because flight 1L tests what the app writes. `probe_blocks_match_shipped_shapes` exempts them by index.
- **Event 7 ("bingo fuel") can fire about 3 s after activation, with full tanks** (flights 1L, 1M and 1P): on upstream's F-80C strike and F-86A CAP, and on flight 1L's single F-80Cs. It is **not** a property of the type: in flight 1P twelve single aircraft as Template Builder writes them (F-80C and F-86A-5 among them) and a Fighter Pack F-80C flight did not raise it. The cause is not known. Do not send a flight home on event 7.
- **A timer that sends `Force Complete` and a waypoint together must fire only once per flight** (flight 1M). When it fired a second time while the flight was already on that waypoint, the flight was left with no orders. **A `Counter` with `Counter = 1` and `Dropcount = 0` between the abort events and that timer fixes it** (flight 1P, the Il-10 strike).

## Found on 2026-10-14 (flight 1P, game 1.005)

- **An aircraft with no bombs raises event 9 (bingo bombs) about 2.7 s after activation; one with no forward guns raises event 8 (bingo main guns)** (B-29, C-47B and Li-2T). Do not hook either as "out of ammunition" on an aircraft that may carry none.
- **The Fighter Pack death count works as decision 0016 built it:** each lost aircraft is counted once, when its 60 s `Delay Delete n` runs out. Pointed straight at `DeathCount`, the death event counts each kill twice.
- **An attack order with no target in its area runs out after its `Time` and reports `OnAreaAttacked`** (flight 1P, H4: no weapon released, report at 120 s).
- **`AType:25` is written when an aircraft releases a weapon** (`OBJID` the aircraft, `TID` the new object). Read from the log; one record per aircraft per release seen.
- **Fighter Pack aircraft are separate formations** (`FlightRole::Independent` in `flights.rs`), so a Damage command on the lead would reach only the lead (flight 1H: a Damage command acts on one formation). Read from the code; flight 1P used one Damage command per aircraft and did not test it.
- **With `AiRTBDecision = 1`, all aircraft of a damaged formation but one stop answering commands** (flight 1M: three of a 4-ship).


## Found on 2026-10-15 (flight 2C, game 1.005)

- **The game rewrites `data\startup.cfg` when it exits.** An edit made while the game runs is lost. Close the game normally (a window-close request), edit, start it, and check the file again.
- **Tacview from a client-hosted server:** `tacviewrecord = 1` under `[KEY = track_record]`, the server's "TacView record" box ticked, press Spectate, then **LCtrl+R** (action `record` in `data\input\current.map`). A red icon shows top right. `data\Tracks\` gets a `.acmi` and a `.trk` (about 750 MB for 15 minutes) named for the load time; both close when the mission ends. The `.acmi` held all 48 AI aircraft, not only those near the camera. Its `ReferenceTime` is the game time when recording began; mission seconds = that minus the mission's `GTime`, plus the `#` time. ACMI object ids are the log's `OBJID`s in hexadecimal.
- **The spectator camera did not answer F2, Shift+F2 or LAlt+F2** on this PC; it stayed at its first view. Recording did not need it.
- **Server overload:** with 48 AI aircraft flying, the test-bed PC showed "Multiplayer server overload" all mission and mission time ran at about 0.4× wall time. Every MCU still fired at its mission time to the tick. Plan wall time from the breadcrumbs, not from the mission clock.
- **The results screen gives about 10 s before the next load.** A Leave Server pressed after that starts another run of the same mission; leave from its briefing screen.
- **`FormationDensity` values (game 1.005, from the editor's own labels):** `bin\editor\IL2Editor.exe` lists "Formation Density:" with the choices Dense, Safe, Loose. By list order 0 = Dense, 1 = Safe, 2 = Loose (inference, not checked in the editor window). The spacing in metres is not documented; flight 2D measures it.
