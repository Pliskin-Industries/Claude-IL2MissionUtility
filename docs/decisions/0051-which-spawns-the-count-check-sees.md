# 0051. Which player spawns Generate checks against the count circles

- **Status:** Accepted
- **Date:** 2026-10-31
- **Decided by:** Claude, building the user's decision 0050 D ("No player spawn inside a count circle. Generate refuses a mission with an airfield or air start inside any area's circle"; the user: "We will need to make sure the air starts are outside of that area"). The limits below are what Generate can see; they are recorded here for the user.

## Context

- Decision 0050 D counts players per area from a Complex Trigger's events. A player who appears inside a trigger is not reported as entering (flight A1), so a spawn inside a count circle would go uncounted.
- A count circle watches the **other** side's players: a NATO area's count watches DPRK players (501), and a DPRK area's watches NATO players (601). A spawn of the area's own side does not matter to it.
- Generate (Map Generate, and the Air Tasking tab's Generate File) builds Air Tasking from the plan and an `AirContext`. Before this change the context held no spawns.

## Decision

- `AirContext.player_spawns` carries the player spawns the Map writes:
  - the Map's own air starts (`placed_air_starts`);
  - the player spawns in the Map's reference groups (`map_refs`): every `Airfield` with a `Planes` list, and every `Plane` with `CoopStart` on, on 501 or 601. These are the spawns that decision 0037 keeps on their country (`presence::player_spawns_in`).
- For each side's areas, a spawn of the watched side strictly inside a count circle is an **error**. The error names the area, its target position, the spawn, the distance and the radius (`presence::spawns_inside`).
- Generate File, on the Air Tasking tab, also checks the Map's spawns. The standalone file is imported into the Map's base map, so the same spawns apply.

## Not checked (cannot be seen)

- Spawns written by the Airfield tab as separate files, and spawns a mission maker adds in the editor. The user manual says to keep them outside every count circle.
- A loaded base map's own spawn groups that the Map does not read back into its air starts or reference groups (not traced).

## Rejected

- **Warn instead of refuse.** The user's 0050 D says refuse, and a spawn inside the circle makes the count go wrong for every player who uses it.
- **Check only the Map's air starts.** Reference groups can hold player airfields that Map Generate writes into the same base map.
