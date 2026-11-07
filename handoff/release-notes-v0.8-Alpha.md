# Version 0.8 Alpha (release notes, for the GitHub release `v0.8-Alpha`)

> Draft for the owner's review (decision 0063), revised 2026-11-07 after a Codex review. After the tag's release build has run, these notes replace the generated ones: `gh release edit v0.8-Alpha --notes-file <this text>`.

---

**An alpha.** It adds **Air Tasking**, a new and experimental way to run a mission's AI air activity, and brings in the upstream author's latest work. Air Tasking has been flown in IL-2 Korea 1.005 with one player at a time; the parts that need two or more players are not yet proven (see **Not yet verified**).

Download `IL2MissionUtility-v0.8-Alpha.exe` below; the `.sha256` file holds its checksum. The exe is the whole app.

## New: Air Tasking

AI sorties that fly where players are, during a multiplayer mission:

- **Areas round targets.** Each Map objective or attack arrow in the pool gets an area. When N players of the other side (1 to 3) fly in and stay through a dwell, the area goes HOT. Sorties go only to HOT areas.
- **Two ways to launch.** A clock per side (a slot every 20 min, random jitter, 20 % of slots quiet) and a roll each time a check finds the area HOT.
- **A budget per side** (2 copies by default; a slot is freed when the ended sortie's aircraft are deleted, and two requests in one game tick can briefly exceed it).
- **A player count per area:** while on station, a sortie ends when its area's count becomes empty. The count has limits for near-simultaneous entries and after a fourth entry (see the manual).
- **A shell round every sortie:** at the end of its on-station time a COLD area sends it home and a HOT area gives one extension (10 min by default); an end when the flight is lost; a hard stop; a clean delete about 60 s after the end.
- **Watched countries.** The area triggers watch the player countries 601 and 501. The Map moves its own AI off them when Air Tasking is in the mission; keep AI added by hand off them too. AI cannot start an area, but can keep a HOT area HOT.
- **Where:**
  - the **Air Tasking** tab (Ctrl 7): pool, weights, budget, threshold, player count, schedules, plan checks;
  - the Map dock's **Air** tab: auto-fill from objectives, areas and count circles on the map;
  - **Generate Base Map**, which writes Air Tasking into the base map;
  - **Generate File**, which writes it as a group to import in the editor.
- **Trace builds and Replay.** Generate with **Trace build** ticked and turn on the server's text log (`mission_text_log = 1`). The **Replay** window reads the generated `.Group`, its `.trace.json` and the session's logs, and shows which logic fired, on the map and as a report.

The manual (Help, F1) has the full description: **Air Tasking** and **Trace builds and replay**.

### Seen in the game

Flown on a test PC with one player (or none):
- areas going HOT and COLD;
- zone and clock launches, quiet slots (28 of 120 in flight 2T), and the budget refusing a third sortie;
- the on-station end and the one extension on both sides;
- the player count ending a sortie;
- deletes, and used copies kept quiet;
- a Yak sortie;
- a Map Generate mission end to end;
- the standalone file imported into the mission editor.

### Not yet verified

- The mission's **load time and tick delay** on a real dedicated server with sorties flying.
- **Two or more players:** the N ≥ 2 threshold (so far flown with AI stand-ins) and the player count with people (what a player who quits in flight counts as).

### Known issues

- **AI formations can collide** in a turn (a game bug). The app flies formations at Safe density, which makes it rarer.
- **A player who spawns inside an area is not seen** until they leave it and fly back in. Keep player airfields and air starts outside the areas and the count circles. Generate rejects a Map air start or reference-group player spawn inside a count circle that watches its side; Airfield-tab and editor spawns are not checked.
- **Il-10 gunners** shoot down a fighter orbiting their target at their height.
- **Front-line icons on the briefing map** may show the mission title as their label (cosmetic).

## Also in this release

- **Upstream's work through 2 November 2026:**
  - the Airfield tab owns base-map fields and air starts;
  - two or more compatible Map strike plans share an Exclusive Activation group;
  - anti-air is placed in clearings;
  - per-country harvested groups and ramp starts.
- **Air starts in a Map base map** now sit under `Airfields` / `DPRK` or `NATO` (upstream). DPRK player spawns are written on 501, the country Air Tasking watches.
- **AI air formations fly at Safe density** (Pairs at Loose, unless flying a Cover order), to reduce the game's wingman collisions.

## Feedback

Please open an issue on `Pliskin-Industries/Claude-IL2MissionUtility`. For a mission problem, attach a trace build's `.Group`, its `.trace.json`, and the server's `missionReport` text logs.
