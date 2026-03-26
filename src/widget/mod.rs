use bevy::{
    app::{Plugin, PluginGroup, PluginGroupBuilder},
    input_focus::{InputDispatchPlugin, directional_navigation::DirectionalNavigationPlugin},
    ui_render::UiMaterialPlugin,
    ui_widgets::UiWidgetsPlugins,
    window::SystemCursorIcon,
};

mod alpha_pattern;
mod button;
mod checkbox;
mod color_plane;
mod color_slider;
mod color_swatch;
mod context_menu;
mod cursor;
mod menu;
mod radio;
mod scroll;
mod slider;
mod text;
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
pub use menu::*;
pub use radio::*;
pub use scroll::*;
pub use slider::*;
pub use text::*;
pub use toggle_switch::*;
pub use virtual_keyboard::*;

pub struct EditorWidgetPlugin;

impl Plugin for EditorWidgetPlugin {
    fn build(&self, app: &mut bevy::app::App) {
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
            ScrollPlugin,
            MenuBarPlugin,
            UiMaterialPlugin::<AlphaPatternMaterial>::default(),
        ));

        app.insert_resource(DefaultCursor(EntityCursor::System(
            SystemCursorIcon::Default,
        )));

        app.init_resource::<AlphaPatternResource>();
    }
}

pub struct EditorWidgetPlugins;

impl PluginGroup for EditorWidgetPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(UiWidgetsPlugins)
            .add(InputDispatchPlugin)
            .add(DirectionalNavigationPlugin)
            .add(EditorWidgetPlugin)
    }
}
