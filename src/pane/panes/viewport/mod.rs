mod camera;
mod grid;

use bevy::{
    app::{App, Plugin, Startup},
    asset::{Assets, RenderAssetUsages},
    camera::{Camera, Camera3d, ClearColorConfig, RenderTarget},
    ecs::{
        lifecycle::Despawn,
        observer::On,
        system::{Commands, In, ResMut},
    },
    image::{BevyDefault, Image},
    math::Vec3,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    transform::components::Transform,
    ui::{Node, percent, widget::ViewportNode},
    utils::default,
};

use crate::{
    pane::{
        PaneStructure, RegisterPane,
        panes::viewport::{
            camera::{ViewportCamera, ViewportCameraPlugin},
            grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
        },
    },
    theme::palette,
};

pub struct ViewportPanePlugin;

impl Plugin for ViewportPanePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_plugins(ViewportCameraPlugin)
            .add_systems(Startup, setup_grid)
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

fn setup(
    In(pane_structure): In<PaneStructure>,
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

    let camera = commands
        .spawn((
            ViewportCamera::default(),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(palette::GRAY_0),
                order: -1,
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
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
                .observe(
                    move |_: On<Despawn, ViewportNode>, mut commands: Commands| {
                        commands.entity(camera).despawn();
                    },
                );
        });
}
