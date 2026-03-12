use egui::{CornerRadius, FontFamily, FontId, Style, TextStyle};

pub fn set_dark_style(style: &mut Style) {
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(13.5, FontFamily::Proportional),
        ),
    ]
    .into();

    style.visuals.window_corner_radius = CornerRadius::same(5);
}
