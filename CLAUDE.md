# UI redesign (egui)

Start every session with `HANDOFF.md`; coding rules are in `.cursorrules`.

The spec for the current UI work is `docs/ui-redesign/README.md`. Read it before touching `src/ui.rs`.

- Build it one phase at a time (README §9). After each phase, run `cargo build` and `cargo clippy`, run that phase's checks, then stop and list what changed.
- It is presentation only. Keep every feature and every call into `crate::*`; no AST or generation logic moves into the UI.
- `src/theme.rs` (tokens, fonts, visuals) and `src/shell.rs` (rail, header, status bar, shared widgets) are ported to egui 0.32 and wired in (phase 1). Build every later phase on them.
- The visual reference is `docs/ui-redesign/design/Group Generator Mockups.dc.html`, turn 2 (screens 2a–2f). The rules are in `docs/ui-redesign/design/Interface Spec.dc.html`.
- In user-facing text, use DPRK / NATO, never Eastern / Western. Minimum text size is 12 px, minimum control height 28 px.
- Output files must not change. For the same inputs, Generate must write the same .Group files as before.
