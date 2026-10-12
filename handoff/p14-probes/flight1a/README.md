# P14 probe flight 1A files

Written by `cargo test --offline write_p14_probe_1a -- --ignored` on `claude/p14-air-tasking` at `8fb376a` (2026-10-12). Breadcrumbs are C0/S0 objectives.

| File | Use |
|---|---|
| `P14_Probe_1A_traced.Group` + `.eng` | The probe that was flown on 2026-10-12. |
| `P14_Probe_1A_traced.trace.json` | Sidecar for `il2_mission_utility replay --trace`. |

## How the flown mission was built (2026-10-12)

```
python tools/p14/build_probe_mission.py --game "C:\Program Files\IL2Series\game" --probe target/p14/P14_Probe_1A_traced.Group --ref "TemplateExamples/K14 AFB_mp.Group" --name P14_Probe_1A_fly1_20261012 --title "P14 probe 1A flying check 2026-10-12" --out target/p14/missions/P14_Probe_1A_fly1_20261012 --spawn-planes f80c10
```

Then one hand edit, because the builder only writes ground starts: in the built `.Mission`, the K-14 spawn plane entry was changed from `StartType = 1;` `Altitude = 500;` `Name = "Grouund Start";` to `StartType = 0;` `Altitude = 3050;` `Name = "Airstart";`, and `MissionResaver.exe` was run again on the file. The game's own dogfight test mission uses the same three values for its air starts. The air start spawns over the airfield at that altitude, heading along the airfield's `YOri` (north for K-14), at about 460 km/h indicated.

Results: `handoff/P14-probe-results.md`. Logs: `src/testdata/missionlog/real_5/`.
