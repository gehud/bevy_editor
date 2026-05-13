use bevy::{
    app::{App, AppExit, Plugin},
    prelude::bevy_main,
};
use bevy_editor::EditorPlugins;
use shared::SharedPlugin;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, _app: &mut App) {}
}

#[bevy_main]
fn main() -> AppExit {
    App::new()
        .add_plugins(EditorPlugins)
        .add_plugins(EditorPlugin)
        .add_plugins(SharedPlugin)
        .run()
}
