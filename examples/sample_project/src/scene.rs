//! The sample game's components and scene.
//!
//! The editor's `open_sample_project` example includes this file with `#[path]`, so it must only
//! use `bevy` APIs.

use std::path::Path;

use bevy::{
    asset::io::{AssetSourceBuilder, embedded::EmbeddedAssetRegistry},
    gltf::GltfAssetLabel,
    prelude::*,
};

/// Registers the same disk and embedded assets in the game and the editor example.
pub struct SampleAssetsPlugin {
    pub project_root: &'static str,
}

impl Plugin for SampleAssetsPlugin {
    fn build(&self, app: &mut App) {
        // Leave the editor's default asset source available for its own icons and shaders.
        let assets = format!("{}/assets", self.project_root);
        app.register_asset_source(
            "sample_project",
            AssetSourceBuilder::platform_default(&assets, None),
        );
    }

    fn finish(&self, app: &mut App) {
        // Use the same embedded paths when this module is compiled into the editor example.
        let source = Path::new(self.project_root).join("src/embedded");
        let registry = app.world().resource::<EmbeddedAssetRegistry>();
        let assets: [(&str, &'static [u8]); 2] = [
            (
                "textures/team-banner.png",
                include_bytes!("embedded/textures/team-banner.png"),
            ),
            (
                "audio/notification.ogg",
                include_bytes!("embedded/audio/notification.ogg"),
            ),
        ];
        for (file, bytes) in assets {
            registry.insert_asset(
                source.join(file),
                &Path::new("sample_project").join(file),
                bytes,
            );
        }
    }
}

/// Registers the sample's reflected types (so editors can show and edit them).
pub struct SampleTypesPlugin;

impl Plugin for SampleTypesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Spinner>()
            .register_type::<Health>()
            .register_type::<Unit>();
    }
}

/// Rotates the entity around `axis`.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Spinner {
    /// Radians per second.
    pub speed: f32,
    pub axis: Vec3,
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Health {
    pub current: u32,
    pub max: u32,
    pub regenerating: bool,
}

#[derive(Reflect, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Team {
    #[default]
    Neutral,
    Red,
    Blue,
}

#[derive(Reflect, Clone, Default)]
pub struct Stats {
    pub speed: f32,
    pub armor: i32,
    pub attack_range: f64,
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct Unit {
    pub display_name: String,
    pub team: Team,
    pub stats: Stats,
}

/// Turns every [`Spinner`].
pub fn spin(time: Res<Time>, mut spinners: Query<(&Spinner, &mut Transform)>) {
    for (spinner, mut transform) in &mut spinners {
        if let Ok(axis) = Dir3::new(spinner.axis) {
            transform.rotate_axis(axis, spinner.speed * time.delta_secs());
        }
    }
}

/// A light, a textured ground, a spinning turret, two units, and a glTF dragon.
pub fn spawn_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut material = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.7,
            ..default()
        })
    };
    let metal = material(Color::srgb(0.35, 0.38, 0.42));
    let red = material(Color::srgb(0.8, 0.25, 0.2));
    let stone = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("sample_project://textures/stone-checker.png")),
        perceptual_roughness: 0.85,
        ..default()
    });
    let blue = materials.add(StandardMaterial {
        base_color_texture: Some(
            asset_server.load("embedded://sample_project/textures/team-banner.png"),
        ),
        perceptual_roughness: 0.7,
        ..default()
    });

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(stone),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Turret"),
        Mesh3d(meshes.add(Cylinder::new(0.6, 0.8))),
        MeshMaterial3d(metal.clone()),
        Transform::from_xyz(0.0, 0.4, 0.0),
        Spinner {
            speed: 0.8,
            axis: Vec3::Y,
        },
        children![(
            Name::new("Barrel"),
            Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 1.4))),
            MeshMaterial3d(metal),
            Transform::from_xyz(0.0, 0.25, 0.8),
        )],
    ));

    let capsule = meshes.add(Capsule3d::new(0.35, 0.8));
    let unit = |name: &str, team: Team, x: f32, material, attack_range| {
        (
            Name::new(name.to_owned()),
            Mesh3d(capsule.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.75, 2.5),
            Health {
                current: 80,
                max: 100,
                regenerating: team == Team::Blue,
            },
            Unit {
                display_name: name.to_owned(),
                team,
                stats: Stats {
                    speed: 3.5,
                    armor: 2,
                    attack_range,
                },
            },
        )
    };
    commands.spawn(unit("Red Knight", Team::Red, -2.0, red, 1.5));
    commands.spawn(unit("Blue Archer", Team::Blue, 2.0, blue, 8.0));
    commands.spawn((
        Name::new("Low-poly Dragon"),
        WorldAssetRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("sample_project://models/low-poly-dragon.gltf"),
        )),
        Transform::from_xyz(0.0, 0.0, -2.8),
    ));
}
