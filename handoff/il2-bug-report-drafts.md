# IL-2 Korea: bug report drafts

Status: **drafts, not posted.** The user decides whether, where and when to post them (the game's bug-report forum is the likely place). Written 2026-10-15 from the probe flights in `handoff/P14-probe-results.md`. Attach only the files named here; they hold no personal data (check before posting).

---

## 1. AI wingmen at FormationDensity 0 (Dense) collide with each other

**Game:** IL-2 Sturmovik: Korea, version 1.005 (`261013`). Multiplayer dogfight mission hosted from the game client, AI only, no players flying.

**Summary:** AI F-80C 4-ships flying a Formation command at `FormationDensity = 0` (Dense) lose aircraft to collisions between their own wingmen, in gentle flight, with no enemy present. The same flights at density 1 (Safe) or 2 (Loose) did not collide.

**Steps to reproduce:**
1. Place a flight of four AI F-80C-10 (one lead, three wingmen), air start at 3,050 m.
2. Give the lead `MCU_CMD_Formation` with `FormationType = 20` (Wedge), `FormationDensity = 0`.
3. Send it to a waypoint about 60 km away at 3,050 m, then give `MCU_CMD_AttackArea` (ground targets, 2 km, 600 s) over an area with no enemy units.
4. Watch the flight (Tacview) for about 10 minutes.

**Expected:** wingmen keep station without touching.

**Actual:** wingmen come within a few metres of each other, and two of them collide (both destroyed in the same instant, no attacker). Most collisions came during the descent from 3,050 m after the waypoint, in turns of about 30° bank; some in transit and on station.

**Evidence (two test flights, five runs):**
- Flight 2C (12 four-ships, server overloaded): 7 aircraft lost in one run, every one a collision between wingmen of one flight, in four different loadouts and orders.
- Flight 2D (4 four-ships, server not overloaded, only the formation setting changed), three runs:

| Formation | Nearest wingman, median | Seconds under 20 m | Losses |
|---|---|---|---|
| Wedge, density 0 (Dense) | about 100 m | 4, 6, 4 | 2 aircraft in 2 of 3 runs |
| Wedge, density 1 (Safe) | about 140 m | 0, 0, 0 | none |
| Wedge, density 2 (Loose) | about 500 m | 0, 0, 0 | none |
| Pairs, density 0 (Dense) | about 50–80 m | 13, 26, 26 | 2 aircraft in 1 of 3 runs |

- Attachments: Tacview tracks `handoff/tacview/P14_Probe_2D_run1.zip.acmi` to `run3` and `P14_Probe_2C_run2.zip.acmi`; the server text logs `src/testdata/missionlog/real_24` to `real_27`; the mission group `handoff/p14-probes/flight2d/P14_Probe_2D_traced.Group`.

**Workaround:** use density 1 (Safe).

---

## 2. Entity OnEvent type 4 (OnPlaneDestroyed) never fires

**Game:** IL-2 Sturmovik: Korea, versions `261002` and 1.005 (`261013`). Multiplayer dogfight mission hosted from the game client.

**Summary:** an `MCU_TR_Entity` for an aircraft with an `OnEvents` entry of type 4 (`OnPlaneDestroyed`) never sends its target a pulse when the aircraft is destroyed. Type 13 (`OnKilled`) fires instead, once per aircraft. Any mission logic that counts losses on type 4 never counts.

**Steps to reproduce:**
1. Place an AI aircraft with its entity (seen with F-80C-10; other types not checked one by one).
2. On the entity, add OnEvents of every type 0 to 13, each pulsing its own timer that logs a `MissionObjective` (or a subtitle), so each event is visible in the text log.
3. Destroy the aircraft: an `MCU_CMD_Damage` (complete), AI gunfire, a collision, or fuel starvation and bail-out.

**Expected:** type 4 (`OnPlaneDestroyed`) fires when the aircraft is destroyed.

**Actual:** type 4 never fires. Type 13 fires once within 0.1 s of the kill; then 0 (pilot killed), 2 (crashed) seconds later, and 12, 1, 3 on the way. Deleting the aircraft with `MCU_Delete` fires nothing (that part is expected).

**Evidence:**
- Flight 1G (game `261002`): two aircraft killed by a Damage command and one by AI gunfire; type 13 once each, type 4 never (`src/testdata/missionlog/real_6`).
- Flight 1K (game `261013`): losses by collision and in a fight; type 13 each time, type 4 never (`handoff/P14-probe-results.md`, "Flight 1K").
- Flight 1L (game `261013`), every type 0 to 13 listened for: losses by Damage command and by fuel starvation with a bail-out; type 4 never (`src/testdata/missionlog/real_15`; `handoff/P14-probe-results.md`, "Flight 1L"; `docs/il2-loader-and-log-facts.md`).
- Attachment: `handoff/p14-probes/flight1l/` (the mission group with every event wired to a breadcrumb).

**Workaround:** count losses on type 13 (`OnKilled`). This repository does so (decisions 0011 and 0012).

**Also seen, possibly related:** type 7 (`OnPlaneBingoFuel`) fired three seconds after activation on aircraft with full tanks in some missions (upstream's F-80C strike and F-86A CAP), but not on others of the same types (flight 1P). Cause not known; not part of this report unless it reproduces.
