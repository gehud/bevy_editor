//! Viewport panels: a tool bar above a live view of the edited scene.
//!
//! Each view is a UI node showing the output of its own editor camera
//! (see [`crate::editor_camera`]).

use bevy::prelude::*;

use bevy_ui_svg::SvgIcon;

use super::axis_gizmo::axis_gizmo;
use crate::ui::icons;

use super::{
    style::{colors, hex, icon, icon_sq, ink, label},
    widgets::vertical_divider,
};

/// Marks the node that displays viewport `index` (0 = main view).
#[derive(Component, Clone, Copy, Default)]
pub struct ViewportSlot(pub u8);

/// A viewport panel: tool bar above a live 3D view.
///
/// `active` shows the tool bar's "snapping on" state, as the design does for the main view.
pub fn viewport_content(index: u8, active: bool) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            width: percent(100),
            flex_direction: FlexDirection::Column,
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
            Children [ @axis_gizmo() ]
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

        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor { top: colors::PANEL_BORDER, bottom: colors::PANEL_BORDER, left: colors::PANEL_BORDER, right: colors::PANEL_BORDER }
        Children [
            Node { height: percent(100), column_gap: px(7), align_items: AlignItems::Center }
            Children [
                // Select tool (active: the image includes its highlighted background).
                Node { width: px(18), height: px(19), margin: UiRect::top(px(1)) }
                SvgIcon { svg: icons::SELECT_TOOL_ACTIVE, mask: false, trim: false }
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
        ]
    }
}

/// Sample content standing in for the design's "Stanford Dragon".
/// The built-in demo scene: a light and a stylized dragon (see `EditorPlugin::demo_scene`).
pub fn spawn_demo_scene(
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
