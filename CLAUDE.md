# IL-2 Mission Utility

Start every session with `STATE.md`; coding rules are in `.cursorrules`.

## Keeping the record

The repo must be enough for someone with no access to the chats. Before a session stops or parks:

1. **Overwrite `STATE.md`.** Keep it to one page: where the work is, what is in progress, what waits on the user, the next steps. Do not append history to it.
2. **Write a decision record** in `docs/decisions/` for each decision made in the session, and for each open question the user must decide (status `Proposed`). Include the rejected options. Never edit a decided record; supersede it.
3. **Write a session note** in `handoff/sessions/YYYY-MM-DD-topic.md` only for detail that is still in flight. One file per session, so two people never edit the same file.
4. **Add game facts** learned the hard way to `docs/il2-loader-and-log-facts.md`.
5. Anything saved to Claude's local memory that another person would need goes in the repo too. Local memory stays on one machine.

Do not add to `HANDOFF.md`; it is a pointer. Where an older document says to add a row to the `HANDOFF.md` session log, update `STATE.md` instead; test and warning counts go on its "Last recorded test run" line.

## UI redesign (egui)

The spec for the current UI work is `docs/ui-redesign/README.md`. Read it before touching `src/ui.rs`.

- Build it one phase at a time (README §9). After each phase, run `cargo build` and `cargo clippy`, run that phase's checks, then stop and list what changed.
- It is presentation only. Keep every feature and every call into `crate::*`; no AST or generation logic moves into the UI.
- All six phases of README §9 are built, plus the §10 readiness checklist (2026-09-24). `src/theme.rs` holds the tokens, fonts and visuals; `src/shell.rs` the rail, header, status bar and shared widgets (`Check`/`readiness`, `Undo::settle`, `dock_tabs`, side-marker textures). Build new UI on them.
- Each tab's minimum output is `readiness_checks` in `ui.rs`; Generate stays disabled until they pass. Keep them in step with the errors the `generate_*` functions return.
- UI tests: `src/ui_tests.rs` drives the app headless. Guard tests cover glyphs missing from the loaded fonts (✓ and ✗ are missing; use ✔ ✖), controls overflowing a side panel, wrapped button labels and 28 px targets. Add to them rather than checking by eye.
- Live runs: `tools/ui-live/drive.ps1` (real input, PrintWindow capture) and `tools/ui-live/dialog.ps1` (types a path into the native file dialog). Ask the user that the screen is free first. Move the pointer before each `shot`: egui repaints on input, and a capture taken too early shows the previous frame.
- The visual reference is `docs/ui-redesign/design/Group Generator Mockups.dc.html`, turn 2 (screens 2a–2f). The rules are in `docs/ui-redesign/design/Interface Spec.dc.html`.
- In user-facing text, use DPRK / NATO, never Eastern / Western. Minimum text size is 12 px, minimum control height 28 px.
- Output files must not change. For the same inputs, Generate must write the same .Group files as before.
