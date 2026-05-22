pub mod assets;
pub mod layout;
pub mod panel;
pub mod theme;
pub mod widget;
pub mod window;
pub mod cursor;

use bevy::{
    DefaultPlugins,
    app::{PluginGroup, PluginGroupBuilder},
    window::WindowPlugin,
};

use crate::{
    assets::EditorAssetsPlugin, cursor::EditorCursorPlugin, layout::EditorLayoutPlugin, panel::EditorPanelPlugin, theme::EditorThemePlugin, window::EditorWindowPlugin
};

pub struct EditorPlugins;

impl PluginGroup for EditorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<EditorPlugins>()
            .add(EditorWindowPlugin)
            .add_group(DefaultPlugins.build().disable::<WindowPlugin>())
            .add(EditorAssetsPlugin)
            .add(EditorPanelPlugin)
            .add(EditorThemePlugin)
            .add(EditorLayoutPlugin)
            .add(EditorCursorPlugin)
            .build()
    }
}
