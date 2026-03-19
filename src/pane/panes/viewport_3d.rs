use bevy::{
    app::{App, First, Plugin, PostUpdate, Startup},
    asset::Assets,
    camera::{Camera, Camera3d, ClearColorConfig, NormalizedRenderTarget, RenderTarget},
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
    math::{Rect, Vec3},
    picking::{
        PickingSystems,
        events::{Out, Over, Pointer},
        hover::Hovered,
        pointer::{Location, PointerId, PointerInput},
    },
    render::render_resource::{Extent3d, TextureFormat},
    transform::components::Transform,
    ui::{
        ComputedNode, Node, UiGlobalTransform, UiSystems, percent,
        widget::{ImageNode, NodeImageMode, update_image_content_size_system},
    },
    utils::default,
};

use crate::{
    pane::{PaneStructure, RegisterPane},
    theme::palette,
};

pub struct Viewport3dPanePlugin;

impl Plugin for Viewport3dPanePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            First,
            render_target_picking_passthrough.in_set(PickingSystems::PostInput),
        )
        .register_pane("Viewport 3D", setup);
    }
}

#[derive(Component)]
struct Viewport3d;

fn render_target_picking_passthrough(
    viewports: Query<Entity, With<Viewport3d>>,
    nodes: Query<(&Hovered, &ComputedNode, &UiGlobalTransform, &ImageNode)>,
    mut pointer_input_reader: MessageReader<PointerInput>,
    mut commands: Commands,
) -> Result {
    // for event in pointer_input_reader.read() {
    //     for viewport in &viewports {
    //         let (hovered, computed_node, global_transform, ui_image) = nodes.get(viewport)?;

    //         if !hovered.0 {
    //             continue;
    //         }

    //         let node_top_left = global_transform.translation - computed_node.size() / 2.0;
    //         let position = event.location.position - node_top_left;

    //         let target = NormalizedRenderTarget::Image(ui_image.image.clone().into());

    //         let event_copy = PointerInput {
    //             action: event.action,
    //             location: Location { position, target },
    //             pointer_id: event.pointer_id,
    //         };

    //         commands.write_message(event_copy);
    //     }
    // }

    Ok(())
}

fn setup(
    In(pane_structure): In<PaneStructure>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let image = Image::new_target_texture(1, 1, TextureFormat::bevy_default(), None);
    let image = images.add(image);

    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(palette::GRAY_0),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Transform::from_translation(Vec3::ONE * 5.0).looking_at(Vec3::ZERO, Vec3::Y),
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
                    Viewport3d,
                    Hovered::default(),
                    ImageNode {
                        image,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .observe(move |_: On<Despawn, Viewport3d>, mut commands: Commands| {
                    commands.entity(camera).despawn();
                });
        });
}
