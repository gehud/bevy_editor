use std::ops::DerefMut;

use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::{AssetServer, Assets},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        entity_disabling::Disabled,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        message::{Message, MessageReader, MessageWriter},
        name::Name,
        observer::On,
        query::With,
        system::{Commands, In, Query, Res, ResMut, Single},
        world::{Ref, World},
    },
    light::PointLight,
    log::info,
    math::{
        Quat,
        primitives::{Circle, Cuboid},
    },
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer},
    },
    scene::{InstanceId, Scene, SceneInstance, SceneLoader, SceneRoot, SceneSpawner},
    text::TextLayout,
    transform::components::Transform,
    ui::{
        AlignItems, FlexDirection, JustifyContent, Node, Overflow, PositionType, UiRect, auto,
        percent, px,
        widget::{ImageNode, Text},
    },
    utils::default,
};

use crate::{
    pane::{PaneApp, PaneStructure},
    theme::{
        ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeTextFont, ThemeTextFontSize,
        constants::size::GAP,
        tokens::{BORDER, BUTTON_BG, PANE_BG, TEXT_MAIN},
    },
    widget::ScrollArea,
};

pub struct SceneTreePlugin;

impl Plugin for SceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_scene)
            .add_systems(Update, redraw_scene_tree)
            .register_pane("Scene Tree", setup);
    }
}

#[derive(Component)]
struct InspectedScene {
    instance: InstanceId,
    outdated: bool,
}

fn spawn_scene(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut scene_spawner: ResMut<SceneSpawner>,
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

    let root = commands.spawn(Name::new("Unnamed")).id();

    let instance = scene_spawner.spawn_as_child(scene_handle, root);

    commands.entity(root).insert((
        Visibility::Visible,
        Transform::IDENTITY,
        InspectedScene {
            instance,
            outdated: true,
        },
    ));
}

#[derive(Component)]
struct SceneTreeRoot;

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {
    commands.entity(pane.content()).with_children(|commands| {
        let area = commands
            .spawn(Node {
                width: percent(100),
                height: percent(100),
                margin: UiRect::all(px(6)),
                ..default()
            })
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
                    position_type: PositionType::Absolute,
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .id();

        commands.commands_mut().entity(area).insert(ScrollArea {
            target,
            vertical: true,
            ..default()
        });
    });
}

fn redraw_scene_tree(
    mut inspected_scene: Single<(Entity, &mut InspectedScene)>,
    roots: Query<(Entity, Ref<SceneTreeRoot>)>,
    scene_spawner: Res<SceneSpawner>,
    asset_server: Res<AssetServer>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) -> Result {
    let (origin, inspected_scene) = inspected_scene.deref_mut();

    if !scene_spawner.instance_is_ready(inspected_scene.instance) {
        return Ok(());
    }

    for (root, scene_root) in roots {
        if !(scene_root.is_added() || inspected_scene.outdated) {
            continue;
        }

        commands.entity(root).despawn_children();
        populate_scene_tree(
            &mut commands,
            &asset_server,
            &children,
            &names,
            root,
            *origin,
        )?;
    }

    inspected_scene.outdated = false;

    Ok(())
}

#[derive(Component)]
struct EntityViewHeader {
    drop: bool,
    content: Entity,
}

fn populate_scene_tree(
    commands: &mut Commands,
    asset_server: &AssetServer,
    children: &Query<&Children>,
    names: &Query<&Name>,
    root: Entity,
    entity: Entity,
) -> Result<Entity> {
    let container = commands
        .spawn((
            ChildOf(root),
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    let header = commands
        .spawn((
            ChildOf(container),
            Node {
                width: percent(100),
                height: px(21),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Start,
                padding: UiRect::all(px(3)),
                column_gap: px(4),
                ..default()
            },
            ThemeBackgroundColor(PANE_BG),
        ))
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.event_target())
                .insert(ThemeBackgroundColor(BUTTON_BG));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.event_target())
                .insert(ThemeBackgroundColor(PANE_BG));
        })
        .with_children(|commands| {
            commands
                .spawn((
                    Pickable {
                        should_block_lower: false,
                        ..default()
                    },
                    Node {
                        width: px(15),
                        height: px(15),
                        ..default()
                    },
                    ImageNode::new(
                        asset_server.load("embedded://bevy_editor/icons/chevron_down.png"),
                    ),
                ))
                .observe(
                    |trigger: On<Pointer<Click>>,
                     mut headers: Query<&mut EntityViewHeader>,
                     asset_server: Res<AssetServer>,
                     parents: Query<&ChildOf>,
                     mut nodes: Query<&mut Node>,
                     mut commands: Commands|
                     -> Result {
                        let mut header =
                            headers.get_mut(parents.get(trigger.event_target())?.parent())?;
                        header.drop = !header.drop;

                        commands
                            .entity(trigger.event_target())
                            .insert(ImageNode::new(asset_server.load(if header.drop {
                                "embedded://bevy_editor/icons/chevron_down.png"
                            } else {
                                "embedded://bevy_editor/icons/chevron_right.png"
                            })));

                        let mut content_node = nodes.get_mut(header.content)?;

                        if header.drop {
                            content_node.height = auto();
                            commands
                                .entity(header.content)
                                .insert(Visibility::Inherited);
                        } else {
                            content_node.height = px(0);
                            commands.entity(header.content).insert(Visibility::Hidden);
                        }

                        Ok(())
                    },
                );

            commands.spawn((
                Pickable::IGNORE,
                Node {
                    width: px(15),
                    height: px(15),
                    ..default()
                },
                ImageNode::new(asset_server.load("embedded://bevy_editor/icons/box.png")),
            ));

            let name = names
                .get(entity)
                .map(|name| name.as_str())
                .unwrap_or_else(|_| "Entity");

            commands
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        overflow: Overflow::hidden(),
                        ..default()
                    },
                ))
                .with_children(|commands| {
                    commands.spawn((
                        Pickable::IGNORE,
                        Text::new(name),
                        TextLayout::new_with_no_wrap(),
                        ThemeTextFont(TEXT_MAIN),
                        ThemeTextColor(TEXT_MAIN),
                        ThemeTextFontSize(TEXT_MAIN),
                    ));
                });
        })
        .id();

    let content_origin = commands
        .spawn((
            ChildOf(container),
            Node {
                width: percent(100),
                ..default()
            },
        ))
        .id();

    let content = commands
        .spawn((
            ChildOf(content_origin),
            Node {
                width: percent(100),
                margin: UiRect::left(px(8)),
                flex_direction: FlexDirection::Column,
                border: UiRect::left(px(3)),
                ..default()
            },
            ThemeBorderColor::all(BORDER),
        ))
        .id();

    commands.entity(header).insert(EntityViewHeader {
        drop: true,
        content,
    });

    if let Ok(nested) = children.get(entity) {
        for child in nested {
            populate_scene_tree(commands, asset_server, children, names, content, *child)?;
        }
    }

    Ok(header)
}
