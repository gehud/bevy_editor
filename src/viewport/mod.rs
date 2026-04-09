mod grid;

use std::f32::consts::PI;

use bevy::{
    app::{App, Plugin, Startup},
    asset::{Assets, Handle, RenderAssetUsages},
    camera::{Camera, Camera3d, ClearColorConfig, RenderTarget, visibility::InheritedVisibility},
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::ChildOf,
        resource::Resource,
        system::{Commands, In, Local, Res, ResMut},
        world::World,
    },
    image::{BevyDefault, Image},
    math::Quat,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    transform::components::Transform,
    utils::default,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, EguiUserTextures};
use egui::{TextureId, Ui, Vec2, load::SizedTexture};

use crate::{
    pane::{Pane, RegisterPane},
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
        ui.image(SizedTexture::new(texture_id, size));

        world.run_system_cached_with(resize_viewport, size)?;

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
                rotation_sensitivity: 0.005,
                pane_sensitivity: 0.015,
                fly_speed: 5.0,
            },
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.25, 0.25, 0.25)),
                order: -1,
                ..default()
            },
            RenderTarget::Image(viewport_target_handle.clone().into()),
            Transform::from_rotation(Quat::from_rotation_x(-PI / 5.0)),
        ))
        .id();

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

pub struct ViewportPlugin;

impl Plugin for ViewportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .register_pane(ViewportPane)
            .add_systems(Startup, setup);
    }
}
