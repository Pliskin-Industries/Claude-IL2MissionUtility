//! # shell.rs — shared page layout and widgets for every mode
//!
//! Presentation only, like ui.rs. Nothing here touches the AST.
//! See docs/ui-redesign/README.md §4 for how `update()` composes these.
//! Ported to egui 0.32. Later redesign phases use the widgets that phase 1
//! (theme + shell) does not, hence the module-wide `dead_code` allow.

#![allow(dead_code)]

use std::f32::consts::PI;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Layout, Pos2, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, TextStyle, TextureHandle, Ui, Vec2,
};

use crate::theme::{bold_family, c, heading_family};

// ── Sizes (px) ──────────────────────────────────────────────────────────────
pub const RAIL_W: f32 = 172.0;
pub const HEADER_H: f32 = 54.0;
pub const STATUS_H: f32 = 30.0;
pub const TOOL_PALETTE_W: f32 = 56.0;
pub const TOOL_SIZE: f32 = 36.0;
pub const ROW_H: f32 = 32.0;

// ── Sides ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Dprk,
    Nato,
}

impl Side {
    pub fn from_eastern(eastern: bool) -> Self {
        if eastern { Side::Dprk } else { Side::Nato }
    }
    pub fn label(self) -> &'static str {
        match self {
            Side::Dprk => "DPRK",
            Side::Nato => "NATO",
        }
    }
    /// Only where the MCU coalition value matters.
    pub fn coalition_label(self) -> &'static str {
        match self {
            Side::Dprk => "DPRK [1]",
            Side::Nato => "NATO [2]",
        }
    }
    pub fn color(self) -> Color32 {
        match self {
            Side::Dprk => c::DPRK,
            Side::Nato => c::NATO,
        }
    }
    /// Screen rotation for an icon authored pointing north (up).
    /// DPRK faces south toward the front, NATO faces north.
    pub fn facing_rad(self) -> f32 {
        match self {
            Side::Dprk => PI,
            Side::Nato => 0.0,
        }
    }
}

/// Fallback marker (a pentagon pointing the side's way) for when no texture is loaded.
/// With textures, draw the existing `assets/Eastern*.svg` / `Nato*.svg` via
/// `paint_rotated_image` at `side.facing_rad()` (plus the unit heading on the map).
pub fn paint_side_marker(painter: &egui::Painter, center: Pos2, size: f32, side: Side) {
    let h = size / 2.0;
    let mut pts = vec![
        Pos2::new(-0.67 * h, 0.75 * h),
        Pos2::new(0.67 * h, 0.75 * h),
        Pos2::new(0.67 * h, -0.25 * h),
        Pos2::new(0.0, -0.92 * h),
        Pos2::new(-0.67 * h, -0.25 * h),
    ];
    if side == Side::Dprk {
        for p in &mut pts {
            p.y = -p.y;
        }
        pts.reverse(); // keep winding after the flip
    }
    let pts = pts.into_iter().map(|p| center + p.to_vec2()).collect();
    painter.add(egui::Shape::convex_polygon(pts, side.color(), Stroke::NONE));
}

pub fn side_marker(ui: &mut Ui, side: Side, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_side_marker(ui.painter(), rect.center(), size, side);
}

// ── Blueprint framing ───────────────────────────────────────────────────────
/// "+" registration marks at each corner, 11 px across, centered on the corner.
pub fn corner_marks(painter: &egui::Painter, rect: Rect, color: Color32) {
    let s = Stroke::new(1.0_f32, color);
    let arm = 5.5;
    for p in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
        painter.line_segment([p - Vec2::X * arm, p + Vec2::X * arm], s);
        painter.line_segment([p - Vec2::Y * arm, p + Vec2::Y * arm], s);
    }
}

/// List card. Selected: accent border, ACCENT_100 fill, corner marks.
/// Unselected: hairline border, no fill, no marks.
pub fn card<R>(ui: &mut Ui, selected: bool, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let (stroke, fill) = if selected {
        (Stroke::new(1.0_f32, c::ACCENT), c::ACCENT_100)
    } else {
        (Stroke::new(1.0_f32, c::DIVIDER), Color32::TRANSPARENT)
    };
    let inner = egui::Frame::new()
        .stroke(stroke)
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    if selected {
        corner_marks(ui.painter(), inner.response.rect, c::MARK);
    }
    inner
}

/// Outlined panel with marks (e.g. "Start checkzones", "Removed").
pub fn blueprint<R>(ui: &mut Ui, border: Color32, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let inner = egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, border))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    corner_marks(ui.painter(), inner.response.rect, c::MARK);
    inner
}

// ── Text helpers ────────────────────────────────────────────────────────────
/// Condensed, uppercase, 14 px. Optional right-aligned note in Small.
pub fn section_title(ui: &mut Ui, title: &str, note: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title.to_uppercase())
                .text_style(TextStyle::Name("section".into()))
                .color(c::TEXT),
        );
        if let Some(n) = note {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(n).small().color(c::NEUTRAL_700));
            });
        }
    });
}

/// Collapsible settings block for the right panel. Collapsed, it still shows a
/// one-line summary on the right. Open/closed state persists by `id`.
pub fn settings_section(
    ui: &mut Ui,
    id: &str,
    title: &str,
    summary: &str,
    default_open: bool,
    body: impl FnOnce(&mut Ui),
) {
    let id = ui.make_persistent_id(id);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open)
        .show_header(ui, |ui| {
            ui.label(RichText::new(title).font(FontId::new(13.0, bold_family())));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(summary).small().color(c::NEUTRAL_700)).truncate());
            });
        })
        .body(|ui| {
            ui.add_space(4.0);
            body(ui);
        });
    ui.separator();
}

/// Small outlined tag (e.g. "Lead ×4"). Accent tags use ACCENT_700 text on
/// ACCENT_100. Painted as one widget so it wraps in `horizontal_wrapped`.
pub fn tag(ui: &mut Ui, text: &str, accent: bool) -> Response {
    let (fg, fill, border) = if accent {
        (c::ACCENT_700, c::ACCENT_100, c::ACCENT_300)
    } else {
        (c::NEUTRAL_800, Color32::TRANSPARENT, c::DIVIDER)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::new(12.0, bold_family()), fg);
    let size = galley.size() + Vec2::new(12.0, 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, fill);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        p.galley(rect.min + Vec2::new(6.0, 3.0), galley, fg);
    }
    resp
}

/// Dashed circle outline (egui has no dashed circle primitive).
pub fn dashed_circle(painter: &egui::Painter, center: Pos2, radius: f32, stroke: Stroke) {
    let n = ((radius * 0.6) as usize).clamp(24, 360);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.extend(egui::Shape::dashed_line(&pts, stroke, 6.0, 5.0));
}

/// One-line explanation under a control (spec item 8). Optional "Help ›" link.
/// Returns true if the Help link was clicked.
pub fn hint(ui: &mut Ui, text: &str, with_help: bool) -> bool {
    let mut help = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(text).small().color(c::NEUTRAL_700));
        if with_help && link(ui, "Help ›").clicked() {
            help = true;
        }
    });
    help
}

/// Text link with a 28 px tall hit target (README §6.6) that keeps the
/// line height of the text around it.
pub fn link(ui: &mut Ui, text: &str) -> Response {
    let resp = ui.add(
        egui::Label::new(RichText::new(text).color(c::ACCENT_700))
            .sense(Sense::click())
            .selectable(false),
    );
    let pad = ((28.0 - resp.rect.height()) / 2.0).max(0.0);
    let hit = ui.interact(resp.rect.expand2(Vec2::new(2.0, pad)), resp.id.with("hit"), Sense::click());
    let resp = resp.union(hit);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        let r = resp.rect.shrink2(Vec2::new(2.0, pad));
        ui.painter().line_segment(
            [r.left_bottom(), r.right_bottom()],
            Stroke::new(1.0_f32, c::ACCENT_700),
        );
    }
    resp
}

pub fn kbd(ui: &mut Ui, keys: &str) {
    egui::Frame::new()
        .stroke(Stroke::new(1.0_f32, c::DIVIDER))
        .inner_margin(egui::Margin::symmetric(5, 1))
        .show(ui, |ui| ui.label(RichText::new(keys).monospace().color(c::NEUTRAL_700)));
}

pub fn warning(ui: &mut Ui, text: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("⚠").color(c::WARN_TEXT));
        ui.label(RichText::new(text).color(c::TEXT));
    });
}

// ── Buttons ─────────────────────────────────────────────────────────────────
/// The one solid object on the page: accent fill, marks, shortcut hint inside.
pub fn primary_button(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> Response {
    // Disabled keeps the ACCENT_300 fill; light text on it would be unreadable.
    let (label_c, key_c) = if enabled {
        (c::BG, c::ACCENT_100)
    } else {
        (c::ACCENT_800, c::ACCENT_700)
    };
    let label_g = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::new(13.0, bold_family()), label_c);
    let key_g = ui
        .painter()
        .layout_no_wrap(shortcut.to_owned(), FontId::monospace(12.0), key_c);
    let pad = 16.0;
    let gap = 10.0;
    let size = Vec2::new(pad * 2.0 + label_g.size().x + gap + key_g.size().x, 32.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let fill = if !enabled {
            c::ACCENT_300
        } else if resp.is_pointer_button_down_on() {
            c::ACCENT_700
        } else if resp.hovered() {
            c::ACCENT_600
        } else {
            c::ACCENT
        };
        let p = ui.painter();
        p.rect_filled(rect, 2.0, fill);
        let y = rect.center().y;
        let key_w = key_g.size().x;
        let key_h = key_g.size().y;
        p.galley(Pos2::new(rect.left() + pad, y - label_g.size().y / 2.0), label_g, label_c);
        p.galley(Pos2::new(rect.right() - pad - key_w, y - key_h / 2.0), key_g, key_c);
        corner_marks(p, rect, c::MARK);
        if resp.has_focus() {
            p.rect_stroke(rect.expand(2.0), 2.0, Stroke::new(2.0_f32, c::ACCENT), StrokeKind::Outside);
        }
    }
    resp.on_hover_text(format!("{label}  ({shortcut})"))
}

/// Mutually exclusive options in one bordered row (e.g. DPRK | NATO).
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            for (v, label) in options {
                if ui.selectable_label(*value == *v, *label).clicked() && *value != *v {
                    *value = *v;
                    changed = true;
                }
            }
        });
    });
    changed
}

/// 36 × 36 map tool. Active tool is filled accent. Key goes in the tooltip.
pub fn tool_button(
    ui: &mut Ui,
    icon: Option<&TextureHandle>,
    fallback_glyph: &str,
    tint: Option<Color32>,
    tooltip: &str,
    key: &str,
    active: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(TOOL_SIZE), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, tooltip));
    let (bg, fg) = if active {
        (c::ACCENT, c::BG)
    } else if resp.hovered() {
        (c::ACCENT_100, tint.unwrap_or(c::TEXT))
    } else {
        (Color32::TRANSPARENT, tint.unwrap_or(c::NEUTRAL_800))
    };
    let p = ui.painter();
    p.rect_filled(rect, 0.0, bg);
    match icon {
        Some(tex) => {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            p.image(tex.id(), rect.shrink(9.0), uv, fg);
        }
        None => {
            p.text(rect.center(), Align2::CENTER_CENTER, fallback_glyph, FontId::proportional(18.0), fg);
        }
    }
    resp.on_hover_text(format!("{tooltip}  ({key})"))
}

// ── Page chrome ─────────────────────────────────────────────────────────────
/// Left mode rail. Returns true if Help was clicked.
pub fn mode_rail(ui: &mut Ui, labels: &[&str], selected: &mut usize) -> bool {
    let mut help = false;
    ui.add_space(14.0);
    ui.label(RichText::new("IL-2 GROUP\nGENERATOR").font(FontId::new(17.0, heading_family())));
    ui.add_space(12.0);
    for (i, label) in labels.iter().enumerate() {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
        let sel = *selected == i;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, sel, *label));
        let p = ui.painter();
        if sel {
            p.rect_filled(rect, 0.0, c::ACCENT_200);
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, c::ACCENT_100);
        }
        let idx_color = if sel { c::ACCENT_800 } else { c::NEUTRAL_600 };
        let fg = if sel { c::ACCENT_900 } else { c::TEXT };
        let family = if sel { bold_family() } else { egui::FontFamily::Proportional };
        p.text(rect.left_center() + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, format!("{:02}", i + 1), FontId::monospace(12.0), idx_color);
        p.text(rect.left_center() + Vec2::new(34.0, 0.0), Align2::LEFT_CENTER, *label, FontId::new(13.0, family), fg);
        if resp.clicked() {
            *selected = i;
        }
        resp.on_hover_text(format!("Ctrl {}", i + 1));
    }
    ui.add_space(4.0);
    ui.label(RichText::new("Ctrl 1–6").small().color(c::NEUTRAL_700));
    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if link(ui, "Help").clicked() {
                help = true;
            }
            kbd(ui, "F1");
        });
    });
    help
}

/// Header row. `leading` draws right after the title (e.g. a sub-mode switch).
/// `secondary` runs inside a right-to-left layout: add buttons in REVERSE order.
/// Returns true when the primary button is clicked.
pub fn page_header(
    ui: &mut Ui,
    title: &str,
    file: Option<&str>,
    primary: &str,
    primary_enabled: bool,
    leading: impl FnOnce(&mut Ui),
    secondary: impl FnOnce(&mut Ui),
) -> bool {
    let mut clicked = false;
    ui.horizontal_centered(|ui| {
        ui.label(RichText::new(title.to_uppercase()).text_style(TextStyle::Heading));
        leading(ui);
        if let Some(f) = file {
            ui.label(RichText::new(f).italics().color(c::NEUTRAL_700));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            clicked = primary_button(ui, primary, "Ctrl G", primary_enabled).clicked();
            ui.add_space(4.0);
            secondary(ui);
        });
    });
    clicked
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

/// Bottom line. Returns true when Undo is clicked.
pub fn status_bar(ui: &mut Ui, severity: Severity, message: &str, undo_label: Option<&str>) -> bool {
    let mut undo = false;
    ui.horizontal_centered(|ui| {
        let dot = match severity {
            Severity::Info => c::ACCENT,
            Severity::Warn => c::WARN,
            Severity::Error => c::DPRK,
        };
        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, dot);
        if severity != Severity::Info {
            ui.label(RichText::new("⚠").color(c::WARN_TEXT));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(label) = undo_label {
                kbd(ui, "Ctrl Z");
                if link(ui, "Undo").clicked() {
                    undo = true;
                }
                ui.label(label);
            }
            // The message takes what is left and elides; the full text shows on hover.
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::Label::new(message).truncate());
            });
        });
    });
    undo
}

/// Dark banner at the top-center of the map naming the active drawing tool.
pub fn map_tool_banner(painter: &egui::Painter, map_rect: Rect, name: &str, hint: &str) {
    let text = format!("{}    {}", name.to_uppercase(), hint);
    let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), c::BG);
    let size = galley.size() + Vec2::new(24.0, 12.0);
    let rect = Rect::from_center_size(
        Pos2::new(map_rect.center().x, map_rect.top() + 12.0 + size.y / 2.0),
        size,
    );
    painter.rect_filled(rect, 0.0, c::ACCENT_900);
    painter.galley(rect.min + Vec2::new(12.0, 6.0), galley, c::BG);
}

/// Confirmation before a destructive action. Returns true on confirm.
/// Caller closes it on Esc by setting `*open = false`.
pub fn confirm_dialog(ctx: &egui::Context, open: &mut bool, title: &str, body: &str, confirm: &str) -> bool {
    if !*open {
        return false;
    }
    let mut confirmed = false;
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.label(body);
            ui.add_space(10.0);
            // A fixed-height row: a bare with_layout would take the window's full height.
            let row = Vec2::new(ui.available_width(), 32.0);
            ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
                if primary_button(ui, confirm, "Enter", true).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    confirmed = true;
                    *open = false;
                }
                if ui.button("Cancel").clicked() {
                    *open = false;
                }
            });
        });
    confirmed
}

// ── Undo (one snapshot per tab, spec item 7) ────────────────────────────────
pub struct Undo<T> {
    slot: Option<(String, T)>,
}

impl<T> Default for Undo<T> {
    fn default() -> Self {
        Self { slot: None }
    }
}

impl<T> Undo<T> {
    /// Call BEFORE the destructive change with a clone of what it removes.
    pub fn record(&mut self, label: impl Into<String>, before: T) {
        self.slot = Some((label.into(), before));
    }
    pub fn label(&self) -> Option<&str> {
        self.slot.as_ref().map(|(l, _)| l.as_str())
    }
    pub fn take(&mut self) -> Option<T> {
        self.slot.take().map(|(_, s)| s)
    }
    pub fn clear(&mut self) {
        self.slot = None;
    }
}

// ── Shortcuts (spec item 10) ────────────────────────────────────────────────
#[derive(Default)]
pub struct Shortcuts {
    pub generate: bool,
    pub load: bool,
    pub undo: bool,
    pub redo: bool,
    pub help: bool,
    pub escape: bool,
    pub tab: Option<usize>,
    pub map_tool: Option<usize>,
}

/// Read once at the top of `update()`, before any panel is drawn.
/// Remove the Ctrl-Z / Ctrl-Y handling inside `map_draw_toolbar` so keys are not consumed twice.
pub fn read_shortcuts(ctx: &egui::Context, map_active: bool) -> Shortcuts {
    use egui::{Key, Modifiers};
    let typing = ctx.wants_keyboard_input();
    let mut s = Shortcuts::default();
    ctx.input_mut(|i| {
        s.generate = i.consume_key(Modifiers::COMMAND, Key::G);
        s.load = i.consume_key(Modifiers::COMMAND, Key::O);
        // Leave Ctrl Z / Ctrl Y to a focused text field's own undo.
        if !typing {
            s.undo = i.consume_key(Modifiers::COMMAND, Key::Z);
            s.redo = i.consume_key(Modifiers::COMMAND, Key::Y);
        }
        s.help = i.consume_key(Modifiers::NONE, Key::F1);
        // Not consumed: egui's own popups, combo boxes and text edits also close on Esc.
        s.escape = i.key_pressed(Key::Escape);
        let nums = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6];
        for (n, k) in nums.iter().enumerate() {
            if i.consume_key(Modifiers::COMMAND, *k) {
                s.tab = Some(n);
            } else if map_active && !typing && i.consume_key(Modifiers::NONE, *k) {
                s.map_tool = Some(n);
            }
        }
    });
    s
}
