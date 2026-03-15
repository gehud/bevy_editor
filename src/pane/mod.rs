use accesskit::Point;
use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    asset::AssetServer,
    camera::{NormalizedRenderTarget, visibility::Visibility},
    ecs::{
        bundle::Bundle,
        children,
        component::Component,
        entity::{self, ContainsEntity, Entity},
        entity_disabling::Disabled,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::Add,
        observer::On,
        query::{Added, Changed, QueryState, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{
            BoxedSystem, Commands, EntityCommands, In, IntoSystem, Local, Query, Res, ResMut,
            Single, SystemId,
        },
        world::{Mut, World},
    },
    log::{info, warn},
    picking::{
        Pickable,
        events::{
            Cancel, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Pointer,
        },
        pointer::PointerButton,
    },
    platform::collections::HashMap,
    text::TextFont,
    ui::{
        AlignItems, AlignSelf, ComputedNode, FlexDirection, JustifyContent, Node, Overflow,
        OverflowClipMargin, PositionType, UiGlobalTransform, UiRect, UiScale, UiSystems, percent,
        px, widget::Text,
    },
    utils::default,
    window::SystemCursorIcon,
};

use crate::{
    theme::{
        InheritableFont, RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor,
        ThemedText,
        constants::fonts::REGULAR,
        palette::ACCENT,
        tokens::{PANE_BG, PANE_TAB_ACTIVE, TEXT_MAIN, WINDOW_BG},
    },
    widget::{EntityCursor, OverrideCursor},
    window::DecoratedWindow,
};

pub const PANE_BORDER_RADIUS: f32 = 6.0;
pub const MIN_PANE_SIZE: f32 = 45.0;
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;

pub struct PanePlugin;

impl Plugin for PanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaneRegistry>()
            .init_resource::<ResizeHandleDragState>()
            .init_resource::<TabbarDropState>()
            // .add_systems(Update, on_show_tab)
            .add_systems(Update, cleanup_divider_single_child)
            .add_systems(PostUpdate, apply_size.before(UiSystems::Layout))
            .add_observer(init);
    }
}

#[derive(Component)]
pub(crate) struct PaneLayoutRoot;

#[derive(Component)]
struct PaneTab {
    pane: Entity,
    tab: String,
}

#[derive(Component)]
struct PaneTabbar {
    drop_indicator: Entity,
    tabgroup: Entity,
}

fn spawn_pane<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    size: f32,
    tabs: Vec<String>,
) -> EntityCommands<'a> {
    let root = commands.spawn((Node::default(), Size(size))).id();

    commands
        .spawn((
            ChildOf(root),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                border_radius: RoundedCorners::All.to_border_radius(PANE_BORDER_RADIUS),
                ..default()
            },
            ThemeBackgroundColor(PANE_BG),
        ))
        .with_children(|commands| {
            // Tabbar
            commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(30),
                        padding: UiRect::left(px(6)).with_right(px(8)),
                        overflow: Overflow::clip(),
                        border: UiRect::horizontal(px(1)).with_top(px(1)),
                        flex_shrink: 0.0,
                        border_radius: RoundedCorners::Top.to_border_radius(PANE_BORDER_RADIUS),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    ThemeBackgroundColor(WINDOW_BG),
                    ThemeBorderColor(PANE_BG),
                ))
                .with_children(|commands| {
                    // Tab group
                    let tabgroup = commands
                        .spawn((
                            Node {
                                height: percent(100),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|commands| {
                            let group = commands.target_entity();
                            let mut first = true;
                            for tab in tabs {
                                spawn_tab(commands.commands_mut(), asset_server, root, tab, first)
                                    .insert(ChildOf(group))
                                    .observe(on_tab_drag_start)
                                    .observe(on_tab_drag)
                                    .observe(on_tab_drag_end)
                                    .observe(on_tab_drag_cancel);

                                first = false;
                            }
                        })
                        .id();

                    // Menu
                    commands.spawn(Node {
                        width: px(12),
                        height: px(12),
                        ..default()
                    });

                    let drop_indicator = commands
                        .commands()
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                height: percent(100),
                                width: px(3),
                                margin: UiRect::horizontal(px(-1)),
                                left: px(0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            ThemeBackgroundColor(PANE_TAB_ACTIVE),
                            Visibility::Hidden,
                        ))
                        .id();

                    commands
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                            Pickable {
                                should_block_lower: false,
                                ..default()
                            },
                            PaneTabbar {
                                drop_indicator,
                                tabgroup,
                            },
                        ))
                        .add_child(drop_indicator)
                        .observe(on_tabbar_drag_enter)
                        .observe(on_tabbar_drag_over)
                        .observe(on_tabbar_drag_drop)
                        .observe(on_tabbar_drag_leave);
                });

            // Content area
            commands.spawn(Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(6)),
                ..default()
            });
        });

    commands.entity(root).insert(PaneStructure { root });

    commands.entity(root)
}

#[derive(Resource)]
struct TabbarDropState {
    initial_tabgroup: Entity,
    drop_index: usize,
}

impl Default for TabbarDropState {
    fn default() -> Self {
        Self {
            initial_tabgroup: Entity::PLACEHOLDER,
            drop_index: 0,
        }
    }
}

fn on_tabbar_drag_enter(
    trigger: On<Pointer<DragEnter>>,
    tabbars: Query<&PaneTabbar>,
    tabs: Query<&PaneTab>,
    mut commands: Commands,
    mut tabbar_drop_state: ResMut<TabbarDropState>,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(tab) = tabs.get(trigger.dragged) else {
        return Ok(());
    };

    commands
        .entity(tabbar.drop_indicator)
        .entry::<Visibility>()
        .and_modify(|mut visibility| {
            *visibility = Visibility::Inherited;
        });

    tabbar_drop_state.initial_tabgroup = tabbar.tabgroup;

    Ok(())
}

fn on_tabbar_drag_over(
    trigger: On<Pointer<DragOver>>,
    tabbars: Query<&PaneTabbar>,
    tabs: Query<&PaneTab>,
    children: Query<&Children>,
    computed_nodes: Query<&ComputedNode>,
    mut nodes: Query<&mut Node>,
    mut tabbar_drop_state: ResMut<TabbarDropState>,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(tab) = tabs.get(trigger.dragged) else {
        return Ok(());
    };

    let tabbar_size = computed_nodes.get(trigger.entity)?.size().x;

    let mut indicator = nodes.get_mut(tabbar.drop_indicator)?;

    let pointer_position = trigger
        .hit
        .position
        .map(|position| (position.x + 0.5) * tabbar_size)
        .unwrap_or_default();

    let mut indicator_position = 0.0;
    tabbar_drop_state.drop_index = 0;
    let tabs = children.get(tabbar.tabgroup)?;
    for tab in tabs {
        let size = computed_nodes.get(*tab)?.size().x;

        if pointer_position < indicator_position + (size / 2.0) {
            break;
        }

        indicator_position += size;
        tabbar_drop_state.drop_index += 1;
    }

    indicator.left = percent(indicator_position / tabbar_size * 100.0);

    Ok(())
}

fn on_tabbar_drag_drop(
    trigger: On<Pointer<DragDrop>>,
    tabbars: Query<&PaneTabbar>,
    tabs: Query<&PaneTab>,
    tabbar_drop_state: Res<TabbarDropState>,
    mut commands: Commands,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(tab) = tabs.get(trigger.dropped) else {
        return Ok(());
    };

    commands
        .entity(tabbar_drop_state.initial_tabgroup)
        .detach_child(trigger.dropped);

    commands
        .entity(tabbar.tabgroup)
        .insert_child(tabbar_drop_state.drop_index, trigger.dropped);

    Ok(())
}

fn on_tabbar_drag_leave(
    trigger: On<Pointer<DragLeave>>,
    tabbars: Query<&PaneTabbar>,
    tabs: Query<&PaneTab>,
    mut commands: Commands,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(tab) = tabs.get(trigger.dragged) else {
        return Ok(());
    };

    commands
        .entity(tabbar.drop_indicator)
        .entry::<Visibility>()
        .and_modify(|mut visibility| {
            *visibility = Visibility::Hidden;
        });

    Ok(())
}

#[derive(Component)]
struct TabDragIndicator;

fn on_tab_drag_start(
    trigger: On<Pointer<DragStart>>,
    decorated: Query<&DecoratedWindow>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tabs: Query<&PaneTab>,
    mut override_cursor: ResMut<OverrideCursor>,
) -> Result {
    let NormalizedRenderTarget::Window(window) = trigger.pointer_location.target else {
        return Ok(());
    };

    let Ok(decorated) = decorated.get(window.entity()) else {
        return Ok(());
    };

    let tab = tabs.get(trigger.event_target())?;
    spawn_tab(
        &mut commands,
        &asset_server,
        tab.pane,
        tab.tab.clone(),
        true,
    )
    .insert(ChildOf(decorated.root()))
    .insert(Pickable::IGNORE)
    .insert(TabDragIndicator)
    .entry::<Node>()
    .and_modify(|mut node| {
        node.position_type = PositionType::Absolute;
        node.border = UiRect::all(px(2));
        node.border_radius = RoundedCorners::All.to_border_radius(2.0);
    });

    override_cursor.0 = Some(EntityCursor::System(SystemCursorIcon::Grabbing));

    Ok(())
}

fn on_tab_drag(
    trigger: On<Pointer<Drag>>,
    mut indicator: Single<&mut Node, With<TabDragIndicator>>,
    ui_scale: Res<UiScale>,
) {
    indicator.left = px(trigger.pointer_location.position.x / ui_scale.0);
    indicator.top = px(trigger.pointer_location.position.y / ui_scale.0);
}

fn on_tab_drag_end(
    _: On<Pointer<DragEnd>>,
    indicator: Single<Entity, With<TabDragIndicator>>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    commands.entity(*indicator).despawn();
    override_cursor.0 = None;
}

fn on_tab_drag_cancel(
    _: On<Pointer<Cancel>>,
    indicator: Single<Entity, With<TabDragIndicator>>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    commands.entity(*indicator).despawn();
    override_cursor.0 = None;
}

fn spawn_tab<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    pane: Entity,
    tab: String,
    active: bool,
) -> EntityCommands<'a> {
    let root = commands
        .spawn((
            PaneTab {
                pane,
                tab: tab.clone(),
            },
            Node {
                height: px(30),
                padding: UiRect::horizontal(px(8)),
                border: UiRect::top(px(2)),
                border_radius: RoundedCorners::Top.to_border_radius(2.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ThemeBackgroundColor(if active { PANE_BG } else { WINDOW_BG }),
            ThemeBorderColor(if active { PANE_TAB_ACTIVE } else { WINDOW_BG }),
            Pickable {
                should_block_lower: false,
                ..default()
            },
            EntityCursor::System(SystemCursorIcon::Pointer),
        ))
        .id();

    commands.spawn((
        Pickable::IGNORE,
        ChildOf(root),
        Text::new(tab),
        TextFont {
            font: asset_server.load(REGULAR),
            font_size: 12.0,
            ..default()
        },
        ThemeFontColor(TEXT_MAIN),
    ));

    commands.entity(root)
}

// fn on_show_tab(
//     world: &mut World,
//     roots_query: &mut QueryState<Entity, Added<PaneRoot>>,
//     pane_root_node_query: &mut QueryState<(&PaneRoot, &PaneStructure)>,
//     mut system_ids: Local<HashMap<String, SystemId<In<PaneStructure>>>>,
// ) {
//     let roots: Vec<_> = roots_query.iter(world).collect();
//     for entity in roots {
//         world.resource_scope(|world, mut pane_registry: Mut<PaneRegistry>| {
//             let (pane_root, &structure) = pane_root_node_query.get(world, entity).unwrap();
//             let pane = pane_registry
//                 .panes
//                 .iter_mut()
//                 .find(|pane| pane.name == pane_root.name);

//             if let Some(pane) = pane {
//                 let id = system_ids.entry(pane.name.clone()).or_insert_with(|| {
//                     world.register_boxed_system(pane.creation_callback.take().unwrap())
//                 });

//                 world.run_system_with(*id, structure).unwrap();
//             } else {
//                 warn!(
//                     "No pane found in the registry with name: '{}'",
//                     pane_root.name
//                 );
//             }
//         });
//     }
// }

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

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Divider {
    Horizontal,
    Vertical,
}

#[derive(Component)]
struct ResizeHandle;

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

#[derive(Resource, Default)]
struct ResizeHandleDragState {
    is_dragging: bool,
    parent_node_size: f32,
}

fn spawn_resize_handle<'a>(commands: &'a mut Commands, divider: Divider) -> EntityCommands<'a> {
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

fn init(trigger: On<Add, PaneLayoutRoot>, mut commands: Commands, asset_server: Res<AssetServer>) {
    let root = trigger.entity;

    let divider = spawn_divider(&mut commands, Divider::Horizontal, 1.)
        .insert(ChildOf(root))
        .id();

    let sub_divider = spawn_divider(&mut commands, Divider::Vertical, 0.2)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(
        &mut commands,
        &asset_server,
        0.4,
        vec!["Scene Tree".into(), "Scene Tree".into()],
    )
    .insert(ChildOf(sub_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(sub_divider));
    spawn_pane(
        &mut commands,
        &asset_server,
        0.6,
        vec!["Properties".into(), "Properties".into()],
    )
    .insert(ChildOf(sub_divider));

    spawn_resize_handle(&mut commands, Divider::Horizontal).insert(ChildOf(divider));

    let asset_browser_divider = spawn_divider(&mut commands, Divider::Vertical, 0.8)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(
        &mut commands,
        &asset_server,
        0.70,
        vec![
            "Viewport 3D".into(),
            "Viewport 3D".into(),
            "Viewport 3D".into(),
            "Viewport 3D".into(),
        ],
    )
    .insert(ChildOf(asset_browser_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(asset_browser_divider));
    spawn_pane(
        &mut commands,
        &asset_server,
        0.30,
        vec![
            "Asset Browser".into(),
            "Asset Browser".into(),
            "Asset Browser".into(),
            "Asset Browser".into(),
        ],
    )
    .insert(ChildOf(asset_browser_divider));
}
