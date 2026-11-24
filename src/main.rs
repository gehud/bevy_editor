//! Bevy Editor: a scene editor, asset inspector and plugin host for Bevy.

mod dev;
mod editing;
mod editor_camera;
mod grid;
mod ui;

use bevy::{prelude::*, window::WindowResolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".into(),
                // Sized like the design's window; the editor draws its own title bar.
                resolution: WindowResolution::new(1660, 880),
                decorations: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            ui::EditorUiPlugin,
            editing::EditingPlugin,
            editor_camera::EditorCameraPlugin,
            grid::InfiniteGridPlugin,
            dev::DevPlugin,
        ))
        .run();
}
