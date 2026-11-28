//! The sample game: the shared scene, a camera, and the systems that animate it.

mod scene;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(scene::SampleAssetsPlugin {
            project_root: env!("CARGO_MANIFEST_DIR"),
        })
        .add_plugins((DefaultPlugins, scene::SampleTypesPlugin))
        .add_systems(Startup, (scene::spawn_scene, spawn_camera))
        .add_systems(Update, scene::spin)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-6.0, 5.0, 9.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
}
