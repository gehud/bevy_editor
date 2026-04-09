use bevy::{
    app::{App, Plugin, Startup},
    log::info,
};

pub struct MyEditorPlugin;

impl Plugin for MyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, greet);
    }
}

fn greet() {
    info!("Hello, Bevy Editor!");
}
