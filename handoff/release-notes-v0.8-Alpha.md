# Version 0.8 Alpha (release notes, for the GitHub release `v0.8-Alpha`)

> Draft for the owner's review (decision 0063). After the tag's release build has run, these notes replace the generated ones: `gh release edit v0.8-Alpha --notes-file <this text>`.

---

**An alpha.** It adds **Air Tasking**, a new and experimental way to run a mission's AI air activity, and brings in the upstream author's latest work. Air Tasking has been flown in IL-2 Korea 1.005 with one player at a time; the parts that need two or more players are not yet proven (see **Not yet verified**).

Download `IL2MissionUtility-v0.8-Alpha.exe` below; the `.sha256` file holds its checksum. The exe is the whole app.

## New: Air Tasking

AI sorties that fly where players are, during a multiplayer mission:

- **Areas round targets.** Each Map objective or attack arrow in the pool gets an area. When N players of the other side (1 to 3) fly in and stay through a dwell, the area goes HOT. Sorties go only to HOT areas.
- **Two ways to launch.** A clock per side (a slot every 20 min, random jitter, 20 % of slots quiet) and a roll each time a check finds the area HOT.
- **A budget per side** (2 copies by default) and a **player count per area** that ends a sortie when its last player leaves.
- **A shell round every sortie:** on-station time, one extension while the area stays HOT, home when it goes COLD, an end when the flight is lost, a hard stop, and a clean delete 60 s after the end.
- **Only players start it.** The Map moves its own AI off the players' countries (601 and 501) when Air Tasking is in the mission.
- **Where:**
  - the **Air Tasking** tab (Ctrl 7): pool, weights, budget, threshold, player count, schedules, plan checks;
  - the Map dock's **Air** tab: auto-fill from objectives, areas and count circles on the map;
  - **Generate Base Map**, which writes Air Tasking into the base map;
  - **Generate File**, which writes it as a group to import in the editor.
- **Trace builds and Replay.** Tick **Trace build** and the server's text log records which logic fired. The **Replay** window then reads the log back on the map and as a report.

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
- **A player who spawns inside an area is not seen** until they leave it and fly back in. Keep player airfields and air starts outside the areas. Generate refuses a Map spawn inside a count circle.
- **Il-10 gunners** shoot down a fighter orbiting their target at their height.
- **Front-line icons on the briefing map** may show the mission title as their label (cosmetic).

## Also in this release

- **Upstream's work through 2 November 2026:**
  - the Airfield tab owns base-map fields and air starts;
  - Map strikes are wrapped in Exclusive Activation;
  - anti-air is placed in clearings;
  - per-country harvested groups and ramp starts.
- **Air starts in a Map base map** now sit under `Airfields` / `DPRK` or `NATO` (upstream). DPRK player spawns are written on 501, the country Air Tasking watches.
- **AI air formations fly at Safe density** (Pairs at Loose), to reduce the game's wingman collisions.

## Feedback

Please open an issue on `Pliskin-Industries/Claude-IL2MissionUtility`. For a mission problem, attach a trace build's `.Group`, its `.trace.json`, and the server's `missionReport` text logs.
