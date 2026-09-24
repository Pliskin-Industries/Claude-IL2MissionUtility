//! # theme.rs — Industry design tokens for egui
//!
//! Light technical ground, one steel accent, Barlow / Barlow Condensed.
//! `ui::run()` calls `theme::apply(ctx)` once at startup. It replaced the old
//! `apply_readable_style`, whose sizes are superseded here (Small is 12 px now).
//!
//! Ported to egui/eframe 0.32 (`CornerRadius`, `corner_radius`, integer
//! `Margin`). The app is pinned to the light theme: every token here is a
//! light-ground value. Later redesign phases use the tokens phase 1 does not.

#![allow(dead_code)]

use eframe::egui::{
    self, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Theme,
    ThemePreference, Vec2,
};

/// Color tokens. Hex values come from the Industry `styles.css` ramps.
pub mod c {
    use eframe::egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf3);
    pub const SURFACE: Color32 = Color32::from_rgb(0xe9, 0xe9, 0xea);
    pub const TEXT: Color32 = Color32::from_rgb(0x1d, 0x1f, 0x20);
    /// Text at 16 % over BG, flattened to opaque. Hairlines and borders.
    pub const DIVIDER: Color32 = Color32::from_rgb(0xd0, 0xd0, 0xd1);
    /// Text at 55 % over BG. Corner registration marks.
    pub const MARK: Color32 = Color32::from_rgb(0x7d, 0x7e, 0x7f);

    pub const ACCENT: Color32 = Color32::from_rgb(0x59, 0x80, 0xa6);
    pub const ACCENT_100: Color32 = Color32::from_rgb(0xee, 0xf6, 0xff);
    pub const ACCENT_200: Color32 = Color32::from_rgb(0xd6, 0xeb, 0xff);
    pub const ACCENT_300: Color32 = Color32::from_rgb(0xb5, 0xd9, 0xfd);
    pub const ACCENT_400: Color32 = Color32::from_rgb(0x94, 0xbc, 0xe3);
    pub const ACCENT_500: Color32 = Color32::from_rgb(0x74, 0x9d, 0xc4);
    pub const ACCENT_600: Color32 = Color32::from_rgb(0x59, 0x7e, 0xa3);
    pub const ACCENT_700: Color32 = Color32::from_rgb(0x41, 0x61, 0x80);
    pub const ACCENT_800: Color32 = Color32::from_rgb(0x2c, 0x45, 0x5d);
    pub const ACCENT_900: Color32 = Color32::from_rgb(0x1d, 0x2d, 0x3d);

    pub const NEUTRAL_100: Color32 = Color32::from_rgb(0xf5, 0xf5, 0xf8);
    pub const NEUTRAL_200: Color32 = Color32::from_rgb(0xe7, 0xe7, 0xea);
    pub const NEUTRAL_300: Color32 = Color32::from_rgb(0xd4, 0xd4, 0xd7);
    pub const NEUTRAL_400: Color32 = Color32::from_rgb(0xb7, 0xb7, 0xba);
    pub const NEUTRAL_500: Color32 = Color32::from_rgb(0x98, 0x98, 0x9b);
    pub const NEUTRAL_600: Color32 = Color32::from_rgb(0x7a, 0x7a, 0x7d);
    pub const NEUTRAL_700: Color32 = Color32::from_rgb(0x5d, 0x5d, 0x60);
    pub const NEUTRAL_800: Color32 = Color32::from_rgb(0x42, 0x42, 0x44);
    pub const NEUTRAL_900: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2d);

    // Data colors (not decoration). Approximations of the OKLCH values in the mockups.
    /// DPRK side, oklch(0.52 0.17 25).
    pub const DPRK: Color32 = Color32::from_rgb(0xad, 0x31, 0x36);
    /// NATO side uses the deep accent step.
    pub const NATO: Color32 = ACCENT_700;
    /// Front line, oklch(0.55 0.18 25).
    pub const FRONT: Color32 = Color32::from_rgb(0xb8, 0x35, 0x3a);
    /// Status dot for warnings, oklch(0.7 0.14 70).
    pub const WARN: Color32 = Color32::from_rgb(0xc9, 0x91, 0x3a);
    /// Warning icon / text, oklch(0.55 0.14 65). Passes 4.5:1 on BG.
    pub const WARN_TEXT: Color32 = Color32::from_rgb(0x93, 0x66, 0x1c);
}

pub const HEADING: &str = "heading";
pub const BOLD: &str = "bold";

pub fn heading_family() -> FontFamily {
    FontFamily::Name(HEADING.into())
}
pub fn bold_family() -> FontFamily {
    FontFamily::Name(BOLD.into())
}

/// Barlow for body, Barlow SemiBold for emphasis, Barlow Condensed SemiBold for headings.
/// Download from Google Fonts (SIL OFL) into `assets/fonts/`.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "barlow".into(),
        FontData::from_static(include_bytes!("../assets/fonts/Barlow-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "barlow_semibold".into(),
        FontData::from_static(include_bytes!("../assets/fonts/Barlow-SemiBold.ttf")).into(),
    );
    fonts.font_data.insert(
        "barlow_condensed".into(),
        FontData::from_static(include_bytes!("../assets/fonts/BarlowCondensed-SemiBold.ttf"))
            .into(),
    );
    // Barlow first; egui's defaults stay behind it for symbols (⚠ ▶ ▼ ✓).
    // Hack (egui's monospace) goes last: it is the only bundled font with
    // arrows (→ ← ↑ ↓), which Barlow and Ubuntu-Light both lack.
    let prop = fonts.families.get_mut(&FontFamily::Proportional).unwrap();
    prop.insert(0, "barlow".into());
    prop.push("Hack".into());
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    let mut heading = vec!["barlow_condensed".to_owned()];
    heading.extend(fallback.iter().cloned());
    let mut bold = vec!["barlow_semibold".to_owned()];
    bold.extend(fallback.iter().cloned());
    fonts.families.insert(heading_family(), heading);
    fonts.families.insert(bold_family(), bold);
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    // Light only: without this egui follows the OS theme and would swap in
    // its dark style, which none of these tokens were chosen for.
    ctx.options_mut(|o| o.theme_preference = ThemePreference::Light);
    ctx.set_visuals_of(Theme::Light, visuals());
    ctx.style_mut_of(Theme::Light, |s| {
        use TextStyle::*;
        s.text_styles = [
            (Heading, FontId::new(22.0, heading_family())),
            (Name("section".into()), FontId::new(14.0, heading_family())),
            (Body, FontId::new(13.0, FontFamily::Proportional)),
            (Button, FontId::new(13.0, FontFamily::Proportional)),
            (Small, FontId::new(12.0, FontFamily::Proportional)), // never smaller
            (Monospace, FontId::new(12.0, FontFamily::Monospace)),
        ]
        .into();
        s.spacing.interact_size = Vec2::new(40.0, 28.0); // 28 px minimum target height
        s.spacing.item_spacing = Vec2::new(8.0, 6.0); // ≥ 4 px between targets
        s.spacing.button_padding = Vec2::new(11.0, 4.0);
        s.spacing.slider_width = 140.0;
        s.spacing.icon_width = 16.0;
    });
}

pub fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.panel_fill = c::BG;
    v.window_fill = c::BG;
    v.extreme_bg_color = c::NEUTRAL_100; // text edits, drag values
    v.faint_bg_color = c::SURFACE;
    v.code_bg_color = c::SURFACE;
    v.hyperlink_color = c::ACCENT_700;
    v.warn_fg_color = c::WARN_TEXT;
    v.error_fg_color = c::DPRK;
    v.selection.bg_fill = c::ACCENT_200;
    v.selection.stroke = Stroke::new(1.0_f32, c::ACCENT_900);
    v.window_stroke = Stroke::new(1.0_f32, c::DIVIDER);
    v.slider_trailing_fill = true;
    v.striped = false;

    let r = CornerRadius::same(2); // square-cornered system; 2 px keeps AA clean
    let w = &mut v.widgets;
    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = r;
        s.expansion = 0.0;
    }
    w.noninteractive.bg_fill = c::BG;
    w.noninteractive.weak_bg_fill = c::BG;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, c::DIVIDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, c::TEXT);

    w.inactive.bg_fill = c::NEUTRAL_100;
    w.inactive.weak_bg_fill = c::NEUTRAL_100;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, c::DIVIDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, c::TEXT);

    w.hovered.bg_fill = c::ACCENT_100;
    w.hovered.weak_bg_fill = c::ACCENT_100;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, c::ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, c::TEXT);

    w.active.bg_fill = c::ACCENT_200;
    w.active.weak_bg_fill = c::ACCENT_200;
    w.active.bg_stroke = Stroke::new(1.0_f32, c::ACCENT_600);
    w.active.fg_stroke = Stroke::new(1.0_f32, c::ACCENT_900);

    w.open = w.active;
    v
}
