# Flight 1P frozen generator inputs (step 0k)

Written once on 2026-10-14, after decision 0016 (`8210f64`), by the ignored
`freeze_flight1p_inputs` test in `src/airtask/probe.rs`:

```
cargo test --offline freeze_flight1p_inputs -- --ignored
```

The writer checks every destination before generating anything and uses
`File::create_new` for each file. Never overwrite these inputs. The ordinary
`flight1p_frozen_inputs_match_current_generators` test regenerates all thirteen
groups in memory and compares their serialized bytes with these files.

Flight 1P is the new step 0k probe. Flight 1N remains the flight flown on
2026-10-12; its generator and files are separate.

## T-f: one four-aircraft Fighter Pack flight

`FighterPack_F80C_4ship.Group` is the unedited output of the normal app path:
`pack::builtin_template`, `flights::configure_aircraft`, then
`pack::generate_pack` with one group. `FlightConfig` uses one flight, maximum
four aircraft, type `f80c10`, recommended skill 2 and country 601. The remaining
settings are defaults: cooldown 180 s, cleanup 60 s, altitude range 1000 to
5500 m. The four seats are Independent, with the generated AttackArea/Cover
orders and `AiRTBDecision = 0`.

Facts read from this generated file:

- `DeathCount`: Counter 4, Dropcount 1; its output starts `COOLDOWN` (180 s).
- Replacement: `COOLDOWN` -> `Randomizer:INPUT` -> `Random 1:100% 500ms`
  -> `Out 1` -> `Spawn 1`. There is no zone or player requirement. The
  randomizer closes its input and reopens it and its output after 1 s.
- Bring-up: `SPAWN UNITS` (0.5 s) -> `Randomizer:INPUT`.
- On each aircraft n, event 7 -> `Mission Complete n` (0.5 s) -> `CLEANUP n`
  (0.1 s) -> `Force Complete n`, `RTB DELAY n` (0.5 s) and `Delay Delete n`.
- Event 13 -> that aircraft's `Delay Delete n` (60 s) -> `Deactivate n`
  and `DeathCount`. This is decision 0016.

The probe places three copies. F1 retains this wiring. F2 removes the four
event 7 hooks. F3 also points the four death events back to `DeathCount`.
All other generated nodes, properties and links are retained apart from the
common placement, plane names, disconnected Mission Begin and added listeners.
Each row's 03:00 timer fires four level-2 Damage commands, one per aircraft
entity. F2 should count once about 60 s after the kills, then replace after
the cooldown; F3 should count at the kills and again about 60 s later. The
second F3 count restarts its running cooldown.

## T-b: the twelve Template Builder aircraft

Each file is unedited `generate_template` output: one Independent seat,
Activate mode, no orders or event hooks, all other options at their defaults.
The source unit is selected from `template::bundled_catalog()` by script file
name in the order below. `simpleb29` is deliberately excluded. A missing
script fails with the flight, row and script name.

`TemplateSeat::new` retains each catalog unit's settings. The standard
`apply_plane_start(PlaneStart::Air)` helper gives every selected unit the
template's default air-start height, 1500 m. Countries remain 601 for B1-B6
and 503 for B7-B12; all have full fuel. Probe copies retain this output and
only change placement and plane names, disconnect Mission Begin, and add
listeners for events 7, 8, 9, 13 and 5.

| Row / file | Aircraft | Script file |
|---|---|---|
| B1 / `B1.Group` | F-80C-10 | `f80c10.txt` |
| B2 / `B2.Group` | F-84E | `f84e.txt` |
| B3 / `B3.Group` | F-86A-5 | `f86a5.txt` |
| B4 / `B4.Group` | F-51D | `f51d.txt` |
| B5 / `B5.Group` | B-29 | `b29.txt` |
| B6 / `B6.Group` | C-47B | `c47b.txt` |
| B7 / `B7.Group` | MiG-15bis | `mig15bis.txt` |
| B8 / `B8.Group` | La-11 | `la11.txt` |
| B9 / `B9.Group` | Yak-9P | `yak9p.txt` |
| B10 / `B10.Group` | Il-10 | `il10.txt` |
| B11 / `B11.Group` | Tu-2 | `tu2.txt` |
| B12 / `B12.Group` | Li-2T | `li2t.txt` |

T-h reads the existing `upstream_a31ad0a` strike files; no additional source
fixtures are needed. `probe_cells_match_the_table` checks every 1P row against
literals and compares each placed copy with its source after undoing only
the allowed changes. The remaining probe guards cover both plain and traced
1P files, including all 220 subtitle observations and their breadcrumbs,
shapes, links and locale.
