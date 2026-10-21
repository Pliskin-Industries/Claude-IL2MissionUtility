//! Air Tasking rail tab. Plan decisions belong to the public `airtask` API.

use super::*;
use crate::airtask::{
    self, AirContext, AirPlan, AirSide, AirSortie, BudgetImpl, EntryStatus, IssueSeverity,
    PoolEntry, SlotMark, SortieSource, TargetChoice, TargetKind, TriggerMode, Ymd,
};

type Edit = Option<(egui::Id, String)>;

#[derive(Default)]
pub(super) struct CategoryUi {
    show_categories: bool,
    filter: Option<String>,
    new_name: String,
    renaming: Option<(String, String)>,
    error: Option<String>,
}

impl GroupGeneratorApp {
    pub(super) fn new_air_plan() -> AirPlan {
        let mut plan = AirPlan {
            categories: airtask::default_categories(),
            seed: Self::placement_seed(),
            ..AirPlan::default()
        };
        for sortie in airtask::builtin_library() {
            airtask::add_library_item(&mut plan, sortie);
        }
        plan
    }

    /// Use exactly the full/composite front used to colour Map's arrows, without AO clipping.
    fn air_front(&self) -> Vec<(f64, f64)> {
        let raw = if self.custom_front_xz.is_empty() {
            preview_front_xz(self.front_t)
        } else {
            self.custom_front_xz.clone()
        };
        let full = crate::frontlines::densify(&raw, 4000.0);
        let (composite, _) = apply_salients(full.clone(), &self.salients);
        if composite.len() >= 2 {
            composite
        } else {
            full
        }
    }

    pub(super) fn with_air_context<R>(&self, read: impl FnOnce(&AirContext<'_>) -> R) -> R {
        let front = self.air_front();
        let mark = self.current_mark();
        read(&AirContext {
            east_objectives: &self.east_objectives,
            nato_objectives: &self.nato_objectives,
            arrows: &self.attack_arrows,
            front: &front,
            aabb: self.front_aabb,
            date: (mark.year, mark.month, mark.day),
        })
    }

    pub(super) fn air_header_label(&self) -> String {
        let mark = self.current_mark();
        let date = date_text((mark.year, mark.month, mark.day));
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
            self.air_categories.renaming = None;
            self.air_categories.error = None;
            self.air_selection_bounds();
        }
    }

    fn air_selection_bounds(&mut self) {
        self.air_selected = self.air_selected.filter(|&i| i < self.air_plan.pool.len());
        self.air_library_selected = self
            .air_library_selected
            .filter(|&i| i < self.air_plan.library.len());
        if self
            .air_categories
            .filter
            .as_ref()
            .is_some_and(|name| !self.air_plan.categories.contains(name))
        {
            self.air_categories.filter = None;
        }
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
            categories: airtask::default_categories(),
            seed: Self::placement_seed(),
            ..AirPlan::default()
        };
        for sortie in library {
            airtask::add_library_item(&mut self.air_plan, sortie);
        }
        self.air_categories = CategoryUi::default();
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
                airtask::add_library_item(&mut self.air_plan, sortie);
                self.air_editing = None;
            }
            Err(error) => self.status = Status::Error(error),
        }
    }

    fn add_air_entry(&mut self, ctx: &AirContext) {
        let Some(sortie) = self
            .air_library_selected
            .and_then(|i| self.air_plan.library.get(i))
        else {
            return;
        };
        let choices = airtask::target_choices(ctx, sortie.side);
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

    fn auto_fill_air_pool(&mut self, ctx: &AirContext) {
        let mut plan = self.air_plan.clone();
        let count: usize = AirSide::ALL
            .into_iter()
            .map(|side| {
                airtask::auto_fill(&mut plan, ctx, side, self.air_categories.filter.as_deref())
            })
            .sum();
        let mut label = format!("Added {count} {}", entry_word(count));
        if let Some(category) = &self.air_categories.filter {
            label.push_str(&format!(" · {category}"));
        }
        if count > 0 {
            self.air_undo.record(&label, self.air_plan.clone());
            self.air_plan = plan;
            self.air_editing = None;
        }
        self.status = Status::Info(label);
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
    pub(super) fn apply_air_edit(&mut self, plan: AirPlan, edit: Edit) {
        if plan == self.air_plan {
            return;
        }
        let (id, label) = edit.unwrap_or_else(|| {
            (
                self.air_editing
                    .unwrap_or_else(|| egui::Id::new("air_draft_edit")),
                "Changed Air Tasking".into(),
            )
        });
        if self.air_editing == Some(id) {
            self.air_undo.rebase();
        } else {
            self.air_undo.record(label, self.air_plan.clone());
        }
        self.air_plan = plan;
        self.air_editing = Some(id);
    }

    pub(super) fn air_page(&mut self, ctx: &egui::Context) {
        // Own the Map inputs for this frame so every panel can edit the plan
        // while sharing one context and one densified/composite front.
        let front = self.air_front();
        let east_objectives = self.east_objectives.clone();
        let nato_objectives = self.nato_objectives.clone();
        let arrows = self.attack_arrows.clone();
        let mark = self.current_mark();
        let air_ctx = AirContext {
            east_objectives: &east_objectives,
            nato_objectives: &nato_objectives,
            arrows: &arrows,
            front: &front,
            aabb: self.front_aabb,
            date: (mark.year, mark.month, mark.day),
        };
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.wants_keyboard_input() {
            self.air_editing = None;
        }
        self.air_selection_bounds();
        egui::SidePanel::left("air_left")
            .exact_width(270.0)
            .resizable(false)
            .show(ctx, |ui| self.air_library(ui, &air_ctx));
        side_panel(ctx, "air_right", false, 304.0, |ui| {
            self.air_settings(ui, &air_ctx)
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_blueprint_grid(ui.painter(), ui.max_rect());
            air_tabs(ui, &mut self.air_categories.show_categories);
            egui::ScrollArea::vertical().id_salt(("air_center", self.air_categories.show_categories)).auto_shrink([false, false]).show(ui, |ui| {
                if self.air_categories.show_categories {
                    air_sheet(ui, |ui| self.air_category_list(ui));
                    return;
                }
                air_sheet(ui, |ui| self.air_pool(ui, &air_ctx));
                ui.add_space(10.0);
                air_sheet(ui, |ui| {
                    shell::section_title(ui, "Slot timeline", Some("0–3 h · one row per side"));
                    let marks = airtask::preview(&self.air_plan, &air_ctx).slot_times;
                    draw_slot_strip(ui, &marks);
                    shell::hint(ui, "Ticks: slot times · shading: jitter span · fainter tick: higher quiet chance.", false);
                });
                ui.add_space(10.0);
                air_sheet(ui, |ui| {
                    shell::section_title(ui, "Plan check", Some("errors block Generate · warnings do not"));
                    let issues = airtask::validate(&self.air_plan, &air_ctx);
                    if issues.is_empty() { shell::hint(ui, "No issues reported.", false); }
                    for issue in issues {
                        let glyph = if issue.severity == IssueSeverity::Error { "✖" } else { "⚠" };
                        let text = RichText::new(format!("{glyph} {}", issue.text)).color(c::WARN_TEXT);
                        ui.label(if issue.severity == IssueSeverity::Error { text.strong() } else { text });
                    }
                });
            });
        });
    }

    fn air_library(&mut self, ui: &mut egui::Ui, ctx: &AirContext) {
        ui.add_space(6.0);
        shell::section_title(
            ui,
            &format!("Library · {}", self.air_plan.library.len()),
            Some(&format!("valid for {}", date_text(ctx.date))),
        );
        // Reserve the footer's own height before giving the cards the remaining space.
        egui::TopBottomPanel::bottom("air_library_footer")
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show_inside(ui, |ui| self.air_library_footer(ui, ctx));
        egui::ScrollArea::vertical()
            .id_salt("air_library_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| self.air_library_cards(ui, ctx.date));
    }

    fn air_library_cards(&mut self, ui: &mut egui::Ui, date: Ymd) {
        for (index, sortie) in self.air_plan.library.iter().enumerate() {
            let card = shell::card(ui, self.air_library_selected == Some(index), |ui| {
                ui.horizontal(|ui| {
                    shell::side_marker(ui, side_style(sortie.side).marker, 12.0);
                    ui.add(egui::Label::new(RichText::new(&sortie.name).strong()).wrap());
                });
                ui.horizontal_wrapped(|ui| {
                    let tag = shell::tag(ui, &sortie.category, true);
                    tag.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Label,
                            true,
                            format!("Sortie {} category: {}", index + 1, sortie.category),
                        )
                    });
                    if sortie.is_escorted() {
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
                ui.horizontal_wrapped(|ui| {
                    shell::tag(ui, &format!("{} × {models}", sortie.planes), false);
                });
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
    }

    fn air_library_footer(&mut self, ui: &mut egui::Ui, ctx: &AirContext) {
        ui.separator();
        if let Some(index) = self.air_library_selected {
            let sortie = &self.air_plan.library[index];
            ui.label(format!("Selected: {}", sortie.name));
            let choices = airtask::target_choices(ctx, sortie.side);
            let removable = matches!(sortie.source, SortieSource::File(_));
            let mut category = sortie.category.clone();
            ui.horizontal(|ui| {
                ui.label("Category");
                let response = egui::ComboBox::from_id_salt("air_sortie_category")
                    .width(ui.available_width())
                    .selected_text(&category)
                    .truncate()
                    .show_ui(ui, |ui| {
                        for name in &self.air_plan.categories {
                            if ui
                                .selectable_label(
                                    category.to_lowercase() == name.to_lowercase(),
                                    name,
                                )
                                .clicked()
                            {
                                category = name.clone();
                            }
                        }
                    })
                    .response;
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Sortie category")
                });
            });
            if category != self.air_plan.library[index].category {
                self.air_undo
                    .record("Changed sortie category", self.air_plan.clone());
                airtask::set_category(&mut self.air_plan, index, &category);
                self.air_editing = None;
            }
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
                        .width(ui.available_width())
                        .selected_text(&label)
                        .truncate()
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
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.air_picked_target.is_some(),
                        egui::Button::new("Add to pool"),
                    )
                    .clicked()
                {
                    self.add_air_entry(ctx);
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

    fn air_category_list(&mut self, ui: &mut egui::Ui) {
        shell::section_title(ui, "Categories", Some("library sorties"));
        let count_width = 80.0;
        let button_width = 72.0;
        let name_width = (ui.available_width()
            - count_width
            - 2.0 * button_width
            - 3.0 * ui.spacing().item_spacing.x)
            .max(0.0);
        for (name, count) in airtask::category_counts(&self.air_plan) {
            ui.push_id(&name, |ui| {
                ui.horizontal(|ui| {
                    let editing = self
                        .air_categories
                        .renaming
                        .as_ref()
                        .is_some_and(|(old, _)| *old == name);
                    air_cell(ui, name_width, |ui| {
                        if editing {
                            let (_, draft) = self.air_categories.renaming.as_mut().unwrap();
                            category_input(ui, draft, name_width, "Rename category");
                        } else {
                            ui.add(egui::Label::new(&name).truncate())
                                .on_hover_text(&name);
                        }
                    });
                    air_cell(ui, count_width, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let word = if count == 1 { "sortie" } else { "sorties" };
                            ui.add(egui::Label::new(format!("{count} {word}")).truncate());
                        });
                    });
                    if editing {
                        if ui
                            .add_sized([button_width, 28.0], egui::Button::new("Save"))
                            .clicked()
                        {
                            let new = self.air_categories.renaming.as_ref().unwrap().1.clone();
                            let mut plan = self.air_plan.clone();
                            match airtask::rename_category(&mut plan, &name, &new) {
                                Ok(()) => {
                                    if self.air_categories.filter.as_ref() == Some(&name) {
                                        self.air_categories.filter = Some(new.trim().to_owned());
                                    }
                                    self.commit_air_category(plan, "Renamed category");
                                    self.air_categories.renaming = None;
                                }
                                Err(error) => self.air_categories.error = Some(error),
                            }
                        }
                        if ui
                            .add_sized([button_width, 28.0], egui::Button::new("Cancel"))
                            .clicked()
                        {
                            self.air_categories.renaming = None;
                            self.air_categories.error = None;
                        }
                    } else {
                        let rename =
                            ui.add_sized([button_width, 28.0], egui::Button::new("Rename"));
                        rename.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Rename {name}"),
                            )
                        });
                        if rename.clicked() {
                            self.air_categories.renaming = Some((name.clone(), name.clone()));
                            self.air_categories.error = None;
                        }
                        let remove =
                            ui.add_sized([button_width, 28.0], egui::Button::new("Remove"));
                        remove.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Remove category {name}"),
                            )
                        });
                        if remove.clicked() {
                            let mut plan = self.air_plan.clone();
                            let moved = airtask::remove_category(&mut plan, &name);
                            self.commit_air_category(plan, &format!("Removed category {name}"));
                            self.status = Status::Info(format!(
                                "Removed {name} · {moved} sorties moved to defaults"
                            ));
                        }
                    }
                });
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 116.0).max(60.0);
            category_input(ui, &mut self.air_categories.new_name, width, "New category");
            if ui.button("Add category").clicked() {
                let mut plan = self.air_plan.clone();
                match airtask::add_category(&mut plan, &self.air_categories.new_name) {
                    Ok(()) => {
                        self.commit_air_category(plan, "Added category");
                        self.air_categories.new_name.clear();
                    }
                    Err(error) => self.air_categories.error = Some(error),
                }
            }
        });
        if let Some(error) = &self.air_categories.error {
            shell::warning(ui, error);
        }
        if shell::hint(
            ui,
            "Categories last for this run. A template keeps its category with a line Category: <name> in its group description.",
            true,
        ) {
            self.open_help(HelpTopic::AirTasking);
        }
    }

    fn commit_air_category(&mut self, plan: AirPlan, label: &str) {
        self.air_undo.record(label, self.air_plan.clone());
        self.air_plan = plan;
        self.air_editing = None;
        self.air_categories.error = None;
    }

    fn air_pool(&mut self, ui: &mut egui::Ui, ctx: &AirContext) {
        ui.horizontal(|ui| {
            let label = self
                .air_categories
                .filter
                .as_deref()
                .unwrap_or("All categories");
            let response = egui::ComboBox::from_id_salt("air_pool_filter")
                .width(175.0)
                .selected_text(label)
                .truncate()
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.air_categories.filter, None, "All categories");
                    for name in &self.air_plan.categories {
                        ui.selectable_value(
                            &mut self.air_categories.filter,
                            Some(name.clone()),
                            name,
                        );
                    }
                })
                .response;
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Pool category filter")
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Auto-fill from objectives").clicked() {
                    self.auto_fill_air_pool(ctx);
                }
            });
        });
        let visible: Vec<_> = self
            .air_plan
            .pool
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                self.air_categories
                    .filter
                    .as_ref()
                    .is_none_or(|category| {
                        self.air_plan
                            .sortie(&entry.sortie_id)
                            .is_some_and(|s| s.category.to_lowercase() == category.to_lowercase())
                    })
                    .then_some(i)
            })
            .collect();
        let counts = airtask::pool_counts(&self.air_plan);
        let count = if self.air_categories.filter.is_some() {
            format!("{} of {}", visible.len(), self.air_plan.pool.len())
        } else {
            self.air_plan.pool.len().to_string()
        };
        ui.horizontal(|ui| {
            shell::section_title(ui, &format!("Pool · {count}"), Some(&side_counts(counts)));
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
                self.add_air_entry(ctx);
            }
            return;
        }
        if visible.is_empty() {
            shell::hint(ui, "No pool entries in this category.", false);
            return;
        }
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
        let popup_open = egui::Popup::is_any_open(ui.ctx());
        for index in visible {
            let entry = &mut plan.pool[index];
            let sortie = self.air_plan.sortie(&entry.sortie_id);
            let name = sortie.map_or("Missing sortie", |s| s.name.as_str());
            let status = airtask::entry_status(&self.air_plan, index, ctx);
            let target = airtask::resolve_target(&entry.target, ctx);
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
                // Let the row receive clicks on its text; controls keep their own responses.
                ui.style_mut().interaction.selectable_labels = false;
                if !entry.enabled { ui.visuals_mut().override_text_color = Some(c::NEUTRAL_600); ui.set_opacity(0.55); }
                air_cell(ui, widths[0], |ui| {
                    let r = ui.add_sized([28.0, 28.0], egui::Checkbox::without_text(&mut entry.enabled));
                    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, entry.enabled, format!("Entry {} enabled", index + 1)));
                    note_edit(&mut edit, &r, "Changed entry on/off");
                });
                air_cell(ui, widths[1], |ui| {
                    if let Some(s) = sortie { shell::side_marker(ui, side_style(s.side).marker, 12.0); }
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
            if row.clicked() && !popup_open && !egui::Popup::is_any_open(ui.ctx()) {
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

    fn air_settings(&mut self, ui: &mut egui::Ui, ctx: &AirContext) {
        ui.spacing_mut().slider_width = 88.0;
        let mut plan = self.air_plan.clone();
        let mut edit = None;
        let mut remove = None;
        if let Some(index) = self.air_selected {
            let state = (
                airtask::entry_status(&plan, index, ctx),
                airtask::resolve_target(&plan.pool[index].target, ctx),
            );
            ui.label(
                RichText::new(format!("Selected · pool entry {}", index + 1))
                    .small()
                    .color(c::ACCENT_700),
            );
            if let Some(sortie) = plan.sortie(&plan.pool[index].sortie_id) {
                ui.label(
                    RichText::new(&sortie.name).font(FontId::new(22.0, theme::heading_family())),
                );
                ui.horizontal_wrapped(|ui| {
                    shell::side_marker(ui, side_style(sortie.side).marker, 12.0);
                    ui.label(
                        RichText::new(format!(
                            "{} · {} · {}",
                            side_style(sortie.side).label,
                            entry_target_text(state.0, state.1),
                            sortie.category
                        ))
                        .small(),
                    );
                });
                let stop = airtask::hard_stop(sortie, &plan, plan.pool[index].mode);
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
                    let text = stop
                        .map_or_else(|| "—".into(), |seconds| format!("{} min", minutes(seconds)));
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
            &side_counts(shown.map(usize::from)),
            true,
            |ui| {
                for side in AirSide::ALL {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} max active", side_style(side).label));
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
                let cruise = airtask::f80_cruise_kmh();
                shell::hint(
                    ui,
                    &format!(
                        "From F-80C cruise {} km/h{}",
                        cruise.kmh,
                        if cruise.measured { ", measured" } else { "" }
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
                        number_row(
                            ui,
                            "Dwell D",
                            dwell,
                            NumberSpec {
                                range: 10.0..=1800.0,
                                step: 10.0,
                                suffix: " s",
                            },
                            &mut edit,
                        );
                    }
                    if let Some(hold) = &mut plan.threshold.hold_s {
                        number_row(
                            ui,
                            "Hold H",
                            hold,
                            NumberSpec {
                                range: 10.0..=3600.0,
                                step: 10.0,
                                suffix: " s",
                            },
                            &mut edit,
                        );
                    }
                }
            },
        );
        for side in AirSide::ALL {
            let schedule = &mut plan.schedule[side.idx()];
            let title = format!("Schedule · {}", side_style(side).label);
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
                        NumberSpec {
                            range: 0.0..=180.0,
                            step: 1.0,
                            suffix: " min",
                        },
                        &mut edit,
                    );
                    number_row(
                        ui,
                        "Period",
                        &mut schedule.period_min,
                        NumberSpec {
                            range: 1.0..=180.0,
                            step: 1.0,
                            suffix: " min",
                        },
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
                        NumberSpec {
                            range: 0.0..=30.0,
                            step: 0.5,
                            suffix: " min",
                        },
                        &mut edit,
                    );
                    percent_row(ui, "Quiet", &mut schedule.quiet_pct, &mut edit);
                    number_row(
                        ui,
                        "Arrive in M",
                        &mut schedule.arrive_min,
                        NumberSpec {
                            range: 0.5..=30.0,
                            step: 0.5,
                            suffix: " min",
                        },
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
                    NumberSpec {
                        range: 1.0..=180.0,
                        step: 1.0,
                        suffix: " min",
                    },
                    &mut edit,
                );
                number_row(
                    ui,
                    "Spawn distance",
                    &mut plan.zone.spawn_dist_km,
                    NumberSpec {
                        range: 1.0..=100.0,
                        step: 1.0,
                        suffix: " km",
                    },
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
                    NumberSpec {
                        range: 0.0..=60.0,
                        step: 1.0,
                        suffix: " min",
                    },
                    &mut edit,
                );
                number_row(
                    ui,
                    "Extension",
                    &mut plan.stop.extension_min,
                    NumberSpec {
                        range: 0.0..=60.0,
                        step: 1.0,
                        suffix: " min",
                    },
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
                let totals = airtask::totals(&plan, ctx);
                egui::Grid::new("air_totals")
                    .min_col_width(34.0)
                    .show(ui, |ui| {
                        let sides = AirSide::ALL.map(|side| side_style(side).label);
                        for heading in ["", sides[0], sides[1], "All"] {
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
                                            [sides[0], sides[1], "All"][column]
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
        let root = match self.with_air_context(|ctx| airtask::build_air_group(&self.air_plan, ctx))
        {
            Ok(root) => root,
            Err(error) => {
                self.status = Status::Error(error);
                return;
            }
        };
        let text = serialize_group(&root);
        let Some(path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .set_file_name("Air_Tasking.Group")
            .save_file()
        else {
            return;
        };
        let paths = airtask::sidecar_paths(&self.air_plan);
        self.status = save_with_sidecars(&path, &text, &paths, "Wrote Air Tasking");
    }
}

/// The dock-tab look at the Air Tasking tab's 28 px control height.
fn air_tabs(ui: &mut egui::Ui, categories: &mut bool) {
    let (strip, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
    for (i, label) in ["Pool", "Categories"].into_iter().enumerate() {
        let rect = Rect::from_min_size(
            strip.min + Vec2::new(i as f32 * strip.width() / 2.0, 0.0),
            Vec2::new(strip.width() / 2.0, 28.0),
        );
        let selected = *categories == (i == 1);
        let response = ui.interact(rect, ui.id().with(("air_tab", i)), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
        });
        if response.clicked() {
            *categories = i == 1;
        }
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, c::ACCENT_100);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::new(
                13.0,
                if selected {
                    theme::bold_family()
                } else {
                    egui::FontFamily::Proportional
                },
            ),
            if selected { c::ACCENT_700 } else { c::TEXT },
        );
        if selected {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom() - 1.0,
                Stroke::new(2.0_f32, c::ACCENT),
            );
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                0.0,
                Stroke::new(2.0_f32, c::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
    }
    ui.add_space(6.0);
}

fn category_input(ui: &mut egui::Ui, value: &mut String, width: f32, label: &str) {
    let r = ui.add_sized(
        [width, 28.0],
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .hint_text(label),
    );
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
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

struct NumberSpec {
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    suffix: &'static str,
}

fn number_row(ui: &mut egui::Ui, label: &str, value: &mut f64, spec: NumberSpec, edit: &mut Edit) {
    ui.horizontal(|ui| {
        ui.label(label);
        let r = ui.add(
            egui::DragValue::new(value)
                .range(spec.range)
                .speed(spec.step)
                .max_decimals(1)
                .suffix(spec.suffix),
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
struct SideStyle {
    label: &'static str,
    marker: shell::Side,
    color: Color32,
}

fn side_style(side: AirSide) -> SideStyle {
    match side {
        AirSide::Dprk => SideStyle {
            label: "DPRK",
            marker: shell::Side::Dprk,
            color: c::DPRK,
        },
        AirSide::Nato => SideStyle {
            label: "NATO",
            marker: shell::Side::Nato,
            color: c::NATO,
        },
    }
}

fn side_counts(counts: [usize; 2]) -> String {
    AirSide::ALL
        .map(|side| format!("{} {}", side_style(side).label, counts[side.idx()]))
        .join(" · ")
}
fn mode_text(mode: TriggerMode) -> &'static str {
    match mode {
        TriggerMode::Clock => "Clock",
        TriggerMode::Zone => "Zone",
        TriggerMode::Both => "Both",
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
    if !sortie.valid_on(date) {
        let (word, boundary) = if date < sortie.valid.from {
            ("from", sortie.valid.from)
        } else {
            ("until", sortie.valid.to)
        };
        return (false, format!("✖ Valid {word} {}", date_text(boundary)));
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
        TargetKind::Objective => format!(
            "{} objective {}",
            side_style(choice.side).label,
            choice.number
        ),
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
    for (lane, side) in AirSide::ALL.into_iter().enumerate() {
        let y = rect.top() + 8.0 + lane as f32 * 30.0;
        shell::paint_side_marker(
            p,
            Pos2::new(rect.left() + 6.0, y + 10.0),
            12.0,
            side_style(side).marker,
        );
        p.text(
            Pos2::new(rect.left() + 17.0, y + 10.0),
            Align2::LEFT_CENTER,
            side_style(side).label,
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
        let color = side_style(side).color;
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
