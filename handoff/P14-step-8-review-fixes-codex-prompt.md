# Delegation prompt: P14 step 8, review fixes and sortie categories (decision 0031)

> Sent with this decision numbered 0029; it was renumbered 0031 on 2026-10-26 (see the record). The references below use the new number.

Second job on `codex/p14-8` in the worktree `eloquent-lumiere-a004d5`, after the build (`162556e`) and the type change for decision 0031 (`6ebead8`). Sent with `codex_start`, model `gpt-6-astra`, `reasoning_effort` = `max`, `sandbox` = `workspace-write`, `max_idle_seconds` = 1500.

---

<task>
Two things on the Air Tasking tab you built (`src/ui/air.rs`, `src/airtask/pool.rs`): **(1) fix the review findings below**, and **(2) build sortie categories as decision 0031 says**. Read first: `docs/decisions/0031-sortie-categories.md`, `docs/decisions/0027-air-tasking-rail-tab-design.md`, `handoff/P14-step-8-codex-prompt.md` (your first brief; its rules still hold), `git show 6ebead8` (the two new fields), then the code.

### 1. Review fixes

1. **AST work out of the UI** (CLAUDE.md "no AST or generation logic moves into the UI"; `.cursorrules`). `generate_air_file` builds an `Il2Entity` root, sets indexes and runs `duplicate_template`. Move that into a new `src/airtask/standalone.rs`: `pub fn build_air_group(plan: &AirPlan, ctx: &AirContext) -> Result<Il2Entity, String>` (calls `build_air_packs`; one top-level group named `Air Tasking`; indexes from 1) and `pub fn sidecar_paths(plan: &AirPlan) -> Vec<PathBuf>` — the files of **user templates used by enabled pool entries only** (today every library file is passed). The UI keeps only `serialize_group`, the save dialog and `save_with_sidecars`. Unit tests for both.
2. **"Added 1 entries"** (`air.rs` ≈201): singular for one, as the undo label does.
3. **Row selection under a popup** (`air.rs` ≈555): picking an option in a row's Mode combo also selects the row below it, because selection tests the click position against the row rect. Select a row only from that row's own click response, and never while a popup is open.
4. **Plan check:** Errors and Warnings share `WARN_TEXT` (the only allowed warning token). Keep the colour; draw Error text strong so the two differ by more than the glyph.
5. **`apply_air_edit`** drops a draft change when a widget forgets `note_edit`. Detect a change by comparing the draft with the plan (the `{:?}` fingerprint or `PartialEq`), so no widget can lose an edit; keep one undo step per drag.
6. **Hard stop must match what Generate writes.** `assemble.rs` uses the clock spawn distance only for `TriggerMode::Clock`; `Zone` and `Both` use the zone distance. Change the helper to `pub fn hard_stop(sortie: &AirSortie, plan: &AirPlan, mode: TriggerMode) -> Option<f64>` — `None` when the spawn speed is not a positive finite number (a user sortie with speed 0 must not give infinity). The selected-entry block shows one value ("{m} min") or "—". Tests: HVAR Clock 2100 s; HVAR Zone and Both ≈ 1963.6 s; ArmedRecon Clock 2700 s; speed 0 Zone → `None`.
7. **`src/airtask/mod.rs`:** the trailing signature comments of both `pub use` lines list every export with its signature (today the timing one lists two of five and the pool one has none).
8. **Duplicated logic → one place each, in `pool.rs`:** `impl AirSortie { pub fn valid_on(&self, date: Ymd) -> bool }` (used by `pool.rs` ×2 and `air.rs`); `impl AirPlan { pub fn sortie(&self, id: &str) -> Option<&AirSortie> }` (the four `library.iter().find(..)`); `impl AirSide { pub const ALL: [AirSide; 2] }`; `pub fn pool_counts(plan) -> [usize; 2]` (per side, for the header note). In `air.rs`, one helper maps a side to its label, marker side and colour; every `[AirSide::Dprk, AirSide::Nato]` literal and per-side switch uses `AirSide::ALL` and that helper.
9. **Role chips leave the UI:** delete `role_text` / `escorted` (they test the literal model `"f51d"`). Categories (part 2) replace the role chip; "Escorted" becomes `impl AirSortie { pub fn is_escorted(&self) -> bool }` in `pool.rs` (true when any lead has `AirRole::Escort`).
10. **`f80_cruise_kmh() -> (f64, bool)`** → `pub struct F80Cruise { pub kmh: f64, pub measured: bool }`.
11. **`number_row`'s seven arguments:** bundle range, step and suffix into one small struct (the triple repeats ~15 times).
12. **Build the `AirContext` once per frame** at the top of the tab's page function and pass it down (today `with_air_context` clones and densifies the front ~8 times a frame). Generate and the readiness arm may build their own.
13. **Format the new files only:** run `rustfmt` on `src/ui/air.rs`, `src/airtask/pool.rs` and `src/airtask/standalone.rs` (and nothing else); do not reformat existing files.

### 2. Sortie categories (decision 0031)

- **Reader, in `src/airtask/library.rs`** (now in scope for this only): `pub const DEFAULT_CATEGORIES: [&str; 5] = ["Air alert", "Strike", "Close support", "Armed recon", "Interdiction"]`. When a sortie is loaded (built-in or file), `category` = the first `Category: <name>` line in the top-level group's `Desc` (key case-insensitive, name trimmed, kept as written; an empty name counts as no label); else `default_category(&sortie)`. `pub fn default_category(sortie: &AirSortie) -> String`: a built-in by its file (AirAlert → "Air alert", HVAR → "Strike", CloseSupport → "Close support", ArmedRecon → "Armed recon", Yak raid → "Strike", Il-10 → "Strike"); a file template from its orders (any lead with `AirRole::AirPatrol` → "Air alert", else "Strike"). Re-export both through `mod.rs`. **Do not edit the six built-in files** (flown probes place them as shipped and must regenerate byte for byte).
- **Plan functions, in `pool.rs`:** `pub fn default_categories() -> Vec<String>`; `add_library_item(plan, sortie)` (appends; adds its category to `plan.categories` when missing; the UI's Add template… uses it); `set_category(plan, sortie_index, name)`; `add_category(plan, name) -> Result<(), String>` (trimmed, not empty, unique ignoring case; user-facing message on error); `rename_category(plan, old, new) -> Result<(), String>` (same rules; every sortie with `old` follows); `remove_category(plan, name) -> usize` (removes it; each sortie that had it goes back to `default_category`, and that category is added back to the list if it was missing; returns how many sorties moved); `category_counts(plan) -> Vec<(String, usize)>` in list order; `auto_fill` gains `category: Option<&str>` and fills only sorties of that category when `Some`. Unit tests for each, with literals.
- **UI:** at startup and on Reset, `plan.categories = default_categories()` plus any built-in category not in it. Library card chips: the category (accent `tag`), "Escorted" when `is_escorted`, planes and model; the selected card's footer gets a "Category" combo over `plan.categories` (record undo first). The centre panel gets two tabs above its content, **Pool** and **Categories** (use `shell::dock_tabs` or the same look; 28 px; both labels on one line):
  - **Pool:** the pool header gets a filter combo "All categories" / each category; the table shows only entries whose sortie has the filtered category (the header count says "{shown} of {n}" when filtered); Auto-fill fills only the filtered category (status "Added n entries · {category}"). The slot timeline and plan check stay under the pool as now.
  - **Categories:** one row per category: name, "{n} sorties", Rename (an inline text field with Save / Cancel) and Remove; under the list a text field and "Add category"; an error from `add_category` / `rename_category` shows inline with `shell::warning`. Hint: "Categories last for this run. A template keeps its category with a line Category: <name> in its group description. Help ›". Undo records before every add, rename, remove and category change.
  The selected-entry block shows the sortie's category.
- **Tests in `src/ui_tests.rs`:** the category combo changes a sortie's category and Ctrl Z restores it; the Categories tab adds, renames (sorties follow) and removes (sorties fall back) a category, each undoable; the pool filter hides other categories' rows and Auto-fill with a filter adds only that category; a user template whose Desc says `Category: Interdiction` shows Interdiction (write a temp copy of a built-in with that Desc and load it through `dialog::answer`); the six built-ins show their 0031 defaults. Extend the guards so the Categories tab and the filter are covered: 28 px (with every control, disabled ones too), panel fit, labels on one line, glyphs.
- **Docs:** `USER_MANUAL.md` "Air Tasking": categories, the filter, and the `Category: <name>` line; `docs/src-guide.md`: `airtask/standalone.rs`.

### Acceptance

- `cargo build --offline` / `cargo clippy --offline` warnings not above 69 / 251; `cargo test --offline` 0 failed, with the passed / ignored counts and every test added or changed reported. `existing_outputs_match_baseline`, `map_generate_without_air_matches_baseline` and every flown-probe regeneration test pass.
- Each of the 13 fixes and each bullet of part 2 can be pointed to in the diff.

Files in scope: `src/ui/air.rs`, `src/airtask/pool.rs`, `src/airtask/standalone.rs` (new), `src/airtask/timing.rs`, `src/airtask/library.rs` (the category reader, `DEFAULT_CATEGORIES`, `default_category` and their tests only), `src/airtask/mod.rs` (`mod standalone;` and the `pub use` lines only; the frozen types are done), `src/ui.rs` (hooks only, as before), `src/ui_tests.rs`, `USER_MANUAL.md`, `docs/src-guide.md`.

Do not touch: `TemplateExamples/` (the six built-ins), the other `src/airtask/` files (`threshold.rs`, `clock.rs`, `zone.rs`, `assemble.rs`, `budget.rs`, `shell.rs`, `place.rs`, `probe.rs`, `testkit.rs`), the frozen types in `mod.rs`, the stub bodies of `validate` / `preview` / `totals` / `build_air_packs`, `src/template.rs`, `src/frontlines.rs`, `src/shell.rs`, `src/help.rs`, `src/testdata/`, `handoff/`, `docs/decisions/`, `STATE.md`, `CLAUDE.md`, `Cargo.toml`, `Cargo.lock`, `.gitattributes`, Template Builder and Map behaviour. If the work cannot be done inside scope, stop and report exactly what and why.
</task>

<git_protocol>
Same-host mode. `codex/p14-8` is checked out here at `6ebead8` and the working tree is clean. Edit files in the working tree only. No git command that writes. If the working tree holds changes that are not yours when you start, stop and report.
</git_protocol>

<completeness_contract>
Resolve the task fully. Every acceptance line must hold before you report. If something cannot be done as written, stop and say what and why. Do not weaken or delete an existing test to make it pass; report the value and the reason.
</completeness_contract>

<verification_loop>
Run `cargo build --offline`, `cargo clippy --offline` and `cargo test --offline` after the change and again after any fix. Before reporting, re-read decision 0031 and the 13 fixes and check each against the code.
</verification_loop>

<missing_context_gating>
Do not guess repository facts (field names, how `Desc` is read from an `Il2Entity`, signatures): read them.
</missing_context_gating>

<action_safety>
Keep changes inside the files in scope. No unrelated refactors, renames or cleanup. Existing generated output must stay byte-identical.
</action_safety>

<compact_output_contract>
Report, in this order: final build / clippy / test numbers; files changed with line counts; each fix (1–13) with where it landed; each new public function and type; each test added or changed (active / ignored); anything in scope you could not do and why; anything you noticed outside scope (do not fix it).
</compact_output_contract>
