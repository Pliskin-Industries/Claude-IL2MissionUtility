# 0054. A base map made for a standalone Air Tasking file also moves the Map's AI off the player countries

- **Status:** Accepted
- **Date:** 2026-11-01
- **Decided by:** the user, 2026-11-01 ("Fix 1–9, and A–C too"), on Claude's triage of Astra's review of `b5acb35..6f02211` (finding B, Astra job `20261101145549-c0f120bc`, item 2).

## Context

- Decision 0037 A moves the Map's own AI aircraft off the player countries (501, 601) so they cannot start Air Tasking cycles. Map Generate does it only when the Air Tasking group is in the base map (`src/ui/map.rs`, `if with_air`).
- The standalone workflow (step 11 item 4, check L) unticks **Include in base map**, generates the base map, then writes the Air Tasking file from the Air Tasking tab and imports it in the editor.
- That base map keeps Map AI on 501 and 601. Once the standalone file is imported, the player counts and thresholds see that AI as players, and it can start sorties with no human present.

## Options

**A. Move Map AI whenever the plan has a pool and Include in base map is off (chosen).** 0 MCUs.
- The tab already treats that state as the standalone workflow: the Map's Air Tasking row says "Not in the base map: tick Include in base map on the Air Tasking tab."
- Base maps written with no Air Tasking plan (an empty pool) are unchanged.

**B. A separate "companion base map" export button.** Rejected: one more control and one more step for the same result.

**C. Refuse the standalone file while the loaded Map has AI on 501 or 601.** Rejected: the standalone file cannot see the base map that will be written, and refusing does not fix the base map.

**D. Leave it.** Rejected: AI can run sorties on an empty server (tick drain, the fault 0050 removed).

## Decision

**A.** Map Generate runs `move_map_ai_off_player_countries` when the Air Tasking group is in the base map (as now) **or** when the plan's pool is not empty and Include in base map is off.

## Consequences

- **Output changes** only for a base map generated with a non-empty, unticked plan and Map AI aircraft on 501 or 601. Player spawns (Airfields, `CoopStart` planes) are not moved, as in 0037.
- Check L's base map (A3's inputs, no Map AI on 501 or 601) is unchanged; its byte comparison is restored (review finding 3).
