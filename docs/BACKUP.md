# Backup and restore

The project holds no user data, accounts or secrets. Everything it needs is in this git repository, and each copy of it is a full backup.

## Where the copies are

| Copy | What it holds |
|---|---|
| `origin`: `github.com/Pliskin-Industries/Claude-IL2MissionUtility` | Every branch and tag; releases with their exe |
| Each local clone (the owner's PC and any worktree) | The full history up to its last fetch |
| `upstream`: `github.com/eszyman/IL2MissionUtility` | The upstream author's code, as far as this fork has merged it |

## What lives in the repository

Everything a fresh reader needs is committed:
- the code and its tests;
- the record: `STATE.md`, `docs/decisions/`, `handoff/sessions/`, `docs/il2-loader-and-log-facts.md`;
- the frozen probe and flight files: `handoff/p14-probes/`, `src/testdata/p14_probe_inputs/`;
- the game's text logs of every flight: `src/testdata/missionlog/real_*`;
- the Tacview tracks, zipped: `handoff/tacview/`.

A flight is copied into the repository as soon as it is read: its logs (`cp -p` from the game's `data\logs\txt`) and its zipped `.acmi`.

## Not backed up (rebuilt or not needed)

- **Missions built from frozen files.** `.Mission` files in the game's `Dogfight` folder and in `target/` are rebuilt from the frozen files with `tools/p14/build_probe_mission.py`.
- **The game's own `.trk` tracks.** These are about 750 MB each; the zipped `.acmi` holds what the flights are read from.
- **Folders outside the repository**, such as `C:\Machine Intelligence\P14 Flight 2N\`. They hold copies of committed files for sending.
- **Claude's local memory.** Anything another person needs is copied into the repository (CLAUDE.md, "Keeping the record").

## Restore

1. `git clone https://github.com/Pliskin-Industries/Claude-IL2MissionUtility.git`
2. Add upstream: `git remote add upstream https://github.com/eszyman/IL2MissionUtility.git`
3. Install the Rust toolchain (rustup) and the Visual Studio C++ Build Tools, then run `cargo fetch`.
4. Check the clone: `cargo test --offline` (the last recorded count is in `STATE.md`).

If GitHub were lost, push any local clone to a new remote. The release exe can be rebuilt by pushing a `v*` tag (`.github/workflows/release.yml`) or with `cargo build --release`.
