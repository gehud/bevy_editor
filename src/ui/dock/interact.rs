//! Mouse interaction with the dock: switching tabs, resizing splits and moving panels.

use bevy::{
    picking::events::{PointerClick, PointerDrag, PointerDragEnd, PointerDragStart},
    prelude::*,
    ui::UiGlobalTransform,
};

use super::{
    layout::{Axis, DockLayout, DropZone, NodeId, PanelId},
    view::{
        DockLeafArea, DockSplitArea, DockSplitChild, DockTab, DockTabBar, DragsPanel, GAP, Splitter,
    },
};
use crate::ui::style::colors;

/// Smallest size a split child can be resized to, in logical pixels.
const MIN_CHILD_SIZE: f32 = 60.0;
/// How far the pointer must move from the press before a tab starts dragging, in logical pixels.
const TAB_DRAG_THRESHOLD: f32 = 8.0;
/// Fraction of a leaf's width/height, measured from each edge, that drops as a split.
const EDGE_ZONE: f32 = 0.25;

/// The panel being dragged and where it would land.
#[derive(Resource, Default)]
pub(super) struct DockDrag {
    panel: Option<PanelId>,
    past_threshold: bool,
    target: Option<(NodeId, DropZone)>,
}

/// Translucent rectangle showing where a dragged panel will go.
#[derive(Component, Clone, Default)]
pub(super) struct DropPreview;

pub(super) fn setup_drop_preview(mut commands: Commands) {
    commands.spawn((
        Name::new("Dock Drop Preview"),
        DropPreview,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        BackgroundColor(colors::ACCENT.with_alpha(0.2)),
        BorderColor::all(colors::ACCENT),
        GlobalZIndex(i32::MAX - 1),
        Pickable::IGNORE,
    ));
}

pub(super) fn click_tab(
    mut click: On<PointerClick>,
    tabs: Query<&DockTab>,
    mut layout: ResMut<DockLayout>,
) {
    let Ok(tab) = tabs.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button == PointerButton::Primary && layout.active_index(tab.leaf) != Some(tab.index) {
        layout.activate(tab.leaf, tab.index);
    }
}

pub(super) fn start_panel_drag(
    mut drag: On<PointerDragStart>,
    handles: Query<(&DragsPanel, Has<DockTab>)>,
    mut state: ResMut<DockDrag>,
) {
    let Ok((DragsPanel(panel), is_tab)) = handles.get(drag.entity) else {
        return;
    };
    drag.propagate(false);
    if drag.button == PointerButton::Primary {
        *state = DockDrag {
            panel: Some(*panel),
            past_threshold: !is_tab,
            target: None,
        };
    }
}

pub(super) fn drag_panel(
    mut drag: On<PointerDrag>,
    mut state: ResMut<DockDrag>,
    leaves: Query<(&DockLeafArea, &ComputedNode, &UiGlobalTransform)>,
    bars: Query<(&DockTabBar, &ComputedNode, &UiGlobalTransform)>,
    tabs: Query<(&DockTab, &ComputedNode, &UiGlobalTransform)>,
    mut preview: Single<(&mut Node, &mut BackgroundColor), With<DropPreview>>,
) {
    drag.propagate(false);
    if drag.button != PointerButton::Primary || state.panel.is_none() {
        return;
    }
    if !state.past_threshold {
        if drag.distance.length_squared() < TAB_DRAG_THRESHOLD * TAB_DRAG_THRESHOLD {
            return;
        }
        state.past_threshold = true;
    }
    let bar_hit = bars.iter().find_map(|(bar, node, transform)| {
        let scale = node.inverse_scale_factor;
        let point = drag.pointer.position / scale;
        if !node.contains_point(*transform, point) {
            return None;
        }
        let bar_rect = Rect::from_center_size(transform.translation, node.size);
        let nearest = tabs
            .iter()
            .filter(|(tab, _, _)| tab.leaf == bar.0)
            .map(|(tab, node, transform)| {
                let rect = Rect::from_center_size(transform.translation, node.size);
                let after = point.x >= transform.translation.x;
                let edge = if after { rect.max.x } else { rect.min.x };
                let distance = (point.x - edge).abs();
                (distance, tab.index + usize::from(after), edge, rect)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let (_, slot, edge, rect) = nearest.unwrap_or((0.0, 0, bar_rect.min.x, bar_rect));
        // Center a two-logical-pixel stroke on the shared edge, snapped to device pixels.
        let width = (2.0 / scale).round().max(1.0);
        let x = (edge - width * 0.5)
            .round()
            .clamp(bar_rect.min.x, (bar_rect.max.x - width).max(bar_rect.min.x));
        let marker = Rect::new(
            x * scale,
            rect.min.y.round() * scale,
            (x + width) * scale,
            rect.max.y.round() * scale,
        );
        Some((bar.0, (DropZone::Tab(slot), marker)))
    });
    let hit = bar_hit.or_else(|| {
        leaves.iter().find_map(|(leaf, node, transform)| {
            let point = drag.pointer.position / node.inverse_scale_factor;
            node.contains_point(*transform, point)
                .then(|| (leaf.0, drop_target(node, transform, point)))
        })
    });
    let (preview, fill) = &mut *preview;
    match hit {
        Some((leaf, (zone, rect))) => {
            state.target = Some((leaf, zone));
            if matches!(zone, DropZone::Tab(_)) {
                fill.0 = colors::ACCENT;
                preview.border = UiRect::ZERO;
                preview.border_radius = BorderRadius::ZERO;
            } else {
                fill.0 = colors::ACCENT.with_alpha(0.2);
                preview.border = UiRect::all(px(2));
                preview.border_radius = BorderRadius::all(px(6));
            }
            preview.display = Display::Flex;
            preview.left = px(rect.min.x);
            preview.top = px(rect.min.y);
            preview.width = px(rect.width());
            preview.height = px(rect.height());
        }
        None => {
            state.target = None;
            preview.display = Display::None;
        }
    }
}

pub(super) fn end_panel_drag(
    mut drag: On<PointerDragEnd>,
    mut state: ResMut<DockDrag>,
    mut layout: ResMut<DockLayout>,
    mut preview: Single<&mut Node, With<DropPreview>>,
) {
    drag.propagate(false);
    if drag.button != PointerButton::Primary {
        return;
    }
    if let DockDrag {
        panel: Some(panel),
        target: Some((leaf, zone)),
        ..
    } = *state
    {
        // Only touch the layout (and trigger a rebuild) when the drop changes something.
        let mut moved = layout.clone();
        moved.move_panel(panel, leaf, zone);
        if moved != *layout {
            *layout = moved;
        }
    }
    *state = DockDrag::default();
    preview.display = Display::None;
}

/// Picks the drop zone under `point` (physical pixels) and the rectangle (logical pixels)
/// the dragged panel would occupy.
fn drop_target(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    point: Vec2,
) -> (DropZone, Rect) {
    let scale = node.inverse_scale_factor;
    let center = transform.translation;
    let rect = Rect::from_center_size(center * scale, node.size * scale);
    let local = (point * scale - rect.min) / rect.size();

    let edges = [
        (local.x, DropZone::Left),
        (1.0 - local.x, DropZone::Right),
        (local.y, DropZone::Top),
        (1.0 - local.y, DropZone::Bottom),
    ];
    let (distance, edge) = edges
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    let zone = if distance < EDGE_ZONE {
        edge
    } else {
        DropZone::Center
    };

    let half = rect.size() / 2.0;
    let target = match zone {
        DropZone::Center => rect,
        DropZone::Tab(_) => unreachable!(),
        DropZone::Left => Rect::from_corners(rect.min, rect.min + Vec2::new(half.x, rect.height())),
        DropZone::Right => Rect::from_corners(rect.min + Vec2::new(half.x, 0.0), rect.max),
        DropZone::Top => Rect::from_corners(rect.min, rect.min + Vec2::new(rect.width(), half.y)),
        DropZone::Bottom => Rect::from_corners(rect.min + Vec2::new(0.0, half.y), rect.max),
    };
    (zone, target)
}

pub(super) fn drag_splitter(
    mut drag: On<PointerDrag>,
    splitters: Query<&Splitter>,
    areas: Query<(&DockSplitArea, &ComputedNode)>,
    mut children: Query<(&DockSplitChild, &mut Node)>,
    mut layout: ResMut<DockLayout>,
) {
    let Ok(splitter) = splitters.get(drag.entity) else {
        return;
    };
    drag.propagate(false);
    let Some((_, area)) = areas.iter().find(|(area, _)| area.split == splitter.split) else {
        return;
    };
    // Resizing must not rebuild the dock, so the layout is edited without change detection
    // and the affected nodes are updated directly.
    let Some(sizes) = layout
        .bypass_change_detection()
        .split_sizes_mut(splitter.split)
    else {
        return;
    };

    let (length, delta) = match splitter.axis {
        Axis::Horizontal => (area.size.x, drag.delta.x),
        Axis::Vertical => (area.size.y, drag.delta.y),
    };
    let available = length * area.inverse_scale_factor - GAP * (sizes.len() - 1) as f32;
    if available <= 0.0 {
        return;
    }
    let total: f32 = sizes.iter().sum();
    let per_pixel = total / available;
    let min = MIN_CHILD_SIZE * per_pixel;

    let (a, b) = (splitter.index, splitter.index + 1);
    let pair = sizes[a] + sizes[b];
    let new_a =
        (sizes[a] + delta * per_pixel).clamp(min.min(pair / 2.0), (pair - min).max(pair / 2.0));
    sizes[a] = new_a;
    sizes[b] = pair - new_a;
    let (size_a, size_b) = (sizes[a], sizes[b]);

    for (child, mut node) in &mut children {
        if child.split != splitter.split {
            continue;
        }
        if child.index == a {
            node.flex_grow = size_a;
        } else if child.index == b {
            node.flex_grow = size_b;
        }
    }
}
