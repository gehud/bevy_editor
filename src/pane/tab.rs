use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    asset::AssetServer,
    camera::{NormalizedRenderTarget, visibility::Visibility},
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::{Changed, Or},
        system::{Commands, EntityCommands, Query, Res, ResMut},
        world::Ref,
    },
    input_focus::{InputFocus, IsFocused, IsFocusedHelper},
    log::warn,
    picking::{
        Pickable,
        events::{
            Cancel, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, Pointer,
            Press,
        },
        pointer::{PointerButton, PointerLocation, PointerMap},
    },
    text::TextFont,
    ui::{
        AlignItems, ComputedNode, Node, PositionType, ScrollPosition, UiRect, UiScale, percent, px,
        widget::Text,
    },
    utils::default,
    window::SystemCursorIcon,
};

use crate::{
    pane::{PaneStructure, PaneRef, PaneRegistry},
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedBorderColor, ThemedTextColor,
        constants::fonts::REGULAR,
        tokens::{PANE_BG, PANE_TAB_ACTIVE, TEXT_MAIN, WINDOW_BG},
    },
    widget::{ContextMenu, ContextMenuMark, EntityCursor, OverrideCursor},
    window::{EditorWindowAutoFocus, EditorWindowStructure},
};

#[derive(Component)]
pub(super) struct PaneTab {
    pub name: String,
}

#[derive(Component)]
pub(super) struct PaneTabbar {
    pub drop_indicator: Entity,
    pub tabgroup: Entity,
}

#[derive(Component)]
pub(super) struct PaneTabgroup {
    pub active_tab_index: usize,
}

#[derive(Component)]
pub(super) struct DraggedTab {
    pub indicator: Entity,
    pub drop_index: usize,
}

pub(super) fn on_tabbar_drag_enter(
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

pub(super) fn on_tabbar_drag_over(
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

pub(super) fn on_tabbar_drag_drop(
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
        .and_modify(move |mut tabgroup| {
            if tabgroup.active_tab_index != drop_index {
                tabgroup.active_tab_index = drop_index;
            }
        });

    Ok(())
}

pub(super) fn on_tabbar_drag_leave(
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

pub(super) fn on_tab_press(
    trigger: On<Pointer<Press>>,
    parents: Query<&ChildOf>,
    mut tabgroups: Query<&mut PaneTabgroup>,
    children: Query<&Children>,
    mut focus: ResMut<InputFocus>,
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
    let mut tabgroup = tabgroups.get_mut(parent)?;
    if tabgroup.active_tab_index != index {
        tabgroup.active_tab_index = index;
    }
    focus.set(trigger.entity);

    Ok(())
}

pub(super) fn on_tab_drag_start(
    trigger: On<Pointer<DragStart>>,
    editor_windows: Query<&EditorWindowStructure>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tabs: Query<(&PaneTab, &PaneRef)>,
    mut auto_focus: ResMut<EditorWindowAutoFocus>,
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
        .insert((
            ThemedBackgroundColor::new(PANE_BG),
            ThemedBorderColor::all(PANE_TAB_ACTIVE),
        ))
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

    auto_focus.0 = true;
    override_cursor.0 = Some(EntityCursor::System(SystemCursorIcon::Grabbing));

    Ok(())
}

pub(super) fn on_tab_drag(
    trigger: On<Pointer<Drag>>,
    dragged_tabs: Query<&DraggedTab>,
    pointer_map: Res<PointerMap>,
    pointers: Query<&PointerLocation>,
    editor_windows: Query<&EditorWindowStructure>,
    parents: Query<&ChildOf>,
    mut nodes: Query<&mut Node>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) -> Result {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return Ok(());
    };

    // The drag event uses the initial drag location.
    // To correctly update the position, we need the current pointer location.
    let Some(location) = pointers
        .get(pointer_map.get_entity(trigger.pointer_id).unwrap())?
        .location
        .clone()
    else {
        return Ok(());
    };

    let NormalizedRenderTarget::Window(window_ref) = location.target else {
        return Ok(());
    };

    let editor_window = editor_windows.get(window_ref.entity())?;

    let indicator_root = parents.get(dragged_tab.indicator)?.parent();

    if indicator_root != editor_window.root() {
        commands
            .entity(editor_window.root())
            .add_child(dragged_tab.indicator);
    }

    let mut indicator = nodes.get_mut(dragged_tab.indicator)?;

    indicator.left = px(location.position.x / ui_scale.0);
    indicator.top = px(location.position.y / ui_scale.0);

    Ok(())
}

pub(super) fn on_tab_drag_end(
    trigger: On<Pointer<DragEnd>>,
    dragged_tabs: Query<&DraggedTab>,
    mut auto_focus: ResMut<EditorWindowAutoFocus>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return;
    };

    commands.entity(dragged_tab.indicator).despawn();
    commands.entity(trigger.entity).remove::<DraggedTab>();
    override_cursor.0 = None;
    auto_focus.0 = false;
}

pub(super) fn on_tab_drag_cancel(
    trigger: On<Pointer<Cancel>>,
    dragged_tabs: Query<&DraggedTab>,
    mut auto_focus: ResMut<EditorWindowAutoFocus>,
    mut commands: Commands,
    mut override_cursor: ResMut<OverrideCursor>,
) {
    let Ok(dragged_tab) = dragged_tabs.get(trigger.entity) else {
        return;
    };

    commands.entity(dragged_tab.indicator).despawn();
    commands.entity(trigger.entity).remove::<DraggedTab>();
    override_cursor.0 = None;
    auto_focus.0 = false;
}

pub(super) fn spawn_tab<'a>(
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
                height: px(29),
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
            ThemedBackgroundColor::new(WINDOW_BG),
            ThemedBorderColor::all(WINDOW_BG),
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
        ThemedTextColor::new(TEXT_MAIN),
    ));

    commands.entity(root)
}

pub(super) fn tab_context_menu(tab: Entity) -> ContextMenu {
    ContextMenu::new().with_option(true, ContextMenuMark::None, "Close", move |world| {
        world.commands().entity(tab).despawn();
        Ok(())
    })
}

fn focus_tabs(
    focus: IsFocusedHelper,
    input_focus: Res<InputFocus>,
    tabgroups: Query<(&PaneRef, Ref<PaneTabgroup>, Ref<Children>)>,
    pane_registry: Res<PaneRegistry>,
    pane_tabs: Query<&PaneTab>,
    pane_structures: Query<&PaneStructure>,
    mut commands: Commands,
) -> Result {
    for (pane, tabgroup, tabs) in tabgroups {
        let pane_structure = pane_structures.get(pane.entity)?;
        let is_tabgroup_changed = tabgroup.is_changed() || tabs.is_changed();

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = i == tabgroup.active_tab_index;

            if is_tabgroup_changed || input_focus.is_changed() {
                let is_focused =
                    focus.is_focused(*tab) || focus.is_focus_within(pane_structure.content());

                commands
                    .entity(*tab)
                    .insert(ThemedBackgroundColor::new(if is_active {
                        PANE_BG
                    } else {
                        WINDOW_BG
                    }))
                    .insert(ThemedBorderColor::all(if is_active {
                        if is_focused { PANE_TAB_ACTIVE } else { PANE_BG }
                    } else {
                        WINDOW_BG
                    }));
            }

            if is_active && is_tabgroup_changed {
                let tab_name = &pane_tabs.get(*tab)?.name;

                commands.entity(pane_structure.content()).despawn_children();

                if let Some(system) = pane_registry.get(tab_name) {
                    commands.run_system_with(system, *pane_structure);
                } else {
                    warn!("Missing tab pane: {}", tab_name);
                }
            }
        }
    }

    Ok(())
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

pub struct PaneTabPlugin;

impl Plugin for PaneTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clamp_active_tab_index)
            .add_systems(PostUpdate, focus_tabs);
    }
}
