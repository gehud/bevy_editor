pub mod pane;
pub mod theme;
pub mod widget;
pub mod window;

mod menu;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    asset::AssetServer,
    ecs::{
        error::Result,
        observer::On,
        system::{Commands, Query, Res, ResMut},
    },
    ui::{AlignItems, FlexDirection, Node, UiRect, UiScale, percent, px},
    utils::default,
    window::{Window, WindowPlugin},
};

use crate::{
    menu::{EditorMenuPlugin, EditorMenuRoot},
    pane::{
        EditorPanePlugin, PaneLayoutRoot,
        panes::{SceneTreePanePlugin, ViewportPanePlugin},
    },
    theme::EditorThemePlugin,
    widget::EditorWidgetPlugins,
    window::{EditorWindow, EditorWindowPlugin, IsWindowMaximized, PrimaryWindowConfigured},
};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                transparent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EditorThemePlugin)
        .add_plugins(EditorWindowPlugin)
        .add_plugins(EditorWidgetPlugins)
        .add_plugins(EditorPanePlugin)
        .add_plugins(SceneTreePanePlugin)
        .add_plugins(ViewportPanePlugin)
        .add_plugins(EditorMenuPlugin)
        .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryWindowConfigured>,
    editor_windows: Query<&EditorWindow>,
    mut windows: Query<&mut IsWindowMaximized>,
    mut commands: Commands,
    mut ui_scale: ResMut<UiScale>,
) -> Result {
    // ui_scale.0 = 1.5;
    // windows.get_mut(trigger.entity)?.0 = true;

    let editor_window = editor_windows.get(trigger.entity)?;

    commands
        .entity(editor_window.content())
        .with_children(|commands| {
            commands
                .spawn(Node {
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::horizontal(px(4)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|commands| {
                    // Panes
                    commands.spawn((
                        PaneLayoutRoot,
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                    ));

                    // Footer
                    commands.spawn(Node {
                        width: percent(100),
                        height: px(24),
                        padding: UiRect::horizontal(px(8)),
                        ..default()
                    });
                });
        });

    commands
        .entity(editor_window.titlebar())
        .with_children(|commands| {
            commands.spawn((
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                EditorMenuRoot,
            ));
        });

    Ok(())
}
