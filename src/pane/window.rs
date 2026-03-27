use bevy::{
    app::{App, Plugin, Update},
    asset::AssetServer,
    camera::visibility::Visibility,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        lifecycle::Remove,
        message::{Message, MessageReader},
        observer::On,
        schedule::IntoScheduleConfigs,
        system::{Commands, EntityCommands, Query, Res, ResMut},
    },
    input_focus::InputFocus,
    picking::{
        Pickable,
        events::{Pointer, Press},
    },
    ui::{FlexDirection, Node, Overflow, PositionType, UiRect, percent, px},
    utils::default,
    window::Window,
};

use crate::{
    pane::{
        PaneRegistrySystems, PaneTab, PaneTabbar, PaneTabgroup, Size, on_tab_drag,
        on_tab_drag_cancel, on_tab_drag_end, on_tab_drag_start, on_tab_press, on_tabbar_drag_drop,
        on_tabbar_drag_enter, on_tabbar_drag_leave, on_tabbar_drag_over, spawn_tab,
        tab_context_menu,
    },
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedBorderColor,
        tokens::{BUTTON_BG, PANE_BG, PANE_TAB_ACTIVE, WINDOW_BG},
    },
    widget::{EntityContextMenu, ScrollArea, ScrollAxis, Scrollbar, ScrollbarThumb},
    window::{EditorWindowConfigured, EditorWindowStructure},
};

const PANE_BORDER_RADIUS: f32 = 6.0;

#[derive(Message)]
pub struct OpenPane {
    pub name: String,
}

#[derive(Component)]
pub(crate) struct PaneLayoutRoot;

#[derive(Component)]
pub(super) struct PaneRef {
    pub entity: Entity,
}

pub(super) fn spawn_pane<'a>(
    commands: &'a mut Commands,
    focus: &mut InputFocus,
    assets: &AssetServer,
    size: f32,
    tabs: Vec<String>,
    focus_first: bool,
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
            ThemedBackgroundColor::new(PANE_BG),
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
                    ThemedBackgroundColor::new(WINDOW_BG),
                    ThemedBorderColor::all(PANE_BG),
                ))
                .with_children(move |commands| {
                    let scrollrect = commands
                        .spawn((Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },))
                        .id();

                    let tabgroup = commands
                        .commands_mut()
                        .spawn((
                            ChildOf(scrollrect),
                            Node {
                                height: percent(100),
                                width: percent(100),
                                overflow: Overflow::scroll_x(),
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
                        .observe(
                            move |_: On<Remove, Children>,
                                  entities: Query<Entity>,
                                  mut commands: Commands| {
                                if entities.contains(root) {
                                    commands.entity(root).despawn();
                                }
                            },
                        )
                        .id();

                    commands
                        .commands_mut()
                        .entity(scrollrect)
                        .insert(ScrollArea {
                            target: tabgroup,
                            vertical: false,
                            main_axis: ScrollAxis::Horizontal,
                            ..default()
                        });

                    let mut is_first_tab = true;
                    for tab in tabs {
                        let id = spawn_tab(commands.commands_mut(), assets, root, tab)
                            .insert(ChildOf(tabgroup))
                            .observe(on_tab_press)
                            .observe(on_tab_drag_start)
                            .observe(on_tab_drag)
                            .observe(on_tab_drag_end)
                            .observe(on_tab_drag_cancel)
                            .id();

                        if focus_first && is_first_tab {
                            focus.set(id);
                        }

                        commands
                            .commands_mut()
                            .entity(id)
                            .insert(EntityContextMenu(tab_context_menu(id)));

                        is_first_tab = false;
                    }

                    let drop_indicator = commands
                        .commands()
                        .spawn((
                            ChildOf(scrollrect),
                            Node {
                                position_type: PositionType::Absolute,
                                height: percent(100),
                                width: px(3),
                                margin: UiRect::horizontal(px(-1)),
                                left: px(0),
                                ..default()
                            },
                            Pickable::IGNORE,
                            ThemedBackgroundColor::new(PANE_TAB_ACTIVE),
                            Visibility::Hidden,
                        ))
                        .id();

                    commands
                        .commands_mut()
                        .spawn((
                            ChildOf(scrollrect),
                            Scrollbar {
                                target: tabgroup,
                                axis: ScrollAxis::Horizontal,
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
                        .with_children(|commands| {
                            commands.spawn((
                                ScrollbarThumb,
                                Node {
                                    width: px(8),
                                    height: percent(100),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                ThemedBackgroundColor::new(BUTTON_BG),
                            ));
                        });

                    // Tab drop area
                    commands
                        .commands_mut()
                        .spawn((
                            ChildOf(scrollrect),
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

    let content_origin = commands
        .spawn((
            Pickable::IGNORE,
            ChildOf(area),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .id();

    let content = commands
        .spawn((
            ChildOf(content_origin),
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                overflow: Overflow::hidden(),
                ..default()
            },
        ))
        .observe(
            |trigger: On<Pointer<Press>>, mut focus: ResMut<InputFocus>| {
                focus.set(trigger.entity);
            },
        )
        .id();

    commands
        .entity(root)
        .insert(PaneStructure { root, content });

    commands.entity(root)
}

#[derive(Component, Clone, Copy)]
pub struct PaneStructure {
    root: Entity,
    content: Entity,
}

impl PaneStructure {
    pub fn content(&self) -> Entity {
        self.content
    }
}

#[derive(Component)]
struct SpawnedPaneWindow {
    name: String,
}

fn open_panes(
    mut requests: MessageReader<OpenPane>,
    pane_tabs: Query<&PaneTab>,
    mut commands: Commands,
) {
    for request in requests.read() {
        if pane_tabs.iter().any(|tab| tab.name == request.name) {
            // TODO: Focus tab
            continue;
        }

        commands.spawn((
            SpawnedPaneWindow {
                name: request.name.clone(),
            },
            Window {
                title: request.name.clone(),
                transparent: true,
                decorations: false,
                ..default()
            },
        ));
    }
}

fn setup_pane_window(
    trigger: On<EditorWindowConfigured>,
    spawned_pane_windows: Query<&SpawnedPaneWindow>,
    editor_windows: Query<&EditorWindowStructure>,
    assets: Res<AssetServer>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) -> Result {
    let target = trigger.entity;

    let Ok(spawned_pane_window) = spawned_pane_windows.get(target) else {
        return Ok(());
    };

    let editor_window = editor_windows.get(trigger.entity)?;

    commands
        .entity(editor_window.content())
        .with_children(|commands| {
            commands
                .spawn(Node {
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::horizontal(px(4)).with_bottom(px(4)),
                    ..default()
                })
                .with_children(|commands| {
                    let root = commands.target_entity();
                    spawn_pane(
                        commands.commands_mut(),
                        &mut focus,
                        &assets,
                        1.0,
                        vec![spawned_pane_window.name.clone()],
                        true,
                    )
                    .insert(ChildOf(root));
                })
                .observe(
                    move |_: On<Remove, Children>,
                          entities: Query<Entity>,
                          mut commands: Commands| {
                        if entities.contains(target) {
                            commands.entity(target).despawn();
                        }
                    },
                );
        });

    commands.entity(target).remove::<SpawnedPaneWindow>();

    Ok(())
}

pub struct PaneWindowPlugin;

impl Plugin for PaneWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenPane>()
            .add_systems(Update, open_panes.after(PaneRegistrySystems::Registration))
            .add_observer(setup_pane_window);
    }
}
