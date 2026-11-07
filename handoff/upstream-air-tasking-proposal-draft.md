# Issue for `eszyman/IL2MissionUtility`: try v0.8-Alpha, and the Air Tasking proposal (draft, not sent)

> Status: **draft for the owner's review. Nothing has been sent.**
> - Decisions: 0060 (two modes, the replace-it table) and 0063 (send the alpha build and the proposal now, in one issue).
> - Post it as a new issue on `eszyman/IL2MissionUtility`, only at the owner's word, after the `v0.8-Alpha` release is up. Check the release link first.
> - Earlier posts: #2 (load test), #3 (flight 2N, still to be flown).

---

**Title:** Try Air Tasking (fork v0.8-Alpha), and a proposal: a player-driven mode beside Exclusive Activation

Hi again. You have seen Air Tasking in #3, the four-pilot test mission. The fork now has an alpha build with it, and with your work through 2 November merged in, so here is a build to try and a question about where it should live.

**Build:** https://github.com/Pliskin-Industries/Claude-IL2MissionUtility/releases/tag/v0.8-Alpha. `IL2MissionUtility-v0.8-Alpha.exe` is the whole app; the `.sha256` file is beside it.

## What Air Tasking does

It runs a side's AI air activity from where players fly. You mark targets on the Map, and Generate writes the logic that decides, during the mission, **when** each side's sorties launch, **against what**, and **how many** fly at once.

- **Areas round targets.** Each target gets an area. When N players of the other side (N from 1 to 3) fly in and stay through a short dwell, the area goes HOT. Sorties go only where an area is HOT, so the AI attacks where players are.
- **Two ways to launch.** A mission clock per side, with random timing and a chance of a quiet slot; and a roll each time a check finds the area HOT. Each sortie in the pool has a weight and says which of the two can launch it.
- **A budget per side.** At most a set number of sorties alive per side. A sortie that ends frees its slot.
- **A shell round every copy:**
  - on-station time, and one extension while the area stays HOT;
  - the trip home when it goes COLD;
  - an early end when the flight is lost;
  - a player count per area, which ends a copy when its last player leaves;
  - a clean Delete 60 seconds after the end.
- **Only players start it.** Its own AI flies on countries the areas do not count, and the Map moves its other AI off the players' countries (601 and 501).
- **UI:**
  - an **Air Tasking** tab (Ctrl 7): pool, weights, budget, threshold, count, schedules, plan checks;
  - an **Air** tab in the Map dock: auto-fill from objectives, areas and count circles drawn;
  - Map Generate writes it into the base map.
  - A **Trace build** and the **Replay** window read the server's text log afterwards and show what fired.
- **Flown** in IL-2 Korea 1.005, on a test PC with one player at a time:
  - areas going HOT and COLD;
  - zone and clock launches, quiet slots (28 of 120);
  - the budget refusing a third sortie;
  - the on-station end and one extension on both sides, a Yak and an F-80 sortie;
  - the player count ending a sortie, and deletes;
  - a mission made end to end by Map Generate;
  - the standalone group imported in the editor.
- **Not yet proven:** two or more players (#3 is that test), and load time and tick delay on a real server with sorties flying.

## Trying it (one pilot is enough)

1. On the **Map** tab, set up a base map as usual, with one objective on each side. Those are the areas.
2. In the Map dock's **Air** tab, click **Auto-fill from objectives**. It adds sorties for both sides.
3. Give yourself an air start on your side, **outside** the player-count circles. They are 50 km by default; set **Count radius** to 20 km on the Air Tasking tab to keep things close. Generate refuses a Map spawn inside a circle.
4. Tick **Trace build** and **Generate Base Map**, then make the mission as you usually do.
5. Host it, spawn, and fly into an area on the **other** side (a NATO pilot over a DPRK objective, or the other way round). Stay within a few km of it for two to three minutes. A zone start sends a flight from 30 km out, so it arrives a few minutes later.
6. Afterwards, open **Replay** (it reads the server's `missionReport` text logs and the build's `.trace.json`) to see what fired. Or send me the logs.

The manual (Help, F1) has the details under **Air Tasking** and **Trace builds and replay**.

## How it fits with what you built

I see Air Tasking as **a second mode next to Exclusive Activation, not a replacement.** Exclusive Activation suits scripted plans that run one at a time; Air Tasking suits a mission whose air war follows the players. A mapper picks the one that fits each mission.

Two pieces would join them:

1. **Template Builder builds the flights for both.** Air Tasking would take your Strike or CAP group as a sortie: it starts the group itself and frees its budget slot from your `MISSION END` / `ALL DESTROYED`. Your route, attack and go-home logic stay as they are.
2. **Your target types become Air Tasking targets:** tanks, trucks, rail, ships, airbases, bridges and dams, next to Map objectives and attack arrows.

The app would also warn when one map uses both, since their aircraft add up with no shared limit.

## If you ever wanted one system

Air Tasking can do each job of Map › Missions and Exclusive Activation. I am not asking for this; it is here so you can judge it.

| Your piece | What does that job in Air Tasking |
|---|---|
| Exclusive Activation: one plan runs at a time | The budget per side. A budget of 1 is the same lock; a higher one lets more flights run at once. |
| A plan's Zone In starts it when a player enters | The area round the target goes HOT when N players of the other side are inside. |
| The 3-hour window reopens the other plans if a plan never finishes | The copy's hard stop, then its Delete, frees the slot. |
| Map › Missions snaps a strike onto the next unused target of a type | A pool entry with a target of that type. |
| `MISSION END` / `ALL DESTROYED` ends a strike | Air Tasking hooks them to free the slot. |
| Several plans in one file, split onto separate targets | Several pool entries, each with its own target and weight. |

**One gap:** Air Tasking's clock launches only over a HOT area, so a plan meant to run with no player nearby would need an "always on" area option. It does not exist yet.

## Your work in the build

Your commits through `fee0a79` (2 November) are merged.
- The fork keeps DPRK player spawns on **501**, because the areas watch 501 and 601. Your `COUNTRY_DPRK` was 502.
- Air starts now sit under your `Airfields` / `DPRK` or `NATO` groups, as in your commit.

## What I am asking

- Do the two joining pieces (your Strike and CAP as Air Tasking sorties, your target types as its targets) belong upstream?
- Is Air Tasking welcome as a second mode, or would you rather it stay in the fork?
- If yes to either: a short design note first, a branch to try, or small PRs?

Air Tasking lives in its own module (`src/airtask/`) and two UI files, and hooks into Map Generate. A mission without Air Tasking is written exactly as before; a test guards that. I am happy to send flight logs, test missions or Tacview tracks.
