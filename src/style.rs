use crate::dock::Style as DockStyle;

use bevy::utils::default;
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, Style, TextStyle};

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

pub trait IntoDockStyle {
    fn into_dock_style(&self) -> DockStyle;
}

impl IntoDockStyle for Style {
    fn into_dock_style(&self) -> DockStyle {
        let mut style = DockStyle::from_egui(self);

        // style.dock_area_padding = Some(Margin {
        //     left: 4,
        //     right: 4,
        //     ..default()
        // });

        // style.separator.width = 4.0;
        // style.separator.color_idle = self.visuals.window_fill;
        // style.separator.color_dragged = self.visuals.window_fill;
        // style.separator.color_hovered = self.visuals.window_fill;

        // style.tab_bar.bg_fill = self.visuals.window_fill;
        // style.tab_bar.inner_margin = Margin {
        //     left: 6,
        //     right: 6,
        //     top: style.tab_bar.stroke.width as i8,
        //     ..default()
        // };
        // style.tab_bar.corner_radius = CornerRadius {
        //     nw: 6,
        //     ne: 6,
        //     ..default()
        // };

        // let pane_bg_color = self.visuals.window_fill.linear_multiply(1.25);

        // style.tab.active.bg_fill = pane_bg_color;
        // style.tab.active_with_kb_focus.bg_fill = pane_bg_color;
        // style.tab.focused.bg_fill = pane_bg_color;
        // style.tab.focused_with_kb_focus.bg_fill = pane_bg_color;
        // style.tab.hovered.bg_fill = pane_bg_color;
        // style.tab.inactive.bg_fill = self.visuals.window_fill;
        // style.tab.inactive_with_kb_focus.bg_fill = self.visuals.window_fill;

        // style.tab.tab_body.bg_fill = pane_bg_color;
        // style.tab.tab_body.corner_radius = CornerRadius {
        //     sw: 6,
        //     se: 6,
        //     ..default()
        // };

        style
    }
}
