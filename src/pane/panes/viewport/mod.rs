mod grid;

use bevy::{
    app::{App, First, Plugin, PostUpdate, PreUpdate, Startup, Update},
    asset::{Assets, RenderAssetUsages, uuid::Uuid},
    camera::{
        Camera, Camera3d, ClearColorConfig, NormalizedRenderTarget, RenderTarget,
        visibility::RenderLayers,
    },
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        lifecycle::{Despawn, Remove},
        message::MessageReader,
        observer::On,
        query::{Changed, With},
        schedule::IntoScheduleConfigs,
        system::{Commands, In, Query, ResMut},
    },
    image::{BevyDefault, Image},
    log::info,
    math::{Rect, Vec3},
    picking::{
        PickingSystems,
        events::{Out, Over, Pointer},
        hover::Hovered,
        input::{mouse_pick_events, touch_pick_events},
        pointer::{Location, PointerId, PointerInput},
    },
    render::render_resource::{Extent3d, TextureFormat, TextureUsages},
    transform::components::Transform,
    ui::{
        AlignContent, AlignItems, AlignSelf, ComputedNode, JustifyContent, Node, UiGlobalTransform,
        UiSystems, percent,
        widget::{ImageNode, ImageNodeSize, NodeImageMode, update_image_content_size_system},
    },
    utils::default,
};

use crate::{
    pane::{
        PaneStructure, RegisterPane,
        panes::viewport::grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    },
    theme::palette,
};

pub struct Viewport3dPanePlugin;

impl Plugin for Viewport3dPanePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InfiniteGridPlugin)
            .add_systems(Startup, setup_grid)
            .add_systems(
                First,
                render_target_picking_passthrough.in_set(PickingSystems::PostInput),
            )
            .add_systems(
                PostUpdate,
                update_render_target_size.after(UiSystems::Layout),
            )
            .register_pane("Viewport", setup);
    }
}

#[derive(Component)]
struct Viewport;

#[derive(Component)]
struct Active;

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

fn render_target_picking_passthrough(
    viewports: Query<Entity, With<Viewport>>,
    nodes: Query<(Entity, &ComputedNode, &UiGlobalTransform, &ImageNode), With<Active>>,
    mut pointer_input_reader: MessageReader<PointerInput>,
    mut commands: Commands,
) {
    for event in pointer_input_reader.read() {
        if event.pointer_id != PointerId::Mouse {
            continue;
        }

        for viewport in &viewports {
            let Ok((entity, computed_node, global_transform, ui_image)) = nodes.get(viewport)
            else {
                continue;
            };

            let node_top_left = global_transform.translation - computed_node.size() / 2.0;
            let position = event.location.position - node_top_left;

            let target = NormalizedRenderTarget::Image(ui_image.image.clone().into());

            let event_copy = PointerInput {
                action: event.action,
                location: Location { position, target },
                pointer_id: pointer_id_from_entity(entity),
            };

            commands.write_message(event_copy);
        }
    }
}

fn pointer_id_from_entity(entity: Entity) -> PointerId {
    let bits = entity.to_bits();
    PointerId::Custom(Uuid::from_u64_pair(bits, bits))
}

fn setup(
    In(pane_structure): In<PaneStructure>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let mut image = Image::new_target_texture(1, 1, TextureFormat::bevy_default(), None);
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;

    let image = images.add(image);

    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(palette::GRAY_0),
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
                    Viewport,
                    ImageNode {
                        image: image,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
                    commands.entity(trigger.entity).insert(Active);
                })
                .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
                    commands.entity(trigger.entity).remove::<Active>();
                })
                .observe(move |_: On<Despawn, Viewport>, mut commands: Commands| {
                    commands.entity(camera).despawn();
                });
        });
}

fn update_render_target_size(
    viewports: Query<(&ImageNode, &ComputedNode), (With<Viewport>, Changed<ComputedNode>)>,
    mut images: ResMut<Assets<Image>>,
) {
    for (image_node, node) in viewports {
        let image = images.get_mut(&image_node.image).unwrap();
        image.resize(Extent3d {
            width: node.size().x.max(1.0) as u32,
            height: node.size().y.max(1.0) as u32,
            ..default()
        });
    }
}
