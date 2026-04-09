mod grid;

use std::f32::consts::PI;

use bevy::{
    app::{App, First, Plugin, PostUpdate, Startup, Update},
    asset::{Assets, RenderAssetUsages, uuid::Uuid},
    camera::{
        Camera, Camera3d, ClearColorConfig, NormalizedRenderTarget, RenderTarget,
        visibility::InheritedVisibility,
    },
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::ChildOf,
        lifecycle::{Add, Despawn, Remove},
        message::{MessageReader, MessageWriter},
        observer::On,
        query::{Changed, Or, With, Without},
        reflect::ReflectComponent,
        schedule::IntoScheduleConfigs,
        system::{Commands, In, Query, Res, ResMut},
        world::Ref,
    },
    image::{BevyDefault, Image, ToExtents},
    input::{ButtonInput, keyboard::KeyCode, mouse::AccumulatedMouseMotion},
    math::{EulerRot, Quat, UVec2},
    mesh::{Mesh2d, Mesh3d},
    picking::{
        Pickable, PickingSystems,
        events::{Click, Drag, DragEnd, DragStart, Move, Pointer, PointerState},
        hover::HoverMap,
        mesh_picking::{
            MeshPickingCamera, MeshPickingPlugin, MeshPickingSettings, ray_cast::RayCastVisibility,
        },
        pointer::{Location, PointerButton, PointerId, PointerInput, PointerLocation},
    },
    reflect::Reflect,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    time::Time,
    transform::components::{GlobalTransform, Transform},
    ui::{
        ComputedNode, Node, PositionType, UiGlobalTransform, UiRect, UiSystems, percent, px,
        widget::{ImageNode, NodeImageMode, ViewportNode},
    },
    utils::default,
};
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};

use crate::{
    gizmo::{GIZMO_LAYER, GizmoCamera, InternalGizmoCamera},
    pane::{PaneApp, PaneStructure},
    panes::viewport::grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    selection::{NoSelect, Selected, Selection},
    theme::{ThemedBorderColor, palette, tokens::PANE_BG},
    window::EditorWindowCursorLock,
};

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

pub(crate) enum ViewportCameraMovement {
    Fly,
    Pan,
}

#[derive(Component)]
pub(crate) struct ViewportCamera {
    pub movement: Option<ViewportCameraMovement>,
    pub origin: Entity,
    pub rotation_sensitivity: f32,
    pub pane_sensitivity: f32,
    pub fly_speed: f32,
}

fn setup(In(pane): In<PaneStructure>, mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    let mut viewport_target = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::bevy_default(),
        RenderAssetUsages::RENDER_WORLD,
    );

    viewport_target.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let viewport_target_handle = images.add(viewport_target);

    let camera_origin = commands
        .spawn((
            InheritedVisibility::VISIBLE,
            Transform::from_xyz(6.0, 6.0, 6.0).with_rotation(Quat::from_rotation_y(PI / 4.0)),
        ))
        .id();

    let viewport_camera = commands
        .spawn((
            ChildOf(camera_origin),
            GizmoCamera,
            ViewportCamera {
                movement: None,
                origin: camera_origin,
                rotation_sensitivity: 0.005,
                pane_sensitivity: 0.015,
                fly_speed: 5.0,
            },
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(palette::GRAY_0),
                order: -2,
                ..default()
            },
            RenderTarget::Image(viewport_target_handle.clone().into()),
            Transform::from_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ))
        .id();

    commands.entity(pane.content()).with_children(|commands| {
        commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    border: UiRect::top(px(1)),
                    ..default()
                },
                ThemedBorderColor::all(PANE_BG),
                ViewportNode::new(viewport_camera),
            ))
            .observe(on_viewport_click)
            .observe(on_viewport_drag_start)
            .observe(on_viewport_drag)
            .observe(on_viewport_drag_end)
            .observe(
                move |_: On<Despawn, ViewportNode>, mut commands: Commands| {
                    commands.entity(camera_origin).despawn();
                },
            );
    });
}

fn on_viewport_click(
    mut trigger: On<Pointer<Click>>,
    nodes: Query<&ViewportNode>,
    cameras: Query<&ViewportCamera>,
) -> Result {
    let viewport = nodes.get(trigger.entity)?;
    let camera_entity = viewport.camera;
    let camera = cameras.get(camera_entity)?;

    if camera.movement.is_some() {
        trigger.propagate(false);
    }

    Ok(())
}

fn on_viewport_drag_start(
    mut trigger: On<Pointer<DragStart>>,
    nodes: Query<&ViewportNode>,
    mut cameras: Query<&mut ViewportCamera>,
    mut cursor_lock: ResMut<EditorWindowCursorLock>,
) -> Result {
    let viewport = nodes.get(trigger.entity)?;
    let mut camera = cameras.get_mut(viewport.camera)?;

    trigger.propagate(false);

    match trigger.button {
        PointerButton::Secondary => {
            camera.movement = Some(ViewportCameraMovement::Fly);
        }
        PointerButton::Middle => {
            camera.movement = Some(ViewportCameraMovement::Pan);
        }
        _ => {}
    }

    if camera.movement.is_some() {
        cursor_lock.0 = true;
    }

    Ok(())
}

fn on_viewport_drag(
    mut trigger: On<Pointer<Drag>>,
    nodes: Query<&ViewportNode>,
    mut transforms: Query<&mut Transform>,
    global_transforms: Query<&GlobalTransform>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    cameras: Query<&ViewportCamera>,
) -> Result {
    let viewport = nodes.get(trigger.entity)?;
    let camera_entity = viewport.camera;
    let camera = cameras.get(camera_entity)?;

    let Some(movement) = &camera.movement else {
        return Ok(());
    };

    trigger.propagate(false);
    let delta = mouse_motion.delta;

    let camera_global_transform = global_transforms.get(camera_entity)?;

    match movement {
        ViewportCameraMovement::Fly => {
            let mut camera_transform = transforms.get_mut(camera_entity)?;
            let mut pitch = camera_transform.rotation.to_euler(EulerRot::XYZ).0;
            pitch = (pitch - delta.y * camera.rotation_sensitivity * time.delta_secs())
                .clamp(-PI / 2.0, PI / 2.0);
            camera_transform.rotation = Quat::from_rotation_x(pitch);

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.rotate(Quat::from_rotation_y(
                -delta.x * camera.rotation_sensitivity * time.delta_secs(),
            ));
        }
        ViewportCameraMovement::Pan => {
            let left = camera_global_transform.left().as_vec3();
            let up = camera_global_transform.up().as_vec3();

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.translation +=
                left * delta.x * camera.pane_sensitivity * time.delta_secs();
            origin_transform.translation +=
                up * delta.y * camera.pane_sensitivity * time.delta_secs();
        }
    }

    Ok(())
}

fn on_viewport_drag_end(
    mut trigger: On<Pointer<DragEnd>>,
    nodes: Query<&ViewportNode>,
    mut cameras: Query<&mut ViewportCamera>,
    mut cursor_lock: ResMut<EditorWindowCursorLock>,
) -> Result {
    let mut camera = cameras.get_mut(nodes.get(trigger.entity)?.camera)?;
    camera.movement = None;
    trigger.propagate(false);
    cursor_lock.0 = false;
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

fn on_pick_mesh(
    mut trigger: On<Pointer<Click>>,
    meshes: Query<Entity, (Or<(With<Mesh2d>, With<Mesh3d>)>, Without<NoSelect>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    selections: Query<(Entity, &Selection)>,
    mut commands: Commands,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    if !meshes.contains(trigger.event_target()) {
        return Ok(());
    }

    trigger.propagate(false);

    let target = trigger.event_target();

    if !keyboard_input.pressed(KeyCode::ControlLeft) {
        for (entity, selection) in selections {
            if matches!(selection, Selection::Entity) {
                commands.entity(entity).remove::<Selected>();
            }
        }

        commands.entity(target).insert(Selected);
    } else {
        if selections.contains(target) {
            commands.entity(target).remove::<Selected>();
        } else {
            commands.entity(target).insert(Selected);
        }
    }

    Ok(())
}

fn on_entity_selected(
    trigger: On<Add, Selection>,
    selections: Query<&Selection>,
    mut commands: Commands,
) -> Result {
    let selection = selections.get(trigger.event_target())?;

    if !matches!(selection, Selection::Entity) {
        return Ok(());
    }

    commands
        .entity(trigger.event_target())
        .insert(OutlineMode::FloodFlat)
        .insert(OutlineVolume {
            visible: true,
            width: 2.0,
            colour: palette::ACCENT,
            ..default()
        });

    Ok(())
}

fn on_entity_deselected(
    trigger: On<Remove, Selection>,
    selections: Query<&Selection>,
    mut commands: Commands,
) -> Result {
    let selection = selections.get(trigger.event_target())?;

    if !matches!(selection, Selection::Entity) {
        return Ok(());
    }

    commands
        .entity(trigger.event_target())
        .remove::<OutlineVolume>();

    Ok(())
}

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_plugins(OutlinePlugin)
            .add_plugins(MeshPickingPlugin)
            .add_systems(Startup, setup_grid)
            .add_systems(Update, move_camera)
            .register_pane("Viewport", setup)
            .add_observer(on_pick_mesh)
            .add_observer(on_entity_selected)
            .add_observer(on_entity_deselected);
    }
}
