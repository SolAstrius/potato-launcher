//! Central visual theme. Restrained, near-black cool surfaces with a single muted
//! sage-green accent used sparingly (primary action, selection, active step). Clean
//! proportional type; the character comes from palette, spacing, and the step reveal.

use eframe::egui;
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

// Surfaces, coolest (window) to lightest (pressed control).
pub const BG: Color32 = Color32::from_rgb(21, 24, 28);
pub const PANEL: Color32 = Color32::from_rgb(27, 31, 36);
pub const SURFACE: Color32 = Color32::from_rgb(34, 39, 46);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(42, 49, 58);
pub const SURFACE_ACTIVE: Color32 = Color32::from_rgb(50, 59, 69);
pub const EXTREME: Color32 = Color32::from_rgb(15, 18, 21);
pub const STROKE: Color32 = Color32::from_rgb(46, 53, 61);

pub const TEXT: Color32 = Color32::from_rgb(198, 205, 213);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(138, 146, 155);

// The one accent. Muted on purpose: a sage/moss green, not Sburb neon.
pub const ACCENT: Color32 = Color32::from_rgb(121, 181, 138);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(46, 74, 57);

/// Install the theme globally. Call once, before any frame is drawn.
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    apply_visuals(&mut style.visuals);
    apply_spacing(&mut style.spacing);

    // Clean proportional type, just sized up a touch from egui's defaults for air.
    for (text_style, family, size) in [
        (TextStyle::Small, FontFamily::Proportional, 10.5),
        (TextStyle::Body, FontFamily::Proportional, 13.0),
        (TextStyle::Button, FontFamily::Proportional, 13.5),
        (TextStyle::Heading, FontFamily::Proportional, 20.0),
        (TextStyle::Monospace, FontFamily::Monospace, 13.0),
    ] {
        style
            .text_styles
            .insert(text_style, FontId::new(size, family));
    }

    ctx.set_style(style);

    // Shrink the whole UI uniformly: the native DPI scale reads too large otherwise.
    ctx.set_zoom_factor(0.85);
}

fn apply_visuals(v: &mut egui::Visuals) {
    v.dark_mode = true;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = BG;
    v.faint_bg_color = PANEL;
    v.extreme_bg_color = EXTREME;
    v.window_stroke = Stroke::new(1.0, STROKE);
    v.window_corner_radius = CornerRadius::same(8);

    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;

    let cr = CornerRadius::same(5);
    let w = &mut v.widgets;

    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, STROKE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_WEAK);
    w.noninteractive.corner_radius = cr;

    w.inactive.bg_fill = SURFACE;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0, STROKE);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.corner_radius = cr;

    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.corner_radius = cr;

    w.active.bg_fill = SURFACE_ACTIVE;
    w.active.weak_bg_fill = SURFACE_ACTIVE;
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0, TEXT);
    w.active.corner_radius = cr;

    w.open.bg_fill = SURFACE;
    w.open.weak_bg_fill = SURFACE;
    w.open.bg_stroke = Stroke::new(1.0, ACCENT);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    w.open.corner_radius = cr;
}

fn apply_spacing(s: &mut egui::Spacing) {
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(10.0, 6.0);
    s.interact_size.y = 28.0;
    s.window_margin = Margin::same(12);
}
