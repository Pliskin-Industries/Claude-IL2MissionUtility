# Handoff 2026-10-14: build decision 0016 and fly flight 1N

For the next session, which starts fresh. The user approved on 2026-10-14: decision 0016 ("0016 acceptable") and flight 1N ("1N go"), and asked for this handoff because the old session's context was long.

## Read first, in this order

1. `STATE.md` (where everything stands).
2. This note.
3. `handoff/P14-air-tasking.md` §8, **Step 0k** (flight 1N's spec), and step 0j above it.
4. `handoff/P14-step-0k-codex-prompts.md` (the two build jobs, ready to send).
5. `handoff/P14-probe-results.md`, sections "Flight 1L" and "Flight 1M", and the summary table at the end.
6. `docs/decisions/0012…` to `0016…`.
7. `handoff/upstream-issue-draft.md` (not sent).
8. `docs/il2-loader-and-log-facts.md` (the last two sections hold this week's traps).
9. `handoff/sessions/2026-10-13-delete-probes.md` (how a no-pilot probe is run).

## The work, in order

| # | Step | Who | Done when |
|---|---|---|---|
| 1 | Job 1 of the prompts file: decision 0016 in `flights.rs` | Astra | Claude has run the checks below, read the baseline diff (only death-event `TarId` lines in `fighter.Group`), committed on `codex/fix-0016`, merged `--no-ff` into `claude/p14-air-tasking` |
| 2 | Job 2: flight 1N | Astra | the same checks; Claude's spec check of the generated file (script below); a standards review by a subagent; fixes if any; committed and merged |
| 3 | Fly flight 1N | Claude, at the keyboard | **ask the user that the screen is free first.** Logs copied to `src/testdata/missionlog/real_18/` (and `real_19/` for a second run) |
| 4 | Write up | Claude | results section in `handoff/P14-probe-results.md`; flown files in `handoff/p14-probes/flight1n/`; facts doc; decision records; `STATE.md` |
| 5 | The upstream issue | Claude, then the user | section 3's fix confirmed or changed by 1N; Fighter Pack's event 7 result added if it applies upstream (Fighter Pack is upstream code too); a `/hfe-mayer` pass on the whole text; **the user says when to post.** Nothing is sent without that |
| 6 | Fighter Pack's event 7 hook | the user decides | if flight 1N shows jet Fighter Pack flights going home at once, write decision 0017 (Proposed) with options, for example: remove event 7 from `FIGHTER_EVENTS`, or keep it only for piston aircraft |

## How Astra is run here

- Broker tool `codex_start` with `cwd` = `C:\Machine Intelligence\Claude-IL2MissionUtility-0e`, `model` = `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500. The prompt says: read `handoff/P14-step-0k-codex-prompts.md` and do job N only. Create the job's branch in that worktree from `claude/p14-air-tasking` first, with a clean tree.
- To wait without polling, run in the background: `until [ -f "$HOME/.codex-broker/jobs/<job_id>/last-message.txt" ]; do sleep 5; done` and read `codex_result` when notified.
- Astra edits the working tree only; Claude verifies and commits (decision 0005). Commit messages end with the attribution lines. Nothing is pushed.

## Checks after every job

```bash
cd "C:/Machine Intelligence/Claude-IL2MissionUtility-0e"
cargo test --offline 2>&1 | grep -E "^test result|FAILED|panicked"
cargo build --offline 2>&1 | grep -E "generated [0-9]+ warnings|^error"
cargo clippy --offline 2>&1 | grep -E "generated [0-9]+ warnings|^error"
cargo test --offline write_p14_probe_ -- --ignored 2>&1 | grep -E "^test result"
WF="C:/Machine Intelligence/Claude-IL2MissionUtility-fix"; same=0; d=0
for f in "$WF"/handoff/p14-probes/*/*; do b=$(basename "$f"); [ "$b" = README.md ] && continue
  if [ -f "target/p14/$b" ]; then if cmp -s "$f" "target/p14/$b"; then same=$((same+1)); else d=$((d+1)); echo "DIFF $b"; fi; fi; done
echo "flown files identical=$same differing=$d"
```

On 2026-10-14 the last run gave 573 passed, 0 failed, 17 ignored; 69 build and 251 clippy warnings; 40 flown files identical (flights 0 to 1M). Warnings must not rise.

## Spec check of a generated probe

A throwaway script was used in the old session to list, from the generated `.Group`, every Damage, Delete, Deactivate and Activate with its firing time (walking timers from Mission Begin) and its Objects by name. It is not in the repo. Rewrite it in a few lines if needed, or ask a subagent to trace the schedule; then compare it line by line with the plan's tables before flying.

## Flying a no-pilot probe on this PC

- The game is `C:\Program Files\IL2Series` (1.005, `261013`). The launcher ("Korea. IL2 Series") signs in by itself; press Play, wait about two minutes, click once on the hangar to show the menu. The resaver crashes since 1.005: build with `--no-resave`, copy the `.Mission` and `.eng` into `game\data\Multiplayer\Dogfight\`.
- Build: `python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1N_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --spawn-planes f80c10 --name P14_Probe_1N_<date> --title "P14 probe 1N <date>" --out target/p14/missions/P14_Probe_1N_<date> --no-resave`.
- Host: Multiplayer (688,790), Create (1194,781), move the pointer to (700,300) and scroll down 8 **over the left pane**, take a screenshot, then Clear, pick the mission, Add to rotation, read the rotation, Continue (1367,781). Coordinates move as the mission list grows: always click from a fresh screenshot. Check that "Available from internet" and "Allow global leaderboard" stay unticked (a click once landed on the leaderboard box when the page scrolled back).
- "#31: Master server response timeout" may appear on the briefing screen; close it. The local server runs normally.
- Nobody spawns. The mission clock: newest log in `game\data\logs\txt`, last `T:` value, seconds = (T − 3) / 50. The session name is when loading began, 10 to 90 s before the clock started.
- Leave after 14:00 and before the 16:00 close: Leave Server (88,781), hold Enter 1 s twice; about 15 s later on the results screen, Leave Server and hold Enter twice **at once**. A late confirm once froze the game on "0 % please wait"; it closed cleanly with a window-close request (PowerShell `(Get-Process IL2Series).CloseMainWindow()`), never a kill. If the server reloads the mission instead, let it run as a second sample and leave from the next briefing.
- Bring the game forward before keys: PowerShell `(New-Object -ComObject WScript.Shell).AppActivate((Get-Process IL2Series).Id)`. Astra's and cargo's windows take focus; do not fly while a job runs.
- Replay: `target/debug/il2_mission_utility.exe replay --group target/p14/P14_Probe_1N_traced.Group --trace target/p14/P14_Probe_1N_traced.trace.json --logs <log copy folder> --out <report.md>`.

## Do not

- Push, merge to `main`, or post the upstream issue without the user.
- Edit anything under `src/testdata/p14_probe_inputs/` or re-run its writers: flown probes regenerate from those files.
- Run the mission builder without `--no-resave`, or force-close the game.

## Open with the user after 1N

- The upstream issue: post it or not, once final.
- Decision 0017 on Fighter Pack's event 7 hook, if 1N shows the fault.
- The six built-in sorties' Delete lists (Air Tasking corrects them on placed copies; whether to change the files).
- The overlap with upstream's strike and CAP placement (`STATE.md`, "Waiting on the user", item 4), before the upstream merge.
