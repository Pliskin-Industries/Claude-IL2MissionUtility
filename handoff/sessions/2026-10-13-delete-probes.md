# Session 2026-10-13: ceilings, and the Delete probes (flights 1H, 1J, 1K)

Continues `2026-10-12-il2-korea-on-this-pc.md`. The user's instruction for the day: start the game, fly flight 1H, change the ceilings (all twelve), and carry on.

## What was done

| Item | Where | State |
|---|---|---|
| Ceilings in `src/model_spec.rs` | commit `71a63b4`, merged `10cd990` | done |
| Flight 1H (step 0f) | results and logs `real_9` to `real_11` | flown, three runs |
| Flight 1J (step 0g) | results and logs `real_12`; code merged `b7c0b57` | flown, one run |
| Flight 1K (step 0h) | plan step 0h, `handoff/P14-step-0h-codex-prompt.md`, branch `codex/p14-0h` | being built by Astra when this note was written |
| Decision 0013 | `docs/decisions/0013-…` | Proposed, the user's to rule on |

Astra jobs today, model `gpt-6-astra`, effort max: `20261013175138-4e28ff2a` (ceilings), `20261013185635-342c009d` (flight 1J), `20261013193236-884f0aa3` (1J quality pass), `20261013194705-450a8aa7` (flight 1K).

Reviews: flight 1H had a standards and a spec review by subagents. Flight 1J had a standards review by a subagent and a spec check by Claude with a script that traces every command's firing time and Objects from the generated file (`scratchpad/check_1j.py`, not kept in the repo). Use the same check on flight 1K before flying it.

## How a no-pilot probe is run now

1. Build with `--no-resave` and copy the `.Mission` and `.eng` into the game's `data\\Multiplayer\\Dogfight\\`.
2. In the game: Multiplayer, Create, scroll down 8, Clear, pick the mission, Add to rotation, Continue. The list of missions is read when the Create screen opens; go Back and Create again after adding a file.
3. Stay on the briefing screen. Nobody needs to spawn: the mission runs and the log is written.
4. The log session name (`missionReport(<date>_<time>)`) is the mission's start time to within a few seconds. Read the newest log file's last `T:` to see the mission clock (seconds = (T − 3) / 50).
5. Leave Server and hold Enter twice; about 15 s later the results screen appears; Leave Server and hold Enter twice again **at once**. The end-of-mission list is written when the mission ends, so leave before the cell's close time.

## Left open when this note was written

- **The game was stuck on "0 % please wait"** after the leave at the end of flight 1J (the second Leave was confirmed as the server began to reload the mission). The process was responding and busy. It had not been closed. The plan was to ask it to close the normal way (a window-close request, not a kill) and start it again from the launcher; the user says the launcher now signs in by itself.
- **The launcher's download source was left on S3** (it was Cdn).
- **`codex/p14-3` (step 3, held)** still counts losses on type 4 and uses the template's own Delete. Both must change before it merges.
