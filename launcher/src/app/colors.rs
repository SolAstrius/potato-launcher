//! Status text colors. Tuned to sit on the dark theme surfaces (see [`super::theme`])
//! instead of egui's raw primaries. `dark_mode` is kept for the signature but the app
//! is effectively always dark.
use super::theme;

pub fn error(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(210, 112, 106)
}

pub fn partial_error(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(110, 146, 200)
}

pub fn offline(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(201, 162, 63)
}

pub fn in_progress(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(111, 168, 201)
}

pub fn timeout(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(181, 123, 192)
}

pub fn action(_dark_mode: bool) -> egui::Color32 {
    egui::Color32::from_rgb(201, 162, 63)
}

pub fn ok(_dark_mode: bool) -> egui::Color32 {
    theme::ACCENT
}
