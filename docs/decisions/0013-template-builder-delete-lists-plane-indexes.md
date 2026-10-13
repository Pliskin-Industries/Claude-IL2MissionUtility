# 0013. Template Builder's clean-up Delete lists Plane indexes and removes nothing

- **Status:** Proposed
- **Date:** 2026-10-13
- **Decided by:** open; the user decides. Finding and recommendation by Claude. No Template Builder code was changed.

## Context
`generate_template` ends a group's life with `Deactivate Units` and, 0.5 s later, `Trigger Delete`. It fills the first with the units' entity indexes and the second with the units' own block indexes (`template.rs`, `delete.set_objects(object_ids.clone())`). The six built-in sorties in `TemplateExamples/Historical1950/` carry the same lists. All 20 example files saved from the mission editor list entities in a Delete; none lists a unit block.

Flight 1H tested it on game 1.005, three runs (`handoff/P14-probe-results.md`, "Flight 1H"; logs `src/testdata/missionlog/real_9` to `real_11`):

- A Delete that listed two live aircraft by **entity** removed both.
- A Delete that listed two live aircraft by **Plane index** removed neither: both were still there 30 s later and died to a Damage command.
- A deactivated aircraft still exists: it is in the end-of-mission list.

So a group that Template Builder "cleans up" is switched off and stays in the mission.

**What it affects.**

| Where | Effect in game |
|---|---|
| Template Builder, any group that reaches `MISSION END` (zone out, mission complete, RTB) | Units are deactivated, never deleted. They vanish from view either way, which is likely why this went unnoticed (an inference; nobody was asked). |
| Template Builder with **Allow multiple spawns** | Each wave is left deactivated in the mission after its clean-up. A long mission collects them. The cost of a deactivated unit to the server has not been measured. |
| Air Tasking (P14) | Its budget frees a slot when a sortie is deleted (U22) and its sorties come from these templates. With the Delete as written, nothing is ever freed in the game while the app's count says it was. P14 will write its own Deletes with entities whatever is decided here. |
| Fighter Pack, Exclusive, Recon, Duplicate | They copy the Delete of the template they are given. A template made in the editor lists entities and works; one made by Template Builder carries the fault through. |

Tested on aircraft only. Vehicles, ships and trains get the same list from the same line of code; whether the game treats their block indexes the same way has not been flown.

## Decision
None yet. Claude recommends option B.

## Options

| | A. Leave it | B. `Trigger Delete` lists entities | C. B, and re-save the six built-in sorties |
|---|---|---|---|
| Generated files | No change | One line per generated group changes: the `Objects` list of `Trigger Delete`. Same MCUs, same indexes. | As B, plus six example files change one line each |
| In game | Units are deactivated and stay | Units are removed | As B; the built-in sorties also clean up when used outside Air Tasking |
| Tests and baselines | none | The Template Builder baselines are re-recorded (the files Generate writes change, which the project rule forbids without the user's word) | As B, plus the fixtures made from the six sorties |
| Risk | Missions grow with each respawn | A Delete that works may show a fault the dead one hid: flight 1G saw one aircraft survive a Delete whose list also named a dead aircraft. Flight 1J (plan step 0g) tests that first. | As B |

## Consequences
- Whichever option is chosen, P14's sortie clean-up lists entities.
- If flight 1J shows that a dead aircraft in the list makes a Delete skip the live ones, option B needs one Delete per unit, or the same fault moves from "never deleted" to "sometimes not deleted".
- `USER_MANUAL.md` describes `Trigger Delete` as object-linked to every unit (line 121) and tells template authors to list every Plane, Vehicle or Ship in a Delete's Objects (line 253). Under B or C those lines need a second look: in the editor an object link goes to the unit's entity.
- Related: decision 0012 (the death event Template Builder and Fighter Pack listen for). Both change Generate output, so they are best settled and released together.
