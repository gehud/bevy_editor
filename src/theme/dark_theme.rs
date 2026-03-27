use bevy::{
    asset::AssetServer,
    color::{Alpha, Luminance},
    ecs::world::{FromWorld, World},
    platform::collections::HashMap,
};

use crate::theme::{
    EditorTheme,
    constants::fonts::{BOLD, MONO, REGULAR},
    palette, tokens,
};

impl FromWorld for EditorTheme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            color: HashMap::from([
                (tokens::WINDOW_BG, palette::GRAY_0),
                (tokens::BORDER, palette::WARM_GRAY_1),
                // Pane
                (tokens::PANE_BG, palette::GRAY_1),
                (tokens::PANE_TAB_ACTIVE, palette::ACCENT),
                // Text
                (tokens::TEXT_MAIN, palette::WHITE),
                (tokens::TEXT_HEADING, palette::WHITE),
                (tokens::TEXT_DIM, palette::WHITE.with_alpha(0.5)),
                // Button
                (tokens::BUTTON_BG, palette::GRAY_2),
                (tokens::BUTTON_BG_HOVER, palette::GRAY_2.lighter(0.05)),
                (tokens::BUTTON_BG_PRESSED, palette::GRAY_2.lighter(0.1)),
                (tokens::BUTTON_BG_DISABLED, palette::GRAY_2),
                (tokens::BUTTON_PRIMARY_BG, palette::ACCENT),
                (
                    tokens::BUTTON_PRIMARY_BG_HOVER,
                    palette::ACCENT.lighter(0.05),
                ),
                (
                    tokens::BUTTON_PRIMARY_BG_PRESSED,
                    palette::ACCENT.lighter(0.1),
                ),
                (tokens::BUTTON_PRIMARY_BG_DISABLED, palette::GRAY_2),
                (tokens::BUTTON_TEXT, palette::WHITE),
                (tokens::BUTTON_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
                (tokens::BUTTON_PRIMARY_TEXT, palette::WHITE),
                (
                    tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                    palette::WHITE.with_alpha(0.5),
                ),
                // Slider
                (tokens::SLIDER_BG, palette::GRAY_1),
                (tokens::SLIDER_BAR, palette::ACCENT),
                (tokens::SLIDER_BAR_DISABLED, palette::GRAY_2),
                (tokens::SLIDER_TEXT, palette::WHITE),
                (tokens::SLIDER_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
                // Checkbox
                (tokens::CHECKBOX_BG, palette::GRAY_2),
                (tokens::CHECKBOX_BG_CHECKED, palette::ACCENT),
                (
                    tokens::CHECKBOX_BG_DISABLED,
                    palette::GRAY_1.with_alpha(0.5),
                ),
                (
                    tokens::CHECKBOX_BG_CHECKED_DISABLED,
                    palette::GRAY_2.with_alpha(0.5),
                ),
                (tokens::CHECKBOX_BORDER, palette::GRAY_2),
                (tokens::CHECKBOX_BORDER_HOVER, palette::GRAY_2.lighter(0.1)),
                (
                    tokens::CHECKBOX_BORDER_DISABLED,
                    palette::GRAY_2.with_alpha(0.5),
                ),
                (tokens::CHECKBOX_MARK, palette::WHITE),
                (tokens::CHECKBOX_MARK_DISABLED, palette::LIGHT_GRAY_2),
                (tokens::CHECKBOX_TEXT, palette::LIGHT_GRAY_1),
                (
                    tokens::CHECKBOX_TEXT_DISABLED,
                    palette::LIGHT_GRAY_1.with_alpha(0.5),
                ),
                // Radio
                (tokens::RADIO_BORDER, palette::GRAY_2),
                (tokens::RADIO_BORDER_HOVER, palette::GRAY_2.lighter(0.1)),
                (
                    tokens::RADIO_BORDER_DISABLED,
                    palette::GRAY_2.with_alpha(0.5),
                ),
                (tokens::RADIO_MARK, palette::ACCENT),
                (tokens::RADIO_MARK_DISABLED, palette::ACCENT.with_alpha(0.5)),
                (tokens::RADIO_TEXT, palette::LIGHT_GRAY_1),
                (
                    tokens::RADIO_TEXT_DISABLED,
                    palette::LIGHT_GRAY_1.with_alpha(0.5),
                ),
                // Toggle Switch
                (tokens::SWITCH_BG, palette::GRAY_2),
                (tokens::SWITCH_BG_CHECKED, palette::ACCENT),
                (tokens::SWITCH_BG_DISABLED, palette::GRAY_1.with_alpha(0.5)),
                (
                    tokens::SWITCH_BG_CHECKED_DISABLED,
                    palette::GRAY_2.with_alpha(0.5),
                ),
                (tokens::SWITCH_BORDER, palette::GRAY_2),
                (tokens::SWITCH_BORDER_HOVER, palette::GRAY_2.lighter(0.1)),
                (
                    tokens::SWITCH_BORDER_DISABLED,
                    palette::GRAY_2.with_alpha(0.5),
                ),
                (tokens::SWITCH_SLIDE, palette::LIGHT_GRAY_2),
                (
                    tokens::SWITCH_SLIDE_DISABLED,
                    palette::LIGHT_GRAY_2.with_alpha(0.3),
                ),
                (tokens::COLOR_PLANE_BG, palette::GRAY_1),
            ]),
            fonts: HashMap::from([
                (tokens::TEXT_MAIN, asset_server.load(REGULAR)),
                (tokens::TEXT_HEADING, asset_server.load(BOLD)),
                (tokens::TEXT_MAIN, asset_server.load(REGULAR)),
                (tokens::TEXT_DIM, asset_server.load(REGULAR)),
                (tokens::BUTTON_TEXT, asset_server.load(REGULAR)),
                (tokens::BUTTON_TEXT, asset_server.load(REGULAR)),
                (tokens::SLIDER_TEXT, asset_server.load(MONO)),
            ]),
            font_sizes: HashMap::from([
                (tokens::TEXT_MAIN, 12.0),
                (tokens::TEXT_HEADING, 13.0),
                (tokens::TEXT_DISABLED, 12.0),
                (tokens::TEXT_DIM, 10.0),
                (tokens::BUTTON_TEXT, 12.0),
                (tokens::RADIO_TEXT, 12.0),
                (tokens::SLIDER_TEXT, 12.0),
            ]),
        }
    }
}
