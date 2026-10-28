# 2026-10-27 — P14 acceptance, flight A0 (no pilot): run sheet

Decision 0036: fly the acceptance test before step 9. A0 is the first stage: Claude hosts the acceptance mission on this PC with no pilot. The piloted sessions (A1 onward, the user, solo) follow.

## Files

- Generated on `main` @ `b5acb35` with `cargo test --offline write_p14_acceptance_mission -- --ignored`. Frozen copies: `handoff/p14-probes/acceptance-a0/` (`P14_Acceptance_1.Group` 1,449,923 bytes, `.eng`, `.trace.json` with 248 Air Tasking sources).
- Mission built without the resaver and without player spawns, as flight 2J was:
  `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Acceptance_1.Group --no-resave --name P14_Acceptance_A0_20261027 --title "P14 acceptance A0 (no pilot)" --out "C:\Machine Intelligence\P14 Flight A0\mission"`
- Copy `P14_Acceptance_A0_20261027.Mission` and `.eng` into `game\data\Multiplayer\Dogfight\` (needs the user's OK).

## What is in the mission

- The base map fixture (`baseline_tests::air_export_options`, 1950-09-15) with 2 DPRK objectives (x 45,000 / 185,000; z 280,000) and 2 NATO objectives (x 55,000; z 270,000 / 300,000).
- `acceptance_test_plan()`, seed 7: NATO 12 copies / 48 aircraft (country 602), DPRK 6 copies / 36 aircraft (503); 1,092 Air Tasking MCUs, 40 check zones.
- Clock, both sides: `FIRST` 600 s from Mission Begin, `PERIOD` 1,200 s, so a `TICK` at 600 s and 1,800 s. Each tick starts `JITTER 1 / 2 / 3` (0.1 / 180.1 / 360.1 s, Random 27 / 36 / 57 %); the first that passes opens `SLOT OPEN`, and about 20 % of slots stay quiet (decision 0029). Slot window (`SLOT CLOSE`): DPRK 16.5 s, NATO 30 s.
- The base map also has the Map's own **NATO Fighters Wave 1** pack: 18 fighters of country 601, started by a check zone of 16 km around (110,000; 280,000) for coalition-1 (DPRK side) planes. Not part of Air Tasking.

## Run

Host from the game client as for flight 2J (recipe: `handoff/sessions/2026-10-13-delete-probes.md`, traps in `docs/il2-loader-and-log-facts.md`): Multiplayer, Create, "Available from internet" off, this mission alone in the rotation. Nobody spawns; stay on the briefing screen. Read the mission time from the newest log file's last `T:` ((T − 3) / 50 s). **Leave at about 38:00** (the latest second slot closes at 1,800 + 360.1 + 30 s = 36:30). Copy the logs with `cp -p` to `src/testdata/missionlog/real_44/`.

Replay: `il2_mission_utility replay --group handoff/p14-probes/acceptance-a0/P14_Acceptance_1.Group --trace handoff/p14-probes/acceptance-a0/P14_Acceptance_1.trace.json --logs src/testdata/missionlog/real_44 --out target/p14/acceptance_a0_report.md`

## Predictions

1. **Loads.** The log's `MFile` names `P14_Acceptance_A0_20261027`; no loader error; the server runs at about 1.0× wall time (nothing flies).
2. **Clock, per side, independently:** at each tick (600 s, 1,800 s) `CLK s SLOT OPEN` logs once at tick + 0.1, + 180.1 or + 360.1 s (± 5 s), with the matching `JITTER k`, or not at all (quiet). Never twice in one tick.
3. **Nothing launches:** no `ED/EN GO`, no `SD/SN START`, `ON STATION`, `DONE HUB` or `DELETE DELAY`; no `THD/THN INNER` or `CLOSE DELAY`; no `ZD/ZN` breadcrumb (a zone's `ARM` is fed only by its `COOLDOWN`). Only clock nodes and the clock's draw fire.
4. **NATO Fighters Wave 1 stays parked** (no DPRK-side plane comes near).
5. **Replay:** no dead links; every fired source is a clock node.

## For the piloted sessions (A1 onward)

- The file needs player air starts (`--spawns`): one per side, plane and place to agree with the user.
- The Map's NATO Fighters Wave 1 will launch when a DPRK-side plane (the player as DPRK, or a DPRK sortie) comes within 16 km of (110,000; 280,000). Its 18 fighters are country 601, NATO's player country, so they can start DPRK sorties the way a NATO player would (flight 2J: AI of 601 counts). That breaks step 11's check "a winner only where the player loitered". Before A1, decide: build the acceptance base map without that pack, or keep it and read the results with it in mind.

## Results

Flown 2026-10-27, session `2026-10-27_17-53-51`, logs `src/testdata/missionlog/real_44/`. **All five predictions held.** Full table and reading: `handoff/P14-probe-results.md`, "Flight A0".

- Loaded on the first attempt; mission time kept pace with wall time; left at about 38:20.
- Slots opened at 600.16 s (NATO), 960.16 s (DPRK) and 2,160.16 s (both). A NATO jitter step that passed at 780.16 s opened nothing, as the latch should.
- No `GO`, no sortie, no threshold or zone breadcrumb; no object spawned during the run; no dead links.

Still open for A1: the player air starts and the NATO Fighters Wave 1 question above.
