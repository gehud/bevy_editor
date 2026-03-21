#[cfg(feature = "editor")]
mod editor;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, Startup},
    log::info,
};

use bevy_editor::{EditorPlugin, is_play_mode};

#[derive(Default)]
struct MyRuntimePlugin;

impl Plugin for MyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, greet);
    }
}

fn greet() {
    info!("Hello, Bevy!");
}

fn main() {
    let mut app = App::new();

    if cfg!(feature = "editor") {
        if is_play_mode() {
            app.add_plugins(DefaultPlugins);
            app.add_plugins(MyRuntimePlugin::default());
        } else {
            app.add_plugins(EditorPlugin::default());
            app.add_plugins(editor::MyEditorPlugin);
        }
    } else {
        app.add_plugins(DefaultPlugins);
        app.add_plugins(MyRuntimePlugin::default());
    }

    app.run();
}
