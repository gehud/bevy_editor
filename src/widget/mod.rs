//! `bevy_editor::widget` is a collection of styled and themed widgets for building editors and
//! inspectors.
//!
//! The aesthetic choices made here are designed with a future Bevy Editor in mind,
//! but this crate is deliberately exposed to the public to allow the broader ecosystem to easily create
//! tooling for themselves and others that fits cohesively together.
//!
//! While it may be tempting to use this crate for your game's UI, it's deliberately not intended for that.
//! We've opted for a clean, functional style, and prioritized consistency over customization.
//! That said, if you like what you see, it can be a helpful learning tool.
//! Consider copying this code into your own project,
//! and refining the styles and abstractions provided to meet your needs.
//!
//! ## Warning: Experimental!
//! All that said, this crate is still experimental and unfinished!
//! It will change in breaking ways, and there will be both bugs and limitations.
//!
//! Please report issues, submit fixes and propose changes.
//! Thanks for stress-testing; let's build something better together.

use bevy::app::{Plugin, PluginGroup, PluginGroupBuilder, PostUpdate, PropagateSet};
use bevy::asset::embedded_asset;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::input_focus::{InputDispatchPlugin, tab_navigation::TabNavigationPlugin};
use bevy::text::TextFont;
use bevy::ui::UiSystems;
use bevy::ui_render::UiMaterialPlugin;
use bevy::ui_widgets::UiWidgetsPlugins;

mod alpha_pattern;
mod button;
mod checkbox;
mod color_plane;
mod color_slider;
mod color_swatch;
mod context_menu;
mod cursor;
mod radio;
mod slider;
mod toggle_switch;
mod virtual_keyboard;

pub use alpha_pattern::*;
pub use button::*;
pub use checkbox::*;
pub use color_plane::*;
pub use color_slider::*;
pub use color_swatch::*;
pub use context_menu::*;
pub use cursor::*;
pub use radio::*;
pub use slider::*;
pub use toggle_switch::*;
pub use virtual_keyboard::*;

use crate::widget::alpha_pattern::{
    AlphaPatternMaterial, AlphaPatternPlugin, AlphaPatternResource,
};

/// Plugin which installs observers and systems for editor themes, cursors, and all controls.
pub struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        // Embedded shader
        embedded_asset!(
            app,
            "src/widget",
            "assets/widget/shaders/alpha_pattern.wgsl"
        );
        embedded_asset!(app, "src/widget", "assets/widget/shaders/color_plane.wgsl");

        embedded_asset!(app, "src/widget", "assets/widget/icons/check.png");
        embedded_asset!(app, "src/widget", "assets/widget/icons/chevron_right.png");

        app.add_plugins((
            AlphaPatternPlugin,
            ButtonPlugin,
            CheckboxPlugin,
            ColorPlanePlugin,
            ColorSliderPlugin,
            ColorSwatchPlugin,
            RadioPlugin,
            SliderPlugin,
            ToggleSwitchPlugin,
            CursorIconPlugin,
            ContextMenuPlugin,
            UiMaterialPlugin::<AlphaPatternMaterial>::default(),
        ));

        // This needs to run in UiSystems::Propagate so the fonts are up-to-date for `measure_text_system`
        // and `detect_text_needs_rerender` in UiSystems::Content
        app.configure_sets(
            PostUpdate,
            PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
        );

        app.insert_resource(DefaultCursor(EntityCursor::System(
            bevy::window::SystemCursorIcon::Default,
        )));

        app.init_resource::<AlphaPatternResource>();
    }
}

/// A plugin group that adds all dependencies for bevy_editor::widget
pub struct WidgetPlugins;

impl PluginGroup for WidgetPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(UiWidgetsPlugins)
            .add(InputDispatchPlugin)
            .add(TabNavigationPlugin)
            .add(WidgetPlugin)
    }
}
