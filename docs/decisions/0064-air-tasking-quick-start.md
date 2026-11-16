# 0064. An Air Tasking quick start in the manual, also shown as a Help topic

- **Status:** Accepted
- **Date:** 2026-11-15
- **Decided by:** the user, 2026-11-15: asked for "a short how to tutorial on the air tasking", chose "repo markdown and in-app help" and the audience "New mission makers", then approved the plan ("Yes, build it").

## Context

- The Air Tasking section of `USER_MANUAL.md` explains every setting, but it is long. A new mission maker has no short path from an empty Map tab to a mission that flies.
- The Help window shows `USER_MANUAL.md` cut at `##` headings (`help::section_for`). A new manual section can therefore also be a Help topic, with no second copy of the text.

## Decision

- A new `## Air Tasking quick start` section, just before `## Air Tasking`: ten numbered steps (date and area, targets, player spawns, pool, pool rows, right panel, the Air dock, Generate, the mission editor, flying it) and three tips. The defaults are the first mission.
- A Help topic **Air Tasking quick start** (`HelpTopic::AirTaskingQuickStart`), listed before **Air Tasking**. F1 on the Air Tasking tab still opens **Air Tasking**, whose first line points to the quick start.
- A test (`help::tests::air_tasking_quick_start_names_real_controls`) fails when a control the quick start names in bold is no longer in the UI code.

## Rejected

- **A Help topic only, or the manual only.** The user asked for both.
- **A shareable document outside the repository.** It would drift from the app; the manual ships with it.
- **A tutorial for testers** (trace build and Replay). The user chose new mission makers. The quick start links to **Trace builds and replay** in one tip.

## Consequences

- No Generate output changes.
- Writing it found two gaps, recorded in `STATE.md`, not fixed here: Generate does not check the Airfield tab's field spawns against the count circles, and the Map section of the manual still describes an **Editor map** label the app no longer shows.
