//! # airfield.rs — Airfield mode
//!
//! One mode of [`super::GroupGeneratorApp`]. This file owns the Airfield
//! page: load and export panels, the harvest section, the field-spawn
//! aircraft list, and the air-start bank. The two aircraft lists are
//! separate. Harvested airfields from the database folder are listed
//! together; each one keeps its own field-spawn aircraft. The list splits
//! airbases inside the map area from the rest, then by DPRK or NATO side of
//! the front, and marks a field with no runway group as data required.
//!
//! `ui.rs` is the anchor. It owns the app struct (the `airfield_*` /
//! `air_start*` / `harvest_*` fields), the mode rail, and the shortcuts,
//! and it calls the hooks listed below. State stays on the app.
//!
//! Hooks `ui.rs` calls (keep these `pub(super)`):
//! * `airfield_page` — the tab
//! * `export_airfield` — Generate / Ctrl G
//! * `load_airfield` — Load / Ctrl O
//! * `poll_harvest` — each frame, on every tab
//!
//! Presentation only. Group text is built by [`crate::airfield`],
//! [`crate::harvest`], and [`crate::airstart`].

use std::path::{Path, PathBuf};

use eframe::egui::{self, Align, Align2, FontId, Layout, RichText, Sense, Stroke, Vec2};

use crate::aircraft::{COUNTRIES, country_is_nato};
use crate::korea_fields::{self, HarvestHit};
use crate::airfield::{
    EASTERN_PLANE_COALITIONS, WESTERN_PLANE_COALITIONS, clean_airfield, inspect_airfield,
};
use crate::airstart::{self, AirStartPlane};
use crate::ast::Il2Entity;
use crate::frontlines::PLACE_MARGIN;
use crate::harvest::{GenWatcher, HarvestOutcome, find_gen_file, harvest_file, place_field_spawn};
use crate::help::HelpTopic;
use crate::mapclip::{WorldAabb, on_dprk_side};
use crate::parser::parse_il2_document;
use crate::provisions::{self, guess_nato};
use crate::serialize::serialize_group;
use crate::shell;
use crate::theme::{self, c};

use super::{
    AppMode, MapDrawingMode, Status, builder, center_panel, country_short, dialog, empty_state,
    group_digits, labeled_slider_suffix, mode_slot, save_with_sidecars, side_panel,
};

/// One row of `catalog.Group`. Kept so a field inside the map area can be
/// named even when its harvested group file is missing.
#[derive(Clone)]
pub(super) struct IndexedAirbase {
    pub name: String,
    pub x: f64,
    pub z: f64,
    pub file: String,
    pub country: i32,
}

/// One airfield open on the Airfield tab. `planes` is that field's spawn
/// list, not the air-start bank.
#[derive(Clone)]
pub(super) struct AirbaseSlot {
    pub path: PathBuf,
    pub root: Il2Entity,
    pub info: crate::airfield::AirfieldInfo,
    pub western: bool,
    pub nato: bool,
    pub planes: Vec<AirStartPlane>,
    /// False when the user loaded a file that is not in the database folder.
    pub from_folder: bool,
}

impl AirbaseSlot {
    fn scanned(path: PathBuf, root: Il2Entity) -> Self {
        let nato = guess_nato(&root, &path);
        let info = inspect_airfield(&root);
        Self {
            path,
            root,
            info,
            western: nato,
            nato,
            planes: Vec::new(),
            from_folder: true,
        }
    }
}

impl super::GroupGeneratorApp {
    // ── Airfield (README §5.5) ─────────────────────────────────────────────

    pub(super) fn airfield_page(&mut self, ctx: &egui::Context) {
        self.remember_airbase_edits();
        self.sync_airbases();
        side_panel(ctx, "airfield_left", true, 290.0, |ui| {
            self.airfield_left_panel(ui)
        });
        side_panel(ctx, "airfield_right", false, 270.0, |ui| {
            self.airfield_right_panel(ui)
        });
        center_panel(ctx, "airfield_center", |ui| self.airfield_center(ui));
    }

    fn airfield_left_panel(&mut self, ui: &mut egui::Ui) {
        shell::section_title(ui, "Get the file", None);
        numbered_step(
            ui,
            1,
            "In game, open a Freeflight mission and take off from the airfield.",
        );
        numbered_step(
            ui,
            2,
            "Open /missions/_gen.mission in the mission editor. Select the field, then File › Save Selection to File.",
        );
        numbered_step(ui, 3, "Load that file here.");
        if shell::link(ui, "Help ›").clicked() {
            self.open_help(HelpTopic::Airfield);
        }
        ui.add_space(6.0);
        ui.separator();
        shell::section_title(ui, "Friendly plane coalition", None);
        shell::segmented(
            ui,
            &mut self.airfield_western,
            &[(true, "NATO [2]"), (false, "DPRK [1]")],
        );
        shell::hint(ui, "USA airfields use NATO.", false);
        ui.add_space(6.0);
        ui.separator();
        self.harvest_section(ui);
        ui.add_space(6.0);
        ui.separator();
        shell::section_title(ui, "Field spawn", None);
        shell::hint(
            ui,
            "The Field spawn list in the center is for the selected airbase. Each airbase keeps its own aircraft. Engine running uses the 80 m line at the hold-short. Engine off snaps a plane to parking. The field stays where it is.",
            false,
        );
        ui.add_enabled_ui(self.airfield_info.is_some(), |ui| {
            if ui
                .add_sized([ui.available_width(), 28.0], egui::Button::new("Export field spawn…"))
                .on_hover_text("Hold-short, nose toward the runway. Engine running on the 80 m line, or engine off snapped to parking.")
                .clicked()
            {
                self.export_field_spawn();
            }
        });
        ui.add_space(6.0);
        ui.separator();
        shell::section_title(ui, "Air starts", Some("fake fields"));
        shell::hint(
            ui,
            "A fake field and the aircraft that can launch from it.",
            false,
        );
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Add air start"),
            )
            .clicked()
        {
            self.add_air_start();
        }
        ui.add_enabled_ui(!self.air_starts.is_empty(), |ui| {
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new("Export one group…"),
                )
                .on_hover_text("One .Group file with every air start.")
                .clicked()
            {
                self.export_air_starts_one();
            }
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new("Export each…"),
                )
                .on_hover_text("One .Group file per air start.")
                .clicked()
            {
                self.export_air_starts_each();
            }
        });
    }

    fn airfield_center(&mut self, ui: &mut egui::Ui) {
        shell::section_title(ui, "What Generate will change", None);
        ui.label("Strips the player and SP logic, then retargets the checkzones that were linked to the player.");
        ui.add_space(10.0);
        let loaded = self.airfield_info.clone();
        let Some(info) = loaded else {
            if empty_state(
                ui,
                "Load an airfield group exported from _gen.mission.",
                "Load airfield…",
            ) {
                self.load_airfield();
            }
            ui.add_space(16.0);
            self.field_spawn_panel(ui);
            self.air_starts_panel(ui);
            self.harvest_log_panel(ui);
            return;
        };
        let side = if self.airfield_western {
            "NATO [2]"
        } else {
            "DPRK [1]"
        };
        ui.columns(2, |cols| {
            shell::blueprint(&mut cols[0], c::DIVIDER, |ui| {
                shell::section_title(ui, "Removed", None);
                if info.player_planes.is_empty() {
                    shell::warning(
                        ui,
                        "No player aircraft found; this file may already be cleaned.",
                    );
                } else {
                    for p in &info.player_planes {
                        fact_row(
                            ui,
                            &format!("Player · {}", p.name),
                            &country_name(p.country),
                        );
                    }
                }
                if info.has_autoremove {
                    fact_row(ui, "AutoRemove subgroup", "");
                }
                fact_row(
                    ui,
                    "Player / SP graph objects",
                    &info.strip_count.to_string(),
                );
            });
            shell::blueprint(&mut cols[1], c::ACCENT, |ui| {
                let n = info.unlink_zones.len();
                let note = format!("{n} checkzone{}", if n == 1 { "" } else { "s" });
                shell::section_title(ui, &format!("Relinked to {side}"), Some(&note));
                if info.unlink_zones.is_empty() {
                    ui.label("No checkzones are linked to the player.");
                }
                for name in &info.unlink_zones {
                    fact_row(ui, name, "");
                }
            });
        });
        ui.add_space(10.0);
        shell::blueprint(ui, c::DIVIDER, |ui| {
            shell::section_title(ui, "Kept", None);
            ui.columns(4, |cols| {
                for (col, (n, label)) in cols.iter_mut().zip([
                    (info.vehicle_count, "vehicles / ships"),
                    (info.ai_plane_count, "AI aircraft"),
                    (info.block_count, "blocks"),
                    (info.checkzone_count, "checkzones"),
                ]) {
                    col.label(
                        RichText::new(n.to_string())
                            .font(FontId::new(24.0, theme::heading_family())),
                    );
                    col.label(RichText::new(label).small().color(c::NEUTRAL_700));
                }
            });
        });
        ui.add_space(10.0);
        shell::warning(
            ui,
            "Still required after export: add planes to fly and set the starting location.",
        );
        ui.add_space(16.0);
        self.field_spawn_panel(ui);
        self.air_starts_panel(ui);
        self.harvest_log_panel(ui);
    }

    fn field_spawn_panel(&mut self, ui: &mut egui::Ui) {
        shell::section_title(
            ui,
            "Field spawn",
            Some(&format!("{}", self.field_planes.len())),
        );
        shell::hint(
            ui,
            "Add planes to the selected airbase. This list is not an air start. A base map writes these planes onto that airfield.",
            false,
        );
        shell::segmented(
            ui,
            &mut self.field_spawn_nato,
            &[(true, "NATO"), (false, "DPRK")],
        );
        ui.add_space(4.0);
        let mut add_plane = false;
        let mut remove_plane: Option<usize> = None;
        for pi in 0..self.field_planes.len() {
            let summary = plane_summary(&self.field_planes[pi], true);
            let mut remove = false;
            egui::CollapsingHeader::new(summary)
                .id_salt(("field_spawn_plane", pi))
                .default_open(false)
                .show(ui, |ui| {
                    plane_card(ui, &mut self.field_planes[pi], &format!("field_{pi}"), true);
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new("Remove aircraft"),
                        )
                        .clicked()
                    {
                        remove = true;
                    }
                });
            if remove {
                remove_plane = Some(pi);
            }
        }
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Add aircraft"),
            )
            .clicked()
        {
            add_plane = true;
        }
        if let Some(pi) = remove_plane {
            self.field_planes.remove(pi);
        }
        if add_plane {
            self.field_planes
                .push(new_field_plane(self.field_spawn_nato));
        }
    }

    fn add_air_start(&mut self) {
        let n = self.air_starts.len() + 1;
        self.air_starts
            .push(airstart::default_field(format!("Air start {n}"), true));
        self.air_start_edit = self.air_starts.len() - 1;
    }

    fn export_air_starts_one(&mut self) {
        if self.air_starts.is_empty() {
            self.status = Status::Error("Add an air start first.".into());
            return;
        }
        let placed = airstart::parked(&self.air_starts);
        let group = airstart::combined_group(&placed, &vec![0.0; placed.len()]);
        let Some(save_path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .set_file_name("Air starts.Group")
            .save_file()
        else {
            return;
        };
        self.status = save_with_sidecars(
            &save_path,
            &serialize_group(&group),
            &[],
            "Wrote the air start group",
        );
    }

    fn export_air_starts_each(&mut self) {
        if self.air_starts.is_empty() {
            self.status = Status::Error("Add an air start first.".into());
            return;
        }
        let Some(dir) = dialog::FileDialog::new().pick_folder() else {
            return;
        };
        let mut written = 0usize;
        for (i, field) in self.air_starts.iter().enumerate() {
            let mut stem = airstart::file_stem(&field.name);
            let mut path = dir.join(format!("{stem}.Group"));
            let mut n = 2u32;
            while path.exists() {
                stem = format!("{}_{n}", airstart::file_stem(&field.name));
                path = dir.join(format!("{stem}.Group"));
                n += 1;
            }
            let group = airstart::field_group(field, 40_000.0 + i as f64 * 2_000.0, 40_000.0, 0.0);
            if let Err(err) = std::fs::write(&path, serialize_group(&group)) {
                self.status = Status::Error(format!("Could not write {}: {err}", path.display()));
                return;
            }
            written += 1;
        }
        self.status = Status::Info(format!(
            "Wrote {written} air start group(s) to {}.",
            dir.display()
        ));
    }

    fn air_starts_panel(&mut self, ui: &mut egui::Ui) {
        shell::section_title(
            ui,
            "Air starts",
            Some(&format!("{}", self.air_starts.len())),
        );
        shell::hint(ui, "Place and turn copies on the Map tab.", false);
        if self.air_starts.is_empty() {
            shell::hint(ui, "Add an air start on the left.", false);
            return;
        }
        if self.air_start_edit >= self.air_starts.len() {
            self.air_start_edit = self.air_starts.len() - 1;
        }
        let mut pick = self.air_start_edit;
        ui.horizontal_wrapped(|ui| {
            for (i, field) in self.air_starts.iter().enumerate() {
                let label = format!("{} · {}", field.name, field.side_label());
                if ui.selectable_label(i == pick, label).clicked() {
                    pick = i;
                }
            }
        });
        self.air_start_edit = pick;
        ui.add_space(6.0);

        let edit = self.air_start_edit;
        let mut remove_field = false;
        let mut add_plane = false;
        let mut remove_plane: Option<usize> = None;
        {
            let field = &mut self.air_starts[edit];
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut field.name)
                        .desired_width(ui.available_width() - 132.0)
                        .min_size(Vec2::new(80.0, 28.0)),
                );
                shell::segmented(ui, &mut field.nato, &[(true, "NATO"), (false, "DPRK")]);
            });
            ui.add_space(4.0);
            for pi in 0..field.planes.len() {
                let summary = plane_summary(&field.planes[pi], false);
                let mut remove = false;
                egui::CollapsingHeader::new(summary)
                    .id_salt(("air_start_plane", edit, pi))
                    .default_open(false)
                    .show(ui, |ui| {
                        plane_card(
                            ui,
                            &mut field.planes[pi],
                            &format!("air_{edit}_{pi}"),
                            false,
                        );
                        if ui
                            .add_sized(
                                [ui.available_width(), 28.0],
                                egui::Button::new("Remove aircraft"),
                            )
                            .clicked()
                        {
                            remove = true;
                        }
                    });
                if remove {
                    remove_plane = Some(pi);
                }
            }
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new("Add aircraft"),
                )
                .clicked()
            {
                add_plane = true;
            }
            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new("Remove air start"),
                )
                .clicked()
            {
                remove_field = true;
            }
        }
        if let Some(pi) = remove_plane {
            self.air_starts[edit].planes.remove(pi);
        }
        if add_plane {
            let nato = self.air_starts[edit].nato;
            let id = if nato { "f51d" } else { "mig15bis" };
            self.air_starts[edit]
                .planes
                .push(airstart::default_plane(id));
        }
        if remove_field {
            self.air_starts.remove(edit);
            if self.air_start_edit >= self.air_starts.len() && !self.air_starts.is_empty() {
                self.air_start_edit = self.air_starts.len() - 1;
            }
            if self
                .air_start_place
                .is_some_and(|i| i == edit || i >= self.air_starts.len())
            {
                self.air_start_place = None;
                if self.map_drawing_mode == MapDrawingMode::PlaceAirStart {
                    self.map_drawing_mode = MapDrawingMode::None;
                }
            }
        }
    }

    /// The drawn front, or the period line when the map is still on the timeline.
    fn front_for_sides(&self) -> Vec<(f64, f64)> {
        let raw = if self.custom_front_xz.len() >= 2 {
            self.custom_front_xz.clone()
        } else {
            crate::frontlines::preview_front_xz(self.front_t)
        };
        crate::frontlines::densify(&raw, 4_000.0)
    }

    fn airbase_side_block(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        slots: &[usize],
        gaps: &[AreaGap],
        pick: &mut Option<usize>,
    ) {
        let count = slots.len() + gaps.len();
        shell::section_title(ui, title, Some(&count.to_string()));
        if count == 0 {
            shell::hint(ui, "None on this side of the front.", false);
            return;
        }
        for &i in slots {
            let slot = &self.airbases[i];
            let selected = self.airfield_path.as_ref() == Some(&slot.path);
            if airbase_button(
                ui,
                &airbase_label(slot),
                selected,
                airbase_needs_data(&slot.root),
            ) {
                *pick = Some(i);
            }
        }
        for gap in gaps {
            ui.add_sized(
                [ui.available_width(), 28.0],
                egui::Label::new(RichText::new(&gap.name).color(c::TEXT)).truncate(),
            );
            ui.label(RichText::new("Data required").color(c::WARN_TEXT));
        }
    }

    fn airfield_right_panel(&mut self, ui: &mut egui::Ui) {
        let count = self.airbases.len();
        shell::section_title(ui, "Airbases", Some(&count.to_string()));
        shell::hint(
            ui,
            "Airbases in the map area are split by the front: DPRK to the north, NATO to the south. Data required means that field has no runway group to place aircraft on.",
            false,
        );
        if self.front_aabb == WorldAabb::full_map() {
            shell::hint(
                ui,
                "The map area is the whole Korea map. Draw a box on the Map tab to crop it.",
                false,
            );
        }
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new("Refresh airbases"),
            )
            .clicked()
        {
            self.airbase_scan_dir.clear();
            self.provisions_loaded_for.clear();
        }
        let area = self.front_aabb.expanded(PLACE_MARGIN);
        let front = self.front_for_sides();
        let mut inside = Vec::new();
        let mut outside = Vec::new();
        for (i, slot) in self.airbases.iter().enumerate() {
            if airbase_in_area(&slot.root, area) {
                inside.push(i);
            } else {
                outside.push(i);
            }
        }
        let gaps = uncovered_in_area(
            &self.airbase_index,
            &self.airbases,
            area,
            &front,
            &self.salients,
        );
        if self.airbases.is_empty() && gaps.is_empty() {
            shell::hint(ui, "None yet. Harvest airfields, or load a file.", false);
        } else {
            let in_count = inside.len() + gaps.len();
            shell::section_title(ui, "In the area", Some(&in_count.to_string()));
            if in_count == 0 {
                shell::hint(ui, "No airbases in the map area.", false);
            }
            let mut dprk_slots = Vec::new();
            let mut nato_slots = Vec::new();
            for i in inside {
                let dprk = airbase_point(&self.airbases[i].root)
                    .is_some_and(|(x, z)| on_dprk_side(&front, &self.salients, x, z));
                if dprk {
                    dprk_slots.push(i);
                } else {
                    nato_slots.push(i);
                }
            }
            let mut dprk_gaps = Vec::new();
            let mut nato_gaps = Vec::new();
            for gap in gaps {
                if on_dprk_side(&front, &self.salients, gap.x, gap.z) {
                    dprk_gaps.push(gap);
                } else {
                    nato_gaps.push(gap);
                }
            }
            let mut pick: Option<usize> = None;
            if in_count > 0 {
                self.airbase_side_block(ui, "DPRK", &dprk_slots, &dprk_gaps, &mut pick);
                self.airbase_side_block(ui, "NATO", &nato_slots, &nato_gaps, &mut pick);
            }
            if !outside.is_empty() {
                shell::section_title(ui, "Outside the area", Some(&outside.len().to_string()));
                for i in outside {
                    let slot = &self.airbases[i];
                    let selected = self.airfield_path.as_ref() == Some(&slot.path);
                    if airbase_button(ui, &airbase_label(slot), selected, false) {
                        pick = Some(i);
                    }
                }
            }
            if let Some(i) = pick {
                self.show_airbase(i);
            }
        }
        self.korea_fields_panel(ui);
        ui.add_space(8.0);
        ui.separator();
        shell::section_title(ui, "Airfield", None);
        let Some(info) = &self.airfield_info else {
            shell::hint(ui, "Load an airfield to see it here.", false);
            return;
        };
        egui::Grid::new("airfield_facts")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Name").color(c::NEUTRAL_700));
                ui.label(RichText::new(&info.name).font(FontId::new(13.0, theme::bold_family())));
                ui.end_row();
                ui.label(RichText::new("Layout").color(c::NEUTRAL_700));
                ui.label(if info.in_group {
                    "Inside a Group"
                } else {
                    "Blocks at the root"
                });
                ui.end_row();
                if let Some((x, z)) = info.origin_xz {
                    ui.label(RichText::new("Origin").color(c::NEUTRAL_700));
                    ui.label(
                        RichText::new(format!("{}, {}", group_digits(x), group_digits(z)))
                            .monospace(),
                    );
                    ui.end_row();
                }
            });
    }

    pub(super) fn load_airfield(&mut self) {
        let Some(path) = dialog::FileDialog::new()
            .add_filter(
                "IL-2 Group / mission",
                &["Group", "group", "Mission", "mission"],
            )
            .pick_file()
        else {
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(err) => {
                self.status = Status::Error(format!("Could not read file: {err}"));
                return;
            }
        };
        match parse_il2_document(&text) {
            Ok(root) => {
                let info = inspect_airfield(&root);
                let players = info.player_planes.len();
                let zones = info.unlink_zones.len();
                self.status = if players == 0 {
                    Status::Info(format!("Loaded {} — no player aircraft found.", info.name))
                } else {
                    Status::Info(format!(
                        "Loaded {}: {players} player aircraft, {zones} checkzones to unlink.",
                        info.name
                    ))
                };
                self.open_loaded_airfield(path, root, info);
            }
            Err(err) => {
                self.status = Status::Error(format!("Parse failed: {err}"));
            }
        }
    }

    /// Airfield tab, left panel: the automatic database harvest (watch the
    /// game's _gen.mission and file every start airfield).
    fn harvest_section(&mut self, ui: &mut egui::Ui) {
        shell::section_title(ui, "Harvest automatically", Some("many airfields"));
        shell::hint(
            ui,
            "Instead of the steps above: watch, then start a Freeflight from each airfield in turn. Each new _gen.mission is cut, cleaned for multiplayer and filed in the database.",
            false,
        );
        ui.add_space(4.0);
        let mut watching = self.harvest_watcher.is_some();
        if ui
            .checkbox(&mut watching, "Watch for new airfields")
            .on_hover_text("Polls the Missions folder for a rewritten _gen.mission, on any tab.")
            .changed()
        {
            if watching {
                self.start_harvest_watch();
            } else {
                self.harvest_watcher = None;
            }
        }
        if let Some(w) = &self.harvest_watcher {
            ui.label(
                RichText::new(format!("Watching {}", w.dir().display()))
                    .small()
                    .color(c::ACCENT_700),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Harvest current")
                .on_hover_text("Harvest the _gen.mission in the Missions folder now")
                .clicked()
            {
                self.harvest_current_gen();
            }
            if ui
                .button("Harvest a file…")
                .on_hover_text("Harvest any .Mission file")
                .clicked()
            {
                if let Some(path) = dialog::FileDialog::new()
                    .add_filter("IL-2 mission", &["Mission", "mission"])
                    .pick_file()
                {
                    self.run_harvest(&path);
                }
            }
        });
        ui.add_space(4.0);
        let mut ignored_browse = false;
        let mut db_browsed = false;
        let folder_row = |ui: &mut egui::Ui,
                          label: &str,
                          value: &mut String,
                          browsed: &mut bool| {
            ui.label(RichText::new(label).small().color(c::NEUTRAL_700));
            // Right to left: the button takes its real width, the path the rest,
            // so the row never overflows the 290 px panel.
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 28.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if ui.button("Browse…").clicked()
                        && let Some(dir) = dialog::FileDialog::new().pick_folder()
                    {
                        *value = dir.display().to_string();
                        *browsed = true;
                    }
                    ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width()));
                },
            );
        };
        folder_row(
            ui,
            "Game Missions folder",
            &mut self.harvest_missions_dir,
            &mut ignored_browse,
        );
        folder_row(
            ui,
            "Database folder",
            &mut self.harvest_db_dir,
            &mut db_browsed,
        );
        if db_browsed {
            self.airbase_scan_dir.clear();
            self.provisions_loaded_for.clear();
        }
        ui.horizontal(|ui| {
            ui.label("Radius");
            ui.add(
                egui::DragValue::new(&mut self.harvest_cfg.radius_m)
                    .range(1000.0..=10000.0)
                    .speed(50.0)
                    .suffix(" m"),
            )
            .on_hover_text("Objects this close to the field are cut out with it; logic further out is kept through links.");
            ui.checkbox(&mut self.harvest_cfg.keep_ai_planes, "Keep AI planes");
        });
    }

    /// Airfield tab, center: what the last harvests did (newest first).
    fn harvest_log_panel(&mut self, ui: &mut egui::Ui) {
        if self.harvest_log.is_empty() {
            return;
        }
        ui.add_space(12.0);
        shell::blueprint(ui, c::DIVIDER, |ui| {
            shell::section_title(ui, "Database harvest", Some("newest first"));
            for line in self.harvest_log.iter().take(12) {
                ui.add(egui::Label::new(RichText::new(line).monospace()).wrap());
            }
        });
    }

    fn harvest_dir(&self) -> PathBuf {
        PathBuf::from(self.harvest_missions_dir.trim())
    }

    fn start_harvest_watch(&mut self) {
        let dir = self.harvest_dir();
        if !dir.is_dir() {
            self.status = Status::Error(format!("Missions folder not found: {}", dir.display()));
            return;
        }
        self.harvest_watcher = Some(GenWatcher::new(dir));
        self.status = Status::Info(
            "Watching for _gen.mission. Start a Freeflight from the next airfield.".into(),
        );
    }

    fn harvest_current_gen(&mut self) {
        let dir = self.harvest_dir();
        let Some(path) = find_gen_file(&dir) else {
            self.status = Status::Error(format!("No _gen.mission in {}", dir.display()));
            return;
        };
        self.run_harvest(&path);
        if let Some(w) = &mut self.harvest_watcher {
            w.acknowledge_current();
        }
    }

    /// Poll the watcher each frame; restart it if the folder field changed.
    pub(super) fn poll_harvest(&mut self, ctx: &egui::Context) {
        let dir = self.harvest_dir();
        let Some(w) = &mut self.harvest_watcher else {
            return;
        };
        if w.dir() != dir.as_path() {
            *w = GenWatcher::new(dir);
        }
        if let Some(path) = w.poll(std::time::Instant::now()) {
            // The watcher runs on every tab; it reports on the Airfield tab's status.
            self.with_tab_status(AppMode::Airfield, |s| s.run_harvest(&path));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    /// Runs `f` with `mode`'s status as `self.status`, so what it reports
    /// lands on that tab even when another tab is shown.
    fn with_tab_status(&mut self, mode: AppMode, f: impl FnOnce(&mut Self)) {
        if mode == self.mode {
            f(self);
            return;
        }
        let slot = mode_slot(mode);
        let shown = std::mem::replace(&mut self.status, std::mem::take(&mut self.tab_status[slot]));
        f(self);
        self.tab_status[slot] = std::mem::replace(&mut self.status, shown);
    }

    fn run_harvest(&mut self, source: &Path) {
        let db = PathBuf::from(self.harvest_db_dir.trim());
        if let Err(err) = std::fs::create_dir_all(&db) {
            self.status = Status::Error(format!("Could not create {}: {err}", db.display()));
            return;
        }
        match harvest_file(source, &db, &self.harvest_cfg) {
            Ok(out) => {
                self.status = Status::Info(format!(
                    "Harvested {} airfield(s) into {}.",
                    out.airfields.len(),
                    db.display()
                ));
                for line in harvest_log_lines(&out).into_iter().rev() {
                    self.harvest_log.insert(0, line);
                }
            }
            Err(err) => {
                self.harvest_log.insert(0, format!("FAILED: {err}"));
                self.status = Status::Error(format!("Harvest failed: {err}"));
            }
        }
        self.harvest_log.truncate(50);
        self.airbase_scan_dir.clear();
        self.provisions_loaded_for.clear();
    }

    /// Copy the selected airbase's aircraft and coalition back into its slot.
    pub(super) fn remember_airbase_edits(&mut self) {
        let Some(path) = self.airfield_path.clone() else {
            return;
        };
        let Some(slot) = self.airbases.iter_mut().find(|s| s.path == path) else {
            return;
        };
        slot.western = self.airfield_western;
        slot.nato = self.field_spawn_nato;
        slot.planes = self.field_planes.clone();
    }

    fn show_airbase(&mut self, index: usize) {
        self.remember_airbase_edits();
        let Some(slot) = self.airbases.get(index).cloned() else {
            return;
        };
        self.airfield_path = Some(slot.path);
        self.airfield_root = Some(slot.root);
        self.airfield_info = Some(slot.info);
        self.airfield_western = slot.western;
        self.field_spawn_nato = slot.nato;
        self.field_planes = slot.planes;
    }

    fn open_loaded_airfield(
        &mut self,
        path: PathBuf,
        root: Il2Entity,
        info: crate::airfield::AirfieldInfo,
    ) {
        self.remember_airbase_edits();
        let prior = self.airfield_path.clone();
        if let Some(slot) = self.airbases.iter().find(|s| s.path == path) {
            self.field_planes = slot.planes.clone();
            self.field_spawn_nato = slot.nato;
            self.airfield_western = slot.western;
        } else if prior.is_some() {
            let nato = guess_nato(&root, &path);
            self.field_spawn_nato = nato;
            self.airfield_western = nato;
            self.field_planes = vec![new_field_plane(nato)];
        }
        let from_folder =
            provisions::harvested_airfield_paths(Path::new(self.harvest_db_dir.trim()))
                .iter()
                .any(|p| p == &path);
        if let Some(slot) = self.airbases.iter_mut().find(|s| s.path == path) {
            slot.root = root.clone();
            slot.info = info.clone();
            slot.planes = self.field_planes.clone();
            slot.nato = self.field_spawn_nato;
            slot.western = self.airfield_western;
            slot.from_folder = from_folder;
        } else {
            self.airbases.push(AirbaseSlot {
                path: path.clone(),
                root: root.clone(),
                info: info.clone(),
                western: self.airfield_western,
                nato: self.field_spawn_nato,
                planes: self.field_planes.clone(),
                from_folder,
            });
        }
        self.airfield_info = Some(info);
        self.airfield_root = Some(root);
        self.airfield_path = Some(path);
    }

    fn sync_airbases(&mut self) {
        if !self.airbase_scan_dir.is_empty() {
            return;
        }
        let dir = self.harvest_db_dir.trim().to_string();
        self.rescan_airbases(Path::new(&dir));
        self.airbase_scan_dir = dir;
        if self.airfield_path.is_none() && !self.airbases.is_empty() {
            self.show_airbase(0);
        } else if let Some(path) = self.airfield_path.clone() {
            if let Some(slot) = self.airbases.iter().find(|s| s.path == path) {
                self.airfield_root = Some(slot.root.clone());
                self.airfield_info = Some(slot.info.clone());
            } else {
                self.airfield_path = None;
                self.airfield_root = None;
                self.airfield_info = None;
            }
        }
    }

    fn rescan_airbases(&mut self, dir: &Path) {
        let loaded: Vec<_> = provisions::harvested_airfield_paths(dir)
            .into_iter()
            .filter_map(|path| provisions::load_group(&path).ok())
            .collect();
        let placed = self.place_harvested_airfields(loaded);
        self.airfield_coalition_notes = placed.notes;
        let old = std::mem::take(&mut self.airbases);
        let mut next = Vec::new();
        for group in placed.groups {
            if let Some(prev) = old.iter().find(|s| s.path == group.path) {
                let mut slot = prev.clone();
                slot.info = inspect_airfield(&group.entity);
                slot.root = group.entity;
                slot.from_folder = true;
                next.push(slot);
                continue;
            }
            next.push(AirbaseSlot::scanned(group.path, group.entity));
        }
        for slot in old {
            if !slot.from_folder && next.iter().all(|s| s.path != slot.path) {
                next.push(slot);
            }
        }
        self.airbases = next;
        self.airbase_index = crate::harvest::read_catalog(&dir.join("catalog.Group"))
            .unwrap_or_default()
            .into_iter()
            .map(|record| IndexedAirbase {
                name: record.name,
                x: record.x,
                z: record.z,
                file: record.file,
                country: record.country,
            })
            .collect();
    }

    pub(super) fn place_harvested_airfields(
        &self,
        groups: Vec<crate::frontlines::MapRefGroup>,
    ) -> provisions::AirfieldPlacement {
        let front = self.front_for_sides();
        provisions::place_matching_coalition(groups, |x, z| {
            !on_dprk_side(&front, &self.salients, x, z)
        })
    }

    pub(super) fn korea_fields_panel(&self, ui: &mut egui::Ui) {
        let report = korea_fields::ao_harvest(
            &self.harvest_hits(),
            self.front_aabb.expanded(PLACE_MARGIN),
            &self.front_for_sides(),
            &self.salients,
        );
        ui.add_space(8.0);
        ui.separator();
        shell::section_title(ui, "Korea fields", Some(&report.inside.len().to_string()));
        shell::hint(
            ui,
            "Listed fields inside the map area. A harvest for the other coalition still counts as missing.",
            false,
        );
        if report.inside.is_empty() {
            shell::hint(ui, "No listed Korea fields in this area.", false);
        }
        for row in &report.inside {
            let state = if row.harvested { "Harvested" } else { "Missing" };
            let text = format!("{} · {} · {state}", row.name, row.coalition);
            let color = if row.harvested { c::TEXT } else { c::WARN_TEXT };
            ui.label(RichText::new(text).color(color))
                .on_hover_text(row.source);
        }
        ui.label(
            RichText::new(format!("Outside this area: {}", report.outside)).color(c::NEUTRAL_700),
        );
        if !report.unplaced.is_empty() {
            ui.label(
                RichText::new(format!(
                    "No position on this map: {}",
                    report.unplaced.join(", ")
                ))
                .color(c::NEUTRAL_700),
            );
        }
    }

    fn harvest_hits(&self) -> Vec<HarvestHit> {
        let mut hits = Vec::new();
        for row in &self.airbase_index {
            hits.push(HarvestHit {
                name: row.name.clone(),
                country: row.country,
            });
            hits.push(HarvestHit {
                name: row.file.clone(),
                country: row.country,
            });
        }
        for slot in &self.airbases {
            let id = provisions::airfield_identity(&slot.root, &slot.path);
            if let Some(country) = id.country {
                hits.push(HarvestHit {
                    name: id.key,
                    country,
                });
            }
        }
        hits
    }

    fn field_spawn_aircraft(&self) -> (Vec<AirStartPlane>, i32) {
        let country = if self.field_spawn_nato {
            airstart::COUNTRY_NATO
        } else {
            airstart::COUNTRY_DPRK
        };
        (self.field_planes.clone(), country)
    }

    fn export_field_spawn(&mut self) {
        self.remember_airbase_edits();
        let Some(mut root) = self.airfield_root.clone() else {
            self.status = Status::Error("Load an airfield first.".into());
            return;
        };
        let (planes, country) = self.field_spawn_aircraft();
        if planes.is_empty() {
            self.status = Status::Error("Add an aircraft to the field spawn first.".into());
            return;
        }
        let spot = match place_field_spawn(&mut root, &planes, None, Some(country)) {
            Ok(spot) => spot,
            Err(err) => {
                self.status = Status::Error(err);
                return;
            }
        };
        let suggested = self
            .airfield_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|stem| format!("{stem}_field.Group"))
            .unwrap_or_else(|| "Airfield_field.Group".into());
        let Some(save_path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .set_file_name(&suggested)
            .save_file()
        else {
            return;
        };
        let locale = self
            .airfield_path
            .as_ref()
            .map(|p| vec![p.clone()])
            .unwrap_or_default();
        let summary = format!(
            "Field spawn at {:.0}, {:.0}, heading {:.0}°",
            spot.x, spot.z, spot.heading_deg
        );
        self.status = save_with_sidecars(&save_path, &serialize_group(&root), &locale, &summary);
    }

    pub(super) fn export_airfield(&mut self) {
        self.remember_airbase_edits();
        let Some(root) = self.airfield_root.clone() else {
            self.status = Status::Error("Load an airfield first.".into());
            return;
        };
        let mut cleaned = root;
        let coalitions = if self.airfield_western {
            WESTERN_PLANE_COALITIONS
        } else {
            EASTERN_PLANE_COALITIONS
        };
        let report = match clean_airfield(&mut cleaned, coalitions) {
            Ok(r) => r,
            Err(err) => {
                self.status = Status::Error(err);
                return;
            }
        };
        if cleaned.block_type == "Group" {
            if matches!(cleaned.name(), Some("Group") | Some("Airfield") | None) {
                if let Some(stem) = self
                    .airfield_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                {
                    cleaned.set_name(stem);
                }
            }
        }
        let text = serialize_group(&cleaned);
        let suggested = self
            .airfield_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|stem| format!("{stem}_mp.Group"))
            .unwrap_or_else(|| "Airfield_mp.Group".into());
        let Some(save_path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .set_file_name(&suggested)
            .save_file()
        else {
            return;
        };
        let locale = self
            .airfield_path
            .as_ref()
            .map(|p| vec![p.clone()])
            .unwrap_or_default();
        let summary = format!(
            "Exported airfield (stripped {} objects, unlinked {} checkzones to {})",
            report.stripped, report.unlinked_checkzones, report.plane_coalitions
        );
        self.status = save_with_sidecars(&save_path, &text, &locale, &summary);
    }
}

fn airbase_button(ui: &mut egui::Ui, label: &str, selected: bool, data_required: bool) -> bool {
    let button = egui::Button::selectable(selected, label)
        .min_size(Vec2::new(ui.available_width(), 28.0));
    let clicked = ui
        .add_sized([ui.available_width(), 28.0], button)
        .clicked();
    if data_required {
        ui.label(RichText::new("Data required").color(c::WARN_TEXT));
    }
    clicked
}

/// The map keeps a reference group when any of its objects falls inside the
/// area box plus 10 km. An airbase is in the area on that same rule.
fn airbase_in_area(root: &Il2Entity, area: WorldAabb) -> bool {
    if let Some((x, z)) = root.pos_xz()
        && area.contains(x, z)
    {
        return true;
    }
    root.children.iter().any(|child| airbase_in_area(child, area))
}

/// A field spawn needs one airfield and two runway thresholds. Without those
/// the harvested file cannot take aircraft.
fn airbase_needs_data(root: &Il2Entity) -> bool {
    let mut airfields = 0;
    let mut ends = 0;
    root.for_each(&mut |entity| {
        if entity.block_type == "Airfield"
            && entity.index.is_some()
            && entity.property("LinkTrId").is_some()
        {
            airfields += 1;
        }
        if entity
            .property("RunwayEnd")
            .is_some_and(|value| value.trim() == "0")
        {
            ends += 1;
        }
    });
    airfields != 1 || ends < 2
}

fn same_airbase_file(path: &Path, file: &str) -> bool {
    let want = Path::new(file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file);
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(want))
}

struct AreaGap {
    name: String,
    x: f64,
    z: f64,
}

/// Catalog rows inside the area whose group file was not loaded.
fn uncovered_in_area(
    index: &[IndexedAirbase],
    slots: &[AirbaseSlot],
    area: WorldAabb,
    front: &[(f64, f64)],
    salients: &[Vec<(f64, f64)>],
) -> Vec<AreaGap> {
    index
        .iter()
        .filter(|record| {
            if !area.contains(record.x, record.z) {
                return false;
            }
            if slots
                .iter()
                .any(|slot| same_airbase_file(&slot.path, &record.file))
            {
                return false;
            }
            if let Some(nato) = country_is_nato(record.country) {
                let on_dprk = on_dprk_side(front, salients, record.x, record.z);
                if nato == on_dprk {
                    return false;
                }
            }
            true
        })
        .map(|record| AreaGap {
            name: if record.name.is_empty() {
                Path::new(&record.file)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Airbase")
                    .to_string()
            } else {
                record.name.clone()
            },
            x: record.x,
            z: record.z,
        })
        .collect()
}

fn airbase_point(root: &Il2Entity) -> Option<(f64, f64)> {
    let mut at = None;
    root.for_each(&mut |entity| {
        if at.is_none() && entity.block_type == "Airfield" {
            at = entity.pos_xz();
        }
    });
    at.or_else(|| root.first_xz())
}

fn airbase_label(slot: &AirbaseSlot) -> String {
    let name =
        if slot.info.name.is_empty() || slot.info.name == "Group" || slot.info.name == "Airfield" {
            slot.path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Airbase")
                .to_string()
        } else {
            slot.info.name.clone()
        };
    if slot.planes.is_empty() {
        name
    } else {
        format!("{name} · {}", slot.planes.len())
    }
}

fn new_field_plane(nato: bool) -> AirStartPlane {
    let id = if nato { "f51d" } else { "mig15bis" };
    let mut plane = airstart::default_plane(id);
    plane.altitude_m = 0;
    plane
}

fn plane_summary(plane: &AirStartPlane, field_spawn: bool) -> String {
    let count = if plane.number < 0 {
        "unlimited".to_string()
    } else {
        plane.number.to_string()
    };
    let kind = if field_spawn {
        if plane.parking_snap {
            "Engine off, parking".to_string()
        } else {
            "Engine running".to_string()
        }
    } else {
        format!("{} m", plane.altitude_m)
    };
    format!(
        "{} · {} · {} · {}",
        airstart::aircraft_label(&plane.type_id),
        builder::skill_name(plane.ai_level),
        kind,
        count
    )
}

/// Shared aircraft form. `field_spawn` draws the ground-start choice and
/// skips altitude, which a field spawn always writes as 0.
fn plane_card(ui: &mut egui::Ui, plane: &mut AirStartPlane, salt: &str, field_spawn: bool) {
    use crate::payloads;
    let ids = airstart::aircraft_ids();
    let current = airstart::aircraft_label(&plane.type_id);
    egui::ComboBox::from_id_salt(format!("{salt}_type"))
        .selected_text(current)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for id in &ids {
                let label = airstart::aircraft_label(id);
                if ui.selectable_label(plane.type_id == *id, label).clicked() {
                    airstart::apply_type(plane, id);
                }
            }
        });
    ui.add_space(4.0);
    if field_spawn {
        ui.label("Start");
        let start = if plane.parking_snap {
            "Engine off, parking"
        } else {
            "Engine running"
        };
        egui::ComboBox::from_id_salt(format!("{salt}_start"))
            .selected_text(start)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(!plane.parking_snap, "Engine running")
                    .clicked()
                {
                    plane.parking_snap = false;
                }
                if ui
                    .selectable_label(plane.parking_snap, "Engine off, parking")
                    .clicked()
                {
                    plane.parking_snap = true;
                }
            });
        ui.add_space(4.0);
    }
    let mut unlimited = plane.number < 0;
    ui.checkbox(&mut unlimited, "Unlimited");
    if unlimited {
        plane.number = -1;
    } else {
        if plane.number < 1 {
            plane.number = 1;
        }
        let mut count = plane.number.clamp(1, 200) as u32;
        labeled_slider_suffix(ui, "Available", &mut count, 1..=200, "");
        plane.number = count as i32;
    }
    ui.label("Skill");
    egui::ComboBox::from_id_salt(format!("{salt}_skill"))
        .selected_text(builder::skill_name(plane.ai_level))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for skill in 0..=4 {
                if ui
                    .selectable_label(plane.ai_level == skill, builder::skill_name(skill))
                    .clicked()
                {
                    plane.ai_level = skill;
                }
            }
        });
    if !field_spawn {
        let mut altitude = plane.altitude_m.clamp(0, 12_000) as u32;
        labeled_slider_suffix(ui, "Altitude", &mut altitude, 0..=12_000, " m");
        plane.altitude_m = altitude as i32;
    }
    let mut fuel_pct = (plane.fuel.clamp(0.0, 1.0) * 100.0).round() as u32;
    labeled_slider_suffix(ui, "Fuel", &mut fuel_pct, 0..=100, "%");
    plane.fuel = fuel_pct as f64 / 100.0;

    let script = plane.script();
    let loadout = payloads::catalog().for_script(&script);
    if loadout.is_some_and(|a| a.has_payloads()) {
        ui.label("Payload");
        let selected = payloads::payload_preview(&script, plane.payload_id);
        egui::ComboBox::from_id_salt(format!("{salt}_payload"))
            .selected_text(selected)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if let Some(ac) = payloads::catalog().for_script(&script) {
                    for p in &ac.payloads {
                        let summary = format!("{}  {}", p.id, p.summary());
                        if ui
                            .selectable_label(plane.payload_id == p.id, summary)
                            .clicked()
                        {
                            plane.payload_id = p.id;
                        }
                    }
                }
            });
    }
    if loadout.is_some_and(|a| a.has_mods()) {
        ui.label("Modifications");
        let preview = payloads::mods_preview(&script, &plane.mod_mask);
        let label = if preview == "—" {
            "Select…"
        } else {
            preview.as_str()
        };
        ui.menu_button(label, |ui| {
            ui.set_min_width(240.0);
            let Some(ac) = payloads::catalog().for_script(&script) else {
                return;
            };
            let mut mask = payloads::parse_mod_mask_for(&script, &plane.mod_mask);
            let mut changed = false;
            for slot in &ac.mod_slots {
                if !payloads::slot_has_choices(slot) {
                    continue;
                }
                ui.separator();
                if payloads::slot_choice_count(slot) > 1 {
                    ui.label(payloads::mod_slot_title(&script, slot.number));
                    let none_on = payloads::exclusive_selection(mask, slot).is_none();
                    if ui.selectable_label(none_on, "None").clicked() {
                        mask = payloads::clear_exclusive_for(&script, mask, slot);
                        changed = true;
                    }
                    for opt in &slot.options {
                        if payloads::extra_bits(&opt.binary_id) == 0 {
                            continue;
                        }
                        let selected = payloads::exclusive_selection(mask, slot)
                            .is_some_and(|s| s.binary_id == opt.binary_id);
                        if ui.selectable_label(selected, &opt.description).clicked() {
                            mask = payloads::select_exclusive_for(&script, mask, slot, opt);
                            changed = true;
                        }
                    }
                } else if let Some(opt) = slot
                    .options
                    .iter()
                    .find(|o| payloads::extra_bits(&o.binary_id) != 0)
                {
                    let mut on = payloads::option_selected(mask, opt);
                    if ui.checkbox(&mut on, &opt.description).changed() {
                        mask = payloads::set_toggle_for(&script, mask, opt, on);
                        changed = true;
                    }
                }
            }
            if changed {
                plane.mod_mask = payloads::encode_mod_mask(mask);
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut plane.renewable, "Renewable");
        ui.checkbox(&mut plane.limit_ammo, "Limit ammo");
        ui.checkbox(&mut plane.engageable, "Engageable");
        ui.checkbox(&mut plane.vulnerable, "Vulnerable");
        ui.checkbox(&mut plane.rtb, "Return to base");
    });
    if plane.renewable {
        let mut renew = plane.renew_time_s.clamp(0, 86_400) as u32;
        labeled_slider_suffix(ui, "Renew time", &mut renew, 0..=7_200, " s");
        plane.renew_time_s = renew as i32;
    }
}

/// Country name without its code: 601 → "USA".
pub(super) fn country_name(country: i32) -> String {
    COUNTRIES
        .iter()
        .find(|(id, _)| *id == country)
        .and_then(|(_, label)| label.split_whitespace().last())
        .map_or_else(|| format!("country {country}"), str::to_owned)
}

/// A label on the left, a monospace value on the right, a hairline below.
fn fact_row(ui: &mut egui::Ui, label: &str, value: &str) {
    let resp = ui.horizontal(|ui| {
        ui.set_min_height(26.0);
        ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).monospace().color(c::NEUTRAL_800));
        });
    });
    let r = resp.response.rect;
    ui.painter().hline(
        r.x_range(),
        r.bottom() + 1.0,
        Stroke::new(1.0_f32, c::DIVIDER),
    );
}

/// "1  In game, …" with a 22 px boxed number.
fn numbered_step(ui: &mut egui::Ui, n: u32, text: &str) {
    ui.horizontal_top(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
        ui.painter().rect_stroke(
            r,
            0.0,
            Stroke::new(1.0_f32, c::NEUTRAL_500),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            n.to_string(),
            FontId::monospace(12.0),
            c::TEXT,
        );
        ui.add(egui::Label::new(text).wrap());
    });
    ui.add_space(4.0);
}

fn harvest_log_lines(out: &HarvestOutcome) -> Vec<String> {
    let mut lines: Vec<String> = out
        .airfields
        .iter()
        .map(|af| {
            let r = &af.record;
            let mut line = format!(
                "{} ({}) -> {}: {} vehicles, {} blocks, {} logic, {} taxi nodes",
                r.name,
                country_short(r.country),
                r.file,
                r.vehicles,
                r.blocks,
                r.logic,
                r.taxi_nodes
            );
            if af.cleaned.stripped > 0 {
                line.push_str(&format!(
                    "; stripped {} player/SP objects",
                    af.cleaned.stripped
                ));
            }
            if af.ai_planes_removed > 0 {
                line.push_str(&format!("; removed {} AI planes", af.ai_planes_removed));
            }
            if af.via_links > 0 {
                line.push_str(&format!(
                    "; {} logic nodes beyond the radius kept via links",
                    af.via_links
                ));
            }
            line
        })
        .collect();
    if out.models_added > 0 {
        lines.push(format!(
            "  {} new model(s) added to models.tsv",
            out.models_added
        ));
    }
    let raw = out
        .archived
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    lines.push(format!("  raw mission archived as raw/{raw}"));
    lines
}
