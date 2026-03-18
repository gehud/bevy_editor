pub(crate) mod panes;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    asset::AssetServer,
    camera::{NormalizedRenderTarget, visibility::Visibility},
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::{Add, Remove},
        observer::On,
        query::{Changed, Or, With},
        resource::Resource,
        schedule::{IntoScheduleConfigs, common_conditions::resource_changed},
        system::{
            BoxedSystem, Commands, EntityCommands, In, IntoSystem, Query, Res, ResMut, SystemId,
            SystemState,
        },
        world::{Mut, World},
    },
    log::{warn, warn_once},
    math::Vec2,
    picking::{
        Pickable,
        events::{
            Cancel, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Pointer,
            Press,
        },
        hover::Hovered,
        pointer::PointerButton,
    },
    platform::collections::HashMap,
    text::TextFont,
    ui::{
        AlignItems, ComputedNode, FlexDirection, JustifyContent, Node, Overflow, PositionType,
        ScrollPosition, UiGlobalTransform, UiRect, UiScale, UiSystems, percent, px, widget::Text,
    },
    utils::default,
    window::SystemCursorIcon,
};

use crate::{
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor,
        constants::fonts::REGULAR,
        tokens::{BUTTON_BG, PANE_BG, PANE_TAB_ACTIVE, TEXT_MAIN, WINDOW_BG},
    },
    widget::{
        ContextMenu, ContextMenuMark, EntityCursor, OverrideCursor, ScrollAxis, ScrollRect,
        Scrollbar,
    },
    window::EditorWindow,
};

pub const PANE_BORDER_RADIUS: f32 = 6.0;
pub const MIN_PANE_SIZE: f32 = 100.0;
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;

pub struct EditorPanePlugin;

impl Plugin for EditorPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaneRegistry>()
            .init_resource::<ResizeHandleDragState>()
            .add_systems(Update, cleanup_divider_single_child)
            .add_systems(
                Update,
                (
                    clamp_active_tab_index,
                    register_pane_callbacks.run_if(resource_changed::<PaneRegistry>),
                    update_active_tab,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, apply_size.before(UiSystems::Layout))
            .add_observer(init);
    }
}

#[derive(Component)]
pub(crate) struct PaneLayoutRoot;

#[derive(Component)]
struct PaneRef {
    entity: Entity,
}

#[derive(Component)]
struct PaneTab {
    name: String,
}

#[derive(Component)]
struct PaneTabbar {
    drop_indicator: Entity,
    tabgroup: Entity,
}

#[derive(Component)]
struct PaneTabgroup {
    active_tab_index: usize,
}

fn spawn_pane<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    size: f32,
    tabs: Vec<String>,
) -> EntityCommands<'a> {
    assert!(tabs.len() != 0, "Cannot spawn pane with no tabs.");

    let root = commands.spawn((Node::default(), Size(size))).id();

    let area = commands
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
            // Header
            commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(30),
                        padding: UiRect::left(px(6)).with_right(px(8)),
                        border: UiRect::horizontal(px(1)).with_top(px(1)),
                        flex_shrink: 0.0,
                        border_radius: RoundedCorners::Top.to_border_radius(PANE_BORDER_RADIUS),
                        ..default()
                    },
                    ThemeBackgroundColor(WINDOW_BG),
                    ThemeBorderColor::all(PANE_BG),
                ))
                .with_children(move |commands| {
                    let tabscroll = commands
                        .spawn((Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },))
                        .id();

                    let tabgroup = commands
                        .commands_mut()
                        .spawn((
                            ChildOf(tabscroll),
                            Node {
                                height: percent(100),
                                width: percent(100),
                                ..default()
                            },
                            Pickable {
                                should_block_lower: false,
                                ..default()
                            },
                            PaneTabgroup {
                                active_tab_index: 0,
                            },
                            PaneRef { entity: root },
                        ))
                        .observe(move |_: On<Remove, Children>, mut commands: Commands| {
                            commands.entity(root).despawn();
                        })
                        .id();

                    for tab in tabs {
                        spawn_tab(commands.commands_mut(), asset_server, root, tab)
                            .insert(ChildOf(tabgroup))
                            .insert(tab_context_menu())
                            .observe(on_tab_press)
                            .observe(on_tab_drag_start)
                            .observe(on_tab_drag)
                            .observe(on_tab_drag_end)
                            .observe(on_tab_drag_cancel);
                    }

                    let scrollbar = commands
                        .commands_mut()
                        .spawn((
                            ChildOf(tabscroll),
                            Pickable {
                                should_block_lower: false,
                                ..default()
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: px(0),
                                width: percent(100),
                                height: px(6),
                                ..default()
                            },
                        ))
                        .id();

                    let handle = commands
                        .commands_mut()
                        .spawn((
                            ChildOf(scrollbar),
                            Node {
                                width: px(8),
                                height: percent(100),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            Pickable {
                                should_block_lower: false,
                                ..default()
                            },
                            ThemeBackgroundColor(BUTTON_BG),
                        ))
                        .id();

                    commands.commands_mut().entity(scrollbar).insert(Scrollbar {
                        handle: Some(handle),
                    });

                    commands
                        .commands_mut()
                        .entity(tabscroll)
                        .insert(ScrollRect {
                            content: Some(tabgroup),
                            horizontal: true,
                            horizontal_srollbar: Some(scrollbar),
                            main_axis: ScrollAxis::Horizontal,
                            ..default()
                        });

                    let drop_indicator = commands
                        .commands()
                        .spawn((
                            ChildOf(tabscroll),
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

                    // Tab drop area
                    commands
                        .commands_mut()
                        .spawn((
                            ChildOf(tabscroll),
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
                        .observe(on_tabbar_drag_enter)
                        .observe(on_tabbar_drag_over)
                        .observe(on_tabbar_drag_drop)
                        .observe(on_tabbar_drag_leave);
                });
        })
        .id();

    let content = commands
        .spawn((
            ChildOf(area),
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(6)),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();

    commands
        .entity(root)
        .insert(PaneStructure { root, content });

    commands.entity(root)
}

fn tab_context_menu() -> ContextMenu {
    ContextMenu::new().with_option(true, ContextMenuMark::None, "Close", |world, tab| {
        world.commands().entity(tab).despawn();
        Ok(())
    })
}

#[derive(Component)]
struct DraggedTab {
    indicator: Entity,
    drop_index: usize,
}

fn on_tabbar_drag_enter(
    trigger: On<Pointer<DragEnter>>,
    tabbars: Query<&PaneTabbar>,
    dragged_tabs: Query<&DraggedTab>,
    mut commands: Commands,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(_) = dragged_tabs.get(trigger.dragged) else {
        return Ok(());
    };

    commands
        .entity(tabbar.drop_indicator)
        .entry::<Visibility>()
        .and_modify(|mut visibility| {
            *visibility = Visibility::Inherited;
        });

    Ok(())
}

fn on_tabbar_drag_over(
    trigger: On<Pointer<DragOver>>,
    tabbars: Query<&PaneTabbar>,
    children: Query<&Children>,
    scroll_positions: Query<&ScrollPosition>,
    computed_nodes: Query<&ComputedNode>,
    mut dragged_tabs: Query<&mut DraggedTab>,
    mut nodes: Query<&mut Node>,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(mut dragged_tab) = dragged_tabs.get_mut(trigger.dragged) else {
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
    dragged_tab.drop_index = 0;
    let tabs = children.get(tabbar.tabgroup)?;
    let mut dragged_tab_index = None;
    for (i, tab) in tabs.iter().enumerate() {
        let size = computed_nodes.get(*tab)?.size().x;
        if *tab == trigger.dragged {
            dragged_tab_index = Some(i);
        }

        if pointer_position < indicator_position + (size / 2.0) {
            break;
        }

        indicator_position += size;
        dragged_tab.drop_index += 1;
    }

    if let Some(dragged_tab_index) = dragged_tab_index {
        if dragged_tab.drop_index > dragged_tab_index {
            dragged_tab.drop_index = dragged_tab.drop_index.saturating_sub(1);
        }
    }

    indicator_position -= scroll_positions
        .get(trigger.entity)
        .map(|position| position.x)
        .unwrap_or_default();

    indicator.left = percent(indicator_position / tabbar_size * 100.0);

    Ok(())
}

fn on_tabbar_drag_drop(
    trigger: On<Pointer<DragDrop>>,
    tabbars: Query<&PaneTabbar>,
    dragged_tabs: Query<&DraggedTab>,
    mut commands: Commands,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    let Ok(dragged_tab) = dragged_tabs.get(trigger.dropped) else {
        return Ok(());
    };

    let drop_index = dragged_tab.drop_index;

    commands
        .entity(tabbar.tabgroup)
        .insert_child(drop_index, trigger.dropped)
        .entry::<PaneTabgroup>()
        .and_modify(move |mut tabgroup| tabgroup.active_tab_index = drop_index);

    Ok(())
}

fn on_tabbar_drag_leave(
    trigger: On<Pointer<DragLeave>>,
    tabbars: Query<&PaneTabbar>,
    mut commands: Commands,
) -> Result {
    let tabbar = tabbars.get(trigger.entity)?;

    commands
        .entity(tabbar.drop_indicator)
        .entry::<Visibility>()
        .and_modify(|mut visibility| {
            *visibility = Visibility::Hidden;
        });

    Ok(())
}

fn on_tab_press(
    trigger: On<Pointer<Press>>,
    parents: Query<&ChildOf>,
    mut tabgroups: Query<&mut PaneTabgroup>,
    children: Query<&Children>,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    let parent = parents.get(trigger.entity)?.parent();
    let tabs = children.get(parent)?;
    let index = tabs
        .iter()
        .position(|sibling| *sibling == trigger.entity)
        .unwrap();
    tabgroups.get_mut(parent)?.active_tab_index = index;

    Ok(())
}

fn on_tab_drag_start(
    trigger: On<Pointer<DragStart>>,
    editor_windows: Query<&EditorWindow>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tabs: Query<(&PaneTab, &PaneRef)>,
    mut override_cursor: ResMut<OverrideCursor>,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    let NormalizedRenderTarget::Window(window) = trigger.pointer_location.target else {
        return Ok(());
    };

    let Ok(editor_window) = editor_windows.get(window.entity()) else {
        return Ok(());
    };

    let (tab, pane) = tabs.get(trigger.entity)?;
    let indicator = spawn_tab(&mut commands, &asset_server, pane.entity, tab.name.clone())
        .insert(ChildOf(editor_window.root()))
        .insert(Pickable::IGNORE)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.border = UiRect::all(px(2));
            node.border_radius = RoundedCorners::All.to_border_radius(2.0);
        })
        .entity()
        .id();

    commands.entity(trigger.entity).insert(DraggedTab {
        indicator,
        drop_index: 0,
    });

    override_cursor.0 = Some(EntityCursor::System(SystemCursorIcon::Grabbing));

    Ok(())
}

fn on_tab_drag(
    trigger: On<Pointer<Drag>>,
    dragged_tabs: Query<&DraggedTab>,
    mut nodes: Query<&mut Node>,
    ui_scale: Res<UiScale>,
) -> Result {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return Ok(());
    };

    let mut indicator = nodes.get_mut(dragged_tab.indicator)?;

    indicator.left = px(trigger.pointer_location.position.x / ui_scale.0);
    indicator.top = px(trigger.pointer_location.position.y / ui_scale.0);

    Ok(())
}

fn on_tab_drag_end(
    trigger: On<Pointer<DragEnd>>,
    dragged_tabs: Query<&DraggedTab>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return;
    };

    commands.entity(dragged_tab.indicator).despawn();
    commands.entity(trigger.entity).remove::<DraggedTab>();
    override_cursor.0 = None;
}

fn on_tab_drag_cancel(
    trigger: On<Pointer<Cancel>>,
    dragged_tabs: Query<&DraggedTab>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return;
    };

    commands.entity(dragged_tab.indicator).despawn();
    commands.entity(trigger.entity).remove::<DraggedTab>();
    override_cursor.0 = None;
}

fn spawn_tab<'a>(
    commands: &'a mut Commands,
    asset_server: &AssetServer,
    pane: Entity,
    tab: String,
) -> EntityCommands<'a> {
    let root = commands
        .spawn((
            PaneRef { entity: pane },
            PaneTab { name: tab.clone() },
            Node {
                flex_shrink: 0.0,
                height: px(30),
                padding: UiRect::horizontal(px(8)),
                border: UiRect::top(px(2)),
                border_radius: RoundedCorners::Top.to_border_radius(2.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable {
                should_block_lower: false,
                ..default()
            },
            EntityCursor::System(SystemCursorIcon::Pointer),
            ThemeBackgroundColor(PANE_BG),
            ThemeBorderColor::all(PANE_TAB_ACTIVE),
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
        ThemeTextColor(TEXT_MAIN),
    ));

    commands.entity(root)
}

fn clamp_active_tab_index(
    tabgroups: Query<
        (&mut PaneTabgroup, &Children),
        Or<(Changed<PaneTabgroup>, Changed<Children>)>,
    >,
) {
    for (mut tabgroup, tabs) in tabgroups {
        tabgroup.active_tab_index = tabgroup.active_tab_index.clamp(0, tabs.len() - 1);
    }
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

fn register_pane_callbacks(world: &mut World) {
    world.resource_scope(|world, mut pane_registry: Mut<PaneRegistry>| {
        for (_, state) in &mut pane_registry.panes {
            if let Some(creation_callback) = state.creation_callback.take() {
                state.creation_system = Some(world.register_boxed_system(creation_callback));
            }
        }
    });
}

fn update_active_tab(
    tabgroups: Query<
        (&PaneRef, &PaneTabgroup, &Children),
        Or<(Changed<PaneTabgroup>, Changed<Children>)>,
    >,
    pane_registry: Res<PaneRegistry>,
    pane_tabs: Query<&PaneTab>,
    pane_structures: Query<&PaneStructure>,
    mut commands: Commands,
) -> Result {
    for (pane, tabgroup, tabs) in tabgroups {
        for (i, tab) in tabs.iter().enumerate() {
            let active = i == tabgroup.active_tab_index;
            commands.entity(*tab).insert((
                ThemeBackgroundColor(if active { PANE_BG } else { WINDOW_BG }),
                ThemeBorderColor::all(if active { PANE_TAB_ACTIVE } else { WINDOW_BG }),
            ));

            if active {
                let tab_name = &pane_tabs.get(*tab)?.name;
                let pane_structure = *pane_structures.get(pane.entity)?;
                commands.entity(pane_structure.content).despawn_children();

                if let Some(pane_state) = pane_registry.panes.get(tab_name) {
                    if let Some(creation_system) = pane_state.creation_system {
                        commands.run_system_with(creation_system, pane_structure);
                    }
                } else {
                    warn!("Missing tab pane: {}", tab_name);
                }
            }
        }
    }

    Ok(())
}

#[derive(Component, Clone, Copy)]
pub struct PaneStructure {
    pub root: Entity,
    pub content: Entity,
}

struct PaneState {
    name: String,
    creation_callback: Option<BoxedSystem<In<PaneStructure>>>,
    creation_system: Option<SystemId<In<PaneStructure>>>,
}

#[derive(Resource, Default)]
pub struct PaneRegistry {
    panes: HashMap<String, PaneState>,
}

impl PaneRegistry {
    pub fn register<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) {
        let name = name.into();
        if let Some(old) = self.panes.insert(
            name.clone(),
            PaneState {
                name: name.clone(),
                creation_callback: Some(Box::new(IntoSystem::into_system(system))),
                creation_system: None,
            },
        ) {
            warn!("'{}' pane replaced with {} pane.", old.name, name);
        }
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
        vec![
            "Scene Tree".into(),
            "Scene Tree".into(),
            "Scene Tree".into(),
            "Scene Tree".into(),
            "Scene Tree".into(),
        ],
    )
    .insert(ChildOf(sub_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(sub_divider));
    spawn_pane(
        &mut commands,
        &asset_server,
        0.6,
        vec![
            "Properties".into(),
            "Properties".into(),
            "Properties".into(),
            "Properties".into(),
            "Properties".into(),
        ],
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
