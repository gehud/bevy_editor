use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        error::{BevyError, Result},
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::Add,
        observer::On,
        query::{Added, Changed, QueryState, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{
            BoxedSystem, Commands, EntityCommands, In, IntoSystem, Local, Query, ResMut, SystemId,
        },
        world::{Mut, World},
    },
    log::warn,
    picking::{
        events::{Cancel, Drag, DragEnd, DragStart, Pointer},
        pointer::PointerButton,
    },
    platform::collections::HashMap,
    ui::{ComputedNode, FlexDirection, Node, UiRect, Val, ZIndex, percent, px},
    utils::default,
    window::SystemCursorIcon,
};

use crate::widget::{
    cursor::EntityCursor, rounded_corners::RoundedCorners, theme::ThemeBackgroundColor,
    tokens::PANE_BG,
};

pub const PANE_BORDER_RADIUS: f32 = 6.0;
pub const MIN_PANE_SIZE: f32 = 20.0;
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;

pub struct PanePlugin;

impl Plugin for PanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaneRegistry>()
            .init_resource::<DragState>()
            .add_systems(Update, on_pane_creation)
            .add_systems(Update, (cleanup_divider_single_child, apply_size).chain())
            .add_observer(init);
    }
}

#[derive(Component)]
pub struct PaneLayoutRoot;

#[derive(Component)]
struct PaneRootNode {
    name: String,
}

fn spawn_pane<'a>(
    commands: &'a mut Commands,
    size: f32,
    name: impl Into<String>,
) -> EntityCommands<'a> {
    let name: String = name.into();

    let root = commands
        .spawn((
            Node::default(),
            Size(size),
            PaneRootNode { name: name.clone() },
        ))
        .id();

    commands.spawn((
        ChildOf(root),
        Node {
            width: percent(100),
            height: percent(100),
            border_radius: RoundedCorners::All.to_border_radius(PANE_BORDER_RADIUS),
            ..default()
        },
        ThemeBackgroundColor(PANE_BG),
    ));

    commands.entity(root).insert(PaneStructure { root });

    commands.entity(root)
}

fn on_pane_creation(
    world: &mut World,
    roots_query: &mut QueryState<Entity, Added<PaneRootNode>>,
    pane_root_node_query: &mut QueryState<(&PaneRootNode, &PaneStructure)>,
    mut system_ids: Local<HashMap<String, SystemId<In<PaneStructure>>>>,
) {
    let roots: Vec<_> = roots_query.iter(world).collect();
    for entity in roots {
        world.resource_scope(|world, mut pane_registry: Mut<PaneRegistry>| {
            let (pane_root, &structure) = pane_root_node_query.get(world, entity).unwrap();
            let pane = pane_registry
                .panes
                .iter_mut()
                .find(|pane| pane.name == pane_root.name);

            if let Some(pane) = pane {
                let id = system_ids.entry(pane.name.clone()).or_insert_with(|| {
                    world.register_boxed_system(pane.creation_callback.take().unwrap())
                });

                world.run_system_with(*id, structure).unwrap();
            } else {
                warn!(
                    "No pane found in the registry with name: '{}'",
                    pane_root.name
                );
            }
        });
    }
}

#[derive(Component, Clone, Copy)]
pub struct PaneStructure {
    pub root: Entity,
}

struct Pane {
    name: String,
    creation_callback: Option<BoxedSystem<In<PaneStructure>>>,
}

#[derive(Resource, Default)]
pub struct PaneRegistry {
    panes: Vec<Pane>,
}

impl PaneRegistry {
    pub fn register<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) {
        self.panes.push(Pane {
            name: name.into(),
            creation_callback: Some(Box::new(IntoSystem::into_system(system))),
        });
    }
}

pub trait RegisterPane {
    fn register_pane<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) -> &mut Self;
}

impl RegisterPane for App {
    fn register_pane<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) -> &mut Self {
        self.world_mut()
            .resource_mut::<PaneRegistry>()
            .register(name, system);
        self
    }
}

#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool,
    offset: f32,
    min: f32,
    max: f32,
    parent_node_size: f32,
}

fn cleanup_divider_single_child(
    mut commands: Commands,
    mut dividers: Query<(Entity, &Children, &ChildOf), (Changed<Children>, With<Divider>)>,
    mut sizes: Query<&mut Size>,
    children: Query<&Children>,
    resize_handles: Query<(), With<ResizeHandle>>,
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
    mut nodes: Query<(Entity, &Size, &mut Node), Changed<Size>>,
    dividers: Query<&Divider>,
    parents: Query<&ChildOf>,
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

/// A node that divides an area into multiple areas along an axis.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Divider {
    Horizontal,
    Vertical,
}

#[derive(Component)]
struct ResizeHandle;

/// The fraction of space this element takes up in the [`Divider`] it's a child of.
#[derive(Component)]
struct Size(f32);

fn spawn_divider<'a>(
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

fn spawn_resize_handle<'a>(commands: &'a mut Commands, divider: Divider) -> EntityCommands<'a> {
    let cursor_icon = match divider {
        Divider::Horizontal => SystemCursorIcon::EwResize,
        Divider::Vertical => SystemCursorIcon::NsResize,
    };

    let mut handle = commands.spawn((
        Node {
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
        ResizeHandle,
        EntityCursor::System(cursor_icon),
    ));

    handle
        .observe(
            move |trigger: On<Pointer<DragStart>>,
                  mut drag_state: ResMut<DragState>,
                  parents: Query<&ChildOf>,
                  children: Query<&Children>,
                  computed_nodes: Query<&ComputedNode>,
                  sizes: Query<&Size>|
                  -> Result {
                if trigger.button != PointerButton::Primary {
                    return Ok(());
                }

                drag_state.is_dragging = true;

                let target = trigger.event_target();
                let parent = parents.get(target)?.parent();

                let parent_node_size = computed_nodes.get(parent)?.size();
                let parent_node_size = match divider {
                    Divider::Horizontal => parent_node_size.x,
                    Divider::Vertical => parent_node_size.y,
                };

                let siblings = children.get(parent)?;
                let index = siblings
                    .iter()
                    .position(|entity| *entity == target)
                    .unwrap();

                let size_a = sizes.get(siblings[index - 1])?.0;
                let size_b = sizes.get(siblings[index + 1])?.0;

                drag_state.offset = 0.;
                drag_state.min = (-size_a * parent_node_size) + MIN_PANE_SIZE;
                drag_state.max = (size_b * parent_node_size) - MIN_PANE_SIZE;
                drag_state.parent_node_size = parent_node_size;

                Ok(())
            },
        )
        .observe(
            move |trigger: On<Pointer<Drag>>,
                  mut drag_state: ResMut<DragState>,
                  parents: Query<&ChildOf>,
                  children: Query<&Children>,
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

                let delta = trigger.event().delta;
                let delta = match divider {
                    Divider::Horizontal => delta.x,
                    Divider::Vertical => delta.y,
                };

                let previous_offset = drag_state.offset;

                drag_state.offset += delta;

                drag_state.offset = drag_state.offset.clamp(drag_state.min, drag_state.max);

                let clamped_delta = drag_state.offset - previous_offset;

                sizes.get_mut(siblings[index - 1])?.0 +=
                    clamped_delta / drag_state.parent_node_size;
                sizes.get_mut(siblings[index + 1])?.0 -=
                    clamped_delta / drag_state.parent_node_size;

                Ok(())
            },
        )
        .observe(
            move |_: On<Pointer<DragEnd>>, mut drag_state: ResMut<DragState>| {
                drag_state.is_dragging = false;
                drag_state.offset = 0.;
            },
        )
        .observe(
            |_: On<Pointer<Cancel>>, mut drag_state: ResMut<DragState>| {
                drag_state.is_dragging = false;
                drag_state.offset = 0.;
            },
        );

    handle
}

fn init(trigger: On<Add, PaneLayoutRoot>, mut commands: Commands) {
    let root = trigger.entity;

    let divider = spawn_divider(&mut commands, Divider::Horizontal, 1.)
        .insert(ChildOf(root))
        .id();

    let sub_divider = spawn_divider(&mut commands, Divider::Vertical, 0.2)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(&mut commands, 0.4, "Scene Tree").insert(ChildOf(sub_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(sub_divider));
    spawn_pane(&mut commands, 0.6, "Properties").insert(ChildOf(sub_divider));

    spawn_resize_handle(&mut commands, Divider::Horizontal).insert(ChildOf(divider));

    let asset_browser_divider = spawn_divider(&mut commands, Divider::Vertical, 0.8)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(&mut commands, 0.70, "Viewport 3D").insert(ChildOf(asset_browser_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(asset_browser_divider));
    spawn_pane(&mut commands, 0.30, "Asset Browser").insert(ChildOf(asset_browser_divider));
}
