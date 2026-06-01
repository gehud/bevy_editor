use bevy::{
    app::{App, Plugin, PostUpdate},
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::Add,
        observer::On,
        query::{Changed, With},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, In, IntoSystem, Query, ResMut, SystemId},
        template::FromTemplate,
    },
    log::warn,
    picking::{
        events::{Cancel, Drag, DragEnd, DragStart, Pointer},
        pointer::PointerButton,
    },
    platform::collections::HashMap,
    scene::{CommandsSceneExt, Scene, SceneList, bsn, bsn_list, on},
    ui::{
        AlignItems, BackgroundColor, ComputedNode, FlexDirection, Node, Overflow,
        UiGlobalTransform, UiRect, UiSystems, Val, percent, px,
    },
    ui_widgets::ControlOrientation,
    utils::default,
    window::SystemCursorIcon,
};

use crate::{
    cursor::{EntityCursor, OverrideCursor},
    panel::PanelStructure,
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedBorderColor,
        tokens::{PANEL_BODY_BG, WINDOW_BG},
    },
    widget::{
        scroll::{EditorScrollArea, ScrollAxes},
        text::{EditorText, EditorTextStyle},
    },
};

const BORDER_RADIUS: Val = Val::Px(6.0);
const RESIZE_HANDLE_SIZE: f32 = 4.0;
const MIN_PANE_SIZE: f32 = 100.0;

#[derive(Clone, Component, Copy, Default, Eq, PartialEq)]
enum Divider {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Component, Copy, Default)]
struct Size(pub f32);

fn divider(divider: Divider, size: f32) -> impl Scene {
    bsn! {
        Node {
            flex_direction: {
                match divider {
                    Divider::Horizontal => FlexDirection::Row,
                    Divider::Vertical => FlexDirection::Column,
                }
            }
        }
        Size({size})
        Divider::from(divider)
    }
}

#[derive(Clone, Component, Copy, Default, FromTemplate)]
struct ResizeHandle {
    divider: Divider,
}

#[derive(Resource, Default)]
struct ResizeHandleDragState {
    parent_node_size: f32,
}

fn resize_handle(divider: Divider) -> impl Scene {
    bsn! {
        Node {
            flex_shrink: 0.0,
            width: {
                match divider {
                    Divider::Horizontal => px(RESIZE_HANDLE_SIZE),
                    Divider::Vertical => percent(100),
                }
            },
            height: {
                match divider {
                    Divider::Horizontal => percent(100),
                    Divider::Vertical => px(RESIZE_HANDLE_SIZE),
                }
            },
        }
        EntityCursor::System({
            match divider {
                Divider::Horizontal => SystemCursorIcon::EwResize,
                Divider::Vertical => SystemCursorIcon::NsResize,
            }
        })
        ResizeHandle {
            divider: divider
        }
        on(resize_handle_drag_start)
        on(resize_handle_drag)
        on(resize_handle_drag_end)
        on(resize_handle_drag_canel)
    }
}

fn resize_handle_drag_start(
    trigger: On<Pointer<DragStart>>,
    mut drag_state: ResMut<ResizeHandleDragState>,
    parents: Query<&ChildOf>,
    entity_cursors: Query<&EntityCursor>,
    resize_handles: Query<&ResizeHandle>,
    computed_nodes: Query<&ComputedNode>,
    mut override_cursor: ResMut<OverrideCursor>,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    let target = trigger.event_target();
    let entity_cursor = entity_cursors.get(target)?.clone();
    let resize_handle = resize_handles.get(target)?.clone();

    override_cursor.0 = Some(entity_cursor);
    let parent = parents.get(target)?.parent();

    let parent_node_size = computed_nodes.get(parent)?.size();
    let parent_node_size = match resize_handle.divider {
        Divider::Horizontal => parent_node_size.x,
        Divider::Vertical => parent_node_size.y,
    };

    drag_state.parent_node_size = parent_node_size;

    Ok(())
}

fn resize_handle_drag(
    trigger: On<Pointer<Drag>>,
    drag_state: ResMut<ResizeHandleDragState>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    resize_handles: Query<&ResizeHandle>,
    ui_global_transforms: Query<&UiGlobalTransform>,
    mut sizes: Query<&mut Size>,
) -> Result {
    let target = trigger.event_target();
    let parent = parents.get(target)?.parent();
    let siblings = children.get(parent)?;
    let resize_handle = resize_handles.get(target)?.clone();

    let index = siblings
        .iter()
        .position(|entity| *entity == target)
        .unwrap();

    let min_size = MIN_PANE_SIZE / drag_state.parent_node_size;

    let pointer_position = match resize_handle.divider {
        Divider::Horizontal => trigger.pointer_location.position.x,
        Divider::Vertical => trigger.pointer_location.position.y,
    };

    let handle_position = match resize_handle.divider {
        Divider::Horizontal => ui_global_transforms.get(target)?.translation.x,
        Divider::Vertical => ui_global_transforms.get(target)?.translation.y,
    };

    let delta = match resize_handle.divider {
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
}

fn resize_handle_drag_end(_: On<Pointer<DragEnd>>, mut override_cursor: ResMut<OverrideCursor>) {
    override_cursor.0 = None;
}

fn resize_handle_drag_canel(_: On<Pointer<Cancel>>, mut override_cursor: ResMut<OverrideCursor>) {
    override_cursor.0 = None;
}

fn panel(size: f32, tabs: Vec<String>) -> impl Scene {
    assert!(tabs.len() != 0, "Cannot spawn pane without tabs");

    bsn! {
        Node
        Size(size)
        PanelStructure {
            content: #Content
        }
        Children [
            #Body
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                border_radius: {RoundedCorners::All.to_border_radius(BORDER_RADIUS)},
            }
            ThemedBackgroundColor::new(PANEL_BODY_BG)
            Children [
                #Header
                Node {
                    width: percent(100),
                    height: px(30),
                    padding: {UiRect::left(px(6)).with_right(px(8))},
                    border: {UiRect::horizontal(px(1)).with_top(px(1))},
                    flex_shrink: 0.0,
                    border_radius: {RoundedCorners::Top.to_border_radius(BORDER_RADIUS)},
                }
                ThemedBackgroundColor::new(WINDOW_BG)
                ThemedBorderColor::all(PANEL_BODY_BG)
                Children [
                    :EditorScrollArea {
                        axes: ScrollAxes::HORIZONTAL,
                        orientation: ControlOrientation::Horizontal,
                        @content: {bsn! {
                            Node {
                                height: percent(100)
                            }
                            Children [
                                {tab_list(tabs)}
                            ]
                        }}
                    }
                    Node {
                        flex_grow: 1.0
                    }
                ],
                #Content
                Node {
                    flex_grow: 1.0
                }
            ]
        ]
    }
}

fn tab(name: String) -> impl Scene {
    bsn! {
        Node {
            height: percent(100),
            padding: UiRect::horizontal(px(8)),
            align_items: AlignItems::Center,
            border_radius: {RoundedCorners::Top.to_border_radius(px(2))}
        }
        ThemedBackgroundColor(PANEL_BODY_BG)
        Children [
            :EditorText {
                @text: name
            }
        ]
    }
}

fn tab_list(tabs: Vec<String>) -> impl SceneList {
    let mut scenes = Vec::new();

    for name in tabs {
        scenes.push(tab(name));
    }

    scenes
}

fn remove_dividers(
    children: Query<&Children>,
    resize_handles: Query<(), With<ResizeHandle>>,
    mut sizes: Query<&mut Size>,
    mut dividers: Query<(Entity, &Children, &ChildOf), (Changed<Children>, With<Divider>)>,
    mut commands: Commands,
) -> Result {
    for (entity, parts, parent) in dividers.iter_mut() {
        let mut parts = parts
            .iter()
            .filter(|child| !resize_handles.contains(**child));

        let Some(first) = parts.next().cloned() else {
            commands.entity(entity).despawn();
            continue;
        };

        if parts.next().is_some() {
            continue;
        }

        let size = sizes.get(entity)?.0;
        sizes.get_mut(first)?.0 = size;

        let siblings = children.get(parent.parent())?;
        let index = siblings
            .iter()
            .position(|sibling| *sibling == entity)
            .unwrap();

        commands
            .entity(parent.parent())
            .insert_children(index, &[first]);

        commands.entity(entity).despawn();
    }

    Ok(())
}

fn apply_size(
    dividers: Query<&Divider>,
    parents: Query<&ChildOf>,
    mut nodes: Query<(Entity, &Size, &mut Node), Changed<Size>>,
) -> Result {
    for (entity, size, mut node) in nodes.iter_mut() {
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

fn default_layout() -> impl Scene {
    bsn! {
        divider(Divider::Horizontal, 1.0)
        Children [
            panel(0.5, vec!["Left".into(), "Left".into()]),
            resize_handle(Divider::Horizontal),
            panel(0.5, vec!["Right".into(), "Right".into()]),
        ]
    }
}

#[derive(Clone, Component, Copy, Default)]
pub struct PanelArea;

#[derive(Clone, Component, Copy, Default)]
pub(crate) struct PanelAreaRoot;

pub fn setup_area(trigger: On<Add, PanelArea>, mut commands: Commands) {
    let area = trigger.event_target();

    commands.entity(area).with_children(|commands| {
        let layout = commands.commands_mut().spawn_scene(default_layout()).id();
        commands
            .spawn((
                PanelAreaRoot,
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ))
            .add_child(layout);
    });
}

pub struct EditorPanelLayoutPlugin;

impl Plugin for EditorPanelLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ResizeHandleDragState>()
            .add_systems(
                PostUpdate,
                (remove_dividers, apply_size)
                    .chain()
                    .before(UiSystems::Layout),
            )
            .add_observer(setup_area);
    }
}
