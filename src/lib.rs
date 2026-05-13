pub mod panel;
pub mod window;

use bevy::{
    DefaultPlugins,
    app::{PluginGroup, PluginGroupBuilder},
    window::WindowPlugin,
};

use crate::{panel::EditorPanelPlugin, window::EditorWindowPlugin};

pub struct EditorPlugins;

impl PluginGroup for EditorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<EditorPlugins>()
            .add(EditorWindowPlugin)
            .add_group(DefaultPlugins.build().disable::<WindowPlugin>())
            .add(EditorPanelPlugin)
            .build()
    }
}
