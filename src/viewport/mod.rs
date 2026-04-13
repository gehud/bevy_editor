mod camera;
mod grid;

use std::{
    f32::consts::PI,
    ops::{Deref, DerefMut},
};

use bevy::{
    app::{App, First, Plugin, PostUpdate, PreUpdate, Startup, Update},
    asset::{Assets, Handle, RenderAssetUsages, uuid::Uuid},
    camera::{
        Camera, Camera3d, ClearColorConfig, NormalizedRenderTarget, Projection, RenderTarget,
        visibility::InheritedVisibility,
    },
    color::{
        Color,
        palettes::tailwind::{PINK_100, RED_500},
    },
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::ChildOf,
        lifecycle::{Add, Remove},
        message::{Message, MessageReader, MessageWriter},
        observer::On,
        query::{Or, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, In, Local, Query, Res, ResMut, Single},
        world::World,
    },
    gizmos::gizmos::Gizmos,
    image::{BevyDefault, Image},
    input::{
        ButtonInput,
        keyboard::KeyCode,
        mouse::{AccumulatedMouseMotion, MouseButton},
    },
    log::info,
    math::{EulerRot, Quat, Rect, Vec2, Vec3},
    mesh::{Mesh2d, Mesh3d},
    picking::{
        Pickable, PickingSystems,
        events::{Click, Drag, DragEnd, DragStart, Move, Pointer, PointerState},
        hover::HoverMap,
        mesh_picking::{MeshPickingPlugin, MeshPickingSettings, ray_cast::RayCastVisibility},
        pointer::{
            Location, PointerButton, PointerId, PointerInput, PointerInteraction, PointerLocation,
        },
    },
    platform::collections::HashMap,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    time::Time,
    transform::components::{GlobalTransform, Transform},
    ui::{Node, UiTargetCamera, percent, widget::ViewportNode},
    utils::default,
    window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, EguiUserTextures};
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use egui::{
    Color32, CornerRadius, Frame, InnerResponse, Margin, Sense, TextureId, Ui, Widget,
    load::SizedTexture,
};

use crate::{
    pane::{Pane, RegisterPane},
    selection::{Deselect, Select, SelectionMap},
    viewport::{camera::{FreeCamera, FreeCameraPlugin, FreeCameraState}, grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings}},
};

pub struct ViewportPane;

impl Pane for ViewportPane {
    fn name(&self) -> &str {
        "Viewport"
    }

    fn padding(&self) -> Option<Margin> {
        Some(Margin::ZERO)
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let texture_id = world.run_system_cached(get_viewport_texture_id)?;

        let size = ui.available_size();

        let response = egui::Image::new(SizedTexture::new(texture_id, size))
            .corner_radius(CornerRadius {
                sw: ui.style().visuals.window_corner_radius.sw,
                se: ui.style().visuals.window_corner_radius.se,
                ..default()
            })
            .ui(ui)
            .interact(Sense::click_and_drag());

        let viewport = world
            .query_filtered::<Entity, With<Viewport>>()
            .single(world)?;

        world.entity_mut(viewport).insert(Viewport {
            rect: Rect::from_corners(
                Vec2::new(response.rect.min.x, response.rect.min.y),
                Vec2::new(response.rect.max.x, response.rect.max.y),
            ),
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

#[derive(Resource)]
struct ViewportRenderTarget(Handle<Image>);

#[derive(Default, Debug, Component)]
struct Viewport {
    rect: Rect,
    interact_pos: Option<Vec2>,
    hover_pos: Option<Vec2>,
}

impl Viewport {
    fn position(&self) -> Option<Vec2> {
        self.hover_pos
            .or_else(|| self.interact_pos)
            .map(|position| position - self.rect.min)
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

    commands.spawn((
        FreeCamera {
            sensitivity: 0.2,
            friction: 25.0,
            walk_speed: 3.0,
            run_speed: 9.0,
            mouse_key_cursor_grab: MouseButton::Right,
            ..default()
        },
        Viewport::default(),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.25, 0.25, 0.25)),
            order: -1,
            ..default()
        },
        RenderTarget::Image(viewport_target_handle.clone().into()),
        Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
        PointerId::Custom(Uuid::new_v4()),
    ));

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

fn viewport_picking(
    mut viewport_camera: Single<(&PointerId, &Viewport, &mut FreeCameraState, &RenderTarget, &mut PointerLocation)>,
    mut pointer_inputs: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    let (viewport_pointer_id, picking, state, render_target, pointer_location) =
        viewport_camera.deref_mut();

    let Some(position) = picking.position() else {
        pointer_location.location = None;
        state.enabled = false;
        return;
    };

    state.enabled = true;

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

fn deselect_all(
    trigger: On<Pointer<Click>>,
    viewport_picking: Single<&Viewport>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    mut map: ResMut<SelectionMap>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    if viewport_picking.hover_pos.is_none() {
        return;
    }

    if trigger.event_target() == *primary_window {
        map.clear();
    }
}

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_plugins(MeshPickingPlugin)
            .add_plugins(OutlinePlugin)
            .add_plugins(FreeCameraPlugin)
            .register_pane(ViewportPane)
            .add_systems(Startup, setup)
            .add_systems(First, viewport_picking.in_set(PickingSystems::PostInput))
            .add_observer(on_pick_mesh)
            .add_observer(on_select)
            .add_observer(on_deselect)
            .add_observer(deselect_all);
    }
}
