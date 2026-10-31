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
- Deactivating a running timer stops it. **It is paused, not cancelled: activating it again resumes the countdown with the time it had left** (flights A2 and 2K, 2026-10-29, below). Re-triggering a running timer restarts it, a resumed one too.
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
- **What `FormationDensity` does (flight 2D, AI F-80C 4-ships):** nearest wingman about 100 m at 0, about 140 m at 1, about 500 m at 2 (Wedge); Pairs at 0 about 50–80 m. At 0 wingmen came within 4 m and collided; at 1 and 2 never under 27 m.
- **This PC's server load:** 16 AI aircraft flying ran at 1.0× wall time with no overload message; 48 ran at 0.4× with it.
- **Use `FormationDensity = 1` for AI aircraft.** The app now writes it everywhere (decision 0019); a group made by hand in the editor still defaults as the editor does. *Since 2026-10-18 (decisions 0020 and 0022, branch `claude/pairs-loose-0020`): Pairs fly at 2 (Loose), except a flight flying a Cover order, which stays at 1; see the 2026-10-17 and 2026-10-18 sections.* Bug drafts for the game's developers: `handoff/il2-bug-report-drafts.md`.

## Found on 2026-10-16 (flight 2E, game 1.005)

- **Safe density by formation type** (flight 2E, built-ins at density 1, three runs): Wedge never came under 20 m within a flight (F-51D closest 54 m, Il-10 20 m). **Pairs did**: F-51D pairs closest 17 m, Yak-9P pairs 3.3 m with collisions in two of three runs, each a wingman crossing its lead in a turn reversal. Pairs at Safe holds the wingman only 65–120 m away (Pairs at Dense, flight 2D: 67–84 m). Decision 0020 (Proposed).
- **`FormationType` numbering (2026-10-18, upstream v0.7.0 merge):** two lists are in use. The six built-ins and flights 2D to 2I use the Great Battles ids from `TemplateExamples/FormationTypes.Group`: **19 flies as Pairs and 20 as a Wedge on game 1.005**, with different spacing (Loose: Pairs about 200 m, Wedge about 1,200 m). Upstream's Template Builder now writes the Korea editor's 1-based list (V-Form 1, Column - Pairs 4, Column - Flights wedge 5, Heavy - Wedge 8; 19 is "Reserved - 15" there), and says 0 crashes the editor. Ids 4, 5 and 8 have not been flown by us; the app treats 4 as Pairs for density (decisions 0020 and 0022).
- **The log has no collision record.** A collision shows as `AType:3 AID:-1` (and `AType:18` bail-outs), sometimes minutes later for a damaged aircraft. Tacview shows it: at the closest point both aircraft's attitudes jump 80–90° within one 0.4 s step.
- **Tacview distances:** subtract latitudes and longitudes first, then scale the longitude difference by cos(latitude). Scaling each longitude by its own cos(latitude) is wrong by about 1.4 m east per metre north at 127° E.
- **This PC's server load, again:** 20 AI aircraft ran at about 0.85× wall time with no overload message during the flight. The message appears when the host leaves, stamped with the in-game time of the leave; it is not evidence of overload in flight.
- **The chat (and any overload message) is hidden while spectating.** Open the menu (Esc) to see it; Esc again returns to the spectator view without side effects.
- **Repeat runs without the Create page:** let the results screen time out; the server reloads the same mission into a new log session. Spectate and press LCtrl+R again on each run (a new `.acmi` per load).

## Found on 2026-10-17 (flight 2F, game 1.005)

- **Wingman collisions are not server load.** With at most 8 AI aircraft (mission time 1.00× wall time) the Yak raid's Pairs at Safe still collided, at the same `WP 1` + 12–13 s reversal as with 20. Every Tacview attitude jump (60° or more in 0.4 s) in flights 2E and 2F starts with another aircraft within 20 m: the jumps are contacts. Of Dense, Safe and Loose in Pairs and in a two-ship Wedge, only Loose kept every pass above 20 m. Decision 0020 (Proposed).
- **Server speed from the logs:** each `missionReport` file's last `T:` against its modification time gives mission seconds per wall second (`tools/p14/flight2f_spacing.py`); copy logs with `cp -p` to keep the times.
- **When every aircraft has been deleted, the log has no end-of-mission (`AType:4`) list**, so no ammunition check is possible.
- **Leaving after the last run can loop:** a Leave Server from the results screen that comes a few seconds late starts another load; leaving from the next briefing reached the results screen and loaded again. A window-close request (`(Get-Process IL2Series).CloseMainWindow()`) closed the game cleanly.
- **Keyboard focus for computer use:** `AppActivate` sometimes leaves the keys with Claude's own window. A left click on the sky in the spectator view gives the game focus without side effects.

## Found on 2026-10-17 (flight 2G, game 1.005)

- **AI skill does not stop the formation crossover.** Yak-9P wingmen at `AILevel` 3 (Veteran) and 4 (Ace) held the same Pairs-at-Safe slot as at 2 (Normal), crossed their lead at the same `WP 1` + 12–13 s and collided (Ace in all three runs).
- **An AI lead's AttackArea entry starts with a left turn at the waypoint, wherever the area is.** With the area centre 3 km straight ahead of `WP 1`, the lead still banked left at `WP 1`, turned about 40° over 14 s, then reversed right toward the area; the wingman crossed at + 23 s instead of + 12 s.
- **A Formation command sent in flight is obeyed without slowing the lead:** a copy of the lead's own command (`WaitForWingmen = 1`) set to Loose and fired by `WP 1` moved the wingman out to the Loose slot within about 10 s; the lead's speed did not change.
- **Tacview reuses a destroyed aircraft's object id for its wreck fragments** (`CAirplaneFragment_<id>`); scripts that key tracks by id must drop frames after the name changes (`tools/p14/flight2g_crossover.py`).
- **The game's `.acmi` in `data/Tracks` is plain text**; the repository keeps tracks zipped as `<name>.zip.acmi`.
- **Hosting from the client, mission rotation:** clicking a mission only selects it; scroll the settings page to reach Remove (rotation) and Add to rotation (available list).
- **Keys in spectator view:** a plain key press sometimes does not reach the game; holding Escape for 0.3 s opens the menu reliably. Click the sky first, then send the key as a separate action.

## Found on 2026-10-18 (flight 2H, game 1.005)

- **A pair flying `MCU_CMD_Cover` weaves in transit.** The F-51 ArmedRecon's Blue pair, covering the Red pair 1,500 m below, flies 50–60° banked S-turns with a reversal every 10–20 s while it descends to the Red pair's height. At Pairs Safe its wingman stays on its side; at Pairs Loose it swings 150–400 m across the lead's track at each reversal (one collision in three runs).
- **A Loose formation arrives later:** F-51 pairs at Loose reached `WP 1` 4–9 s after the same raid at Safe (318.5 s transit).
- **Computer use, in spectator view:** a slow press on the sky (button down, 0.5 s, up), then 3 s before the key, gave the game keyboard focus where a quick click did not. `open_application` on the running game starts a second copy, which only shows an "already running" error; dismiss it with OK. A taskbar preview (from a remote-desktop window) can take focus from the game; only the user could dismiss it.
- **Pairs at Loose removes the entry-reversal collision, not the steady-turn crossing** (flight 2I, the built-ins as shipped under decisions 0020 and 0022): the Yak raid lost nothing in twelve raids, but on station its wingmen still crossed their leads at 9.7–10.8 m; an F-51 covering pair at Safe crossed at 8.1 m and lost an aircraft.

## Found on 2026-10-18 (flight 2J, game 1.005)

- **A Complex Trigger switched on (MCU_Activate) does not report a plane that is already inside.** Three switch-ons with a 601 pair inside a 601 trigger gave nothing (a check zone at the same place fired). A threshold cell that switched its trigger on again as a presence check missed its own loitering pair at the re-check. A switch-on can report an entry the trigger had not reported yet: 0.02 s after one switch-on it reported, most likely the wingman whose entry came just after the trigger was switched off. Do not use a switched-on trigger as a presence check; pulse a check zone (decision 0024).
- **A switched-off Complex Trigger reports nothing**, and it does not report later what crossed it while it was off (KB). Its country filter holds when it is switched on (KC).
- **An aircraft activated inside a switched-off trigger is not reported as entering when the trigger is switched on** (KA).
- A Yak-9P pair with its `Country` changed to 601 or 602 flies a 2 km waypoint octagon at 3050 m and 500 km/h and stays inside a 6 km circle: every control zone and every ZonePulse check found it inside.

## Found on 2026-10-28 (flight A1, game 1.005)

- **A player who spawns inside a switched-on Complex Trigger is not reported as entering.** A ground start at K14, at the centre of a 6 km country-601 trigger that had been on since Mission Begin, fired nothing in 98 s or later. Flying in across the ring fired it. So a player at an airfield inside an Air Tasking area is not seen until they leave the ring and come back (or land there after flying in, not yet flown).
- **A check zone pulsed by a 120 s timer runs every 120.00 s**, not 120 + 0.1 s.
- **When the last plane a "Zone Out" check zone watches dies, it fires** (3.06 s after the kill in flight A1), as if the plane had left.
- **On this PC, the client that hosts cannot spawn twice in one session.** After Finish Mission, or after being shot down, the briefing shows "Waiting for mission results…", which never clears, and Start stays disabled. Leaving to the results screen and letting the server reload the mission starts a new session, where the first spawn works. Plan piloted tests with one spawn per session.
- **Il-10 rear gunners shoot down a fighter that orbits over their target at their height** (F-80C at 500 m over K14).
- **Computer use, game display:** the game sometimes stops repainting for screen capture (and drops clicks) for tens of seconds; a mouse-wheel scroll over the page or map makes it repaint. On the briefing map, the second button on the left toolbar is a protractor, not "centre on me": clicking it again turns it off.
- **Flying an orbit by keyboard:** with the level-flight helper on, holding LAlt+Z gives a steady left turn (about 4°/s at 380 km/h, radius about 1.5 km), but the circle drifts a few km in 3 to 5 minutes; check the minimap every minute. An air start may begin with the helper already on (LShift+A shows "Helper disabled" on the first press).

## Found on 2026-10-28 (P14 step 9 load check, game 1.005)

- **`MissionResaver.exe` still crashes** (exit 0xC0000005) on a mission built with `build_probe_mission.py`. Use `--no-resave`.
- **The Mission Editor opens a `.Mission` from any folder.** File › Open shows a standard Windows dialog, so a load check needs no copy into `game\data`. The editor rejects a bad file with an empty error box at once; a good one goes on to "Loading landscape" and "Updating models", which takes about 2 minutes on this PC. The mission tree's root expands on a double-click, not a single one.
- **Computer use:** `open_application` on a running Mission Editor starts a second copy, which asks whether to continue. End that copy's process (`STEditor` window) and bring the first copy forward with `AppActivate`.

## Found on 2026-10-29 (flight A2, game 1.005)

- **A deactivated running timer is paused, not cancelled.** An `MCU_Activate` later resumes it, and it fires after the rest of its time. Each zone's `MISS WAIT` (1 s) was switched off about 0.1 s after the pass started it. It fired 0.88 s after the zone's `ARM` re-activated it, 300 s later, four times out of four. Flight 0's J1 ("never fires") only covered a timer that was never activated again.
- **An `MCU_Activate` traced as a source never logs its breadcrumb** (`ZNaa ARM`), while timers in the same chain do. Replay listed it as unobserved. Its Targets are switched, not pulsed, so a breadcrumb there is only switched on. Since branch `claude/p14-paused-timers` the trace build pulses a switch's breadcrumb from the switch's own inputs; its line then means "the switch was pulsed", whether or not the switch itself was on.
- **Yak-9P air start:** the level-flight helper is on at the spawn. One LShift+A switches it off, and with no stick input the Yak rolls right into a climbing turn, then a spiral dive. The arrow keys did not recover it in time. Leave LShift+A alone after an air start; read the helper icon (the first icon in the right-hand column) instead.
- **A Yak-9P at 85 % throttle and 2,000 m accelerates past 520 km/h indicated** on the helper. 42 % held about 450 km/h. The `-` key works with the helper on; each 1 s hold takes about 17 %.
- **Helper turns:** with the level-flight helper on, holding LAlt+X or LAlt+Z gives about 3°/s at 450 km/h indicated (radius about 2.7 km), height held to ±2 m. It held a 16 km area's 8 km presence check for 25 minutes once centred. On the minimap, the target circle of AI attack aircraft marks their target.
- **Tacview's `.acmi` is locked exclusively while recording.** It cannot be read, even with shared read access, until the mission ends. For a live position, use the minimap grid: squares are 10 km, labelled row (north to south) then column (west to east); the column 23/24 line is at z ≈ 258 km and the row 28/29 line at x ≈ 190 km.
- **One spawn per session, again:** after the crash, Leave Server then Accept (Enter held 1 s, twice) went to the results screen. The server reloaded the mission into a new session, where the spawn worked.

## Found on 2026-10-29 (flight 2K, game 1.005)

- **A switched-off running timer is paused; switching it on resumes it with the time it had left.** A 10 s timer switched off 4 s in and on 16 s later fired 6 s after the switch-on (85.98 for 86). Two off/on cycles add up (94.96 for 95). Resumed timers fired one tick (0.02 s) early.
- **A pulse restarts a resumed timer** (full time from the pulse), and **a pulse while the timer is off is dropped**, leaving its time left unchanged.
- **A failed random roll on a running timer does not leave it running.** 20 timers at Random 50 %, pulsed at 60 and again at 65 while running: none fired at 70; 8 fired at 75. Either the failed roll stops the timer, or the roll is made when the timer ends.
- **Check zones:**
  - a pulse arms a zone, and it stays armed;
  - an armed zone fires when its condition becomes true;
  - switching an armed zone on is a check: it fires if the condition holds, also if it already fired before it was switched off;
  - a zone never pulsed does not fire, on an entry or on a switch-on;
  - a first pulse with the condition already true fires at once;
  - a further pulse into an armed zone whose condition still holds does nothing.
- **Decision 0043's switch breadcrumbs log in the game** (all 18 at their times).

## Found on 2026-10-29 (flight 2L, game 1.005)

- **The D11 relay works as a cancel.** A running timer that runs out into a switched-off zero-second relay timer is dropped there: `ZNii MISS WAIT` into `MISS WAIT OUT` (6 times) and `CLK N ATTEMPT END` into `REROLL` (6 times). Nothing fired later.
- **The real threshold passes exactly `HOLD` apart** while a plane stays inside (120.00 s, five in a row): `CHECK ON` switching the armed `INNER` on is the check.
- **AI on 501 triggers a NATO Air Tasking area** with the exported logic, as in flight 2J.

## Found on 2026-10-30 (flight A3, game 1.005)

- **The app's own Map air start works as a player spawn.** The fake-field Airfield with a Planes list (`StartType 0`, country 501), written by Map Generate with no `--spawns`, shows in the DPRK spawn list under its name. The Yak-9P spawned there at 2,000 m (logged `COUNTRY:501`) with the level-flight helper on: a 3 s LAlt+Z held height to ±2 m.
- **Do not move the mouse while flying.** In run 1 I moved the mouse from the bottom-right corner to the top of the screen and clicked there, to give the game keyboard focus. I then held LAlt+Z. The Yak pulled into a stall and a spin from 2,000 m, and the instructor showed "Don't pull the stick so much!" and "Spin!". In run 2 the mouse stayed still and the same turns flew clean.
  - Most likely the mouse movement works the stick.
  - If keyboard focus is lost in flight, use PowerShell `AppActivate`, not a click.
- **Two copies on one area end in one tick.** Their `Zone Out`s have the same centre and 35 km radius, so both `Zone Out`s and both `DONE HUB`s logged at the same tick (1,280.66). Both `DELETE DELAY`s followed 60.60 s later, also in one tick.
- **The clock's no-repeat works in the game.** A copy started after a slot's `DRAW` keeps its entry out of the next slot: slot 2 opened (1,800.10) with A HOT and the budget open, and no `GO` followed.
- **A clock copy's arrival:** F-51D Close Support, spawned 43.3 km out (520 km/h × 5 min), logged `Zone IN` at `START` + 314.0 s, 14 s after the nominal 5 min. A2's zone copy of the same type was 12 s late.
- **Yak-9P at 58 % throttle and 2,000 m:** about 485 km/h indicated. A held helper turn there gave about 3°/s (radius about 2.9 km) and held height to ±1 m for 15 minutes.
- **Leaving the server in flight logs the player's aircraft as destroyed** (`AType:3`, `AID:-1`, at its last position and height).
- **The launcher once failed with "wget: unable to resolve host address s3.il2korea.com".** The local DNS timed out once; Restart in the error box went straight on.

## Found on 2026-10-30 (standalone-file check L, game 1.005)

- **The Mission Editor's File › Import From File re-indexes the imported group.** A standalone Air Tasking file indexed from 1 was imported into a mission whose indexes also started at 1.
  - The editor renumbers the whole mission, not only the import. Right after the import, before any save, the tree showed the base map's group as 3 (1,126 in the file) and the imported group as 2,251 (1 in the file). The saved file has the base map at 3 to 2,249 and the imported group at 2,251 to 2,995. Whether the base map was renumbered on load or on import was not checked.
  - No index was used twice. Every block, property, position and link (`Targets`, `Objects`, `LinkTrId`, `MisObjID`, `TarId`) was kept in both parts.
  - The imported group kept its file coordinates.
  - Check: `standalone_l_editor_import_renumbers_and_keeps_every_link`.
- **The editor cuts a Group's name to 31 characters when it saves.** The six sortie copies' group names (48 and 52 characters) were cut. Every MCU name, and every other group name, was shorter and is unchanged. The app's own code matches copies on the `SNnn ` prefix, which survives the cut.
- **The editor gives every Mission Objective its own `LCName` and `LCDesc` on save**, with the text `<no name>`, where the file had 0. The `AType:8` log line carries no text, so the trace breadcrumbs are unaffected.
- The editor writes `90` where the file had `90.000`. The value is the same.
- **The editor's own Save works on game 1.005** (`MissionResaver.exe` crashes there). Next to the `.Mission` and six language files, it writes a `.msnbin` and an empty `.list`.
- **With a `.msnbin` next to the `.Mission`, the client host loads the `.msnbin`** (log header `MFile:Multiplayer/Dogfight/….msnbin`).
- **A map icon with `LCName = 0` seems to show language id 0, the mission title, on the briefing map.** In flight L every front-line icon of the Map base map was labelled with the mission title. This was read from the screen only; no file or screenshot was kept. Earlier Map builds (A2, A3) have the same icons; it was not looked for there.
- **Breadcrumbs still match after an import.** Replay matches an objective's `AType:8` line by position, so the standalone file's own `.trace.json` reads the log of the imported mission: no dead links. The logged `OBJID`s were A3's plus 1,996.
- **Closing the game from the results screen** (`CloseMainWindow`) brought up a run of "CAllocTable … leaks detected … Break program?" boxes. Cancel on each (about eight) let the game exit normally. OK would break into a debugger.
- **A four-ship Wedge at Safe can collide in a long steady turn.** SN01, four AI F-80C-10 (`AILevel` 2) under one Formation command (`FormationType` 20, `FormationDensity` 1), lost one aircraft to its own wingman. Flight 2E's "Wedge never under 20 m at Safe" came from shorter turns; this fits flight 2I's "Loose removes the entry-reversal collision, not the steady-turn crossing".
  - **Slots in transit** (400–425 s, 3,050 m, about 730 km/h), from the lead: one wingman 100 m behind and 103 m right; two on the left, 100 m behind and 108 m left, and 170–205 m behind and 215 m left (outboard).
  - **Over an area with no ground target the flight circles.** The HVAR AttackArea (`AttackGround 0`, `AttackAir 0`, `Time 600`) had nothing at A. After `Zone IN` (432.68) the lead flew a steady descending right turn at 30° bank: heading 311° to 172°, 3,050 m to 940 m, about 650 km/h and 1.8°/s (radius about 5.7 km).
  - **The collision.** Both left-side wingmen were on the outside of the turn, and their slots moved 100–150 m each way. The outboard one ("Red 14") slid inside the inner one ("Red 13") while that one rolled about 28° left against the turn. They closed from 53 m at 561.0 s to 5.2 m at 563.4 s, and both attitudes jumped at 563.2 s (the contact sign). The log has `AType:3 AID:-1` for Red 14 at 563.88, a bail-out at 570.14 and the crash at 578.14. Red 13 flew on to the end.
  - **Reading this Tacview:** mission seconds = `.acmi` seconds + 102 (its `ReferenceTime` 12:31:42 minus the mission's start time 12:30:00). Object ids are the log's `OBJID`s in hex. Tacview `handoff/tacview/P14_Standalone_L.zip.acmi`.
