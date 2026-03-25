use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::{Changed, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, EntityCommands, Query, ResMut},
    },
    picking::{
        events::{Cancel, Drag, DragEnd, DragStart, Pointer},
        pointer::PointerButton,
    },
    ui::{ComputedNode, FlexDirection, Node, UiGlobalTransform, UiSystems, percent, px},
    utils::default,
    window::SystemCursorIcon,
};

use crate::widget::{EntityCursor, OverrideCursor};

const RESIZE_HANDLE_SIZE: f32 = 4.0;
const MIN_PANE_SIZE: f32 = 100.0;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum Divider {
    Horizontal,
    Vertical,
}

#[derive(Component)]
pub(super) struct Size(pub(super) f32);

pub(super) fn spawn_divider<'a>(
    commands: &'a mut Commands,
    divider: Divider,
    size: f32,
) -> EntityCommands<'a> {
    commands.spawn((
        Node {
            flex_direction: match divider {
                Divider::Horizontal => FlexDirection::Row,
                Divider::Vertical => FlexDirection::Column,
            },
            ..default()
        },
        Size(size),
        divider,
    ))
}

#[derive(Resource, Default)]
struct ResizeHandleDragState {
    is_dragging: bool,
    parent_node_size: f32,
}

#[derive(Component)]
struct ResizeHandle;

pub(super) fn spawn_resize_handle<'a>(
    commands: &'a mut Commands,
    divider: Divider,
) -> EntityCommands<'a> {
    let cursor_icon = match divider {
        Divider::Horizontal => SystemCursorIcon::EwResize,
        Divider::Vertical => SystemCursorIcon::NsResize,
    };

    let mut handle = commands.spawn((
        Node {
            flex_shrink: 0.0,
            width: match divider {
                Divider::Horizontal => px(RESIZE_HANDLE_SIZE),
                Divider::Vertical => percent(100),
            },
            height: match divider {
                Divider::Horizontal => percent(100),
                Divider::Vertical => px(RESIZE_HANDLE_SIZE),
            },
            ..default()
        },
        EntityCursor::System(cursor_icon),
        ResizeHandle,
    ));

    handle
        .observe(
            move |trigger: On<Pointer<DragStart>>,
                  mut drag_state: ResMut<ResizeHandleDragState>,
                  parents: Query<&ChildOf>,
                  computed_nodes: Query<&ComputedNode>,
                  mut override_cursor: ResMut<OverrideCursor>|
                  -> Result {
                if trigger.button != PointerButton::Primary {
                    return Ok(());
                }

                override_cursor.0 = Some(EntityCursor::System(cursor_icon));

                drag_state.is_dragging = true;

                let target = trigger.event_target();
                let parent = parents.get(target)?.parent();

                let parent_node_size = computed_nodes.get(parent)?.size();
                let parent_node_size = match divider {
                    Divider::Horizontal => parent_node_size.x,
                    Divider::Vertical => parent_node_size.y,
                };

                drag_state.parent_node_size = parent_node_size;

                Ok(())
            },
        )
        .observe(
            move |trigger: On<Pointer<Drag>>,
                  drag_state: ResMut<ResizeHandleDragState>,
                  parents: Query<&ChildOf>,
                  children: Query<&Children>,
                  ui_global_transforms: Query<&UiGlobalTransform>,
                  mut sizes: Query<&mut Size>|
                  -> Result {
                if !drag_state.is_dragging {
                    return Ok(());
                }

                let target = trigger.event_target();
                let parent = parents.get(target)?.parent();
                let siblings = children.get(parent)?;

                let index = siblings
                    .iter()
                    .position(|entity| *entity == target)
                    .unwrap();

                let min_size = MIN_PANE_SIZE / drag_state.parent_node_size;

                let pointer_position = match divider {
                    Divider::Horizontal => trigger.pointer_location.position.x,
                    Divider::Vertical => trigger.pointer_location.position.y,
                };

                let handle_position = match divider {
                    Divider::Horizontal => ui_global_transforms.get(target)?.translation.x,
                    Divider::Vertical => ui_global_transforms.get(target)?.translation.y,
                };

                let delta = match divider {
                    Divider::Horizontal => trigger.delta.x,
                    Divider::Vertical => trigger.delta.y,
                }
                .abs()
                    / drag_state.parent_node_size;

                if pointer_position > handle_position {
                    let mut next = sizes.get_mut(siblings[index + 1])?;
                    let last_next_size = next.0;
                    next.0 = (next.0 - delta).max(min_size);
                    let size_change = last_next_size - next.0;

                    let mut prev = sizes.get_mut(siblings[index - 1])?;
                    prev.0 += size_change;
                } else {
                    let mut prev = sizes.get_mut(siblings[index - 1])?;
                    let last_prev_size = prev.0;
                    prev.0 = (prev.0 - delta).max(min_size);
                    let size_change = last_prev_size - prev.0;

                    let mut next = sizes.get_mut(siblings[index + 1])?;
                    next.0 += size_change;
                }

                Ok(())
            },
        )
        .observe(
            |_: On<Pointer<DragEnd>>,
             mut drag_state: ResMut<ResizeHandleDragState>,
             mut override_cursor: ResMut<OverrideCursor>| {
                override_cursor.0 = None;
                drag_state.is_dragging = false;
            },
        )
        .observe(
            |_: On<Pointer<Cancel>>,
             mut drag_state: ResMut<ResizeHandleDragState>,
             mut override_cursor: ResMut<OverrideCursor>| {
                override_cursor.0 = None;
                drag_state.is_dragging = false;
            },
        );

    handle
}

fn remove_dividers(
    children: Query<&Children>,
    resize_handles: Query<(), With<ResizeHandle>>,
    mut sizes: Query<&mut Size>,
    mut dividers: Query<(Entity, &Children, &ChildOf), (Changed<Children>, With<Divider>)>,
    mut commands: Commands,
) -> Result {
    for (entity, parts, parent) in &mut dividers {
        let mut iter = parts
            .iter()
            .filter(|child| !resize_handles.contains(**child));

        let child = iter.next().unwrap();
        if iter.next().is_some() {
            continue;
        }

        let size = sizes.get(entity)?.0;
        sizes.get_mut(*child)?.0 = size;

        let siblings = children.get(parent.parent())?;
        let index = siblings.iter().position(|s| *s == entity).unwrap();

        commands
            .entity(parent.parent())
            .insert_children(index, &[*child]);

        commands.entity(entity).despawn();
    }

    Ok(())
}

fn apply_size(
    dividers: Query<&Divider>,
    parents: Query<&ChildOf>,
    mut nodes: Query<(Entity, &Size, &mut Node), Changed<Size>>,
) -> Result {
    for (entity, size, mut node) in &mut nodes {
        let parent = parents.get(entity)?.parent();
        let Ok(divider) = dividers.get(parent) else {
            node.width = percent(100);
            node.height = percent(100);
            continue;
        };

        match divider {
            Divider::Horizontal => {
                node.width = percent(size.0 * 100.0);
                node.height = percent(100);
            }
            Divider::Vertical => {
                node.width = percent(100);
                node.height = percent(size.0 * 100.0);
            }
        }
    }

    Ok(())
}

pub(super) struct PaneLayoutPlugin;

impl Plugin for PaneLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ResizeHandleDragState>().add_systems(
            PostUpdate,
            (remove_dividers, apply_size)
                .chain()
                .before(UiSystems::Layout),
        );
    }
}
