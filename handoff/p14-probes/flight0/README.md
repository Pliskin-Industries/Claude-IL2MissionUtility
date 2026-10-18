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
`startup.cfg`, default 0). Files land in `data\logs\txt\`
(`text_log_folder`). Copy them to `target/p14/logs/flight0/` afterwards.

If the probe code changes, regenerate and copy again; these files are not
checked against the generator by any test.

## Without the editor (built 2026-09-28)

`tools/p14/build_probe_mission.py` builds the flight as a finished Korea
dogfight mission: the Options block of the game's own
`data/Multiplayer/Dogfight/_test_dogfight_IL-3.Mission` (MissionType 2),
the probe (indexes unchanged, so the `.trace.json` still matches), and
`K14 AFB_mp.Group` renumbered from 1000 with its language table merged. It
then runs `bin/resaver/MissionResaver.exe` for the `.msnbin`, the six
language files and the `.list`. Checked after the build: all indexes unique,
every link resolves, all 51 breadcrumbs present, every text id in the table.

```
python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_0_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_0_traced --title "P14 probe flight 0 (traced)" --out "<folder>" --spawn-planes f80c10,f86a5
```

`K14 AFB_mp.Group` has no `Planes` list (harvested airfields are stripped),
so nobody can spawn. `--spawn-planes` gives every reference airfield without
one the ground-start entries of the game's own dogfight test airfield of the
same country (default `f80c10`; that NATO airfield has no ground-start `f51d`).

Deploy: copy `P14_Probe_0_traced.msnbin`, `.list` and the six language
files (`.eng` `.chs` `.fra` `.ger` `.rus` `.spa`) into the server's
`data\Multiplayer\Dogfight\` and add `Multiplayer/Dogfight/P14_Probe_0_traced`
to the server's mission rotation.
