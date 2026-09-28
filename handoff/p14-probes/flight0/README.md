# P14 probe flight 0 files

Written by `cargo test --offline write_p14_probe_0 -- --ignored` on
`claude/p14-air-tasking` (2026-09-28, after the block-shape and loader-safe string fixes; all load in MissionResaver). Copies of `target/p14/`, committed so
the mission can be built on another machine.

| File | Use |
|---|---|
| `P14_Probe_0_traced.Group` + `.eng` | **Fly this one.** Import first into a blank Korea mission, then `TemplateExamples/K14 AFB_mp.Group` (NATO spawn). Do not move either group. |
| `P14_Probe_0_traced.trace.json` | Sidecar for `il2_mission_utility replay --trace`. Not needed in the game. |
| `P14_Probe_0.Group` + `.eng` | Plain file, only for a re-fly if T-e or T-j look wrong. |

Procedure: plan §8 step 0, "0c", and the flight 0 run sheet in
`handoff/P14-air-tasking.md`. Server: `mission_text_log = 1` under
`[KEY = system]` in `startup.cfg` (the key exists in the game's own
`startup.cfg`, default 0). Files land in `data\logs	xt\`
(`text_log_folder`). Copy them to `target/p14/logs/flight0/` afterwards.

If the probe code changes, regenerate and copy again; these files are not
checked against the generator by any test.
