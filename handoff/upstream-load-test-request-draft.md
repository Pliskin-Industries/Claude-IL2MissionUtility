# Load-test request to eszyman (posted 2026-10-14 as eszyman/IL2MissionUtility#2)

Venue and file delivery: see the note at the end. Text to post:

---

**Could you run one load-test mission on your server and send the logs back?**

I'm building the Air Tasking tab on the fork (`Pliskin-Industries/Claude-IL2MissionUtility`). It places pre-built AI sorties in a mission, switched off until a trigger starts them. I want to know what that costs a real server before I settle the default limits, and my PC is only a test bed: its load times and tick delay don't say anything about your host.

**The mission:** `P14_Load_2_20261014` (`.Mission` + `.eng`), game version 1.005 (`261013`), Korea, dogfight, Kimpo (K-14) as the base. It holds 24 sorties per side, 240 aircraft in all, every one switched off, plus their logic. **Nothing flies by itself:** the parts that would start them aren't built yet. All it does is load and sit there. A player can join and spawn at K-14 normally.

**Before you start the server, please switch on both logs** in `data\startup.cfg` (then restart the server so it reads them):

```
[KEY = system]
	mission_text_log = 1
[END]

[KEY = track_record]
	tacviewrecord = 1
[END]
```

(Change only those two values; leave the rest of each section as it is.) In the server settings, also tick **TacView record** if it is not already on.

**Steps:**
1. Copy `P14_Load_2_20261014.Mission` and `P14_Load_2_20261014.eng` into `data\Multiplayer\Dogfight\`.
2. Make it the only mission in the rotation, internet visibility as you like.
3. Note the time from starting the mission to the server being ready.
4. Let it run idle for 10 minutes and note the server's tick delay (or SPS) at the start, at 5 minutes and at 10 minutes. If you can, compare with the same numbers for a normal mission of yours.
5. Optional: join, spawn an aircraft at K-14 and fly for a couple of minutes, and say whether anything felt different.
6. End the mission normally.

**Please send back:**
- the mission text logs for that session: `data\logs\txt\missionReport(<date_time>)[*].txt`, every numbered file of that session;
- the Tacview recording (`.acmi`) of that session;
- the load time and the tick-delay numbers from steps 3 and 4, plus a line on the server's CPU and RAM.

A zip attached to this thread is fine. Thanks!

---

**Note (not part of the post):**
- **Venue (user, 2026-10-14):** a new issue on `eszyman/IL2MissionUtility`.
- **Files:** `P14_Load_2_20261014.zip` (the `.Mission` and `.eng`, 90 KB; `target/p14/missions/P14_Load_2_20261014/` in the step 3b worktree; rebuild with `write_p14_load_probe` and `tools/p14/build_probe_mission.py --no-resave`). Claude could not attach it through Chrome (the extension never saw the page as ready, and GitHub blocks fetching a local file), so the issue was posted as text by `gh` and the user attaches the zip in the first comment.
- Where the Tacview file lands on his server is not checked here; the post asks for "the recording of that session" rather than a path.
