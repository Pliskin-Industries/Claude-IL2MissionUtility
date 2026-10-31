//! Replay… window and the Map's Replay layer (P14 step 9T).
//!
//! Presentation only. Parsing the logs, the replay itself, the report's rows
//! and group paths are `crate::missionlog`; the sidecar is `crate::trace`.

use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Color32, Pos2, Rect, RichText, Stroke, Vec2, ViewportBuilder, ViewportId,
};

use super::{dialog, load_group, seat_side, GroupGeneratorApp, Status};
use crate::help::HelpTopic;
use crate::missionlog::{self, group_label, MapMarker, MarkerKind, Replay, ReportRows};
use crate::shell::{self, Check};
use crate::theme::c;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum ReplaySection {
    #[default]
    DeadLinks,
    Unobserved,
    NeverFired,
    Fired,
    NeverSpawned,
    Timeline,
}

impl ReplaySection {
    /// The sections that need a trace sidecar.
    fn needs_trace(self) -> bool {
        matches!(
            self,
            Self::DeadLinks | Self::Unobserved | Self::NeverFired | Self::Fired
        )
    }
}

/// What the Replay… window shows, and the Map layer it feeds.
#[derive(Default)]
pub(super) struct ReplayPanel {
    pub(super) open: bool,
    /// Drawn inside the main window this frame (tests, or no OS viewports).
    /// Esc then reaches the main window's shortcuts, which close the panel.
    pub(super) embedded: bool,
    pub(super) group: Option<PathBuf>,
    /// A `.trace.json` picked by hand; otherwise the one next to the `.Group`.
    pub(super) trace_pick: Option<PathBuf>,
    pub(super) log_dir: Option<PathBuf>,
    pub(super) log_files: Vec<PathBuf>,
    pub(super) loaded: Option<LoadedReplay>,
    /// The last pick or run that failed, shown in the window.
    pub(super) error: Option<String>,
    pub(super) section: ReplaySection,
    /// The Timeline shows this group and its subgroups; None shows all.
    pub(super) group_filter: Option<Vec<String>>,
    /// The Map's Replay layer. Turned on when a replay loads.
    pub(super) layer: bool,
    /// Markers the Map painted last frame: `[spawns, kills]`.
    pub(super) drawn: [usize; 2],
}

pub(super) struct LoadedReplay {
    pub(super) replay: Replay,
    pub(super) rows: ReportRows,
    /// Every group path in the Timeline, and each of its parents, sorted.
    pub(super) groups: Vec<Vec<String>>,
}

const NO_SIDECAR: &str =
    "No .trace.json next to this .Group: the report has no breadcrumb sections.";

impl ReplayPanel {
    /// The trace file Run replay reads: the picked one, else the `.Group`'s sidecar.
    fn trace_path(&self) -> Option<PathBuf> {
        self.trace_pick
            .clone()
            .or_else(|| self.group.as_deref().map(crate::trace::sidecar_path))
    }

    /// What Run replay needs: a .Group and at least one log file.
    fn checks(&self) -> Vec<Check> {
        vec![
            Check::new(self.group.is_some(), "A .Group file"),
            Check::new(
                !self.log_files.is_empty(),
                "At least one log file (missionReport*.txt)",
            ),
        ]
    }

    /// A new pick makes the shown report stale: drop it and its Map layer.
    fn clear_report(&mut self) {
        self.loaded = None;
        self.group_filter = None;
        self.layer = false;
        self.error = None;
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn timeline_groups(replay: &Replay) -> Vec<Vec<String>> {
    let mut groups = std::collections::BTreeSet::new();
    for path in replay.timeline.iter().filter_map(|e| e.group_path.as_ref()) {
        for n in 1..=path.len() {
            groups.insert(path[..n].to_vec());
        }
    }
    groups.into_iter().collect()
}

impl GroupGeneratorApp {
    fn replay_pick_group(&mut self) {
        let Some(path) = dialog::FileDialog::new()
            .add_filter("IL-2 Group", &["Group"])
            .pick_file()
        else {
            return;
        };
        let panel = &mut self.replay;
        panel.clear_report();
        panel.group = Some(path);
        panel.trace_pick = None;
    }

    fn replay_pick_trace(&mut self) {
        let Some(path) = dialog::FileDialog::new()
            .add_filter("Trace", &["json"])
            .pick_file()
        else {
            return;
        };
        let panel = &mut self.replay;
        panel.clear_report();
        panel.trace_pick = Some(path);
    }

    fn replay_pick_logs(&mut self) {
        let Some(dir) = dialog::FileDialog::new().pick_folder() else {
            return;
        };
        let panel = &mut self.replay;
        panel.clear_report();
        match missionlog::find_log_files(&dir) {
            Ok(files) => panel.log_files = files,
            Err(err) => {
                panel.log_files.clear();
                panel.error = Some(err);
            }
        }
        panel.log_dir = Some(dir);
    }

    fn replay_run(&mut self) {
        let panel = &mut self.replay;
        let Some(group_path) = panel.group.clone() else {
            return;
        };
        let trace_path = panel.trace_path();
        let result = (|| -> Result<LoadedReplay, String> {
            let group = load_group(&group_path)?;
            let trace = match trace_path.filter(|p| p.is_file() || panel.trace_pick.is_some()) {
                Some(path) => {
                    let trace = crate::trace::read_trace_sidecar(&path)?;
                    crate::trace::check_sidecar(&group, &trace)
                        .map_err(|e| format!("{}: {e}", file_name(&path)))?;
                    Some(trace)
                }
                None => None,
            };
            let log = missionlog::parse_log_files(&panel.log_files)?;
            let replay = missionlog::replay(&log, &group, trace.as_ref());
            Ok(LoadedReplay {
                rows: missionlog::report_rows(&replay),
                groups: timeline_groups(&replay),
                replay,
            })
        })();
        let loaded = match result {
            Ok(loaded) => loaded,
            Err(err) => {
                panel.clear_report();
                panel.error = Some(err);
                return;
            }
        };
        let (r, replay) = (&loaded.rows, &loaded.replay);
        let found = if replay.trace_supplied {
            format!(
                "{} dead links, {} unobserved successors",
                r.dead_links.len(),
                r.unobserved.len()
            )
        } else {
            "no trace, so no breadcrumb sections".into()
        };
        self.status = Status::Info(format!(
            "Replay of {}: {} log files, {found}, {} names with fewer spawns than objects.",
            file_name(&group_path),
            replay.files.len(),
            r.never_spawned.len()
        ));
        if replay.trace_supplied {
            panel.section = ReplaySection::DeadLinks;
        } else if panel.section.needs_trace() {
            panel.section = ReplaySection::NeverSpawned;
        }
        panel.group_filter = None;
        panel.loaded = Some(loaded);
        panel.layer = true;
        panel.error = None;
    }

    /// The Replay… window: its own OS window like Help, so the Map's Replay
    /// layer stays in view. Embedded (tests, or a backend without viewports),
    /// it is a window inside the app.
    pub(super) fn replay_window(&mut self, ctx: &egui::Context) {
        if !self.replay.open {
            return;
        }
        let mut action = PanelAction::None;
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("replay"),
            ViewportBuilder::default()
                .with_title("IL-2 Group Generator — Replay")
                // Fits a 1920 x 1080 screen at 150 %, title bar and taskbar
                // included, when it opens near the top left.
                .with_position([60.0, 40.0])
                .with_inner_size([900.0, 600.0])
                .with_min_inner_size([640.0, 420.0])
                .with_decorations(true),
            |ctx, class| {
                self.replay.embedded = class == egui::ViewportClass::Embedded;
                if self.replay.embedded {
                    // Below the header, so it never covers the Generate row.
                    egui::Window::new("Replay")
                        .collapsible(false)
                        .resizable(true)
                        .default_pos([shell::RAIL_W + 40.0, shell::HEADER_H + 40.0])
                        .default_size([900.0, 640.0])
                        .show(ctx, |ui| action = self.replay_body(ui));
                } else {
                    egui::CentralPanel::default().show(ctx, |ui| action = self.replay_body(ui));
                    // Its own input: Esc here never reaches the main window.
                    let esc = !egui::Popup::is_any_open(ctx)
                        && ctx.input(|i| i.key_pressed(egui::Key::Escape));
                    if esc || ctx.input(|i| i.viewport().close_requested()) {
                        action = PanelAction::Close;
                    }
                }
            },
        );
        match action {
            PanelAction::None => {}
            PanelAction::Close => self.replay.open = false,
            PanelAction::Help => self.open_help(HelpTopic::TraceReplay),
            PanelAction::PickGroup => self.replay_pick_group(),
            PanelAction::PickTrace => self.replay_pick_trace(),
            PanelAction::PickLogs => self.replay_pick_logs(),
            PanelAction::Run => self.replay_run(),
        }
    }

    fn replay_body(&mut self, ui: &mut egui::Ui) -> PanelAction {
        let mut action = PanelAction::None;
        let panel = &mut self.replay;
        let mut pick =
            |ui: &mut egui::Ui, button: &str, hover: &str, to: PanelAction, text: String| {
                ui.horizontal(|ui| {
                    if ui.button(button).on_hover_text(hover).clicked() {
                        action = to;
                    }
                    ui.label(text);
                });
            };
        let group = panel
            .group
            .as_deref()
            .map_or_else(|| "No .Group picked".to_string(), file_name);
        pick(
            ui,
            "Pick .Group…",
            "The mission file the logs are from",
            PanelAction::PickGroup,
            group,
        );
        let trace = match (&panel.trace_pick, panel.trace_path()) {
            (Some(p), _) => file_name(p),
            (None, Some(p)) if p.is_file() => format!("{} (next to the .Group)", file_name(&p)),
            _ => "None".to_string(),
        };
        pick(
            ui,
            "Trace file…",
            "Only when the .trace.json is not next to the .Group",
            PanelAction::PickTrace,
            trace,
        );
        let logs = match &panel.log_dir {
            None => "No log folder picked".to_string(),
            Some(dir) => format!("{} · {} log files", file_name(dir), panel.log_files.len()),
        };
        pick(
            ui,
            "Log folder…",
            "The server's text log folder",
            PanelAction::PickLogs,
            logs,
        );
        if panel.group.is_some() && panel.trace_path().is_none_or(|p| !p.is_file()) {
            shell::warning(ui, NO_SIDECAR);
        }
        let checks = panel.checks();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(shell::all_ok(&checks), egui::Button::new("Run replay"))
                .clicked()
            {
                action = PanelAction::Run;
            }
            shell::readiness(ui, &checks);
            if shell::hint(ui, "Reads the logs against the .Group and its trace.", true) {
                action = PanelAction::Help;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").on_hover_text("Esc").clicked() {
                    action = PanelAction::Close;
                }
            });
        });
        if let Some(err) = &panel.error {
            ui.label(RichText::new(err).color(c::DPRK));
        }
        ui.separator();
        let Some(loaded) = &panel.loaded else {
            ui.label(
                RichText::new("Pick a .Group and a log folder, then Run replay.")
                    .color(c::NEUTRAL_700),
            );
            return action;
        };
        let replay = &loaded.replay;
        let rate = &replay.tick_rate;
        ui.label(
            RichText::new(format!(
                "{} log files · T {}–{} · {:.2} ticks/s ({}) · {} unknown records",
                replay.files.len(),
                replay
                    .first_t
                    .map_or_else(|| "none".into(), |t| t.to_string()),
                replay
                    .last_t
                    .map_or_else(|| "none".into(), |t| t.to_string()),
                rate.rate,
                if rate.fitted {
                    "fitted"
                } else {
                    "50 ticks/s fallback"
                },
                replay.unknown_records
            ))
            .small()
            .color(c::NEUTRAL_700),
        );
        let rows = &loaded.rows;
        let mut tabs = Vec::new();
        if replay.trace_supplied {
            tabs.push((
                ReplaySection::DeadLinks,
                "Dead links",
                Some(rows.dead_links.len()),
            ));
            tabs.push((
                ReplaySection::Unobserved,
                "Unobserved",
                Some(rows.unobserved.len()),
            ));
            tabs.push((
                ReplaySection::NeverFired,
                "Never fired",
                Some(rows.never_fired_total),
            ));
            tabs.push((ReplaySection::Fired, "Fired", Some(rows.fired.len())));
        }
        tabs.push((
            ReplaySection::NeverSpawned,
            "Never spawned",
            Some(rows.never_spawned.len()),
        ));
        tabs.push((
            ReplaySection::Timeline,
            "Timeline",
            Some(replay.timeline.len()),
        ));
        shell::dock_tabs(ui, &mut panel.section, &tabs);
        ui.add_space(6.0);
        match panel.section {
            ReplaySection::DeadLinks => dead_links(ui, rows),
            ReplaySection::Unobserved => unobserved(ui, rows),
            ReplaySection::NeverFired => never_fired(ui, rows),
            ReplaySection::Fired => fired(ui, rows),
            ReplaySection::NeverSpawned => never_spawned(ui, rows),
            ReplaySection::Timeline => timeline(ui, loaded, &mut panel.group_filter),
        }
        action
    }

    /// The Map's Replay layer: spawn dots and kill crosses, coloured by side.
    pub(super) fn draw_map_replay(&mut self, painter: &egui::Painter, map_rect: Rect) {
        let panel = &mut self.replay;
        panel.drawn = [0, 0];
        let Some(loaded) = panel.loaded.as_ref().filter(|_| panel.layer) else {
            return;
        };
        for marker in &loaded.replay.markers {
            let pos = super::map::world_to_pos(map_rect, marker.x, marker.z);
            paint_marker(painter, pos, marker.kind, marker_color(marker));
            panel.drawn[usize::from(marker.kind == MarkerKind::Kill)] += 1;
        }
    }
}

enum PanelAction {
    None,
    Close,
    Help,
    PickGroup,
    PickTrace,
    PickLogs,
    Run,
}

/// DPRK or NATO by the object's country; neutral when the log gives none.
fn marker_color(marker: &MapMarker) -> Color32 {
    marker
        .country
        .and_then(seat_side)
        .map_or(c::NEUTRAL_700, shell::Side::color)
}

/// A spawn is a ringed dot, a kill a cross. Also the legend's symbols.
pub(super) fn paint_marker(
    painter: &egui::Painter,
    center: Pos2,
    kind: MarkerKind,
    color: Color32,
) {
    match kind {
        MarkerKind::Spawn => {
            painter.circle_filled(center, 4.0, color);
            painter.circle_stroke(center, 4.0, Stroke::new(1.0_f32, c::BG));
        }
        MarkerKind::Kill => {
            let d = Vec2::splat(4.5);
            let stroke = Stroke::new(2.0_f32, color);
            painter.line_segment([center - d, center + d], stroke);
            painter.line_segment(
                [center + Vec2::new(-d.x, d.y), center + Vec2::new(d.x, -d.y)],
                stroke,
            );
        }
    }
}

/// A striped table in a vertical scroll area. Each column gets its share of
/// the width (`cols`: header and weight); long text wraps inside its column,
/// so the MCU paths never push the other columns out of view. `hover` gives
/// one column a full text on hover, per row.
fn table(
    ui: &mut egui::Ui,
    id: &str,
    cols: &[(&str, f32)],
    rows: Vec<Vec<String>>,
    hover: Option<(usize, Vec<String>)>,
) {
    const GAP: f32 = 16.0;
    let total: f32 = cols.iter().map(|c| c.1).sum();
    let free = (ui.available_width() - GAP * cols.len() as f32).max(120.0);
    let widths: Vec<f32> = cols
        .iter()
        .map(|c| (free * c.1 / total).max(48.0))
        .collect();
    let cell = |ui: &mut egui::Ui, w: f32, text: RichText, tip: Option<&String>| {
        // Sized by its content, so the row (and its stripe) is as tall as
        // the wrapped text.
        ui.vertical(|ui| {
            ui.set_width(w);
            let label = ui.add(egui::Label::new(text).wrap());
            if let Some(tip) = tip.filter(|t| !t.is_empty()) {
                label.on_hover_text(tip);
            }
        });
    };
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new(id)
                .striped(true)
                .spacing([GAP, 6.0])
                .num_columns(cols.len())
                .show(ui, |ui| {
                    for (c, w) in cols.iter().zip(&widths) {
                        cell(ui, *w, RichText::new(c.0).strong(), None);
                    }
                    ui.end_row();
                    for (r, row) in rows.into_iter().enumerate() {
                        for (col, (text, w)) in row.into_iter().zip(&widths).enumerate() {
                            let tip = hover
                                .as_ref()
                                .filter(|(c, _)| *c == col)
                                .and_then(|(_, tips)| tips.get(r));
                            cell(ui, *w, RichText::new(text), tip);
                        }
                        ui.end_row();
                    }
                });
        });
}

/// At most three names, then how many more: one successor can have dozens.
fn short_list(names: &[String]) -> String {
    match names.len() {
        0..=3 => names.join("; "),
        n => format!("{}; +{} more", names[..3].join("; "), n - 3),
    }
}

fn none(ui: &mut egui::Ui) {
    ui.label(RichText::new("None.").color(c::NEUTRAL_700));
}

fn dead_links(ui: &mut egui::Ui, rows: &ReportRows) {
    ui.label("Faults: the next breadcrumb on an unconditional path never logged.");
    if rows.dead_links.is_empty() {
        return none(ui);
    }
    let cols = [
        ("From (fired)", 4.0),
        ("To (never fired)", 4.0),
        ("Fired (s)", 1.0),
        ("Delay (s)", 1.0),
    ];
    let body = rows
        .dead_links
        .iter()
        .map(|d| {
            vec![
                d.from.clone(),
                d.to.clone(),
                format!("{:.1}", d.fired_s),
                format!("{:.1}", d.delay_s),
            ]
        })
        .collect();
    table(ui, "replay_dead", &cols, body, None);
}

fn unobserved(ui: &mut egui::Ui, rows: &ReportRows) {
    ui.label(missionlog::UNOBSERVED_NOTE);
    if rows.unobserved.is_empty() {
        return none(ui);
    }
    let cols = [
        ("Never fired", 1.0),
        ("Upstream fired", 1.0),
        ("Downstream (also never fired)", 1.0),
    ];
    let body = rows
        .unobserved
        .iter()
        .map(|gap| {
            vec![
                gap.successor.clone(),
                gap.upstream.join("; "),
                short_list(&gap.downstream_short),
            ]
        })
        .collect();
    let full = rows.unobserved.iter().map(|gap| gap.downstream.join("
")).collect();
    table(ui, "replay_unobserved", &cols, body, Some((2, full)));
}

fn never_fired(ui: &mut egui::Ui, rows: &ReportRows) {
    ui.label(format!(
        "{} traced sources never observed; {} of them are listed under Unobserved.",
        rows.never_fired_total, rows.never_fired_grouped
    ));
    if rows.never_fired.is_empty() {
        return none(ui);
    }
    let body = rows.never_fired.iter().map(|l| vec![l.clone()]).collect();
    table(ui, "replay_never_fired", &[("Source", 1.0)], body, None);
}

fn fired(ui: &mut egui::Ui, rows: &ReportRows) {
    ui.label("Traced sources the log saw, with each firing's time.");
    if rows.fired.is_empty() {
        return none(ui);
    }
    let body = rows
        .fired
        .iter()
        .map(|f| {
            let times: Vec<_> = f.times_s.iter().map(|s| format!("{s:.1}")).collect();
            vec![
                f.source.clone(),
                f.times_s.len().to_string(),
                times.join(", "),
            ]
        })
        .collect();
    table(
        ui,
        "replay_fired",
        &[("Source", 5.0), ("Firings", 1.0), ("At (s)", 3.0)],
        body,
        None,
    );
}

fn never_spawned(ui: &mut egui::Ui, rows: &ReportRows) {
    ui.label(missionlog::NEVER_SPAWNED_NOTE);
    if rows.never_spawned.is_empty() {
        return none(ui);
    }
    let cols = [
        ("Name", 2.0),
        ("Spawned", 1.0),
        ("In the mission", 1.0),
        ("Group", 5.0),
    ];
    let body = rows
        .never_spawned
        .iter()
        .map(|s| {
            vec![
                s.name.clone(),
                s.spawned_count.to_string(),
                s.mission_count.to_string(),
                s.group_path.as_deref().map(group_label).unwrap_or_default(),
            ]
        })
        .collect();
    table(ui, "replay_never_spawned", &cols, body, None);
}

fn timeline(ui: &mut egui::Ui, loaded: &LoadedReplay, filter: &mut Option<Vec<String>>) {
    ui.horizontal(|ui| {
        ui.label("Group");
        let shown = filter
            .as_deref()
            .map_or_else(|| "All groups".to_string(), group_label);
        egui::ComboBox::from_id_salt("replay_group_filter")
            .width(420.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(filter, None, "All groups");
                for g in &loaded.groups {
                    ui.selectable_value(filter, Some(g.clone()), group_label(g));
                }
            })
            .response
            .widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Timeline group")
            });
    });
    let shown: Vec<_> = loaded
        .replay
        .timeline
        .iter()
        .filter(|e| {
            filter
                .as_deref()
                .is_none_or(|g| missionlog::within_group(e.group_path.as_ref(), g))
        })
        .collect();
    if filter.is_some() {
        ui.label(
            RichText::new(format!(
                "{} of {} events. Events of objects whose name repeats have no group.",
                shown.len(),
                loaded.replay.timeline.len()
            ))
            .small()
            .color(c::NEUTRAL_700),
        );
    }
    if shown.is_empty() {
        return none(ui);
    }
    let cols = [
        ("Seconds", 1.0),
        ("T", 1.0),
        ("Event", 5.0),
        ("Position X, Y, Z", 2.0),
        ("Group", 4.0),
    ];
    let body = shown
        .into_iter()
        .map(|e| {
            vec![
                format!("{:.1}", e.seconds),
                e.t_ticks.to_string(),
                e.description.clone(),
                e.pos
                    .map(|(x, y, z)| format!("{x:.0}, {y:.0}, {z:.0}"))
                    .unwrap_or_default(),
                e.group_path.as_deref().map(group_label).unwrap_or_default(),
            ]
        })
        .collect();
    table(ui, "replay_timeline", &cols, body, None);
}
