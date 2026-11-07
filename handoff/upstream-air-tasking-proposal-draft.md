# Issue for `eszyman/IL2MissionUtility`: try v0.8-Alpha, and the Air Tasking proposal (draft, not sent)

> Status: **draft for the owner's review. Nothing has been sent.**
> - Decisions: 0060 (two modes, the replace-it table) and 0063 (send the alpha build and the proposal now, in one issue).
> - Revised 2026-11-07 after a Codex review: 5 blockers and 8 smaller points, all applied. The blockers were the trial steps (Auto-fill and the clock), the budget versus Exclusive Activation's lock, and the shell's end-of-station rule.
> - Post it as a new issue on `eszyman/IL2MissionUtility`, only at the owner's word. The release `v0.8-Alpha` is up.
> - Earlier posts: #2 (load test), #3 (flight 2N, still to be flown).

---

**Title:** Try Air Tasking (fork v0.8-Alpha), and a proposal: a player-driven mode beside Exclusive Activation

Hi again. You have seen Air Tasking in #3, the four-pilot test mission. The fork now has an alpha build with it, and with your work through 2 November merged in, so here is a build to try and a question about where it should live.

**Build:** https://github.com/Pliskin-Industries/Claude-IL2MissionUtility/releases/tag/v0.8-Alpha. `IL2MissionUtility-v0.8-Alpha.exe` is the whole app; the `.sha256` file is beside it.

## What Air Tasking does

It runs a side's AI air activity from where players fly. You mark targets on the Map, and Generate writes the logic that decides, during the mission, **when** each side's sorties launch, **against what**, and **how many** fly at once.

- **Areas round targets.** Each target gets an area. When N players of the other side (N from 1 to 3) fly in and stay through a short dwell, the area goes HOT. Sorties go only where an area is HOT, so the AI attacks where players are.
- **Two ways to launch.** A mission clock per side, with random timing and a chance of a quiet slot; and a roll each time a check finds the area HOT. Each sortie in the pool has a weight and says which of the two can launch it.
- **A budget per side.** Each side has a sortie budget (2 by default). A sortie that ends frees its slot when its aircraft are deleted, about a minute later. Two requests in the same game tick can briefly exceed it.
- **A shell round every copy:**
  - at the end of its on-station time, a COLD area sends the sortie home; a HOT area gives one extension (10 minutes by default), after which it ends;
  - an early end when the flight is lost;
  - while on station, an end when its area's player count becomes empty. The count has limits for players entering at almost the same moment, and after a fourth entry it waits for every aircraft of that side to leave; the manual lists them.
  - a hard stop, and a clean Delete about 60 seconds after the end.
- **Watched countries.** The area triggers watch the player countries 601 (NATO) and 501 (DPRK side). Air Tasking's own AI and the Map's other AI are put on other countries; keep any AI you add by hand off 601 and 501 too. AI cannot start an area, but once a player has made it HOT, AI of that side inside it can keep it HOT.
- **UI:**
  - an **Air Tasking** tab (Ctrl 7): pool, weights, budget, threshold, count, schedules, plan checks;
  - an **Air** tab in the Map dock: the pool for one side at a time, and the areas, count circles and approach lines drawn on the map;
  - **Generate Base Map** writes it into the base map.
  - **Trace build** and the **Replay** window: with the server's text log on, Replay shows afterwards which logic fired.
- **Flown** in IL-2 Korea 1.005, on a test PC with one player at a time:
  - areas going HOT and COLD;
  - zone and clock launches, quiet slots (28 of 120);
  - the budget refusing a third sortie;
  - the on-station end and one extension on both sides, with Yak and F-80 sorties;
  - the player count ending a sortie, and deletes;
  - a mission made end to end by Map Generate;
  - the standalone group imported in the editor.
- **Not yet proven:** two or more players (#3 is that test), and load time and tick delay on a real server with sorties flying.

## Trying it (one pilot is enough)

1. On the **Map** tab, set up a base map as usual, with one objective on each side. Those are the areas.
2. On the **Air Tasking** tab (Ctrl 7), click **Auto-fill from objectives**. That button fills both sides. The one in the Map dock's Air tab fills only the side selected there.
3. Auto-filled entries launch from the clock, and its first slot is at 10 minutes. For a short test:
   - set the **Mode** of one entry on the side you will fly **against** to **Zone**;
   - under **Zone triggers**, set **Chance** to **100 %**;
   - leave **Threshold › Players** at **1**.
4. Give yourself an air start on your side, **outside** the player-count circles. They are 50 km by default; set **Count radius** to 20 km on the Air Tasking tab to keep things close. Generate rejects a Map air start, or a player spawn in a Map reference group, inside a circle that watches its side. Spawns added in the Airfield tab or the editor are not checked, so keep those outside too.
5. Tick **Trace build**, click **Generate Base Map**, and make the mission as you usually do. Keep the generated `.Group` and its `.trace.json` together.
6. On the server, set `mission_text_log = 1` under `[KEY = system]` in `startup.cfg` before hosting.
7. Spawn and fly to that objective (a NATO pilot over a DPRK objective, or the other way round). Get within 6 km of it (half the default 12 km threshold radius) and stay there through the 60-second dwell. The sortie starts 30 km out, so allow a few more minutes for it to arrive.
8. Afterwards, open **Replay**, choose the generated `.Group` and the session's log folder, and click **Run replay** to see what fired. Or send me the `missionReport` logs.

The manual (Help, F1) has the details under **Air Tasking** and **Trace builds and replay**.

## How it fits with what you built

I see Air Tasking as **a second mode next to Exclusive Activation, not a replacement.** Exclusive Activation suits scripted plans that run one at a time; Air Tasking suits a mission whose air war follows the players. A mapper picks the one that fits each mission.

Two pieces would join them. Neither is built yet:

1. **Template Builder builds the flights for both.** Air Tasking would take your Strike or CAP group as a sortie: it would start the group itself and free its budget slot from your `MISSION END` / `ALL DESTROYED`. Your route, attack and go-home logic would stay as they are.
2. **Your target types become Air Tasking targets:** tanks, trucks, rail, ships, airbases, bridges and dams, next to Map objectives and attack arrows. Today Air Tasking targets only objectives and arrows.

The app would also warn when one map uses both, since their aircraft add up with no shared limit.

## If you ever wanted one system

With the two pieces above built and tested, Air Tasking could cover the jobs of Map › Missions and Exclusive Activation. I am not asking for this; it is here so you can judge it.

| Your piece | What would do that job in Air Tasking |
|---|---|
| Exclusive Activation: one plan runs at a time | The budget per side. A budget of 1 limits each side to one live sortie, but both sides can fly at once; one lock across both sides would need more work. |
| A plan's Zone In starts it when a player enters | The area round the target goes HOT when N players of the other side are inside. |
| The 3-hour window reopens the other plans if a plan never finishes | The copy's hard stop, then its Delete, frees the slot. |
| Map › Missions snaps a strike onto the next unused target of a type | A pool entry with a target of that type (piece 2). |
| `MISSION END` / `ALL DESTROYED` ends a strike | Air Tasking would hook them to free the slot (piece 1). |
| Two or more plans share Exclusive Activation, split onto separate targets | Several pool entries, each with its own target and weight. |

**Other missing parts:** the one lock across both sides above, and an "always on" area. Air Tasking's clock launches only over a HOT area, so a plan meant to run with no player nearby would need that option.

## Your work in the build

Your commits through `fee0a79` (2 November) are merged.
- The fork keeps DPRK player spawns on **501**, because the areas watch 501 and 601. Your `COUNTRY_DPRK` was 502.
- Air starts sit under your `Airfields` / `DPRK` or `NATO` groups, as in your commit.

## What I am asking

- Do the two joining pieces (your Strike and CAP as Air Tasking sorties, your target types as its targets) belong upstream?
- Is Air Tasking welcome as a second mode, or would you rather it stay in the fork?
- If yes to either: a short design note first, a branch to try, or small PRs?

Air Tasking lives in its own module (`src/airtask/`) and two UI files, and hooks into Map Generate. With an empty Air Tasking pool, Map Generate adds no Air Tasking logic and moves no AI off its country; a test guards that. Your own changes above still apply. I am happy to send flight logs, test missions or Tacview tracks.
