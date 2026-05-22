use bevy::{
    app::{App, Plugin, PostUpdate},
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
        system::{Commands, In, IntoSystem, Query, SystemId},
        template::FromTemplate,
    },
    log::warn,
    picking::events::{DragStart, Pointer},
    platform::collections::HashMap,
    scene::{CommandsSceneExt, Scene, bsn, on},
    ui::{AlignItems, FlexDirection, Node, UiRect, UiSystems, Val, percent, px},
    utils::default,
    window::SystemCursorIcon,
};

use crate::{
    cursor::EntityCursor,
    panel::PanelStructure,
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedBorderColor,
        tokens::{PANEL_BODY_BG, WINDOW_BG},
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

#[derive(Clone, Component, Copy, Default)]
struct ResizeHandle;

fn resize_handle(divider: Divider) -> impl Scene {
    let cursor_icon = match divider {
        Divider::Horizontal => SystemCursorIcon::EwResize,
        Divider::Vertical => SystemCursorIcon::NsResize,
    };

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
        EntityCursor::System(cursor_icon)
        ResizeHandle
        on(resize_handle_drag_start)
    }
}

fn resize_handle_drag_start(trigger: On<Pointer<DragStart>>) {}

fn panel<'a>(size: f32, tabs: Vec<String>) -> impl Scene {
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
            ThemedBackgroundColor(PANEL_BODY_BG)
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
                ThemedBackgroundColor::new(WINDOW_BG),
                ThemedBorderColor::all(PANEL_BODY_BG),
                #Content
                Node {
                    flex_grow: 1.0
                }
            ]
        ]
    }
}

fn remove_dividers(
    children: Query<&Children>,
    resize_handles: Query<(), With<ResizeHandle>>,
    mut sizes: Query<&mut Size>,
    mut dividers: Query<(Entity, &Children, &ChildOf), (Changed<Children>, With<Divider>)>,
    mut commands: Commands,
) -> Result {
    // for (entity, parts, parent) in dividers.iter_mut() {
    //     let mut iter = parts
    //         .iter()
    //         .filter(|child| !resize_handles.contains(**child));

    //     let child = iter.next().unwrap();
    //     if iter.next().is_some() {
    //         continue;
    //     }

    //     let size = sizes.get(entity)?.0;
    //     sizes.get_mut(*child)?.0 = size;

    //     let siblings = children.get(parent.parent())?;
    //     let index = siblings.iter().position(|s| *s == entity).unwrap();

    //     commands
    //         .entity(parent.parent())
    //         .insert_children(index, &[*child]);

    //     commands.entity(entity).despawn();
    // }

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
            panel(0.5, vec!["Left".into()]),
            resize_handle(Divider::Horizontal),
            panel(0.5, vec!["Right".into()]),
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
        app.add_systems(
            PostUpdate,
            (remove_dividers, apply_size)
                .chain()
                .before(UiSystems::Layout),
        )
        .add_observer(setup_area);
    }
}
