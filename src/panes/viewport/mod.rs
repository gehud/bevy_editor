mod grid;

use std::f32::consts::PI;

use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::{Assets, RenderAssetUsages},
    camera::{Camera, Camera3d, ClearColorConfig, RenderTarget, visibility::InheritedVisibility},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::ChildOf,
        lifecycle::Despawn,
        observer::On,
        system::{Commands, In, Query, Res, ResMut},
    },
    image::{BevyDefault, Image},
    input::{ButtonInput, keyboard::KeyCode},
    math::{EulerRot, Quat},
    picking::{
        events::{Drag, DragEnd, DragStart, Pointer},
        pointer::PointerButton,
    },
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    time::Time,
    transform::components::{GlobalTransform, Transform},
    ui::{Node, PositionType, UiRect, percent, px, widget::ViewportNode},
    utils::default,
};

use crate::{
    pane::{Pane, PaneApp},
    panes::viewport::grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    theme::palette,
};

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_systems(Startup, setup_grid)
            .add_systems(Update, move_camera)
            .register_pane("Viewport", setup);
    }
}

fn setup_grid(mut commands: Commands) {
    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            x_axis_color: palette::X_AXIS,
            z_axis_color: palette::Z_AXIS,
            major_line_color: palette::WARM_GRAY_1,
            minor_line_color: palette::GRAY_2,
            ..default()
        },
    ));
}

enum ViewportCameraMovement {
    Fly,
    Pan,
}

#[derive(Component)]
struct ViewportCamera {
    movement: Option<ViewportCameraMovement>,
    origin: Entity,
    rotation_sensitivity: f32,
    pane_sensitivity: f32,
    fly_speed: f32,
}

fn setup(
    In(pane_structure): In<Pane>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let mut image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::bevy_default(),
        RenderAssetUsages::RENDER_WORLD,
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image = images.add(image);

    let camera_origin = commands
        .spawn((
            InheritedVisibility::VISIBLE,
            Transform::from_xyz(0.0, 4.0, 6.0),
        ))
        .id();

    let camera = commands
        .spawn((
            ChildOf(camera_origin),
            ViewportCamera {
                movement: None,
                origin: camera_origin,
                rotation_sensitivity: 0.1,
                pane_sensitivity: 0.3,
                fly_speed: 5.0,
            },
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(palette::GRAY_0),
                order: -1,
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Transform::from_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ))
        .id();

    commands
        .entity(pane_structure.content)
        .with_children(|commands| {
            commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    ViewportNode::new(camera),
                ))
                .observe(on_viewport_drag_start)
                .observe(on_viewport_drag)
                .observe(on_viewport_drag_end)
                .observe(
                    move |_: On<Despawn, ViewportNode>, mut commands: Commands| {
                        commands.entity(camera).despawn();
                    },
                );
        });
}

fn on_viewport_drag_start(
    trigger: On<Pointer<DragStart>>,
    nodes: Query<&ViewportNode>,
    mut cameras: Query<&mut ViewportCamera>,
) -> Result {
    let mut camera = cameras.get_mut(nodes.get(trigger.entity)?.camera)?;

    match trigger.button {
        PointerButton::Secondary => {
            camera.movement = Some(ViewportCameraMovement::Fly);
        }
        PointerButton::Middle => {
            camera.movement = Some(ViewportCameraMovement::Pan);
        }
        _ => {}
    }

    Ok(())
}

fn on_viewport_drag(
    trigger: On<Pointer<Drag>>,
    nodes: Query<&ViewportNode>,
    mut transforms: Query<&mut Transform>,
    global_transforms: Query<&GlobalTransform>,
    time: Res<Time>,
    cameras: Query<&ViewportCamera>,
) -> Result {
    let camera_entity = nodes.get(trigger.entity)?.camera;
    let camera = cameras.get(camera_entity)?;

    let Some(movement) = &camera.movement else {
        return Ok(());
    };

    let camera_global_transform = global_transforms.get(camera_entity)?;

    match movement {
        ViewportCameraMovement::Fly => {
            let mut camera_transform = transforms.get_mut(camera_entity)?;
            let mut pitch = camera_transform.rotation.to_euler(EulerRot::XYZ).0;
            pitch = (pitch - trigger.delta.y * camera.rotation_sensitivity * time.delta_secs())
                .clamp(-PI / 2.0, PI / 2.0);
            camera_transform.rotation = Quat::from_rotation_x(pitch);

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.rotate(Quat::from_rotation_y(
                -trigger.delta.x * camera.rotation_sensitivity * time.delta_secs(),
            ));
        }
        ViewportCameraMovement::Pan => {
            let left = camera_global_transform.left().as_vec3();
            let up = camera_global_transform.up().as_vec3();

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.translation +=
                left * trigger.delta.x * camera.pane_sensitivity * time.delta_secs();
            origin_transform.translation +=
                up * trigger.delta.y * camera.pane_sensitivity * time.delta_secs();
        }
    }

    Ok(())
}

fn on_viewport_drag_end(
    trigger: On<Pointer<DragEnd>>,
    nodes: Query<&ViewportNode>,
    mut cameras: Query<&mut ViewportCamera>,
) -> Result {
    let mut camera = cameras.get_mut(nodes.get(trigger.entity)?.camera)?;
    camera.movement = None;
    Ok(())
}

fn move_camera(
    cameras: Query<(Entity, &ViewportCamera)>,
    global_transforms: Query<&GlobalTransform>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut transforms: Query<&mut Transform>,
) -> Result {
    for (camera_entity, camera) in cameras {
        let Some(movement) = &camera.movement else {
            continue;
        };

        if !matches!(movement, ViewportCameraMovement::Fly) {
            continue;
        }

        let camera_global_transform = global_transforms.get(camera_entity)?;
        let mut origin_transform = transforms.get_mut(camera.origin)?;

        let forward = camera_global_transform.forward().as_vec3();
        let right = camera_global_transform.right().as_vec3();
        let up = camera_global_transform.up().as_vec3();

        if keyboard_input.pressed(KeyCode::KeyW) {
            origin_transform.translation += forward * camera.fly_speed * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyS) {
            origin_transform.translation -= forward * camera.fly_speed * time.delta_secs();
        }

        if keyboard_input.pressed(KeyCode::KeyD) {
            origin_transform.translation += right * camera.fly_speed * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyA) {
            origin_transform.translation -= right * camera.fly_speed * time.delta_secs();
        }

        if keyboard_input.pressed(KeyCode::KeyE) {
            origin_transform.translation += up * camera.fly_speed * time.delta_secs();
        } else if keyboard_input.pressed(KeyCode::KeyQ) {
            origin_transform.translation -= up * camera.fly_speed * time.delta_secs();
        }
    }

    Ok(())
}
