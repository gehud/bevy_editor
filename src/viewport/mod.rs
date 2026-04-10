mod grid;

use std::{f32::consts::PI, ops::DerefMut};

use bevy::{
    app::{App, First, Plugin, PostUpdate, PreUpdate, Startup, Update},
    asset::{Assets, Handle, RenderAssetUsages, uuid::Uuid},
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
        lifecycle::{Add, Remove},
        message::{MessageReader, MessageWriter},
        observer::On,
        query::{Or, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, In, Local, Query, Res, ResMut, Single},
        world::World,
    },
    image::{BevyDefault, Image},
    input::{ButtonInput, keyboard::KeyCode, mouse::AccumulatedMouseMotion},
    log::info,
    math::{EulerRot, Quat, Rect, Vec2},
    mesh::{Mesh2d, Mesh3d},
    picking::{
        Pickable, PickingSystems,
        events::{Click, Drag, DragEnd, DragStart, Move, Pointer, PointerState},
        hover::HoverMap,
        mesh_picking::MeshPickingPlugin,
        pointer::{Location, PointerButton, PointerId, PointerInput, PointerLocation},
    },
    platform::collections::HashMap,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    time::Time,
    transform::components::{GlobalTransform, Transform},
    ui::{Node, UiTargetCamera, percent, widget::ViewportNode},
    utils::default,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, EguiUserTextures};
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use egui::{Sense, TextureId, Ui, load::SizedTexture};

use crate::{
    cursor::CursorLock,
    pane::{Pane, RegisterPane},
    selection::{Deselect, Select, SelectionMap},
    viewport::grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
};

pub struct ViewportPane;

impl Pane for ViewportPane {
    fn name(&self) -> &str {
        "Viewport"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let texture_id = world.run_system_cached(get_viewport_texture_id)?;

        let size = ui.available_size();
        let response = ui
            .image(SizedTexture::new(texture_id, size))
            .interact(Sense::click_and_drag());

        let viewport = world
            .query_filtered::<Entity, With<ViewportCamera>>()
            .single(world)?;
        world.entity_mut(viewport).insert(ViewportPicking {
            min: Vec2::new(response.rect.min.x, response.rect.min.y),
            interact_pos: response
                .interact_pointer_pos()
                .map(|position| Vec2::new(position.x, position.y)),
            hover_pos: response
                .hover_pos()
                .map(|position| Vec2::new(position.x, position.y)),
        });

        world.run_system_cached_with(resize_viewport, Vec2::new(size.x, size.y))?;

        Ok(())
    }
}

fn get_viewport_texture_id(contexts: EguiContexts, target: Res<ViewportRenderTarget>) -> TextureId {
    contexts.image_id(&target.0).unwrap()
}

fn resize_viewport(
    In(size): In<Vec2>,
    target: Res<ViewportRenderTarget>,
    mut images: ResMut<Assets<Image>>,
    mut last_size: Local<Vec2>,
) {
    let image = images.get_mut(&target.0).unwrap();
    if size != *last_size {
        image.resize(Extent3d {
            width: size.x as u32,
            height: size.y as u32,
            ..default()
        });

        *last_size = size;
    }
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

#[derive(Resource)]
struct ViewportRenderTarget(Handle<Image>);

#[derive(Default, Debug, Component)]
struct ViewportPicking {
    min: Vec2,
    interact_pos: Option<Vec2>,
    hover_pos: Option<Vec2>,
}

impl ViewportPicking {
    fn position(&self) -> Option<Vec2> {
        self.hover_pos
            .or_else(|| self.interact_pos)
            .map(|position| position - self.min)
    }
}

fn setup(
    mut images: ResMut<Assets<Image>>,
    mut user_textures: ResMut<EguiUserTextures>,
    mut commands: Commands,
) {
    let mut viewport_target = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::bevy_default(),
        RenderAssetUsages::RENDER_WORLD,
    );

    viewport_target.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let viewport_target_handle = images.add(viewport_target);
    user_textures.add_image(EguiTextureHandle::Strong(viewport_target_handle.clone()));
    commands.insert_resource(ViewportRenderTarget(viewport_target_handle.clone()));

    let camera_origin = commands
        .spawn((
            InheritedVisibility::VISIBLE,
            Transform::from_xyz(6.0, 6.0, 6.0).with_rotation(Quat::from_rotation_y(PI / 4.0)),
        ))
        .id();

    let viewport_camera = commands
        .spawn((
            ChildOf(camera_origin),
            ViewportCamera {
                movement: None,
                origin: camera_origin,
                rotation_sensitivity: 0.05,
                pane_sensitivity: 0.15,
                fly_speed: 5.0,
            },
            ViewportPicking::default(),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.25, 0.25, 0.25)),
                order: -1,
                ..default()
            },
            RenderTarget::Image(viewport_target_handle.clone().into()),
            Transform::from_rotation(Quat::from_rotation_x(-PI / 5.0)),
            PointerId::Custom(Uuid::new_v4()),
        ))
        .id();

    commands
        .spawn((
            Pickable {
                should_block_lower: false,
                ..default()
            },
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            UiTargetCamera(viewport_camera),
        ))
        .observe(on_viewport_drag_start)
        .observe(on_viewport_drag)
        .observe(on_viewport_drag_end);

    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            x_axis_color: Color::srgb(0.67, 0.25, 0.32),
            z_axis_color: Color::srgb(0.13, 0.38, 0.64),
            major_line_color: Color::srgb(0.32, 0.32, 0.32),
            minor_line_color: Color::srgb(0.27, 0.27, 0.27),
            ..default()
        },
    ));
}

fn on_viewport_drag_start(
    trigger: On<Pointer<DragStart>>,
    mut camera: Single<&mut ViewportCamera>,
    mut cursor_lock: ResMut<CursorLock>,
) -> Result {
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
    _: On<Pointer<Drag>>,
    camera: Single<(Entity, &ViewportCamera)>,
    mut transforms: Query<&mut Transform>,
    global_transforms: Query<&GlobalTransform>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
) -> Result {
    let (entity, camera) = *camera;

    let Some(movement) = &camera.movement else {
        return Ok(());
    };

    let delta = mouse_motion.delta * time.delta_secs();

    let camera_global_transform = global_transforms.get(entity)?;

    match movement {
        ViewportCameraMovement::Fly => {
            let mut camera_transform = transforms.get_mut(entity)?;
            let mut pitch = camera_transform.rotation.to_euler(EulerRot::XYZ).0;
            pitch = (pitch - delta.y * camera.rotation_sensitivity).clamp(-PI / 2.0, PI / 2.0);
            camera_transform.rotation = Quat::from_rotation_x(pitch);

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.rotate(Quat::from_rotation_y(
                -delta.x * camera.rotation_sensitivity,
            ));
        }
        ViewportCameraMovement::Pan => {
            let left = camera_global_transform.left().as_vec3();
            let up = camera_global_transform.up().as_vec3();

            let mut origin_transform = transforms.get_mut(camera.origin)?;
            origin_transform.translation += left * delta.x * camera.pane_sensitivity;
            origin_transform.translation += up * delta.y * camera.pane_sensitivity;
        }
    }

    Ok(())
}

fn on_viewport_drag_end(
    _: On<Pointer<DragEnd>>,
    mut camera: Single<&mut ViewportCamera>,
    mut cursor_lock: ResMut<CursorLock>,
) -> Result {
    camera.movement = None;
    cursor_lock.0 = false;
    Ok(())
}

fn move_camera(
    camera: Single<(Entity, &ViewportCamera)>,
    global_transforms: Query<&GlobalTransform>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut transforms: Query<&mut Transform>,
) -> Result {
    let (camera_entity, camera) = *camera;

    let Some(movement) = &camera.movement else {
        return Ok(());
    };

    if !matches!(movement, ViewportCameraMovement::Fly) {
        return Ok(());
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

    Ok(())
}

fn viewport_picking(
    mut viewport_camera: Single<(
        &PointerId,
        &ViewportPicking,
        &RenderTarget,
        &mut PointerLocation,
    )>,
    mut pointer_inputs: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let (viewport_pointer_id, interaction, render_target, pointer_location) =
        viewport_camera.deref_mut();

    let Some(position) = interaction.position() else {
        pointer_location.location = None;
        return;
    };

    for input in pointer_inputs
        .read()
        .filter(|input| input.pointer_id == PointerId::Mouse)
    {
        let location = Location {
            position,
            target: NormalizedRenderTarget::Image(render_target.as_image().unwrap().clone().into()),
        };

        pointer_location.location = Some(location.clone());

        commands.write_message(PointerInput {
            action: input.action,
            location,
            pointer_id: **viewport_pointer_id,
        });
    }
}

fn on_pick_mesh(
    mut trigger: On<Pointer<Click>>,
    meshes: Query<Entity, Or<(With<Mesh2d>, With<Mesh3d>)>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selection_map: ResMut<SelectionMap>,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    let target = trigger.event_target();

    if !meshes.contains(target) {
        return Ok(());
    }

    trigger.propagate(false);

    let is_selected = selection_map.is_selected(target);

    if !keyboard_input.pressed(KeyCode::ControlLeft) {
        selection_map.clear();
        selection_map.select(target);
    } else {
        if is_selected {
            selection_map.deselect(target);
        } else {
            selection_map.select(target);
        }
    }

    Ok(())
}

fn on_select(trigger: On<Select>, mut commands: Commands) {
    commands
        .entity(trigger.event_target())
        .insert(OutlineMode::FloodFlat)
        .insert(OutlineVolume {
            visible: true,
            width: 2.0,
            colour: Color::srgb(0.13, 0.43, 0.79),
            ..default()
        });
}

fn on_deselect(trigger: On<Deselect>, mut commands: Commands) {
    commands
        .entity(trigger.event_target())
        .remove::<OutlineVolume>();
}

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_plugins(MeshPickingPlugin)
            .add_plugins(OutlinePlugin)
            .register_pane(ViewportPane)
            .add_systems(Startup, setup)
            .add_systems(First, viewport_picking.in_set(PickingSystems::PostInput))
            .add_systems(Update, move_camera)
            .add_observer(on_pick_mesh)
            .add_observer(on_select)
            .add_observer(on_deselect);
    }
}
