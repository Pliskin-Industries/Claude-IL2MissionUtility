# Frozen flight 1L inputs

`W1.Group` through `W6.Group` are the six raw `generate_template` results for
flight 1L's T-w cell. They were written with the app's `serialize_group` at
generator commit `a3b8790f3e286ffa0de7757240ac54b55332ed29`, before any probe
re-indexing, placement, event, Delete or cooldown edits.

Flight 1L was flown from these inputs on 2026-10-13. Its flown files are in
`handoff/p14-probes/flight1l/`. W1-W4 use Spawn with multiple spawns enabled;
W5-W6 use Activate. Each has two independent F-80Cs with the options recorded
in `src/airtask/probe.rs`.

These files are never regenerated or edited. The probe parses them, then
applies the original flight 1L transformations to the in-memory copies, so
later changes to the live generator cannot change the flown probe's inputs.

The ignored `airtask::probe::freeze_flight1l_w_inputs` test produced these
files. It refuses to overwrite an existing file. Do not re-run it after the
generator changes, even if the files are missing: doing so would replace the
frozen inputs with different bytes. Restore missing files from version control.
