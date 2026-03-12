use bevy::{app::{App, Plugin, Startup}, log::info};
use bevy_editor::EditorPlugin;

#[derive(Default)]
struct SimplePlugin;

impl Plugin for SimplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, || {
            info!("Hello, world!");
        });
    }
}

fn main() {
    App::new()
        .add_plugins(EditorPlugin::default())
        .add_plugins(SimplePlugin::default())
        .run();
}
