//! Air Tasking rail tab. Plan decisions belong to the public `airtask` API.

use super::*;
use crate::airtask::{
    self, AirContext, AirPlan, AirRole, AirSide, AirSortie, BudgetImpl, EntryStatus, IssueSeverity,
    PoolEntry, SlotMark, SortieSource, TargetChoice, TargetKind, TriggerMode, Ymd,
};

type Edit = Option<(egui::Id, String)>;

impl GroupGeneratorApp {
    pub(super) fn new_air_plan() -> AirPlan {
        AirPlan {
            library: airtask::builtin_library(),
            seed: Self::placement_seed(),
            ..AirPlan::default()
        }
    }

    /// Use exactly the full/composite front used to colour Map's arrows, without AO clipping.
    pub(super) fn with_air_context<R>(&self, read: impl FnOnce(&AirContext<'_>) -> R) -> R {
        let raw = if self.custom_front_xz.is_empty() {
            preview_front_xz(self.front_t)
        } else {
            self.custom_front_xz.clone()
        };
        let full = crate::frontlines::densify(&raw, 4000.0);
        let (composite, _) = apply_salients(full.clone(), &self.salients);
        let mark = self.current_mark();
        read(&AirContext {
            east_objectives: &self.east_objectives,
            nato_objectives: &self.nato_objectives,
            arrows: &self.attack_arrows,
            front: if composite.len() >= 2 {
                &composite
            } else {
                &full
            },
            aabb: self.front_aabb,
            date: (mark.year, mark.month, mark.day),
        })
    }

    pub(super) fn air_header_label(&self) -> String {
        let date = self.with_air_context(|ctx| date_text(ctx.date));
        format!(
            "Map date {date} · {} {}",
            self.air_plan.pool.len(),
            entry_word(self.air_plan.pool.len())
        )
    }

    pub(super) fn air_header_buttons(ui: &mut egui::Ui, load: &mut bool, reset: &mut bool) {
        *reset = ui
            .button("Reset")
            .on_hover_text("Restore defaults and clear the pool. Library stays.")
            .clicked();
        *load = ui
            .button("Add template…")
            .on_hover_text("Add an Activate-mode sortie from a .Group file.  (Ctrl O)")
            .clicked();
    }

    pub(super) fn air_readiness(&self) -> Vec<shell::Check> {
        self.with_air_context(|ctx| {
            let mut checks = vec![shell::Check::new(ctx.front.len() >= 2, "A front is loaded")];
            checks.extend(
                airtask::validate(&self.air_plan, ctx)
                    .into_iter()
                    .filter(|issue| issue.severity == IssueSeverity::Error)
                    .map(|issue| shell::Check::new(false, issue.text)),
            );
            checks
        })
    }

    pub(super) fn air_idle_problem(&self) -> Option<String> {
        self.air_readiness()
            .into_iter()
            .find(|check| !check.ok)
            .map(|check| check.text)
    }

    pub(super) fn air_fingerprint(&self) -> String {
        format!("{:?}", self.air_plan)
    }

    pub(super) fn undo_air_plan(&mut self) {
        if let Some(plan) = self.air_undo.take() {
            self.air_plan = plan;
            self.air_editing = None;
            self.air_selection_bounds();
        }
    }

    fn air_selection_bounds(&mut self) {
        self.air_selected = self.air_selected.filter(|&i| i < self.air_plan.pool.len());
        self.air_library_selected = self
            .air_library_selected
            .filter(|&i| i < self.air_plan.library.len());
    }

    pub(super) fn air_reset_prompt(&self) -> (&'static str, String, &'static str) {
        (
            "Reset Air Tasking?",
            format!(
                "This clears {} {}, restores every setting and draws a new seed. The library stays. You can undo with Ctrl Z.",
                self.air_plan.pool.len(),
                entry_word(self.air_plan.pool.len())
            ),
            "Reset",
        )
    }

    pub(super) fn reset_air_plan(&mut self) {
        self.air_undo
            .record("Reset Air Tasking", self.air_plan.clone());
        let library = self.air_plan.library.clone();
        self.air_plan = AirPlan {
            library,
            seed: Self::placement_seed(),
            ..AirPlan::default()
        };
        self.air_selected = None;
        self.air_editing = None;
        self.status = Status::Info("Reset Air Tasking. Library kept.".into());
    }

    pub(super) fn add_air_template(&mut self) {
        let Some(path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group", "group"])
            .pick_file()
        else {
            return;
        };
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if let Some(index) = self
            .air_plan
            .library
            .iter()
            .position(|sortie| match &sortie.source {
                SortieSource::File(existing) => {
                    existing.canonicalize().unwrap_or_else(|_| existing.clone()) == canonical
                }
                SortieSource::BuiltIn(_) => false,
            })
        {
            self.air_library_selected = Some(index);
            self.status = Status::Info("That template is already in the library.".into());
            return;
        }
        match airtask::load_user_sortie(&path) {
            Ok(sortie) => {
                self.air_undo
                    .record(format!("Added {}", sortie.name), self.air_plan.clone());
                self.status = Status::Info(format!("Added {} to the library.", sortie.name));
                self.air_library_selected = Some(self.air_plan.library.len());
                self.air_picked_target = None;
                self.air_plan.library.push(sortie);
                self.air_editing = None;
            }
            Err(error) => self.status = Status::Error(error),
        }
    }

    fn add_air_entry(&mut self) {
        let Some(sortie) = self
            .air_library_selected
            .and_then(|i| self.air_plan.library.get(i))
        else {
            return;
        };
        let choices = self.with_air_context(|ctx| airtask::target_choices(ctx, sortie.side));
        let Some(target) = self.air_picked_target.filter(|t| choices.contains(t)) else {
            return;
        };
        let entry = PoolEntry::new(sortie.id.clone(), target.target);
        self.air_undo.record(
            format!("Added {} to the pool", sortie.name),
            self.air_plan.clone(),
        );
        self.air_selected = Some(self.air_plan.pool.len());
        airtask::add_entry(&mut self.air_plan, entry);
        self.air_editing = None;
    }

    fn auto_fill_air_pool(&mut self) {
        let mut plan = self.air_plan.clone();
        let count = self.with_air_context(|ctx| {
            airtask::auto_fill(&mut plan, ctx, AirSide::Dprk)
                + airtask::auto_fill(&mut plan, ctx, AirSide::Nato)
        });
        if count > 0 {
            self.air_undo.record(
                format!("Added {count} {}", entry_word(count)),
                self.air_plan.clone(),
            );
            self.air_plan = plan;
            self.air_editing = None;
        }
        self.status = Status::Info(format!("Added {count} entries"));
    }

    fn remove_air_library_item(&mut self, index: usize) {
        let name = self.air_plan.library[index].name.clone();
        let mut plan = self.air_plan.clone();
        match airtask::remove_library_item(&mut plan, index) {
            Ok(count) => {
                let label = format!("Removed {name} and {count} {}", entry_word(count));
                self.air_undo.record(&label, self.air_plan.clone());
                self.air_plan = plan;
                self.air_selected = None;
                self.air_library_selected =
                    index
                        .checked_sub(1)
                        .or((!self.air_plan.library.is_empty()).then_some(0));
                self.air_picked_target = None;
                self.air_editing = None;
                self.status = Status::Info(label);
            }
            Err(error) => self.status = Status::Error(error),
        }
    }

    fn remove_air_entry(&mut self, index: usize) {
        self.air_undo.record(
            format!("Removed pool entry {}", index + 1),
            self.air_plan.clone(),
        );
        self.air_plan.pool.remove(index);
        self.air_selected = None;
        self.air_editing = None;
    }

    /// Widgets edit a draft; record the live plan before committing it. A drag or
    /// text edit keeps its first snapshot across frames, then settles on release.
    fn apply_air_edit(&mut self, plan: AirPlan, edit: Edit) {
        if plan == self.air_plan {
            return;
        }
        let Some((id, label)) = edit else {
            return;
        };
        if self.air_editing == Some(id) {
            self.air_undo.rebase();
        } else {
            self.air_undo.record(label, self.air_plan.clone());
        }
        self.air_plan = plan;
        self.air_editing = Some(id);
    }

    pub(super) fn air_page(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.wants_keyboard_input() {
            self.air_editing = None;
        }
        self.air_selection_bounds();
        side_panel(ctx, "air_left", true, 270.0, |ui| self.air_library(ui));
        side_panel(ctx, "air_right", false, 304.0, |ui| self.air_settings(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_blueprint_grid(ui.painter(), ui.max_rect());
            egui::ScrollArea::vertical().id_salt("air_center").auto_shrink([false, false]).show(ui, |ui| {
                air_sheet(ui, |ui| self.air_pool(ui));
                ui.add_space(10.0);
                air_sheet(ui, |ui| {
                    shell::section_title(ui, "Slot timeline", Some("0–3 h · one row per side"));
                    let marks = self.with_air_context(|ctx| airtask::preview(&self.air_plan, ctx).slot_times);
                    draw_slot_strip(ui, &marks);
                    shell::hint(ui, "Ticks: slot times · shading: jitter span · fainter tick: higher quiet chance.", false);
                });
                ui.add_space(10.0);
                air_sheet(ui, |ui| {
                    shell::section_title(ui, "Plan check", Some("errors block Generate · warnings do not"));
                    let issues = self.with_air_context(|ctx| airtask::validate(&self.air_plan, ctx));
                    if issues.is_empty() { shell::hint(ui, "No issues reported.", false); }
                    for issue in issues {
                        let glyph = if issue.severity == IssueSeverity::Error { "✖" } else { "⚠" };
                        ui.colored_label(c::WARN_TEXT, format!("{glyph} {}", issue.text));
                    }
                });
            });
        });
    }

    fn air_library(&mut self, ui: &mut egui::Ui) {
        let date = self.with_air_context(|ctx| ctx.date);
        shell::section_title(
            ui,
            &format!("Library · {}", self.air_plan.library.len()),
            Some(&format!("valid for {}", date_text(date))),
        );
        for (index, sortie) in self.air_plan.library.iter().enumerate() {
            let card = shell::card(ui, self.air_library_selected == Some(index), |ui| {
                ui.horizontal(|ui| {
                    shell::side_marker(ui, marker_side(sortie.side), 12.0);
                    ui.add(egui::Label::new(RichText::new(&sortie.name).strong()).wrap());
                });
                ui.horizontal_wrapped(|ui| {
                    shell::tag(ui, role_text(sortie), true);
                    if escorted(sortie) {
                        shell::tag(ui, "Escorted", false);
                    }
                    if matches!(sortie.source, SortieSource::File(_)) {
                        shell::tag(ui, "File", false);
                    }
                });
                let models = sortie
                    .models
                    .iter()
                    .map(|model| {
                        model_spec::spec_for(model)
                            .map_or_else(|| model.clone(), |spec| spec.label.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
                ui.label(RichText::new(format!("{} × {models}", sortie.planes)).small());
                let (valid, badge) = date_badge(sortie, date);
                ui.label(RichText::new(badge).small().color(if valid {
                    c::ACCENT_700
                } else {
                    c::WARN_TEXT
                }));
            });
            let response = ui.interact(
                card.response.rect,
                ui.id().with(("sortie", index)),
                Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    true,
                    self.air_library_selected == Some(index),
                    format!("Library sortie {}", index + 1),
                )
            });
            if response.clicked() {
                self.air_library_selected = Some(index);
                self.air_picked_target = None;
            }
            ui.add_space(4.0);
        }
        ui.separator();
        if let Some(index) = self.air_library_selected {
            let sortie = &self.air_plan.library[index];
            ui.label(format!("Selected: {}", sortie.name));
            let choices = self.with_air_context(|ctx| airtask::target_choices(ctx, sortie.side));
            if !self
                .air_picked_target
                .is_some_and(|picked| choices.contains(&picked))
            {
                self.air_picked_target = choices.first().copied();
            }
            ui.horizontal(|ui| {
                ui.label("Target");
                ui.add_enabled_ui(!choices.is_empty(), |ui| {
                    let label = self
                        .air_picked_target
                        .map_or_else(|| "No targets".into(), target_label);
                    let response = egui::ComboBox::from_id_salt("air_target")
                        .width(175.0)
                        .selected_text(&label)
                        .show_ui(ui, |ui| {
                            for choice in choices.iter().copied() {
                                ui.selectable_value(
                                    &mut self.air_picked_target,
                                    Some(choice),
                                    target_label(choice),
                                );
                            }
                        })
                        .response;
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::ComboBox,
                            ui.is_enabled(),
                            &label,
                        )
                    });
                });
            });
            if choices.is_empty() {
                shell::hint(ui, "Place objectives or arrows on the Map first.", false);
            }
            let removable = matches!(sortie.source, SortieSource::File(_));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.air_picked_target.is_some(),
                        egui::Button::new("Add to pool"),
                    )
                    .clicked()
                {
                    self.add_air_entry();
                }
                if ui
                    .add_enabled(removable, egui::Button::new("Remove"))
                    .clicked()
                {
                    self.remove_air_library_item(index);
                }
            });
        } else {
            shell::hint(ui, "Select a library sortie to add it to the pool.", false);
        }
        shell::hint(ui, "Built-in sorties stay in the library.", false);
    }

    fn air_pool(&mut self, ui: &mut egui::Ui) {
        let counts = [AirSide::Dprk, AirSide::Nato].map(|side| {
            self.air_plan
                .pool
                .iter()
                .filter(|entry| {
                    self.air_plan
                        .library
                        .iter()
                        .any(|s| s.id == entry.sortie_id && s.side == side)
                })
                .count()
        });
        ui.horizontal(|ui| {
            air_cell(ui, 80.0, |ui| {
                shell::section_title(ui, &format!("Pool · {}", self.air_plan.pool.len()), None)
            });
            ui.label(RichText::new(format!("DPRK {} · NATO {}", counts[0], counts[1])).small());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Auto-fill from objectives").clicked() {
                    self.auto_fill_air_pool();
                }
            });
        });
        if self.air_plan.pool.is_empty() {
            shell::hint(
                ui,
                "Pick a library sortie and a target, then Add to pool.",
                false,
            );
            if ui
                .add_enabled(
                    self.air_picked_target.is_some(),
                    egui::Button::new("Add to pool"),
                )
                .clicked()
            {
                self.add_air_entry();
            }
            return;
        }
        let states = self.with_air_context(|ctx| {
            self.air_plan
                .pool
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    (
                        airtask::entry_status(&self.air_plan, i, ctx),
                        airtask::resolve_target(&entry.target, ctx),
                    )
                })
                .collect::<Vec<_>>()
        });
        let mut plan = self.air_plan.clone();
        let mut edit = None;
        let mut remove = None;
        let width = ui.available_width();
        let widths = [
            28.0,
            (width - 358.0).max(42.0),
            86.0,
            46.0,
            72.0,
            40.0,
            28.0,
            28.0,
        ];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (label, width) in ["", "Sortie", "Target", "Weight", "Mode", "Runs", "OK", ""]
                .into_iter()
                .zip(widths)
            {
                air_cell(ui, width, |ui| {
                    ui.label(RichText::new(label).small().color(c::NEUTRAL_700));
                });
            }
        });
        ui.separator();
        for (index, entry) in plan.pool.iter_mut().enumerate() {
            let sortie = plan.library.iter().find(|s| s.id == entry.sortie_id);
            let name = sortie.map_or("Missing sortie", |s| s.name.as_str());
            let (status, target) = states[index];
            let (rect, row) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
            row.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Other,
                    true,
                    self.air_selected == Some(index),
                    format!("Pool entry {}", index + 1),
                )
            });
            if self.air_selected == Some(index) {
                ui.painter().rect_filled(rect, 0.0, c::ACCENT_100);
            }
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().button_padding.x = 4.0;
                if !entry.enabled { ui.visuals_mut().override_text_color = Some(c::NEUTRAL_600); ui.set_opacity(0.55); }
                air_cell(ui, widths[0], |ui| {
                    let r = ui.add_sized([28.0, 28.0], egui::Checkbox::without_text(&mut entry.enabled));
                    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, entry.enabled, format!("Entry {} enabled", index + 1)));
                    note_edit(&mut edit, &r, "Changed entry on/off");
                });
                air_cell(ui, widths[1], |ui| {
                    if let Some(s) = sortie { shell::side_marker(ui, marker_side(s.side), 12.0); }
                    ui.add(egui::Label::new(name).truncate()).on_hover_text(name);
                });
                air_cell(ui, widths[2], |ui| {
                    let text = entry_target_text(status, target);
                    let color = if matches!(status, EntryStatus::Orphaned | EntryStatus::SideChanged) { c::WARN_TEXT } else { ui.visuals().text_color() };
                    ui.add(egui::Label::new(RichText::new(text).color(color)).truncate());
                });
                air_cell(ui, widths[3], |ui| {
                    let r = ui.add(egui::DragValue::new(&mut entry.weight).range(1..=10).speed(0.1));
                    name_number(&r, &format!("Entry {} weight", index + 1), entry.weight as f64);
                    note_edit(&mut edit, &r, "Changed weight");
                });
                air_cell(ui, widths[4], |ui| {
                    let before = entry.mode;
                    let response = egui::ComboBox::from_id_salt(("air_mode", index)).width(68.0)
                        .selected_text(mode_text(entry.mode)).show_ui(ui, |ui| {
                            for mode in [TriggerMode::Clock, TriggerMode::Zone, TriggerMode::Both] {
                                ui.selectable_value(&mut entry.mode, mode, mode_text(mode));
                            }
                        }).response;
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, format!("Entry {} mode", index + 1)));
                    if entry.mode != before { edit = Some((response.id, "Changed mode".into())); }
                });
                air_cell(ui, widths[5], |ui| {
                    let r = ui.add(egui::DragValue::new(&mut entry.runs).range(1..=6).speed(0.1))
                        .on_hover_text("At most this many sorties of this entry per mission (clock and zone together)");
                    name_number(&r, &format!("Entry {} runs", index + 1), entry.runs as f64);
                    note_edit(&mut edit, &r, "Changed runs");
                });
                air_cell(ui, widths[6], |ui| {
                    let ok = status == EntryStatus::Ok;
                    ui.colored_label(if ok { c::ACCENT_700 } else { c::WARN_TEXT }, if ok { "✔" } else { "✖" })
                        .on_hover_text(status_text(status));
                });
                air_cell(ui, widths[7], |ui| {
                    if ui.add_sized([28.0, 28.0], egui::Button::new("✖")).on_hover_text("Remove entry").clicked() { remove = Some(index); }
                });
            });
            if row.clicked()
                || ui.input(|i| {
                    i.pointer.any_click()
                        && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
                })
            {
                self.air_selected = Some(index);
            }
            ui.painter().hline(
                rect.x_range(),
                rect.bottom(),
                Stroke::new(1.0_f32, c::NEUTRAL_200),
            );
        }
        self.apply_air_edit(plan, edit);
        if let Some(index) = remove {
            self.remove_air_entry(index);
        }
    }

    fn air_settings(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().slider_width = 88.0;
        let mut plan = self.air_plan.clone();
        let mut edit = None;
        let mut remove = None;
        if let Some(index) = self.air_selected {
            let state = self.with_air_context(|ctx| {
                (
                    airtask::entry_status(&plan, index, ctx),
                    airtask::resolve_target(&plan.pool[index].target, ctx),
                )
            });
            ui.label(
                RichText::new(format!("Selected · pool entry {}", index + 1))
                    .small()
                    .color(c::ACCENT_700),
            );
            if let Some(sortie) = plan
                .library
                .iter()
                .find(|s| s.id == plan.pool[index].sortie_id)
            {
                ui.label(
                    RichText::new(&sortie.name).font(FontId::new(22.0, theme::heading_family())),
                );
                ui.horizontal_wrapped(|ui| {
                    shell::side_marker(ui, marker_side(sortie.side), 12.0);
                    ui.label(
                        RichText::new(format!(
                            "{} · {} · {}",
                            side_text(sortie.side),
                            entry_target_text(state.0, state.1),
                            role_text(sortie)
                        ))
                        .small(),
                    );
                });
                let stop = airtask::hard_stop(sortie, &plan);
                let entry = &mut plan.pool[index];
                ui.horizontal(|ui| {
                    ui.label("Heading");
                    let mut enabled = entry.heading_override.is_some();
                    let r = ui.checkbox(&mut enabled, "Override");
                    if r.changed() {
                        entry.heading_override = enabled.then_some(0.0);
                    }
                    note_edit(&mut edit, &r, "Changed heading override");
                    if let Some(heading) = &mut entry.heading_override {
                        let r = ui.add(
                            egui::DragValue::new(heading)
                                .range(0.0..=359.0)
                                .speed(1.0)
                                .suffix("°"),
                        );
                        name_number(&r, "Heading degrees", *heading);
                        note_edit(&mut edit, &r, "Changed heading");
                    } else {
                        ui.monospace("auto");
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Hard stop");
                    let text = match entry.mode {
                        TriggerMode::Clock => format!("{} min", minutes(stop.clock_s)),
                        TriggerMode::Zone => format!("Zone {} min", minutes(stop.zone_s)),
                        TriggerMode::Both => format!(
                            "{} min · Zone {} min",
                            minutes(stop.clock_s),
                            minutes(stop.zone_s)
                        ),
                    };
                    ui.monospace(text);
                });
            } else {
                shell::warning(ui, "Missing sortie");
            }
            if ui.button("Remove from pool").clicked() {
                remove = Some(index);
            }
        } else {
            shell::hint(ui, "Select a pool entry to edit it.", false);
        }
        ui.separator();

        let single = plan.strategies.budget == BudgetImpl::SingleSlot;
        let shown = if single { [1, 1] } else { plan.budget.per_side };
        shell::settings_section(
            ui,
            "air_budget",
            "Budget",
            &format!("DPRK {} · NATO {}", shown[0], shown[1]),
            true,
            |ui| {
                for side in [AirSide::Dprk, AirSide::Nato] {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} max active", side_text(side)));
                        let mut value = shown[side.idx()];
                        let r =
                            ui.add_enabled(!single, egui::Slider::new(&mut value, 1..=6).integer());
                        if r.changed() {
                            plan.budget.per_side[side.idx()] = value;
                        }
                        note_edit(&mut edit, &r, "Changed budget");
                    });
                }
                if single {
                    shell::hint(ui, "Counters are not confirmed in game", false);
                }
            },
        );
        shell::settings_section(
            ui,
            "air_threshold",
            "Threshold",
            &format!("1 player · {} km", plan.threshold.radius_m / 1000.0),
            true,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Players");
                    let mut players = 1_u8;
                    let r = ui.add_enabled(false, egui::DragValue::new(&mut players).range(1..=1));
                    name_number(&r, "Players", 1.0);
                });
                shell::hint(
                    ui,
                    "Two or more players comes with the multi-player phase.",
                    false,
                );
                ui.horizontal(|ui| {
                    ui.label("Radius R");
                    let mut km = plan.threshold.radius_m / 1000.0;
                    let r = ui.add(
                        egui::Slider::new(&mut km, 1.0..=50.0)
                            .step_by(0.5)
                            .suffix(" km"),
                    );
                    if r.changed() {
                        plan.threshold.radius_m = km * 1000.0;
                    }
                    note_edit(&mut edit, &r, "Changed threshold radius");
                });
                let timing = airtask::derived_timing(&plan.threshold);
                ui.label(format!(
                    "Dwell D {} s · Hold H {} s",
                    timing.dwell_s, timing.hold_s
                ));
                ui.label(format!("Inner r_in {} km", timing.inner_m / 1000.0));
                let (cruise, measured) = airtask::f80_cruise_kmh();
                shell::hint(
                    ui,
                    &format!(
                        "From F-80C cruise {cruise} km/h{}",
                        if measured { ", measured" } else { "" }
                    ),
                    false,
                );
                let mut manual =
                    plan.threshold.dwell_s.is_some() || plan.threshold.hold_s.is_some();
                let r = ui.checkbox(&mut manual, "Manual timing");
                if r.changed() {
                    plan.threshold.dwell_s = manual.then_some(timing.dwell_s);
                    plan.threshold.hold_s = manual.then_some(timing.hold_s);
                }
                note_edit(&mut edit, &r, "Changed manual timing");
                if manual {
                    if let Some(dwell) = &mut plan.threshold.dwell_s {
                        number_row(ui, "Dwell D", dwell, 10.0..=1800.0, 10.0, " s", &mut edit);
                    }
                    if let Some(hold) = &mut plan.threshold.hold_s {
                        number_row(ui, "Hold H", hold, 10.0..=3600.0, 10.0, " s", &mut edit);
                    }
                }
            },
        );
        for side in [AirSide::Dprk, AirSide::Nato] {
            let schedule = &mut plan.schedule[side.idx()];
            let title = format!("Schedule · {}", side_text(side));
            let summary = format!(
                "{} min, every {} · +{}",
                schedule.first_slot_min,
                schedule.period_min,
                schedule.jitter_steps.saturating_sub(1) as f64 * schedule.jitter_step_min
            );
            shell::settings_section(
                ui,
                &format!("air_schedule_{}", side.idx()),
                &title,
                &summary,
                false,
                |ui| {
                    let r = ui.checkbox(&mut schedule.enabled, "Enabled");
                    note_edit(&mut edit, &r, &format!("Changed {title}"));
                    number_row(
                        ui,
                        "First slot",
                        &mut schedule.first_slot_min,
                        0.0..=180.0,
                        1.0,
                        " min",
                        &mut edit,
                    );
                    number_row(
                        ui,
                        "Period",
                        &mut schedule.period_min,
                        1.0..=180.0,
                        1.0,
                        " min",
                        &mut edit,
                    );
                    ui.horizontal(|ui| {
                        ui.label("Jitter steps");
                        let r = ui.add(
                            egui::DragValue::new(&mut schedule.jitter_steps)
                                .range(1..=10)
                                .speed(0.1),
                        );
                        name_number(&r, "Jitter steps", schedule.jitter_steps as f64);
                        note_edit(&mut edit, &r, "Changed jitter steps");
                    });
                    number_row(
                        ui,
                        "× step",
                        &mut schedule.jitter_step_min,
                        0.0..=30.0,
                        0.5,
                        " min",
                        &mut edit,
                    );
                    percent_row(ui, "Quiet", &mut schedule.quiet_pct, &mut edit);
                    number_row(
                        ui,
                        "Arrive in M",
                        &mut schedule.arrive_min,
                        0.5..=30.0,
                        0.5,
                        " min",
                        &mut edit,
                    );
                },
            );
        }
        shell::settings_section(
            ui,
            "air_zone",
            "Zone triggers",
            &format!(
                "{} % · {} min · {} km",
                plan.zone.chance_pct, plan.zone.cooldown_min, plan.zone.spawn_dist_km
            ),
            false,
            |ui| {
                percent_row(ui, "Chance", &mut plan.zone.chance_pct, &mut edit);
                number_row(
                    ui,
                    "Cooldown",
                    &mut plan.zone.cooldown_min,
                    1.0..=180.0,
                    1.0,
                    " min",
                    &mut edit,
                );
                number_row(
                    ui,
                    "Spawn distance",
                    &mut plan.zone.spawn_dist_km,
                    1.0..=100.0,
                    1.0,
                    " km",
                    &mut edit,
                );
            },
        );
        shell::settings_section(
            ui,
            "air_stop",
            "Hard stop",
            &format!(
                "+{} min · extend {} min",
                plan.stop.margin_min, plan.stop.extension_min
            ),
            false,
            |ui| {
                number_row(
                    ui,
                    "Margin",
                    &mut plan.stop.margin_min,
                    0.0..=60.0,
                    1.0,
                    " min",
                    &mut edit,
                );
                number_row(
                    ui,
                    "Extension",
                    &mut plan.stop.extension_min,
                    0.0..=60.0,
                    1.0,
                    " min",
                    &mut edit,
                );
                shell::hint(
                    ui,
                    "Extend once if a player was over the target in the last few minutes.",
                    false,
                );
            },
        );
        shell::settings_section(
            ui,
            "air_output",
            "Output",
            if plan.include_in_base_map {
                "in base map"
            } else {
                "standalone"
            },
            true,
            |ui| {
                let r = ui.checkbox(&mut plan.include_in_base_map, "Include in base map");
                note_edit(&mut edit, &r, "Changed base map inclusion");
                let totals = self.with_air_context(|ctx| airtask::totals(&plan, ctx));
                egui::Grid::new("air_totals")
                    .min_col_width(34.0)
                    .show(ui, |ui| {
                        for heading in ["", "DPRK", "NATO", "All"] {
                            ui.label(RichText::new(heading).small());
                        }
                        ui.end_row();
                        for (label, values, limit) in [
                            ("Copies", totals.copies, None),
                            ("Aircraft", totals.aircraft, Some(plan.limits.aircraft)),
                            ("MCUs", totals.mcus, Some(plan.limits.mcus)),
                            (
                                "Check zones",
                                totals.check_zones,
                                Some(plan.limits.check_zones),
                            ),
                        ] {
                            ui.label(label);
                            let all = values[0] as u64 + values[1] as u64;
                            for (column, value) in [values[0] as u64, values[1] as u64, all]
                                .into_iter()
                                .enumerate()
                            {
                                let color =
                                    if column == 2 && limit.is_some_and(|cap| all > cap as u64) {
                                        c::WARN_TEXT
                                    } else {
                                        c::TEXT
                                    };
                                let r = ui.label(
                                    RichText::new(value.to_string()).monospace().color(color),
                                );
                                r.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Label,
                                        true,
                                        format!(
                                            "{label} {}: {value}",
                                            ["DPRK", "NATO", "All"][column]
                                        ),
                                    )
                                });
                            }
                            ui.end_row();
                        }
                    });
            },
        );
        self.apply_air_edit(plan, edit);
        if let Some(index) = remove {
            self.remove_air_entry(index);
        }
    }

    pub(super) fn generate_air_file(&mut self) {
        let packs = match self.with_air_context(|ctx| airtask::build_air_packs(&self.air_plan, ctx))
        {
            Ok(packs) => packs,
            Err(error) => {
                self.status = Status::Error(error);
                return;
            }
        };
        let mut root = crate::ast::Il2Entity::new("Group");
        root.set_name("Air Tasking");
        root.children = packs.into_iter().map(|pack| pack.root).collect();
        root.index = Some(root.max_index() + 1);
        root.set_property("Index", root.index.unwrap().to_string());
        let (root, _) = crate::duplicate::duplicate_template(&root, &mut 1);
        let text = serialize_group(&root);
        let Some(path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .set_file_name("Air_Tasking.Group")
            .save_file()
        else {
            return;
        };
        let paths = self
            .air_plan
            .library
            .iter()
            .filter_map(|sortie| match &sortie.source {
                SortieSource::File(path) => Some(path.clone()),
                SortieSource::BuiltIn(_) => None,
            })
            .collect::<Vec<_>>();
        self.status = save_with_sidecars(&path, &text, &paths, "Wrote Air Tasking");
    }
}

fn air_sheet(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(c::BG)
        .stroke(Stroke::new(1.0_f32, c::DIVIDER))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn air_cell(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 28.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
}

fn note_edit(edit: &mut Edit, response: &egui::Response, label: &str) {
    if response.changed() {
        *edit = Some((response.id, label.into()));
    }
}

fn name_number(response: &egui::Response, label: &str, value: f64) {
    response.widget_info(|| {
        let mut info =
            egui::WidgetInfo::labeled(egui::WidgetType::DragValue, response.enabled(), label);
        info.value = Some(value);
        info
    });
}

fn number_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
    suffix: &str,
    edit: &mut Edit,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let r = ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(speed)
                .max_decimals(1)
                .suffix(suffix),
        );
        name_number(&r, label, *value);
        note_edit(edit, &r, &format!("Changed {label}"));
    });
}

fn percent_row(ui: &mut egui::Ui, label: &str, value: &mut u8, edit: &mut Edit) {
    ui.horizontal(|ui| {
        ui.label(label);
        let r = ui.add(
            egui::DragValue::new(value)
                .range(0..=100)
                .speed(1.0)
                .suffix(" %"),
        );
        name_number(&r, label, *value as f64);
        note_edit(edit, &r, &format!("Changed {label}"));
    });
}

fn entry_word(n: usize) -> &'static str {
    if n == 1 { "entry" } else { "entries" }
}
fn side_text(side: AirSide) -> &'static str {
    match side {
        AirSide::Dprk => "DPRK",
        AirSide::Nato => "NATO",
    }
}
fn marker_side(side: AirSide) -> shell::Side {
    match side {
        AirSide::Dprk => shell::Side::Dprk,
        AirSide::Nato => shell::Side::Nato,
    }
}
fn mode_text(mode: TriggerMode) -> &'static str {
    match mode {
        TriggerMode::Clock => "Clock",
        TriggerMode::Zone => "Zone",
        TriggerMode::Both => "Both",
    }
}
fn escorted(sortie: &AirSortie) -> bool {
    sortie
        .lead_roles
        .iter()
        .flatten()
        .any(|role| *role == AirRole::Escort)
}

fn role_text(sortie: &AirSortie) -> &'static str {
    let has = |role| sortie.lead_roles.iter().flatten().any(|r| *r == role);
    if has(AirRole::AirPatrol) {
        "Air alert"
    } else if has(AirRole::GroundAttack) && escorted(sortie) {
        "Armed recon"
    }
    // GroundAttack is shared by strike and CAS; the model distinguishes the
    // Mustang close-support label in the role table, never the template name.
    else if has(AirRole::GroundAttack) && sortie.models.iter().any(|m| m == "f51d") {
        "Close support"
    } else if has(AirRole::GroundAttack) {
        "Strike"
    } else {
        "Escort"
    }
}

fn date_text((year, month, day): Ymd) -> String {
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month.saturating_sub(1) as usize)
    .copied()
    .unwrap_or("?");
    format!("{day} {month} {year}")
}

fn date_badge(sortie: &AirSortie, date: Ymd) -> (bool, String) {
    if date < sortie.valid.from {
        return (
            false,
            format!("✖ Valid from {}", date_text(sortie.valid.from)),
        );
    }
    if date > sortie.valid.to {
        return (
            false,
            format!("✖ Valid until {}", date_text(sortie.valid.to)),
        );
    }
    let unsourced = sortie
        .models
        .iter()
        .all(|m| crate::in_theatre::for_model(m).is_none_or(|row| !row.sourced));
    (
        true,
        if unsourced {
            "✔ Valid · war span (unsourced)".into()
        } else {
            format!(
                "✔ Valid · {}–{}",
                date_text(sortie.valid.from),
                date_text(sortie.valid.to)
            )
        },
    )
}

fn target_label(choice: TargetChoice) -> String {
    match choice.kind {
        TargetKind::Objective => format!("{} objective {}", side_text(choice.side), choice.number),
        TargetKind::Arrow => format!("Arrow {}", choice.number),
    }
}

fn entry_target_text(status: EntryStatus, target: Option<TargetChoice>) -> String {
    if status == EntryStatus::SideChanged {
        return "Side changed".into();
    }
    target.map_or_else(
        || "Orphaned".into(),
        |choice| {
            format!(
                "{} {}",
                if choice.kind == TargetKind::Objective {
                    "Objective"
                } else {
                    "Arrow"
                },
                choice.number
            )
        },
    )
}

fn status_text(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Ok => "Valid sortie and target",
        EntryStatus::Orphaned => "Orphaned: the Map target no longer resolves within 50 m",
        EntryStatus::SideChanged => "Side changed: the target is now on the other side",
        EntryStatus::OutOfDate => "The Map date is outside the sortie's valid range",
        EntryStatus::MissingSortie => "Missing sortie: the template is no longer in the library",
    }
}

fn minutes(seconds: f64) -> String {
    let value = seconds / 60.0;
    if value.fract().abs() < 0.0001 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// Shared with step 9's Map dock: two lanes and a fixed 0–3 hour axis, even empty.
pub(super) fn draw_slot_strip(ui: &mut egui::Ui, marks: &[Vec<SlotMark>; 2]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 90.0), Sense::hover());
    let start = rect.left() + 62.0;
    let end = rect.right() - 14.0;
    let x = |seconds: f64| start + (seconds / 10800.0).clamp(0.0, 1.0) as f32 * (end - start);
    let p = ui.painter();
    for (lane, side) in [AirSide::Dprk, AirSide::Nato].into_iter().enumerate() {
        let y = rect.top() + 8.0 + lane as f32 * 30.0;
        shell::paint_side_marker(
            p,
            Pos2::new(rect.left() + 6.0, y + 10.0),
            12.0,
            marker_side(side),
        );
        p.text(
            Pos2::new(rect.left() + 17.0, y + 10.0),
            Align2::LEFT_CENTER,
            side_text(side),
            FontId::proportional(12.0),
            c::TEXT,
        );
        let lane_rect = Rect::from_min_max(Pos2::new(start, y), Pos2::new(end, y + 22.0));
        p.rect_stroke(
            lane_rect,
            0.0,
            Stroke::new(1.0_f32, c::DIVIDER),
            egui::StrokeKind::Inside,
        );
        for half_hour in 1..6 {
            p.vline(
                x(half_hour as f64 * 1800.0),
                lane_rect.y_range(),
                Stroke::new(1.0_f32, c::NEUTRAL_200),
            );
        }
        let color = if side == AirSide::Dprk {
            c::DPRK
        } else {
            c::NATO
        };
        for mark in &marks[lane] {
            let span = Rect::from_min_max(
                Pos2::new(x(mark.t_s), y),
                Pos2::new(x(mark.t_s + mark.jitter_span_s), y + 22.0),
            );
            p.rect_filled(span, 0.0, color.gamma_multiply(0.15));
            p.vline(
                x(mark.t_s),
                lane_rect.y_range(),
                Stroke::new(
                    2.0_f32,
                    color.gamma_multiply(1.0 - mark.quiet_pct.min(100) as f32 / 100.0),
                ),
            );
        }
    }
    for half_hour in 0..=6 {
        let at = Pos2::new(x(half_hour as f64 * 1800.0), rect.top() + 70.0);
        p.text(
            at,
            Align2::CENTER_TOP,
            format!("{}:{:02}", half_hour / 2, half_hour % 2 * 30),
            FontId::monospace(12.0),
            c::NEUTRAL_700,
        );
    }
}
