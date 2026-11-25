//! A large, deep hierarchy for testing the Scene Tree: scrolling, filtering, multi-select,
//! drag-and-drop and rebuild cost. `LARGE_SCENE_ROOTS` sets the number of roots (default 20;
//! each root has 5 children with 5 children each, so 31 entities per root).
//!
//! `cargo run --example large_scene`

use bevy::prelude::*;
use bevy_editor::EditorPlugin;

const CHILDREN: usize = 5;

fn main() {
    App::new()
        .add_plugins(EditorPlugin {
            demo_scene: false,
            ..default()
        })
        .add_systems(Startup, spawn)
        .run();
}

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let roots: usize = std::env::var("LARGE_SCENE_ROOTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);
    let cube = meshes.add(Cuboid::from_length(0.4));
    let material = materials.add(Color::srgb(0.6, 0.7, 0.9));

    commands.spawn((
        Name::new("Light"),
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    for r in 0..roots {
        let x = (r % 10) as f32 * 3.0 - 13.5;
        let z = (r / 10) as f32 * 3.0;
        let root = commands
            .spawn((
                Name::new(format!("Group {r}")),
                Transform::from_xyz(x, 0.0, z),
                Visibility::default(),
            ))
            .id();
        for c in 0..CHILDREN {
            let child = commands
                .spawn((
                    Name::new(format!("Group {r} / Item {c}")),
                    Transform::from_xyz(0.0, c as f32 * 0.5, 0.0),
                    Visibility::default(),
                    ChildOf(root),
                ))
                .id();
            for g in 0..CHILDREN {
                commands.spawn((
                    Name::new(format!("Group {r} / Item {c} / Cube {g}")),
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(g as f32 * 0.5 - 1.0, 0.0, 0.0),
                    ChildOf(child),
                ));
            }
        }
    }
}
