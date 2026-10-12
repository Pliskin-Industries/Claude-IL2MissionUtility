# Session note: IL-2 Korea on this PC, Claude as the test pilot (2026-10-12)

> A session note holds detail that is in flight. Once its content has moved to `STATE.md` or a decision record, it is history.

## Why

The user could not reach the other machine to make the Complex Trigger sample or fly more probes. The user asked Claude to use the game on this PC, rebuild the test missions and fly them itself. Approved for this session: (1) editor check and Complex Trigger sample, (2) game check, (3) re-run flight 0 here as a calibration. Reshaping flights 1A and 1B for an autopilot pilot was proposed and is **not** approved yet.

## This PC

- Lenovo 11JJ008BUS, Ryzen 3 PRO 4350GE, **integrated Radeon graphics**, 31 GB RAM. The game's published minimum is an 8 GB card (RTX 3060 / RX 7600 class).
- IL-2 Korea is in `C:\Program Files\IL2Series` (installed 2026-09-28). The launcher updated it on 2026-10-12 from `260807` to `261002` (launcher shows 1.004). **The other machine's server must be on the same version before a mission built here is flown there.**
- No account was saved in `startup.cfg`, and the game had never been run here.

## Done

| Step | Result |
|---|---|
| Editor | `IL2Editor.exe` runs on this GPU. Loading the Korea terrain takes about 3 minutes and uses 7.5 GB of RAM. The map view is slow: a click can take 10 s or more to land, and clicks made while it is busy are dropped. |
| Complex Trigger sample | Saved from the editor: `TemplateExamples/Reference/ComplexTrigger_reference.Group` (trigger plus one Timer target) and `ComplexTrigger_reference_properties.jpg`. Facts read from it are in `docs/il2-loader-and-log-facts.md`. |
| Game config | In the game's `startup.cfg`: `mission_text_log = 1` (was 0) and `exclusive = 0` (was 1, a mouse test, see below). Backup of the original: `startup.cfg.bak-20261012` next to it. |
| Flight 0 rebuild | `tools/p14/build_probe_mission.py` built `P14_Probe_0_cal_20261012` from the committed flight 0 probe (same files as flown on 2026-10-03); resaver exit 0 on game `261002`. Output in `target/p14/missions/P14_Probe_0_cal_20261012/`, copied to the game's `data\Multiplayer\Dogfight\`. |
| Game start | `IL2Series.exe` starts and draws its hangar screen on this GPU. Frame rate in a mission is **not measured yet**. |
| Driving the screen | Computer use can see and click both the editor and the game. Each app must be granted by its exe name (`IL2Series.exe` is separate from the launcher). Bring a window forward with `AppActivate` from PowerShell; opening the editor a second time starts a second copy. |

## Driving the game (worked out 2026-10-12)

The user accepted the agreement and signed in (remote session, screen 1280x720).

- **Set `exclusive = 0`** under `[KEY = input]` in `startup.cfg`. With 1, the game reads raw relative mouse input and its cursor jumps to a screen corner on every pointer move. With 0, the cursor follows the pointer exactly.
- **Click slowly:** move to the control, press, wait 1 s, release. A quick click only hovers.
- **Confirm dialogs (Accept / Cancel) ignore the mouse.** Hold Enter for 1 s; if Accept only turns solid, hold Enter again.
- The game must have focus. Bring it forward with `AppActivate` from PowerShell first. Without focus it hides the menu and shows a camera tour; Escape brings the menu back.
- In the cockpit the mouse turns the pilot's head.
- **Leaving a server you host takes two steps.** Leave Server on the briefing only ends the round; a results screen follows with about 10 s before the next mission loads (4 minutes each time). On that results screen press Leave Server again and hold Enter at once. Put one mission in the rotation, not three.
- **Do not chain extra Enter presses.** After the server closed, spare Enter presses reached the public server list and pressed Join on its first entry; the game began downloading that server's mission. It was cancelled with Escape after about 2 MB. The two partial files were moved out of the game folder to `target/p14/quarantine/`.
- **Never force-close the game.** Claude did once, and it lost the saved sign-in; the user had to sign in again.

## Flight 0 calibration: flown, matches

Hosted from the game client: Multiplayer, Create, Dogfight, "Available from internet" off, mission added with Add to rotation. Loading takes about 4 minutes. Result and comparison: `handoff/P14-probe-results.md`, last section. **This PC reproduces the 2026-10-03 dedicated-server results.**

## Flying (checked 2026-10-12 on flight 1A)

Claude can fly level legs, orbits and turns with the keyboard alone. It cannot take off, land or fight. Results of the flight: `handoff/P14-probe-results.md`, last section.

| Key (hold about 1 s unless noted) | Effect |
|---|---|
| LShift+A | Level-flight helper on or off ("Helper enabled: Level flight"). It holds the height and heading it has when switched on, so switch it on **at once** after an air start. A quick tap does nothing. |
| LAlt+Z / LAlt+X (keep held) | Banked turn left / right while held: about 1.8 deg/s at 700 km/h indicated, 3.5 deg/s at 350. |
| LShift+Z / LShift+X (keep held) | Gentle turn, about 1 deg/s. |
| `-` / `=` (keep held) | Throttle down / up, about 30 % per second. |
| B | Airbrakes. |
| M | Cycles the map: Around Me, Me and Target (about 22 km wide), Mission (whole map), off. |

What the flight taught:
- **The HUD speed is indicated airspeed.** At 4560 m the ground speed was about 20 % higher. Dead reckoning with the HUD figure put a turn 15 km off. Navigate by the map icons instead; they show only within about 11 km.
- An orbit fits inside a 5 km zone at 350 km/h or less (turn radius about 1.6 km). At 700 km/h the radius is about 6 km.
- Do not let the speed fall in a turn: at 54 % throttle it decayed to 211 km/h and the game warned "Your speed is too low".
- A 12 km zone is easy to hit. A 2 km zone needs the icon on the map and a straight run-in of 10 km or more; both were hit, but one only by clipping it in a turn.
- Each look at the screen costs 5 to 10 s. Plan in 30 s steps.
- The simulation kept real time for the whole 28 minutes. The frame rate was not measured (the counter is RCtrl+Backspace).
- Leaving the server worked first time with: Leave Server, hold Enter twice; on the results screen Leave Server, hold Enter twice; then Back, Back with the mouse only.

## Step 0e (same day)

- Astra built it in three jobs (`20261012202902-ad183d72`, `20261012205208-e28920ae`, `20261012210822-e97094ac`), model `gpt-6-astra` confirmed in the first job's log. Commits `54138eb` and `49c6356` on `codex/p14-0e`, merged as `aa3b048`.
- A two-axis review found no breach of the repo's rules and three spec problems, all fixed before any flight: the trigger breadcrumbs hung on a subtitle's outputs; the hunters' Attack order could be replaced by their template ground attack; G2 and G3 started on one point.
- Style notes left for the user to rule on: duplicated airfield lookup in the builder (`spawn_airfields` and `planes_block`), raw tracebacks if the game's test mission changes shape, bare country and event numbers, flag arguments (`build_y(p, true)`).
- Build commands for the three missions: as in `handoff/p14-probes/flight1a/README.md`, with `--spawns target/p14/P14_Probe_1N_traced.spawns.json` (or `1D`) in place of `--ref`; flight 1G uses `--ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10`.
- **The game loses input whenever something else takes the foreground.** Astra's and the test runs' shell windows did it, and so did a RustDesk window the user opened on this PC. With no foreground window at all, the computer-use tool refuses keys ("no app is frontmost"). Fly only when nothing else is running and the user is not using the PC.
- The handbook is reached from the in-flight Esc menu (HANDBOOK); the Museum (main menu) lists 12 aircraft and tabs for ground vehicles, armor, artillery, railroad, equipment, ships and theater of war. The country-name file `data\GUI\DefaultCountryNames.eng` names only 000, 501, 502, 503 and 601.

## For decision 0008 (country filter)

The editor lists five countries: Neutral Country (Neutral), USSR, PRC, DPRK (Eastern Bloc) and USA (Western Bloc). **NATO has one named country.** The mission log header also lists 602 and 603 on the NATO side, but the editor gives them no name, so a Complex Trigger cannot pick them from its country list (writing `Country = 602;` by hand is untested). Cell Y1 as proposed on 2026-10-07 (NATO AI on country 602, players on 601) has no second NATO country to use. The DPRK side has three, so the test is possible there. Not tried: giving Neutral Country the Western Bloc coalition in the mission's Countries table.
