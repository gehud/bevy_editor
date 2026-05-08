#[cfg(feature = "editor")]
mod editor;

use bevy::{
    app::{App, Plugin, Startup},
    log::info,
    prelude::bevy_main,
};
use bevy_editor::EditorApp;

#[derive(Default)]
struct MyRuntimePlugin;

impl Plugin for MyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start);
    }
}

fn start() {
    info!("The game begins");
}

#[derive(Default)]
struct MySharedPlugin;

impl Plugin for MySharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, greet);
    }
}

fn greet() {
    info!("Hello, Bevy!");
}

#[bevy_main]
fn main() {
    let mut app = EditorApp::new()
        .shared_plugin(MySharedPlugin)
        .shared_plugin(MyRuntimePlugin);

    #[cfg(feature = "editor")]
    {
        app = app.editor_plugin(editor::MyEditorPlugin);
    }

    app.run();
}
