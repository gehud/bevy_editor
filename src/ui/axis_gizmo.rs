//! The orientation gizmo in each viewport's corner: the world axes as seen by its camera.
//!
//! Positive axes are bright, labeled circles joined to the center; negative axes are plain
//! circles in the darker axis colors. Circles nearer the viewer are drawn on top. Clicking a
//! circle turns the camera to look along that axis (e.g. clicking X views the scene from +X).

use bevy::{
    feathers::palette, picking::events::PointerClick, prelude::*, ui::widget::ViewportNode,
    ui_widgets::Button,
};

use super::style::{Weight, hex, text};
use crate::editor_camera::{FlyCamera, look_along};

/// Width and height of the gizmo.
const SIZE: f32 = 56.0;
/// Distance from the center to each circle's center.
const REACH: f32 = 18.0;
/// Diameter of the axis circles.
const CIRCLE: f32 = 13.0;
/// Thickness of the lines to the positive axes.
const LINE: f32 = 2.0;

/// Positive-axis colors from the design; negative axes use the feathers axis colors.
const POSITIVE: [Color; 3] = [hex(0xF53352), hex(0x87D603), hex(0x298CF5)];
const NEGATIVE: [Color; 3] = [palette::X_AXIS, palette::Y_AXIS, palette::Z_AXIS];
const LABELS: [&str; 3] = ["X", "Y", "Z"];
const LABEL_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);

/// Root of an axis gizmo; must be a child of a viewport node.
#[derive(Component, Clone, Default)]
pub struct AxisGizmo;

/// One axis circle. `axis` is 0, 1 or 2 for X, Y, Z.
#[derive(Component, Clone, Copy, Default)]
pub(super) struct GizmoCircle {
    axis: usize,
    positive: bool,
}

/// The line from the center to a positive axis circle.
#[derive(Component, Clone, Copy, Default)]
pub(super) struct GizmoLine {
    axis: usize,
}

fn world_axis(axis: usize) -> Vec3 {
    [Vec3::X, Vec3::Y, Vec3::Z][axis]
}

pub fn axis_gizmo() -> impl Scene {
    let mut parts: Vec<Box<dyn Scene>> = Vec::new();
    for axis in 0..3 {
        parts.push(Box::new(line(axis)));
        parts.push(Box::new(circle(axis, true)));
        parts.push(Box::new(circle(axis, false)));
    }
    bsn! {
        Node {
            width: px(SIZE),
            height: px(SIZE),
            flex_shrink: 0.0,
        }
        AxisGizmo
        Children [ {parts} ]
    }
}

fn line(axis: usize) -> impl Scene {
    let marker = GizmoLine { axis };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            height: px(LINE),
            border_radius: BorderRadius::all(px(LINE)),
        }
        BackgroundColor({POSITIVE[axis]})
        marker
        Pickable::IGNORE
    }
}

fn circle(axis: usize, positive: bool) -> impl Scene {
    let marker = GizmoCircle { axis, positive };
    let color = if positive {
        POSITIVE[axis]
    } else {
        NEGATIVE[axis]
    };
    let label: Vec<Box<dyn Scene>> = if positive {
        vec![Box::new(bsn! {
            @text(LABELS[axis], 9.0, Weight::SemiBold, LABEL_COLOR)
            Pickable::IGNORE
        })]
    } else {
        Vec::new()
    };
    bsn! {
        Button
        Node {
            position_type: PositionType::Absolute,
            width: px(CIRCLE),
            height: px(CIRCLE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(percent(50)),
        }
        BackgroundColor(color)
        marker
        on(click_axis)
        Children [ {label} ]
    }
}

/// Positions the circles and lines from each viewport camera's current rotation.
pub(super) fn update_axis_gizmos(
    gizmos: Query<(&ChildOf, &Children), With<AxisGizmo>>,
    viewports: Query<&ViewportNode>,
    cameras: Query<&Transform>,
    mut circles: Query<(&GizmoCircle, &mut Node, &mut ZIndex), Without<GizmoLine>>,
    mut lines: Query<(&GizmoLine, &mut Node, &mut ZIndex, &mut UiTransform), Without<GizmoCircle>>,
) {
    let center = Vec2::splat(SIZE / 2.0);
    for (child_of, parts) in &gizmos {
        let Some(rotation) = viewports
            .get(child_of.parent())
            .ok()
            .and_then(|viewport| viewport.camera)
            .and_then(|camera| cameras.get(camera).ok())
            .map(|transform| transform.rotation)
        else {
            continue;
        };
        // Each world axis in view space: x right, y up, z toward the viewer.
        let view = |axis: usize| rotation.inverse() * world_axis(axis);

        for &part in parts {
            if let Ok((circle, mut node, mut z)) = circles.get_mut(part) {
                let sign = if circle.positive { 1.0 } else { -1.0 };
                let v = view(circle.axis) * sign;
                let position = center + Vec2::new(v.x, -v.y) * REACH - CIRCLE / 2.0;
                node.left = px(position.x);
                node.top = px(position.y);
                // Nearer circles on top; lines sit just under their circle.
                *z = ZIndex((v.z * 100.0) as i32 * 2 + 1);
            } else if let Ok((line, mut node, mut z, mut transform)) = lines.get_mut(part) {
                let v = view(line.axis);
                let offset = Vec2::new(v.x, -v.y) * REACH;
                let length = offset.length();
                let middle = center + offset / 2.0;
                node.width = px(length);
                node.left = px(middle.x - length / 2.0);
                node.top = px(middle.y - LINE / 2.0);
                transform.rotation = Rot2::radians(offset.y.atan2(offset.x));
                *z = ZIndex((v.z * 100.0) as i32 * 2);
            }
        }
    }
}

/// Turns the viewport's camera to look along the clicked axis, toward the scene.
fn click_axis(
    mut click: On<PointerClick>,
    circles: Query<(&GizmoCircle, &ChildOf)>,
    gizmos: Query<&ChildOf, With<AxisGizmo>>,
    viewports: Query<&ViewportNode>,
    mut cameras: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let Ok((circle, gizmo)) = circles.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button != PointerButton::Primary {
        return;
    }
    let Some(camera) = gizmos
        .get(gizmo.parent())
        .ok()
        .and_then(|viewport| viewports.get(viewport.parent()).ok())
        .and_then(|viewport| viewport.camera)
    else {
        return;
    };
    if let Ok((mut transform, mut fly)) = cameras.get_mut(camera) {
        let sign = if circle.positive { 1.0 } else { -1.0 };
        // Viewing "from +X" means looking toward -X.
        look_along(&mut transform, &mut fly, -world_axis(circle.axis) * sign);
    }
}
