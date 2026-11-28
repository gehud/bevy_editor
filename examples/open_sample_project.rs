//! Opens `examples/sample_project` in the editor, with the sample game's scene.
//!
//! `cargo run --example open_sample_project`

#[path = "sample_project/src/scene.rs"]
#[allow(
    dead_code,
    reason = "the game's systems (`spin`) don't run in the editor"
)]
mod scene;

use bevy::prelude::*;
use bevy_editor::EditorPlugin;

fn main() {
    let project_root = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample_project");
    App::new()
        .add_plugins(scene::SampleAssetsPlugin { project_root })
        .add_plugins(EditorPlugin {
            demo_scene: false,
            project: Some(project_root.into()),
        })
        // The game's types and scene, but not its systems: the editor doesn't play the game.
        .add_plugins(scene::SampleTypesPlugin)
        .add_systems(Startup, scene::spawn_scene)
        .run();
}
