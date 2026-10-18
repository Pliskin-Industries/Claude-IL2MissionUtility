# Flight 1M: current generator inputs

X1.Group, X2.Group and X3.Group were written once by `generate_template`
on 2026-10-13, after decisions 0012, 0014 and 0015 were merged into
`codex/p14-0j`. Do not edit or regenerate these inputs to hide a generator change.

Each has two independent F-80C seats, country 601, altitude 3050 m, no
orders or custom events, and the remaining seat/placement defaults.
`spawn_cooldown_min = 1` is supplied as an option before generation.

| File | Bring-up | Multiple spawns | Generated behavior |
|---|---|---|---|
| X1.Group | Spawn | true | Event 13 feeds DeathCount; COOLDOWN is 60 s; Delete lists entities |
| X2.Group | Spawn | true | Same options as X1; a separate named copy for cleanup/restart |
| X3.Group | Activate | false | No DELETE DELAY or Trigger Delete; COOLDOWN retains the generator's 0 s |

The ignored `freeze_flight1m_x_inputs` test writes these files with
`File::create_new` and refuses to overwrite any existing input. It was run once:

```
cargo test --offline freeze_flight1m_x_inputs -- --ignored
```

The normal `flight1m_frozen_inputs_match_current_generator` test regenerates
all three in memory and compares their serialized bytes with these files.

The probe loads the files, reallocates indexes, translates all positions by
the Zone IN offset, disconnects Translator Mission Begin, renames only Plane
blocks, and adds event 13 and timer-output listeners. It preserves every other
generated property and link. The probe supplies startup, cleanup/restart or
kill pulses, detectors, and one final Delete per entity outside each copy.

The upstream strike/CAP evidence is separately preserved in
`../upstream_a31ad0a/`; flight 1L's historical inputs remain in `../flight1l/`.
