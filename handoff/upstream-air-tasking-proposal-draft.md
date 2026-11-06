# Proposal for `eszyman/IL2MissionUtility`: Air Tasking (draft, not sent)

> Status: **draft for the user's review. Nothing has been sent.** Direction: decision 0060 (2026-11-05), which supersedes 0026's framing. Send it after the alpha has logs from real operations, and only when the user says so. Suggested place: a new issue, linking #3 (flight 2N). Before sending, update the "Flown" lines with the alpha's results and 2N's if it has flown.

---

**Title:** Proposal: Air Tasking, a player-driven mode for AI air activity, beside Exclusive Activation

Hi again. You have seen Air Tasking in #3, the four-pilot test mission. The fork has now flown it in real operations, so I would like to explain what it is and ask whether any of it belongs upstream.

## What Air Tasking does

It runs a side's AI air activity from where players fly. You mark targets on the Map, and Generate writes the logic that decides, during the mission, **when** each side's sorties launch, **against what**, and **how many** fly at once.

- **Areas round targets.** Each target gets an area. When N players of the other side (N from 1 to 3) fly in and stay through a short dwell, the area goes HOT. Sorties go only where an area is HOT, so the AI attacks where players are.
- **Two ways to launch.** A mission clock per side, with random timing and a chance of a quiet slot; and a roll each time a check finds the area HOT. Each sortie in the pool has a weight and says which of the two can launch it.
- **A budget per side.** At most a set number of sorties alive per side. A sortie that ends frees its slot.
- **A shell round every copy.** On-station time, one extension while the area stays HOT, the trip home when it goes COLD, an early end when the flight is lost, a player count per area that ends a copy when its last player leaves, and a clean Delete 60 seconds after the end.
- **Only players start it.** Its own AI flies on countries the areas do not count.
- **UI.** An Air Tasking tab (pool, weights, budget, slot timeline, plan checks), an Air tab in the Map dock (auto-fill from objectives, drawn areas and count circles), and Map Generate writes it into the base map. A trace build and a Replay window read the server's text log afterwards and show what fired.
- **Flown:** probe flights checked each piece of logic the game had to honour, acceptance flights ran the whole cycle in IL-2 Korea 1.005, and the fork's alpha has flown it in operations. *(Update with the alpha's and 2N's results before sending.)*

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

## What I am asking

- Do you want the two joining pieces (your Strike and CAP as Air Tasking sorties, your target types as its targets) upstream?
- Is the whole of Air Tasking welcome as a second mode, or would you rather it stay in the fork?
- If yes to either: a short design note first, a branch to try, or small PRs?

Air Tasking lives in its own module (`src/airtask/`) and two UI files, and hooks into Map Generate. A mission without Air Tasking is written exactly as before; a test guards that. I am happy to send the flight logs, the test missions, or a build.
