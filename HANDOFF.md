# HANDOFF — IL-2 Mission Utility (Claude working copy)

> Read this first in every new session. Update the **Session log** and
> **State** sections before you stop. Last updated: 2026-09-23.

## 1. Where things live

| What | Path |
|---|---|
| **Working copy (all work happens here)** | `C:\Claude\IL2MissionUtility\Claude IL2Mission Utility\` |
| Original drop (read-only reference, do not edit) | `C:\Claude\IL2MissionUtility\` (everything except this folder) |
| Module map for `src/` (read before editing) | `docs/src-guide.md` |
| End-user manual (embedded in the Help window) | `USER_MANUAL.md` |
| Coding rules | `.cursorrules` (nom parser only, schema-agnostic AST, never skip tests, GUI decoupled from AST, minimal UI) |

**Git.** The working copy is a local git repo (`main`). There is **no
GitHub remote** yet. The first commit is the untouched 2026-09-04 source.
`.gitattributes` is `* -text`: files are stored byte-for-byte because `.Group`
fixtures are line-ending sensitive. Do not remove it.

**Build and test.**

```bash
cargo test --offline     # 376 passed after P1 (371 at baseline); ~45 s cold build
cargo build --release    # ships target/release/il2_mission_utility.exe
```

`--offline` works because every crate is already in the local cargo cache.

## 2. Team roles (codex-delegation skill)

| Role | Who | Notes |
|---|---|---|
| Overlord / reviewer | **Claude Opus acting as Fable** (Fable quota exhausted, per user 2026-09-23) | Plans, scopes, reviews every diff, merges, owns accountability. |
| Executor | **GPT-6 Astra via Codex broker**, if the account gets access | All coding work. `max` effort for implementation, `ultra` for reviews. **Not available as of 2026-09-23** (see below). |

**Delegation loop in use.** There is no remote, but Claude and Codex share
this machine's disk. So the loop is:

1. Claude commits on `main`. The primary clone must be clean.
2. Astra `git clone`s the primary clone into its own temp dir, branches
   `codex/<slug>`, implements, runs `cargo test --offline`, and commits
   there. The Codex sandbox cannot write the primary clone's `.git`.
3. Claude runs `git fetch <temp-clone> codex/<slug>:codex/<slug>`, reviews
   the diff, and re-runs the tests itself.
4. Claude merges with `--no-ff` into `main`, or sends it back with `codex_resume`.

Once a GitHub remote exists, switch to the skill's standard `git_push`/`git_pull` flow.

**Astra is not on this OpenAI account (verified 2026-09-23).**
`~/.codex/models_cache.json` lists only `gpt-6-luna` (the CLI default when no
model is set), `gpt-5.6-terra`, `gpt-5.6-luna`, `gpt-5.5`, `gpt-reserve` and
`codex-auto-review`. There is no Astra and no Sol. `config.toml` sets no
`model`, so a delegation that does not pass `model` silently runs **Luna**.
Always pass `model` explicitly, and confirm the model in
`~/.codex/sessions/.../rollout-*.jsonl` (`"model":`). For P1 the user chose
Claude to implement (one-task waiver). Ask again for each new task: Astra
after a plan upgrade, `gpt-5.6-terra`, or Claude.

**Codex install (done 2026-09-23).** The user installs the CLI:
`npm.cmd i -g @openai/codex`, then `codex.cmd login`. Use the `.cmd` forms
because the PowerShell execution policy blocks `npm.ps1`. Then **fully quit
Claude from the system tray** and reopen it. Closing the window is not
enough: the broker caches the binary lookup once per process. It
auto-finds the vendored exe under `%APPDATA%\npm\node_modules\@openai\codex\...`,
which is installed (codex-cli 0.156.1). If the
broker still reports `ENOENT`, set `CODEX_BIN` to the absolute path of the
real `codex.exe`, not the `codex.cmd` shim. Saved delegation prompts live
in `handoff/`.

## 3. What the application is

A native Windows desktop tool (Rust 2024, `eframe`/`egui` 0.32, `nom`
parser) that removes the tedious wiring from building **IL-2 Sturmovik:
Great Battles** missions on the **Korea** map. It reads and writes the mission
editor's `.Group` text files; it does not replace the editor. Version
`0.5.0-alpha`; ~40k lines in `src/` (`ui.rs` 10.2k, `template.rs` 7.9k).

Pipeline: `.Group` text → `parser.rs` (nom) → `ast::Il2Entity` tree (unknown
keys preserved) → generation modules transform/clone the tree →
`serialize.rs` writes it back (CRLF, lossless round-trip) → `locale.rs`
writes the UTF-16 translation sidecars (`.eng`, `.rus`, …).

### The six modes (tabs)

| Mode | What it does | Core modules |
|---|---|---|
| **Template Builder** | Builds one self-contained, proximity-triggered unit group: seats (planes/vehicles/trains/ships), a tree of orders (formation → waypoint → attack → complete), Zone IN/OUT checkzones, spawn/cooldown, cleanup on zone-out. | `template.rs`, `payloads.rs`, `model_spec.rs`, `weapon_range.rs` |
| **Fighter Pack** | Clones a fighter group N times and wires the copies with NodeGates so they take turns spawning. Includes a randomizer, pair skills, finger-four altitudes and tail codes. | `pack.rs`, `flights.rs`, `aircraft.rs` |
| **Exclusive Activation** | Links several pre-built checkzone "plans" so only one fires (mutex). | `bombers.rs` |
| **Army Generator** | Clones ground/ship/train templates and adds a per-type randomizer that decides at mission load which copies spawn. | `recon.rs`, `mapground.rs`, `mapnet.rs` |
| **Map** | Dated Korea front line from a 1950–53 timeline, AO box, salients, arrows, influence areas. Auto-places fighters, ground units, ships, trains and convoys on terrain, roads and rails. Exports a base-map `.Group`. | `frontlines.rs` (+`timeline.rs`), `mapclip.rs`, `mapfighters.rs`, `mapground.rs`, `mapshipping.rs`, `mapnet.rs`, `watermap.rs`, `geo.rs` |
| **Airfield** | Strips single-player clutter from an SP airfield so it is MP-ready. | `airfield.rs` |

`MapHelper/` is a separate small Rust CLI. It pre-bakes
`assets/combined_terrain.bin` (water/road/open bitmask) from map images.

### Current state (verified 2026-09-23)

- All six modes are implemented and wired in the UI.
- `cargo test`: **371 pass, 0 fail**.
- `cargo build`: **70 warnings**, mostly dead code: unused constants,
  `mapload::PointKind`, and pack helpers marked `#[allow(dead_code)]`.
- `mapload.rs` is compiled and tested but **not called from the UI**. Map
  labels use hardcoded `geo::cities_on_map` instead.
- The aircraft identity tables (`aircraft.rs`) cover the 7 Korea fighters only.
- The UI is English-only.

## 4. Next development phases

**Author-stated** (README "Todo"):
- **Rewrite `USER_MANUAL.md`** into plain, human-readable text. The current
  manual is dense and reads as AI-written.
- **Localization** of the app UI. `locale.rs` handles mission sidecars only;
  UI strings are hardcoded in `ui.rs`.

**Proposed by Claude.** These are inferences from the code and need the user's confirmation.

| # | Phase | Why | Size |
|---|---|---|---|
| P1 | **Template altitude defaults** (in progress, see §5) | User request, 2026-09-23. | S |
| P2 | **Hygiene:** clear the 70 warnings (wire or delete dead code); add a GitHub remote | Warnings hide real regressions. A remote unlocks the standard broker flow. | S |
| P3 | **Wire `mapload.rs`** reference catalog (airfields/buildings from `References/`) into Map-mode labels and snapping | Already built and tested, but unused. | M |
| P4 | **Split `ui.rs`** (10k lines) into per-mode panel modules, with no behavior change | Prerequisite for localization. Lowers merge risk for every UI task. | M–L |
| P5 | **UI localization:** string table plus a language picker | Author todo. Easier after P4. | L |
| P6 | **Manual rewrite** (author todo) | Should follow the feature changes so it does not go stale twice. | M |
| P7 | **Template altitude follow-ups** (see §5) | All done or closed (2026-09-23). | S |

## 5. Feature log

### P1 — Auto altitude at 50% of service ceiling (Template Builder)

**Request (2026-09-23):** "allow aircraft under template mode for aircraft
altitude to automatically be set at 50% of operating ceiling."

**Design (Claude, as Fable):**
- **Logic.** `model_spec::auto_altitude_m(script)` returns
  `round(ceiling_m × 0.5)`. For example, MiG-15bis gives 7500 m, F-86A-5
  gives 7620 m, and an unknown aircraft gives 4000 m.
  `template::apply_auto_altitude(_all)` sets the seat altitude and re-derives
  `StartType`, which becomes Airstart.
- **UI.** A checkbox, **Auto altitude (50% ceiling)**, sits in the Template
  "Add unit" row and defaults to **on**. When it is on, new planes and model
  swaps get 50% of their own ceiling. Switching it on re-heights every plane.
  A per-seat **50% ceiling** button next to the altitude slider works
  regardless of the checkbox.
- **Not touched by auto.** Loaded template files, manual slider edits made
  afterwards, Fighter Pack altitudes (`flights.rs`), and "Copy attributes to all".
- **Behavior change to note.** Every catalog plane has `YPos = 0`, so new
  planes used to default to a **ground start**. With auto on (the default)
  they now airstart at half their ceiling. To build a runway or parked
  flight, turn auto off.

**Follow-ups (P7). User rulings 2026-09-23:**
- **Mixed-model formations. RULED: wingmen match their lead.** DONE, merge
  `72106da`. With auto on, a wingman takes its lead's altitude, capped at
  its own ceiling. A ground-start lead keeps its wingmen on the ground with
  the same engine state. Moving the lead's slider moves its wingmen, and
  switching a seat to Follows adopts the lead's height. The per-seat button
  reads **Match lead** on a wingman. 381 tests pass. Smoke-tested: MiG
  lead at 7500 m brings its La-11 wingman to 7500 m (not 5075 m); dragging
  the lead to 1500 m moves the wingman to 1500 m. The ground-start Engine
  choice (Running/Warm/Cold) on a lead now syncs to its wingmen too. It was
  missed at first; fixed in the next merge below.
- **Ceiling clamp on copy. DONE** (user said yes, 2026-09-23). This was a
  pre-existing bug, not a design question. "Copy attributes to all" copies the selected
  plane's altitude onto every other plane without capping it at that
  plane's own ceiling. For example, an F-86 at 12,000 m copied onto an
  IL-10 (6,950 m ceiling) gives 12,000 m. The proposed fix is a one-line
  clamp in `copy_seat_attributes`. Fixed, with 383 tests passing. Merged
  together with the engine sync.
- **Persistence. RULED: default on is correct.** Resetting to on at each
  app start is the intended behavior. No persistence is needed. Closed.

**Status: DONE, merged to `main` 2026-09-23** (`aab880c`, merge `9389c6a`).
Claude implemented it (user waiver: Astra is unavailable on the account).
The tests went from 371 to 376, and the build still has the same 70
warnings, so none are new. It was smoke-tested in the running app: adding
a B-29 gives 5334 m airstart; dragging to 0 m gives a ground start
(Running); the **50% ceiling** button restores 5334 m.

The notes below record the earlier blocker.

~~BLOCKED: Codex CLI is not installed~~, so the design was ready
but not implemented. The broker extension exists (`~/.codex-broker/`), but
there is no `codex.exe` and no `~/.codex/config.toml`. Job
`20260924024246-e38e95ca` failed at spawn with `ENOENT` and changed nothing.
The user chose to install Codex rather than have Claude implement it.
**To resume:** once `codex.exe` resolves, re-send
`handoff/P1-auto-altitude-codex-prompt.md` verbatim with `codex_start`, then
continue at delegation-loop step 3 (§2).

## 6. Session log

| Date | Who | What |
|---|---|---|
| 2026-09-23 | Claude (Fable role) | Created working copy and git baseline (`.gitattributes * -text`). Verified 371 tests pass. Wrote this handoff. Designed P1 and delegated it to Astra (job `20260924024246-e38e95ca`, branch `codex/auto-altitude`). The job failed because the Codex CLI is not installed. The user will install Codex; the prompt is saved in `handoff/`. |
| 2026-09-23 | Claude (Fable role) | Codex was installed, but the broker kept ENOENT because Claude never fully quit (the lookup is cached). Ran the CLI directly and found the default model is **gpt-6-luna**. Astra is not on the account, so I stopped the run before it made any edits. The user chose Claude to implement P1. Built on `claude/auto-altitude`, 376 tests pass, smoke-tested in the app, merged to `main`. An untracked `handoff/R1-historical-reference-codex-prompt.md` (a Korea 1950–53 unit-reference task, not written by Claude) was left untouched and was not run; it waits for the user. |
| 2026-09-23 | Claude (Fable role) | User rulings on P7: wingmen match their lead (built on `claude/auto-altitude-follow-lead`, 381 tests, smoke-tested, merged `72106da`), and auto altitude default on is correct (closed). Explained that the copy-attributes clamp is a bug, not a question; awaiting the go-ahead. |
| 2026-09-23 | Claude (Fable role) | The user asked why the engine choice did not sync: it was an oversight, since only the slider and button were hooked up. Fixed so the Engine choice on a lead syncs to its wingmen, and added the copy-attributes ceiling clamp (user said yes). 383 tests pass, and there are still 70 warnings. Both merged to `main`. |
| 2026-09-23 | Claude Opus (Fable role, parallel session) | **R1, historical reference: done, committed (branch `claude/historical-reference`).** The `handoff/R1-…` prompt came from this parallel session, not from an outside author. It was drafted for Codex, but the user dropped Codex ("misbehaving") and asked Opus to do it. Sources (9 public PDFs) were downloaded with the user's approval into `docs/historical-reference/sources/`. That folder is git-ignored except `SOURCES.md`. Futrell and the Pacific Fleet report were scans, OCR'd with the Windows built-in OCR. Five subagents extracted the data, and every Evidence line was checked by script against its cited page. Outputs: `docs/historical-reference/korea-1950-53-unit-reference.md` (10 sections, ~600 table rows) and `…-evidence.md`. Findings: archive.org's "FM 6-140" file is really FM 6-120, and the Wikimedia DA Pam 30-51 is the 1960 edition. The R1 Codex prompt is now obsolete. **Sources still needed are listed by priority in `handoff/R2-sources-still-needed.md`.** Also added reference section 8.9 on searchlight placement: 4–8 lights per defended area, set along the bomb-run approach, zone-activated, and paired with flak or night fighters, in place of 20–50. |
