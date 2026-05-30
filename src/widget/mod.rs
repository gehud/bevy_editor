pub mod scroll;
pub mod text;

use bevy::{
    app::{App, Plugin},
};

use crate::widget::scroll::EditorScrollPlugin;

pub struct EditorWidgetPlugin;

impl Plugin for EditorWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorScrollPlugin);
    }
}
