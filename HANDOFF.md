# HANDOFF — IL-2 Mission Utility (Claude working copy)

> Read this first in every new session. Update the **Session log** and
> **State** sections before you stop. Last updated: 2026-09-27.

## 1. Where things live

| What | Path |
|---|---|
| **GitHub (source of truth)** | `Pliskin-Industries/Claude-IL2MissionUtility` (public fork of `eszyman/IL2MissionUtility`, remote `upstream`) |
| **Backlog** | **GitHub Issues** on the fork (#1–#27, imported 2026-09-26). §4 below and `handoff/*.md` hold the design detail the issues link to. |
| **Working copy (current machine)** | `C:\Machine Intelligence\Claude-IL2MissionUtility\` (cloned 2026-09-26) |
| Old working copy (previous machine, not present here) | `C:\Claude\IL2MissionUtility\Claude IL2Mission Utility\` |
| Module map for `src/` (read before editing) | `docs/src-guide.md` |
| End-user manual (embedded in the Help window) | `USER_MANUAL.md` |
| Coding rules | `.cursorrules` (nom parser only, schema-agnostic AST, never skip tests, GUI decoupled from AST, minimal UI) |

**Git.** `main` tracks `origin` (the Pliskin-Industries fork). The first
commit is the untouched 2026-09-04 source. Don't push `v*` tags casually:
they trigger the release build.
`.gitattributes` is `* -text`: files are stored byte-for-byte because `.Group`
fixtures are line-ending sensitive. Do not remove it.

**Build and test.**

```bash
cargo test --offline     # 469 passed, 3 ignored at a34a00e (main); 482 passed, 5 ignored on claude/p14-air-tasking (2026-09-27)
cargo build --release    # ships target/release/il2_mission_utility.exe
```

`--offline` works only once every crate is in the local cargo cache. On a
fresh machine, run `cargo fetch` (or one online `cargo build`) first. Windows
needs rustup plus the Visual Studio C++ Build Tools (MSVC linker).

## 2. Team roles (codex-delegation skill)

| Role | Who | Notes |
|---|---|---|
| Overlord / reviewer | **Claude Fable 5.1** when the user runs a Fable session (it ran the P14 review and revision, 2026-09-27); otherwise Claude Opus in the Fable role | Plans, scopes, reviews every diff, merges, owns accountability. Writes code only with the user's permission for that task. |
| Executor | **GPT-6 Astra via Codex broker** | All coding work. `max` effort for implementation, `ultra` for reviews. On the account since 2026-09-27 (see below). The user chose Astra to build P14. |

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

**Update 2026-09-27: Astra is on the account.** `~/.codex/config.toml` sets
`model = "gpt-6-astra"` and the models cache lists it. The P14 review job ran
on it (confirmed in the job's `output.log`). The note below is history.

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

**Tracked as GitHub Issues since 2026-09-26.** Map: P2 #4, P3 #5, P4 #6,
P5 #7, P6 #8, R3 F1–F9 #9–#17, in-game checks #18, harvester in-game test
#19, harvester next phase (Map airfield states) #20, `Developer_AAA.Group`
parse bug #21, P9–P13 #22–#26, R2 sources #27, P14 Air Tasking #28 (probe flights #29),
Historical1950 README AttackArea bug #30. Close an issue when its work
merges, and keep this table for design context only.

**Terrain heights (user direction 2026-09-26).**
- The original creator (eszyman) is building a **macro for 3D terrain
  mapping**. It is now the expected source of the height grid (#1).
- The in-app **probe-snap load-in** (T-34 probe export, editor *set to
  ground*, Import snapped; `heightprobe.rs`) is **deferred for now** (#3).
  Keep it working; don't extend it (no 200 m / rough-100 m pass buttons).
- **Long term:** build the macro into the app (#2).
- P12 terrain-aware placement (#25) is blocked on #1.

**Author-stated** (README "Todo"):
- **Rewrite `USER_MANUAL.md`** into plain, human-readable text. The current
  manual is dense and reads as AI-written.
- **Localization** of the app UI. `locale.rs` handles mission sidecars only;
  UI strings are hardcoded in `ui.rs`.

**Proposed by Claude.** These are inferences from the code and need the user's confirmation.

| # | Phase | Why | Size |
|---|---|---|---|
| P1 | **Template altitude defaults** (done 2026-09-23; **replaced 2026-09-27** by the creator's upstream commit `c1d88c3`: airstart is 1500 m for every aircraft, see §5) | User request, 2026-09-23. | S |
| P2 | **Hygiene:** clear the 70 warnings (wire or delete dead code); add a GitHub remote | Warnings hide real regressions. A remote unlocks the standard broker flow. | S |
| P3 | **Wire `mapload.rs`** reference catalog (airfields/buildings from `References/`) into Map-mode labels and snapping | Already built and tested, but unused. | M |
| P4 | **Split `ui.rs`** (10k lines) into per-mode panel modules, with no behavior change | Prerequisite for localization. Lowers merge risk for every UI task. | M–L |
| P5 | **UI localization:** string table plus a language picker | Author todo. Easier after P4. | L |
| P6 | **Manual rewrite** (author todo) | Should follow the feature changes so it does not go stale twice. | M |
| P7 | **Template altitude follow-ups** (see §5) | All done or closed (2026-09-23). Match-lead and the 50 % ceiling button went with `c1d88c3`. | S |
| P8 | **Features the historical templates need.** Details and acceptance criteria are in `handoff/R3-template-feature-gaps.md`. F1: respawn linked flights on a cooldown. F2: loader round-trip for every spawn layout. F3: break off after N losses. F4: cross-template trigger hooks. F5: date-aware aircraft warning (La-11 in 1950). F6: headless generation via lib/CLI. F7: waypoint speed without the UI. F8: searchlight defended-area preset. F9: per-element altitude step. | Found while building `TemplateExamples/Historical1950/` (2026-09-24). Each one forced a workaround or left out sourced behaviour. **Not scheduled; not in a 0.6 release.** | S–M each |
| P9 | **Mission logic checker** (frontier pick). A "Check" panel that takes any `.Group` and lists broken targets/object links, empty checkzones, MCUs nothing triggers, timer-less loops, and spawn/cooldown settings that will be ignored. A dry-run trigger simulator follows in v0.2. Details: `handoff/R4-frontier-proposals.md`. | The editor has no debugger; today broken wiring is found by flying. `inspect_plan` only checks that a checkzone and a timer exist. F1 shows even the app's own output can fail silently. | M |
| P10 | **Playtest replay.** *Now part of P14 #28 (2026-09-25): the core (log parser, replay, CLI) is P14 step 0T, built before the probe flights together with trace builds; the UI is P14 step 9T.* Parse IL-2's text mission report (ATYPE log) into a spawn timeline and Map-tab overlay; flag templates that never spawned. | Runtime half of P9. Also field-checks the heightmap (underground spawns). Needs text logging on in `startup.cfg`. | M |
| P11 | **Historical order of battle from date.** Reference §2.2/§3/§7 as data; Map-mode "Historical fill" seeds Fighter Pack / Army Generator pickers with date-valid types and writes source citations into group descriptions. | Goes beyond F5 (a warning) to generation. Uses the evidence-checked reference and `timeline.rs`. Coverage is uneven past 1950 (R2), so show a coverage indicator. | M–L |
| P12 | **Terrain-aware placement.** v0.1: terrain clearance along whole flight legs. Later: line of sight for AAA/searchlights (§8.9), reverse-slope CCF positions (§5 D). | Blocked on the heightmap. Store heights as a queryable grid, not only a relief image. Settles the heightmap "Revisit" on AGL waypoints. | M |
| P13 | **Semantic `.Group` diff.** Show what changed between two mission versions (added units, changed radii, retargeted MCUs), matched by index + name. | Cheap on the lossless parser. Niche unless co-authoring MP missions. First check whether the editor renumbers indices on import. | S–M |
| P14 | **Air Tasking** (the user also calls it "Wing Commander / WG/CC"). Historical air sorties against Map objectives: a shared sortie shell (START in, DONE exactly once, a final hard stop, one extension decided from the target area's state), a per-side air budget kept as a head count, an in-mission player threshold for one player (two or more players is step 12), a mission-clock front end (timer chain, weighted reroll, jitter, quiet chance) and a check-zone front end. New "Air Tasking" rail tab (Ctrl 7) and Map dock "Air" tab. **Planned, reviewed and revised (round 4, 2026-09-27). Steps 1a and 1 are built on `claude/p14-air-tasking` (2026-09-27, not merged to `main`, not pushed); next is step 1b (walker); issue #28.** The whole brief is `handoff/P14-air-tasking.md`; the review and the user's decisions are `handoff/P14-review-fable-astra.md`. | User request 2026-09-24. **Astra builds it, one step at a time.** Step 0 is three short solo probe flights (0, 1A, 1B); step 3 waits for flight 0 and step 4 for flight 1A. Design rule: fewest MCUs and check zones. User templates: tested shapes only. The budget is a hard limit on the flights whose aircraft exist. | L (steps 0–12, S–L each) |

P9–P13 came from the frontier-feature review on 2026-09-24. **All are proposals, not scheduled.** Ranked in that order; P9 and P10 together make one "debugging" phase. If most users only produce missions and rarely debug them, P11 moves to the top.

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

**Replaced 2026-09-27.** The user chose to merge the creator's upstream commit `c1d88c3` in full. It removes the 50 % ceiling auto altitude, the "Auto altitude" checkbox, match-lead and `model_spec::auto_altitude_m`. Airstart now puts every aircraft at 1500 m (`template::AIR_START_ALTITUDE_M`, `apply_plane_start`); the slider still changes one plane. The same commit removes the Fighter Pack reinforcement timer, which could start another flight during cleanup. The text below is the record of what P1 was.

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
| 2026-09-24 | Claude Opus (Fable role, parallel session) | **Early-war 1950 air templates.** Added reference §2.2 (1950 F-51, F-80, Yak and Il-10 data: about 110 rows, 120/120 evidence lines checked). Found that **no La-9 or La-11 flew in 1950**: La-9 first appears Nov 1951, La-11 Apr 1953. The shipped Eastern random packs mix La-11, Yak-9P and MiG, which is anachronistic for 1950 dates. Built 6 templates in `TemplateExamples/Historical1950/` (see its README) with the app's own `generate_template`, using a scratch crate that `#[path]`-includes the non-UI `src/` modules plus `build.rs`. The repo code is unchanged. Every file round-trips through `load_template` with no warnings and passes `bombers::inspect_plan`. Findings: `generate_template` forces Activate when wingmen are linked, so a linked flight can't respawn on a cooldown; that needs Fighter Pack. The Template loader does not recognise the Pairs spawn layout ("~37 m off" warning), so the 2+2 templates use finger-four placement with per-group 2. Not smoke-tested in the game itself. |
| 2026-09-24 | Claude Opus (Fable role) | Frontier-feature review. Added proposed phases **P9–P13** to §4 (logic checker, playtest replay, historical OOB from date, terrain-aware placement, semantic diff), with acceptance criteria in `handoff/R4-frontier-proposals.md`. They don't overlap P2–P8 or R3 F1–F9. No code changed. |
| 2026-09-24 | Claude Opus (Fable role) | **UI redesign phase 1 (theme + shell), branch `claude/ui-shell` (`b5aac43`), merged to `main` as `fb26e94` (not pushed).** The source is the Claude Design handoff `UI mockups form survey.zip` (repo root, untracked); its spec is now at `docs/ui-redesign/README.md`, with 6 phases in §9. Ported `theme.rs`/`shell.rs` to egui 0.32. Barlow TTFs were downloaded with the user's OK into `assets/fonts/`. The UI now has a rail, a header with Generate (Ctrl G) and Load/Add (Ctrl O), per-tab scrolling bodies and a status bar. 383 tests pass, 70 warnings, clippy clean on new code. All 6 tabs were captured via PrintWindow. **Not click-tested:** another session held computer-use, so Ctrl 1–6, rail clicks and header buttons are untested in the running app. Next: phase 2 (Template, §5.1). |
| 2026-09-24 | Claude Opus (Fable role) | **UI redesign phase 2 (Template tab), branch `claude/ui-template` (`7f12739`), merged to `main` as `1f095ca` (not pushed).** Three-panel layout per README §5.1: add units and drag-reorder unit cards on the left; the formation view (light grid, Zone In solid / Zone Out dashed, zoom group) over a 236 px order tree in the center; the selection plus collapsible settings on the right. Order/event add moved from each tree row to the tree header (acts on the selected unit). Verified: the 6 `Historical1950` templates generate byte-identical files on `main` and on the branch (temporary env-var hook, not committed). 383 tests, 70 warnings. **Not click-tested:** the user declined computer-use this session, so drag-reorder, + Add, and the menus are unverified in the running app. Deferred to phase 5: DPRK/NATO labels (`ZoneCoalition::label`), Reset confirm, undo, moving long help texts. Side markers use the fallback shape, not the SVG textures. |
| 2026-09-24 | Claude Opus (Fable role) | **UI redesign phase 3 (Map tab), branch `claude/ui-map` (`c3b00ee`), NOT merged.** Layout per README §5.6: 56 px tool palette (keys 1–6, Undo/Redo), the map filling the center with banner, AO readout, legend chip and zoom group, a 64 px front-date strip, and a right dock with Period / Forces / References tabs (`MapDock`). Tool safety (§6.2): Esc cancels, objectives are one-shot unless Shift is held, leaving the tab resets the tool, and the status bar shows the tool hint. Verified: a fixed period + AO + arrow + objective generates a byte-identical base map on `main` and on the branch (temporary hook, not committed). 383 tests, 70 warnings. **Not click-tested** (no computer-use); keys 1–6, Esc and Shift-place are unverified in the running app. Map drawing colors (front, AO box, sides) are unchanged; restyling them is not in the spec's phase list. |
| 2026-09-24 | Claude Opus (Fable role) | **UI redesign phases 4–6, branch `claude/ui-final` (`39062bc`, `71499af`, `dfdf764`), stacked on phase 3 (`c3b00ee`); none merged.** P4: Army Generator, Fighter Pack, Exclusive Activation and Airfield rebuilt as side panels + center (README §5.2–5.5). The Fighter preview uses the new `flights::preview_flights`, with a test tying it to `build_seats` (384 tests). P5: DPRK/NATO and Zone In wording in user text; names written to or matched in `.Group` files keep "Eastern"/"Zone IN" (commented at each site). Per-tab undo (Template, Army, Exclusive, Map) on Ctrl Z and the status bar. Confirm dialogs on Reset and on Load over unsaved edits. Manual labels updated. P6: no text under 12 px; small buttons, move arrows and links have 28 px targets. Verified: no generate/export/load function changed (text compare of 38 fns); a headless hook checked that undo restores identical state; captured every tab. Release build OK. **Not click-tested** (no computer-use). The README §10 readiness checklist is still backlog. |
| 2026-09-24 | Claude Opus (Fable role) | **Harvester + terrain integrated into the redesign; headless UI tests.** Merged `main` (harvester `3ea0053`, terrain `06fa1b3`) into `claude/ui-final` (`289e351`). Harvester controls now sit at the top of the Airfield left panel, with its log in the center. New Map dock tab **Terrain** (`1f0328b`): store status, Export survey pass / AO tiles (100 m), Import snapped (refuses unsnapped) / Learn from mission, Coverage and Relief layers, pointer height readout. No export toggle yet (terrain phase 2 is not built). The 3-pass plan's 200 m / rough-100 m generators don't exist in `heightprobe` yet; add buttons when they do. **Testing:** `src/ui_tests.rs` drives the app headless (AccessKit labels, synthetic clicks/drags/keys, file dialogs from a queue): 22 end-to-end tests over every tab. 434 tests pass, twice in a row. Live click-through **not done**: the computer-use screenshot timed out twice (another session likely held the screen). Nothing pushed; `claude/ui-map` + `claude/ui-final` still unmerged to main. |
| 2026-09-24 | Claude Opus (Fable role) | **Handoff for the next session: `handoff/U1-ui-redesign-test-and-fix.md`.** It covers the live click-through checklist (only Fighter Pack done), the known issues, and the merge-and-push steps (approved once the live run is clean). The live driver is `tools/ui-live/drive.ps1`: real input and PrintWindow capture, because the computer-use screenshot hangs here. |
| 2026-09-24 | Claude Opus (Fable role) | **UI redesign finished on `claude/ui-final` (`85ca06e` + docs), NOT merged or pushed: the user wants to check it first.** Audited every tab against README §4–§8 and the mockups (≈75 gaps), then fixed them in three parallel worktrees (Template+Army `33f87dd`, Fighter/Exclusive/Airfield `88d2153`, Map/side markers/shell/Help `8c2c84a`). Added the README §10 readiness chip (`b855a2f`: Generate disabled until each tab's minimum is met), stale-undo guard, clickable section headers, flight colour names, per-tab status, Map front/clear undo, painted tool icons, dock tabs, fighter side markers (ring removed, cropped), Keyboard shortcuts help. **Live click-through done** with `tools/ui-live/drive.ps1` + new `dialog.ps1` (native file dialogs): every tab, Generate via Ctrl G + save dialog, Load/Add via dialogs, undo/confirm, map tools, Terrain layers + readout, Help Esc, 1280×800. Live bugs fixed in `85ca06e` (missing ✓ glyph, Airfield panel overflow + black gap, COPY MIX clipped, chip names cut, wrapped button), each with a guard test. 466 tests, 69 warnings. Only message strings and two count fields changed outside the UI; generation untouched. User decisions: skill 3 reads **Veteran** (Plain/Low/Normal/Veteran/Ace); Template translation sidecars **on hold** (Army's warning stays for now); `claude/terrain-apply` stays out of this merge, its toggle to default off. |
| 2026-09-24 | Claude Opus (Fable role) | **UI polish merged to local `main` (`e796a3c`), not pushed.** Exclusive undo reselects the restored plan; Terrain counts digit-grouped; Fighter flights stripes full width; status info/error messages retire after the next edit on that tab (`age_status`); order tree gets a visible solid scrollbar; Map AO/height readouts fade under the pointer. Verified live. 468 tests, 69 warnings. |
| 2026-09-24 | Claude Opus (Fable role) | **All branches merged to `main`; tagged v0.6.** `claude/historical-templates-1950` (+ `historical-reference`) were stale pre-rebase copies whose content was already on `main` (`fc8e98f`, `989b3fe`); merged to close them. `claude/terrain-apply` (terrain heights phase 2) merged onto the redesigned UI: conflicts in the Terrain tab and the Exclusive generate path resolved; **Apply terrain heights on export now defaults to off** (user decision), test added; manual and src-guide updated. 476 tests, 69 warnings. |
| 2026-09-26 | Claude Opus | **New machine; backlog moved to GitHub Issues.** Installed Git, GitHub CLI, rustup and VS C++ Build Tools. Cloned the fork to `C:\Machine Intelligence\Claude-IL2MissionUtility`. Turned on Issues for the fork and imported the backlog as #1–#27 (map in §4), with labels `terrain`, `template-builder`, `verify-in-game`, `proposal`, `deferred`, `blocked`, `long-term`, `author-todo`, `research`, `airfield`, `cleanup`. User direction: the creator's 3D-mapping macro replaces the probe-snap method for terrain heights (probe-snap deferred), and integrating the macro into the app is a long-term goal. Updated §1 paths and git notes. No code changed. |
| 2026-09-24 | Claude Opus (Fable role) | **P14 Air Tasking planned (`handoff/P14-air-tasking.md`, committed 2026-09-27).** User decisions: Option A in-mission player threshold (no external server helper), window/dwell from F-80 transit time at cruise, N set later, timer-chain slots, reroll on threshold fail, one engaged extension at the hard stop, drop-and-re-arm when the budget is full, rail tab + Map dock, air starts in v1. Plan built by a 13-agent workflow (5 grounding readers, draft, 4 adversarial critics, 2 revise rounds), then a 3-agent verification round (19 more fixes) and one Claude fix (`LAST CLEAR`). §10 lists 12 additions/deviations for the user to confirm. Implementation is for a separate Opus ultracode session. |
| 2026-09-25 | Claude Opus (Fable role) | **P14 plan extended with trace builds and the P10 replay core (user request U12).** New step 0T: Mission Objective breadcrumbs (spawned-vehicle fallback) that write to the server's text mission log, a `.trace.json` sidecar, a `missionlog.rs` parser with replay (fired / never fired / first dead link / never spawned) and a `replay` CLI, all built before probe flight 1. Flight 1 flies a traced probe; new cell T-t tests the breadcrumbs (risks t1–t6). Step 9T adds the UI. One reviewer found 9 issues, all fixed. Committed 2026-09-27. |
| 2026-09-27 | Claude Opus (Fable role) | **P14 plan pushed to `main`; issues filed.** Fast-forwarded to `88850a5` (new machine / Issues), re-applied the P14 edits to the new §4 layout, committed `handoff/P14-air-tasking.md`. New issues: #28 P14 Air Tasking, #29 P14 probe flights (verify in game), #30 Historical1950 README says the ground AttackArea is flown at WP 1, but it sits on the spawn point. Commented on #23 (P10 core → P14 step 0T, UI → 9T), #9, #12, #13 and #22. `.gitignore` now ignores `__pycache__/`. The untracked `UI mockups form survey.zip` was left out: its contents are already committed under `docs/ui-redesign/`. |
| 2026-09-27 | Claude Fable 5.1 + GPT-6 Astra | **P14 plan reviewed before build (user calls it "Wing Commander / WG/CC"). Verdict: not ready as written; one revision round needed, no redesign.** Both reviews found the code anchors and template facts accurate. Main defect: the budget loses a release when two DONEs land within the 0.1 s settle gap, so a side can lock itself out (two sorties on one objective share a Zone Out). Also: HOT drops at each re-check, slot close does not stop the attempt in flight, fallback matrix holes, user templates have no compatibility contract, replay follows state links as pulses, the cruise fallback would change Template Builder output, several tests cannot pass as written, `trace.rs` uses `i64` against the codebase `i32`. Triage, decisions for the user and the full Astra report: `handoff/P14-review-fable-astra.md`. Astra job `20260927054911-7a36f64e` (`ultra`, read-only). **No plan or code changed.** Astra is now on the account (§2). Not committed. |
| 2026-09-27 | Claude Fable 5.1 + GPT-6 Astra | **Upstream merged; P14 plan revised (round 4) and reviewed twice more.** User decisions U13–U23: fewest MCUs is a design rule; head-count budget; all testing solo, with a flight 0; merge upstream `c1d88c3` in full; multi-player testing (and the threshold for 2+ players) after everything else is confirmed; extension from the area state; user templates stay, **tested shapes only**; **hard limit on aircraft** (budget released at deletion); Astra builds; no further full review. **Merge:** `c1d88c3` on `main` as `a34a00e` (branch `claude/upstream-sync`); 469 tests pass, 3 ignored. It removes the 50 % ceiling auto altitude (P1/P7) and the Fighter Pack reinforcement timer. **Plan:** `handoff/P14-air-tasking.md` rewritten in §3.2–3.5, §3.7–3.9, step 0 (three solo flights 0 / 1A / 1B and a table of supported probe outcomes), steps 3–6, 11, new step 12 and §10 (serial build by Astra, gates G0 and G1). MCU counts: 17 per copy (was about 38), 16 per side (14 at budget 1), 23 per area, 12 per clock entry, 17 per zone entry, 1 shared per entry. **Reviews:** Astra jobs `20260927094211-595e56e3` (second) and `20260927105821-1e970592` (third); each found the core designs and every MCU count sound; findings R4-1 to R4-8 and T1 to T15 all applied. All three reports and the triage: `handoff/P14-review-fable-astra.md`. **The third-review fixes have had no second-model review (user decision U23);** Fable re-reads each step's section when it scopes that step. **No code written.** Open for the user: §10 items 22 (one check zone per area) and 23 (smaller Auto-fill). Committed on `main`; **not pushed**. Next: push when the user says; then step 1a (skeleton), and flight 0 once steps 0T and 0b are built. |
| 2026-09-27 | Claude Fable 5.1 + GPT-6 Astra | **P14 (Wing Commander) steps 1a and 1 built; branch `claude/p14-air-tasking` (`9591114`), cut from `main` @ `8d67385`. Not merged to `main`, not pushed.** Astra built both steps in same-host mode (jobs `20260927211204-bc760103` and `20260927212818-113c1f31`, `gpt-6-astra` confirmed in the session files, effort `max`); Fable reviewed each diff and re-ran build, clippy and tests. **1a (`86816f5`):** `src/airtask/` with `mod.rs` as plan §5, stubs for `in_theatre`, `trace`, `missionlog`, `baseline_tests`; `MapAirPack`; `pub(crate)` on the MCU builders; `timer_random`. `il2_mission_utility.exe replay` now prints a stub line and returns 2. **1 (`3c9e2de`):** 37 baseline files in `src/testdata/p14_baseline/` (7 `.Group` files and sidecars, 2.3 MB), `existing_outputs_match_baseline`, `map_generate_without_air_matches_baseline`, `placement::rotate_tree`, `weapon_range::snap_air_attack_areas`, `airtask/timing.rs`, `in_theatre.rs` (13 rows; only `la11` and `mig15bis` are sourced). **Checks:** 482 tests pass, 5 ignored (was 469 / 3); 69 build warnings and 251 clippy warnings, both unchanged. Fable ran both fixture writers in a separate checkout of unmodified `main`: all 37 files are byte-identical. A changed fixture makes the baseline test fail (checked). **Deviations:** one permanent `#[allow(clippy::type_complexity)]` on `AirContext::arrows` (the frozen field type); `in_theatre` gives `f86a5` and `f84e` the whole war span, although the reference has month-level arrival dates (Dec 1950) for both. The delegation prompts are `handoff/P14-step-1a-codex-prompt.md` and `handoff/P14-step-1-codex-prompt.md`. **Next:** step 1b (walker, `airtask/testkit.rs`), then 0T and 0b, then the user's flight 0. |
