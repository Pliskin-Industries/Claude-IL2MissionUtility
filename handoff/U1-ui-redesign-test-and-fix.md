# U1 — UI redesign: live test, fix, merge, push

Status (2026-09-24, later): **DONE except merge/push.** Every gap from a spec audit was
fixed and the live click-through below was completed (see HANDOFF §6, last row).
The user wants to check the app before `claude/ui-final` is merged and pushed.

Earlier status: **built and unit/UI-tested on branch `claude/ui-final`,
not merged.** The live click-through in the real window was started and
stopped early. The user wants this session to test the running app, fix what
it finds, then **merge `claude/ui-final` into `main` and push once the live
run is clean** (they already approved that sequence).

Read first: `HANDOFF.md`, then `docs/ui-redesign/README.md` (the spec, §4–§9),
`CLAUDE.md`, and `docs/src-guide.md` (sections on `ui.rs`, `shell.rs`,
`theme.rs`, `terrain.rs`, `heightprobe.rs`).

## What is on the branch

`git log --oneline main..claude/ui-final`:

| Commit | What |
|---|---|
| `c3b00ee` | Phase 3: Map tab: tool palette with keys 1–6, banner, Esc, one-shot objectives, dock (Period / Forces / References), front-date strip |
| `39062bc` | Phase 4: Army Generator, Fighter Pack, Exclusive Activation, Airfield on the three-panel layout; `flights::preview_flights` + test |
| `71499af` | Phase 5: DPRK/NATO/Zone In wording, per-tab undo (Ctrl Z + status bar), confirm dialogs, manual updates |
| `dfdf764` | Phase 6: 12 px minimum text, 28 px targets, `shell::link` |
| `289e351` | Merge of `main`: airfield harvester (controls moved to the Airfield left panel, log in the center) and terrain modules |
| `1f0328b` | Map dock **Terrain** tab (store status, export survey / AO tiles, import snapped / learn, Coverage + Relief layers, pointer height readout); headless UI tests `src/ui_tests.rs`; `widget_info` labels on custom widgets |

Phases 1–2 are already on `main` (and `origin/main`).

## Verified so far

- `cargo test --offline`: **434 pass**, 2 ignored, twice in a row. 22 of these
  are end-to-end UI tests in `src/ui_tests.rs`. They run the app on a headless
  `egui::Context`, find widgets through the AccessKit tree, click, drag and
  type, and answer file dialogs from a queue. See the file's header and the
  list in commit `1f0328b`.
- `cargo build`: 70 warnings (the pre-existing baseline; keep it there).
  `cargo build --release` OK.
- Output unchanged. Phases 2 and 3 were compared byte for byte against `main`:
  the 6 `TemplateExamples/Historical1950` templates, and a fixed base map.
  Phases 4–6 changed no generate, export or load function (a text compare of
  38 functions).

## Live click-through: how

1. `cargo build --offline`, then start `target\debug\il2_mission_utility.exe`.
   The window is 1400 × 1280 client pixels.
2. The **computer-use screenshot hung** (it timed out after 300 s, 3 times,
   even with the screen free). Use `tools/ui-live/drive.ps1` instead, from
   the PowerShell tool:
   - `-Action shot -Out <png>` captures the app window only (PrintWindow).
     Then Read the PNG. Image pixels are click coordinates.
   - `click`, `drag`, `move`, `key -Keys 1,Esc`, `ctrl -Keys G`, `shiftclick`.
   - It uses **real** mouse and keyboard input (winit ignores posted messages),
     so it moves the user's cursor. It checks that the app is in front and
     owns the target pixel before every action. **Ask the user that the screen
     is free before starting.**
   - It attaches only to a window titled "IL-2 Group Generator". Other
     sessions run the same exe windowless for `--probe-*` terrain commands;
     leave those processes alone.
   - PowerShell aliases bite: don't name helper functions `Move`, `LP`,
     `Post`, etc.
3. Rfd file dialogs are native windows. Either drive them (type a path in the
   dialog's file-name box and press Enter via `key`), or cover Load / Generate
   with the headless tests, which already do this end to end.

## Live checklist

Done in the live window: **Fighter Pack**. Ticking Yak-9P and dragging
Flights to 10 updated the preview (27 aircraft).

Not yet done:

- [ ] **Fighter Pack**: Reset → dialog → Esc cancels; Reset → Enter resets.
      (These keys were sent but the screenshots were never checked.)
- [ ] **Rail / shortcuts**: click all six tabs; Ctrl 1–6; F1 opens Help on
      the current topic.
- [ ] **Template**:
  - "+ Add" on a model row; double-click a row.
  - Card select; drag-reorder cards; × then Ctrl Z; the status bar Undo link.
  - "+ Order ▾" / "+ Event ▾"; chip select; ‹ › move arrows; the model-swap
    popup on the tree's unit chip.
  - Formation view: scroll zoom, right-drag pan, − / + / Fit.
  - Settings sections collapse and remember their state.
  - Reset confirm; Load… over edits asks first; Ctrl G.
- [ ] **Army Generator**:
  - New / Rework switch; Add templates (use a Template Builder vehicle group;
    `TemplateExamples/Developer_AAA.Group` fails to parse, see below).
  - Type buttons; influence; Remove + Ctrl Z.
  - Parking grid and copy-mix render; Timing section.
- [ ] **Exclusive Activation**: add `TemplateExamples/Exclusive_Activation_6plan.Group`;
      plan cards and sequence bars select; Add again / Remove + Ctrl Z; the
      end-timer combo.
- [ ] **Airfield**:
  - Load `TemplateExamples/Airfield_Mess.Group`; NATO / DPRK switch; Generate
    is disabled until loaded.
  - Harvester: Watch toggle, Harvest current, Harvest a file, folder
    Browse…, and the log panel.
- [ ] **Map**:
  - Tools and keys: keys 1–6; the banner and crosshair; Esc cancels a
    salient; an objective tool returns to Select after one click;
    Shift-click keeps it; leaving the tab resets the tool.
  - Drawing undo: undo / redo drawings (tool strip and Ctrl Z / Y).
  - Dock tabs:
    - Period: year / season / battle focus / clear buttons.
    - Forces: Place / Load / Clear + Ctrl Z.
    - References.
  - Front-date slider and ← →; legend chip "All layers ▾"; zoom / Reset
    view / Reset AO.
- [ ] **Map › Terrain**:
  - Status reads the real store (`%APPDATA%\IL2MissionUtility\terrain\korea_100m.hgt`
    holds the user's test data, so don't overwrite it with junk).
  - Coverage / Relief layers draw over the map; the pointer readout.
  - Export AO tiles into a temp folder.
- [ ] **Sizes**: nothing clipped or overflowing at the 1280 × 800 minimum
      window size.

## Known issues to fix or decide

1. **Fighter Pack altitude strip**: with many flights, the flight-number
   labels under the strip overlap (seen at 10 flights). Skip a label that
   would touch the previous one (`fighter_preview` in `ui.rs`).
2. **Flight numbers repeat after 9 flights** (flight 10 reads "11"). This is
   by design in `aircraft::flight_number` (nine flight colours), not a UI
   bug. Consider showing the colour name in the preview table so repeats are
   clear. Ask the user before changing generation.
3. `TemplateExamples/Developer_AAA.Group` fails to parse in the Army Generator
   ("trailing unparsed input … MCU_CheckZone"). This is pre-existing. Check
   whether the file or the parser is at fault.
4. The Map dock's four-tab segmented control ("Period Forces References n
   Terrain") may be tight at 304 px. Check it with a two-digit reference count.
5. Not done yet (spec'd, but deferred):
   - The README §10 readiness checklist.
   - SVG side markers in lists (the fallback shape is used).
   - An "Apply terrain heights" toggle. It arrives with terrain phase 2;
     export doesn't use heights yet.
   - Buttons for the 200 m / rough-100 m probe passes (no generator exists
     yet in `heightprobe`).

For every fix: add or extend a test in `src/ui_tests.rs` when it's reachable
headlessly. Keep warnings at 70 and LF line endings. Python on Windows
writes CRLF in text mode, so write bytes. Don't change generated output.

## Merge and push (after a clean live run)

- `git status` first. Another session may have uncommitted edits in this
  folder (`HANDOFF.md` carried some). Never `git add -A`: `References/` holds
  tens of GB, and `UI mockups form survey.zip` sits untracked in the root.
- `git switch main && git merge --no-ff claude/ui-final`, then run
  `cargo test --offline` on `main`.
- `git push origin main`. The user approved this push, conditional on the
  live run coming back without errors. The repo is public
  (Pliskin-Industries fork). Don't push tags; `v*` tags trigger a release
  build.
- Add a HANDOFF §6 row, and update `CLAUDE.md` if anything about the UI
  workflow changed.
