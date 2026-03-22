use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::Assets,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        message::{Message, MessageReader},
        name::Name,
        query::With,
        system::{Commands, In, Query, ResMut, Single},
        world::World,
    },
    light::PointLight,
    math::{
        Quat,
        primitives::{Circle, Cuboid},
    },
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    picking::Pickable,
    scene::{Scene, SceneInstance, SceneRoot},
    transform::components::Transform,
    ui::{Node, Overflow, percent, widget::Text},
    utils::default,
};

use crate::{
    pane::{PaneStructure, RegisterPane},
    theme::{ThemeTextColor, ThemeTextFont, ThemeTextFontSize, tokens::TEXT_MAIN},
    widget::ScrollArea,
};

pub struct SceneTreePanePlugin;

impl Plugin for SceneTreePanePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RedrawScene>()
            .add_systems(Startup, spawn_scene)
            .add_systems(Update, redraw_scene)
            .register_pane("Scene Tree", setup);
    }
}

#[derive(Component)]
struct InspectedScene;

fn spawn_scene(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut commands: Commands,
) {
    let mut scene = Scene::new(World::new());

    scene.world.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    scene.world.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    scene.world.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    let scene_handle = scenes.add(scene);

    commands.spawn((
        InspectedScene,
        Name::new("Unnamed"),
        SceneRoot(scene_handle),
    ));
}

#[derive(Component)]
struct SceneTreeRoot;

fn setup(In(pane_structure): In<PaneStructure>, mut commands: Commands) {
    commands
        .entity(pane_structure.content)
        .with_children(|commands| {
            let area = commands
                .spawn((Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },))
                .id();

            let target = commands
                .commands_mut()
                .spawn((
                    ChildOf(area),
                    SceneTreeRoot,
                    Pickable::IGNORE,
                    Node {
                        width: percent(100),
                        height: percent(100),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .id();

            commands.commands_mut().entity(area).insert(ScrollArea {
                target,
                vertical: true,
                ..default()
            });

            commands.commands_mut().write_message(RedrawScene);
        });
}

#[derive(Message)]
struct RedrawScene;

fn redraw_scene(
    mut requests: MessageReader<RedrawScene>,
    inspected_scene: Single<Entity, With<InspectedScene>>,
    roots: Query<Entity, With<SceneTreeRoot>>,
    mut commands: Commands,
) -> Result {
    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    for root in roots {
        commands.run_system_cached_with(populate_scene, (root, *inspected_scene));
    }

    Ok(())
}

fn populate_scene(In((root, entity)): In<(Entity, Entity)>) {}
