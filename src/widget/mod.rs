pub mod scroll;
pub mod text;

use bevy::app::{App, Plugin};

use crate::widget::{scroll::EditorScrollPlugin, text::EditorTextPlugin};

pub struct EditorWidgetPlugin;

impl Plugin for EditorWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorTextPlugin)
            .add_plugins(EditorScrollPlugin);
    }
}
