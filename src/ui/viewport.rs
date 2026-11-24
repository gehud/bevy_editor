//! Viewport grid: three views of the edited scene, each with its own tool bar.
//!
//! Every viewport is a UI node showing the output of its own 3D camera
//! ([`ViewportNode`]), so the views are live renders, not images.

use bevy::{
    camera::RenderTarget, feathers::constants::icons, prelude::*,
    render::render_resource::TextureFormat, ui::widget::ViewportNode,
};

use super::{
    style::{colors, hex, icon, icon_sq, ink, label},
    widgets::{grip, vertical_divider},
};

/// Background of the 3D views.
const VIEWPORT_CLEAR: Color = hex(0x3B3B3B);

/// Marks the node that displays viewport `index` (0 = main view).
#[derive(Component, Clone, Copy, Default)]
pub struct ViewportSlot(pub u8);

/// Camera poses for the three views, looking at the origin.
const CAMERA_POSITIONS: [Vec3; 3] = [
    Vec3::new(-5.5, 3.2, 6.5),
    Vec3::new(-6.5, 3.0, 5.0),
    Vec3::new(-6.5, 1.5, 5.0),
];

pub fn viewport_grid() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            width: percent(100),
            column_gap: px(4),
        }
        Children [
            // Main view.
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(6)),
            }
            Children [ @viewport_pane(0, true) ]
            --
            // Two secondary views stacked on the right.
            Node {
                width: px(351),
                flex_shrink: 0.0,
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
            }
            Children [
                @viewport_pane(1, false)
                --
                @viewport_pane(2, false)
            ]
        ]
    }
}

fn viewport_pane(index: u8, active: bool) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            width: percent(100),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(px(5)),
        }
        Children [
            @viewport_toolbar(active)
            --
            // The 3D view; the axis gizmo sits in its top-right corner.
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                padding: UiRect::axes(px(12), px(11)),
            }
            ViewportSlot(index)
            Children [
                Node { width: px(53.947), height: px(50) }
                ImageNode { image: "axis-gizmo.png" }
            ]
        ]
    }
}

fn viewport_toolbar(active: bool) -> impl Scene {
    // The main view shows its snapping toggle switched on.
    let (snap_bg, snap_border, snap_border_width) = if active {
        (colors::TOGGLE_ACTIVE_BG, Color::NONE, 0.0)
    } else {
        (colors::ITEM_BG, colors::ITEM_BORDER, 1.0)
    };
    let (value_bg, value_border) = if active {
        (Color::NONE, colors::DIVIDER)
    } else {
        (colors::ITEM_BG, colors::ITEM_BORDER)
    };

    bsn! {
        Node {
            height: px(30),
            width: percent(100),
            flex_shrink: 0.0,
            padding: UiRect { left: px(6), right: px(8) },
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::top(px(6)),
        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor { top: colors::PANEL_BORDER, bottom: colors::PANEL_BORDER, left: colors::PANEL_BORDER, right: colors::PANEL_BORDER }
        Children [
            Node { height: percent(100), column_gap: px(7), align_items: AlignItems::Center }
            Children [
                // Select tool (active: the image includes its highlighted background).
                Node { width: px(18), height: px(19), margin: UiRect::top(px(1)) }
                ImageNode { image: "icons/select-tool-active.png" }
                --
                @vertical_divider()
                --
                @icon_sq(icons::MOVE_3D, 18.0, ink::AFAF)
                --
                @icon_sq(icons::ROTATE_3D, 18.0, ink::AFAF)
                --
                Node { width: px(18), height: px(18), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [ @icon(icons::SCALE_3D, 15.0, 14.5, ink::AFAF) ]
                --
                @vertical_divider()
                --
                // Snapping toggle.
                Node {
                    height: px(18),
                    padding: UiRect::horizontal(px(2)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(snap_border_width)),
                    border_radius: BorderRadius::all(px(3)),
                }
                BackgroundColor(snap_bg)
                BorderColor { top: snap_border, bottom: snap_border, left: snap_border, right: snap_border }
                Children [ @icon_sq(icons::MAGNET, 14.0, ink::E6) ]
                --
                // Snapping step.
                Node {
                    width: px(40),
                    height: px(18),
                    padding: UiRect::horizontal(px(6)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(2.5)),
                }
                BackgroundColor(value_bg)
                BorderColor { top: value_border, bottom: value_border, left: value_border, right: value_border }
                Children [ @label("5.00", ink::B9B9) ]
            ]
            --
            @grip()
        ]
    }
}

/// Spawns a camera for every new viewport node and connects the two.
pub(super) fn attach_viewport_cameras(
    mut commands: Commands,
    slots: Query<(Entity, &ViewportSlot), Without<ViewportNode>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (node, slot) in &slots {
        // The render target is resized to the node's size by `ViewportNode`.
        let image = Image::new_target_texture(1, 1, TextureFormat::Bgra8UnormSrgb, None);
        let position = CAMERA_POSITIONS[slot.0 as usize % CAMERA_POSITIONS.len()];
        let camera = commands
            .spawn((
                Name::new(format!("Viewport Camera {}", slot.0)),
                Camera3d::default(),
                Camera {
                    // Render before the UI camera that displays the result.
                    order: -1 - slot.0 as isize,
                    clear_color: ClearColorConfig::Custom(VIEWPORT_CLEAR),
                    ..default()
                },
                RenderTarget::Image(images.add(image).into()),
                Transform::from_translation(position).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
            ))
            .id();
        commands.entity(node).insert(ViewportNode::new(camera));
    }
}

/// Ground grid and world axes, drawn in every view.
pub(super) fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        UVec2::splat(40),
        Vec2::splat(1.0),
        hex(0x4A4A4A),
    );
    let extent = 20.0;
    gizmos.line(
        Vec3::new(-extent, 0.001, 0.0),
        Vec3::new(extent, 0.001, 0.0),
        hex(0xC0392B),
    );
    gizmos.line(
        Vec3::new(0.0, 0.001, -extent),
        Vec3::new(0.0, 0.001, extent),
        hex(0x2F6FD0),
    );
}

/// Sample content standing in for the design's "Stanford Dragon".
pub(super) fn spawn_demo_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: hex(0x8FB3DE),
        perceptual_roughness: 0.6,
        ..default()
    });

    commands.spawn((
        Name::new("Directional Light"),
        DirectionalLight {
            illuminance: 8_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Stanford Dragon"),
        Transform::default(),
        Visibility::default(),
        children![
            (
                Name::new("Head"),
                Mesh3d(meshes.add(Sphere::new(0.55))),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(-1.4, 1.9, 0.2),
            ),
            (
                Name::new("Body"),
                Mesh3d(meshes.add(Capsule3d::new(0.75, 1.6))),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(0.0, 1.0, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 * 0.6)),
            ),
            (
                Name::new("Tail"),
                Mesh3d(meshes.add(Cone::new(0.45, 1.6))),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(1.5, 0.45, -0.3)
                    .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            ),
        ],
    ));
}
