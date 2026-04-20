use crate::{
    assets::{ICON_TEXT_STYLE, LUCIDE_FONT_FAMILY},
    dock::Style as DockStyle,
};

use bevy::utils::default;
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, Style, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(32, 110, 201);

pub fn set_dark_style(style: &mut Style) {
    style.debug.warn_if_rect_changes_id = false;

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
        (
            TextStyle::Name(ICON_TEXT_STYLE.into()),
            FontId::new(15.0, FontFamily::Name(LUCIDE_FONT_FAMILY.into())),
        ),
    ]
    .into();

    style.wrap_mode = Some(egui::TextWrapMode::Truncate);

    style.visuals.text_edit_bg_color = Some(Color32::from_rgb(63, 63, 63));

    style.visuals.widgets.active.fg_stroke.color = Color32::from_rgb(242, 242, 242);
    style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(242, 242, 242);
    style.visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(242, 242, 242);
    style.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(242, 242, 242);

    style.visuals.window_fill = Color32::from_rgb(23, 23, 23);
    style.visuals.panel_fill = Color32::from_rgb(23, 23, 23);
    style.visuals.window_corner_radius = CornerRadius::same(6);

    style.visuals.popup_shadow = Shadow {
        blur: 12,
        spread: 2,
        color: Color32::from_black_alpha(96),
        ..default()
    };
}

pub trait IntoDockStyle {
    fn into_dock_style(&self) -> DockStyle;
}

impl IntoDockStyle for Style {
    fn into_dock_style(&self) -> DockStyle {
        let mut style = DockStyle::from_egui(self);

        style.dock_area_padding = Some(Margin {
            left: 4,
            right: 4,
            ..default()
        });

        style.separator.width = 4.0;
        style.separator.color_idle = self.visuals.window_fill;
        style.separator.color_dragged = self.visuals.window_fill;
        style.separator.color_hovered = self.visuals.window_fill;

        let pane_bg_color = Color32::from_rgb(42, 42, 42);

        style.tab_bar.bg_fill = self.visuals.window_fill;
        style.tab_bar.stroke = Stroke {
            width: 1.0,
            color: pane_bg_color,
        };
        style.tab_bar.height = 30.0;
        style.tab_bar.inner_margin = Margin {
            top: 1,
            left: 8,
            right: 8,
            ..default()
        };
        style.tab_bar.corner_radius = CornerRadius {
            nw: 6,
            ne: 6,
            ..default()
        };

        style.tab.tab_body.inner_margin = Margin::same(6);
        style.tab.tab_body.bg_fill = pane_bg_color;
        style.tab.tab_body.stroke = Stroke::NONE;
        style.tab.tab_body.corner_radius = CornerRadius {
            sw: self.visuals.window_corner_radius.sw,
            se: self.visuals.window_corner_radius.se,
            ..default()
        };

        style.tab.active.bg_fill = pane_bg_color;
        style.tab.spacing = 5.0;
        style.tab.active.corner_radius = CornerRadius {
            nw: 2,
            ne: 2,
            ..default()
        };
        style.tab.active.marker_color = pane_bg_color;
        style.tab.active.outline_color = pane_bg_color;
        style.tab.active_with_kb_focus = style.tab.active.clone();

        style.tab.inactive.bg_fill = self.visuals.window_fill;
        style.tab.inactive.corner_radius = CornerRadius {
            nw: 2,
            ne: 2,
            ..default()
        };
        style.tab.inactive.marker_color = self.visuals.window_fill;
        style.tab.inactive.outline_color = self.visuals.window_fill;
        style.tab.inactive_with_kb_focus = style.tab.inactive.clone();

        style.tab.focused.bg_fill = pane_bg_color;
        style.tab.focused.corner_radius = CornerRadius {
            nw: 2,
            ne: 2,
            ..default()
        };
        style.tab.focused.marker_color = ACCENT;
        style.tab.focused.outline_color = pane_bg_color;
        style.tab.focused_with_kb_focus = style.tab.focused.clone();

        style.tab.hovered.bg_fill = pane_bg_color;
        style.tab.hovered.corner_radius = CornerRadius {
            nw: 2,
            ne: 2,
            ..default()
        };
        style.tab.hovered.marker_color = pane_bg_color;
        style.tab.hovered.outline_color = pane_bg_color;

        style
    }
}
