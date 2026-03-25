mod asset_browser;
mod properties;
mod scene_tree;
mod viewport;

pub use asset_browser::*;
pub use properties::*;
pub use scene_tree::*;
pub use viewport::*;

use bevy::app::{PluginGroup, PluginGroupBuilder};

use crate::pane::EditorPanePlugin;

pub struct EditorPanePlugins;

impl PluginGroup for EditorPanePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(EditorPanePlugin)
            .add(AssetBrowserPlugin)
            .add(PropertiesPlugin)
            .add(SceneTreePlugin)
            .add(ViewportPlugin)
    }
}
