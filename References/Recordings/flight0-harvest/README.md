# P14 flight 0: what the video shows

Source: `references/Recordings/2026-10-03 20-46-21.mp4` (OBS, 1920×1080 at 60 fps, 27:27, VR mirror view).
Mission: `p14_probe_0_traced` on the user's dedicated server, session `2026-10-03_20-45-36`.
Log digest from the other machine: `claude/p14-air-tasking`, `handoff/P14-probe-results.md` and
`handoff/P14-probe-replay.md` (fixture `src/testdata/missionlog/real_3/`).

This file covers only what the video adds to the log. Where they agree, the log is the record.

## Clock

**Mission time = video time + 45.5 s** (± 0.3 s).

The offset is fitted from 25 "F2 out k" subtitles against the logged F2 firings. Other events agree with it:
- Player spawn: logged at mission 04:12, which is video 03:26.5. The cockpit is in view from video 03:25.
- Log session start: 20:45:36. Video start: 20:46:21.

| Mission | Video | What happens (log + video) |
|---|---|---|
| 00:00–03:51 | before start – 03:05 | Player in menus or the server list. Nothing in the mission is visible. |
| 04:12 | 03:26 | Spawn at Kimpo (F-80C). The player stays parked until 16:01. |
| 04:30–05:30 | 03:44–04:44 | T-t cell. Player in the cockpit, panel in view most of the time. |
| 05:30–08:50 | 04:44–08:05 | T-f F2. |
| 09:00–09:15 | 08:14–08:30 | T-j (J4, J5). |
| 10:00–10:19 | 09:14–09:34 | T-z. At 10:19 the player switches to SimShaker / Joystick Gremlin; back in game at about 10:43. |
| 11:00 | 10:14 | Last cue. |
| 16:01–27:15 | 15:15–26:30 | Player flying: takeoff 16:01, landing 18:59, takeoff 21:55, landing 26:23, despawn 27:15. No probe events. |

Everything before 04:12 (T-e, ZA1, E-cells, F1 up to 04:03, CUE 01–03) fired while no player was in the mission. The video cannot see it.

## Findings the log does not show

1. **The cue subtitles showed stale text.** The server or client used the language table from the
   earlier T-o build (`5e42ff2`, ids 43–56; copy in `data/T-o-build_lang_43-56.eng`). The flown mission
   (`c87f492`) uses ids 43–50. Ids 2–42 are the same in both builds, so every per-event subtitle was correct.
   Only the eight cues were wrong:

   | Cue | Fired | Should say | Showed | Seen |
   |---|---|---|---|---|
   | CUE 01 | 00:00 | Flight 0: stay within 20 km… | T-o O1 fired | (before join) |
   | CUE 02 | 01:00 | T-e start | T-o O2 fired | (before join) |
   | CUE 03 | 02:30 | T-f F1 start | T-o O3 fired | (before join) |
   | CUE 04 | 04:30 | T-t start: note any objective message… | **T-o O4 fired** | 04:30.4–04:34.6 |
   | CUE 05 | 05:30 | T-f F2 start | **T-o O5 fired** | 05:29.8–05:30.2 |
   | CUE 06 | 09:00 | T-j start | Flight 0: stay within 20 km… | not seen |
   | CUE 07 | 10:00 | T-z start | **T-e start** | 10:00.0–10:15.6 |
   | CUE 08 | 11:00 | Flight 0 done… | **T-f F1 start** | 11:02.9–11:19.5 |

   Fix before flight 1: delete the old `P14_Probe_0_traced.*` language files from the server's
   `data\Multiplayer\Dogfight\` (and from any client cache) before deploying. Then check that one cue shows the right text.

2. **T-t T5 fired.** "T-t T5 fired" is on screen at 05:14.8 and 05:15.6 (one appearance). Its Spawn
   carrier writes no log line, so the digest records T5 only as "no line". The timer itself works. A second
   firing near 05:20 was not seen: the player was looking at the gunsight.

3. **The objective breadcrumbs (C0/S0) put nothing on screen.** From 04:30 to 05:30 the player is in the
   cockpit, and the only message boxes are AI radio chatter (for example "Radio: … search for ground targets
   on your own, over" at 05:23). There is no objective or task popup anywhere in the recording. Map markers
   cannot be checked, because the map was never opened. This is the t3 visibility answer for the cockpit view.

4. **A new subtitle replaces the one on screen.** A cue with `Duration = 20` lasted only until the next
   subtitle fired:
   - O4 was cut by "T-t T1 fired" at 04:34.8, after about 4 s.
   - O5 was cut by "F2 out 3" at 05:31.
   - CUE 06 fired on the same tick as J4, and only "J4 out" appeared.

   The cues ran their full 20 s only when nothing else fired (CUE 07, CUE 08). This matters for any design
   that relies on players reading a long subtitle while other subtitles fire.

5. **Z1 stayed silent, with evidence.** "T-e start" (CUE 07) stays readable from 10:00.0 to 10:15.6. A
   "Z1 fired" at 10:00, or a "Z1 fired again" at 10:15, would have replaced it. This supports the log's
   "z NOT confirmed" result.

## Per-event subtitles after the spawn

**Seen** means OCR or a check by eye found the text in the 1–2 s after the logged firing.
**Not seen** means it was not found; the cause may be that the VR head was turned away.

| Source | Logged | Seen | Notes |
|---|---|---|---|
| F1 hit | 4 (04:12–04:24) | 2 (04:15, 04:24) | 04:12 is the spawn moment |
| T-t T1 | 1 (04:35) | 1 | "T1fired" at 04:34.8 |
| T-t T2 | 5 (04:37–04:45) | 5 | |
| T-t T3a / T3b | 1 + 1 (04:47) | probable | partial read "Tabfired" at 04:47.0 |
| T-t T5 | 0 (no log line) | 1 | see finding 2 |
| T-t T7 | 1 (05:25) | 0 | cockpit in view, radio box on screen; not found |
| F2 out 1 / 2 / 3 / 4 | 9 / 10 / 12 / 9 | 8 / 6 / 6 / 3 | no wrong-number reads |
| J4 out | 1 (09:00) | 1 | 09:00.3, in external view |
| J5 out | 1 (09:15) | 0 | panel in view at 09:15–09:16; not found. Open question |
| Unlogged subtitles | – | none | apart from T5, every subtitle seen matches a log line |

## Files

- `data/ocr_1fps.jsonl`: full-frame OCR of the whole video at 1 fps (`s` = video seconds).
- `data/f5_main.jsonl`: full-frame OCR at 5 fps, video 200–660 s.
- `data/verify.jsonl`: green channel at 2× scale, 5 fps, in the windows listed in `data/windows.json` (mission seconds).
- `data/compare.json`: each logged firing, with the subtitle labels seen in its window.
- `scripts/`: the scripts that made these files. They take the scratch folder as their first argument and
  expect `p14/` copies of the replay report and the `.eng` beside it. ffmpeg is the winget `Gyan.FFmpeg` build.
  OCR is `rapidocr_onnxruntime`.

Querying the data: video seconds + 45.5 = mission seconds. Subtitles are small green text fixed in the cockpit.
At native resolution OCR often misses them. For a definite answer, extract the frame with ffmpeg and look at it.
