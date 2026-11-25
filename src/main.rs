//! The editor binary. `cargo run -- <project folder>` opens a project.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_editor::EditorPlugin;

fn main() {
    let project = std::env::args_os().nth(1).map(PathBuf::from);
    App::new()
        .add_plugins(EditorPlugin {
            project,
            ..default()
        })
        .run();
}
