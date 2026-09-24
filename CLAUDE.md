# UI redesign (egui)

Start every session with `HANDOFF.md`; coding rules are in `.cursorrules`.

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
