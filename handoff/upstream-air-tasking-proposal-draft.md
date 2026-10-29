# Proposal for `eszyman/IL2MissionUtility`: Air Tasking (draft, not sent)

> Status: **draft for the user's review. Nothing has been sent.** Decision 0026 (accepted 2026-10-28) is the direction. Where to post it is the user's call: a new issue (suggested, since the topic is new) or a comment on issue #1 (https://github.com/eszyman/IL2MissionUtility/issues/1), whose last section first mentioned Air Tasking and got no reply on that part.

---

**Title:** Proposal: Air Tasking, a scheduler for AI air activity built on Template Builder's flights and your strike targets

Hi again, and thanks for picking up the three fixes so quickly.

In issue #1 I mentioned the tab the fork was building, Air Tasking. It now works end to end in the game, so I would like to show it to you and ask whether you want it upstream.

## What Air Tasking does

It runs the AI air war of a mission from the Map. You mark targets, and the app writes the logic that decides, during the mission, **when** each side's sorties launch, **against what**, and **how many** fly at once.

- **Areas around targets.** Each target gets an area. When a player of the other side flies into it and stays 60 seconds, the area is "hot". Sorties go only where the area is hot, so AI attacks where players are.
- **Two ways to launch.** A mission clock per side, with random timing so launches are not predictable. And check zones that roll a chance when players enter. Each sortie in the pool has a weight and says which of the two can launch it.
- **A budget per side.** At most N sorties alive per side. A sortie that ends frees its slot. This replaces a one-at-a-time lock.
- **Reusable copies.** Each sortie is placed as a few copies, so the same raid can fly again later in a long mission.
- **A shell around every copy.** It handles the on-station time, one extension while the area stays hot, the trip home when it goes cold, an early end when the flight is lost, and a clean Delete 60 seconds after the end. A copy that has ended goes quiet; no timer in it runs on.
- **AI cannot trigger it.** Its own AI flies on countries the area triggers do not count, so only players start sorties.
- **UI.** An Air Tasking tab (the pool, weights, budget, a slot timeline, plan checks), an Air tab in the Map dock (auto-fill from objectives, the drawn areas and routes), and Map Generate writes it into the base map. A "trace build" option and a Replay window read the server's text log afterwards and show what fired.

It has been flown, not only tested in code: a series of probe flights checked each piece of logic the game had to honour, and two acceptance flights ran the whole cycle (clock, area, launch, transit, on station, budget, deletion) in IL-2 Korea 1.005.

## The proposal

Your Template Builder and Air Tasking do different halves of the same job, so I would like to join them rather than run two systems side by side:

1. **Template Builder stays the way to build a flight:** aircraft, formation, loadout, and your Strike and CAP packages. Air Tasking would take a Strike or CAP group as a sortie. It would start the group itself (instead of the group's Zone IN) and free the budget slot from your `MISSION END` / `ALL DESTROYED`. Your route, attack and go-home logic stay as they are.
2. **Your target picker's types become Air Tasking targets:** tanks, trucks, rail, ships, airbases, bridges and dams, next to Map objectives and attack arrows.
3. **For scheduled air activity, Air Tasking takes the place of Map › Missions and Exclusive Activation.** The budget does the lock's job and allows more than one flight per side. Exclusive Activation could stay for hand-made plans if you want it.

Before writing any of that, I would fly one test of a wrapped Strike and a wrapped CAP, as with the fixes in issue #1.

## What I am asking

- Does this direction work for you?
- If yes, how would you like it: a short design note first, a branch you can try, or a series of small PRs?

Air Tasking lives in its own module (`src/airtask/`) and two UI files, and hooks into Map Generate; and a mission without Air Tasking is written exactly as before (a test guards that). I am happy to send the flight logs, the test missions, or a build.
