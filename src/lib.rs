pub mod pane;
pub mod widget;
mod window;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    ecs::{
        error::Result,
        hierarchy::ChildOf,
        observer::On,
        system::{Commands, Query},
    },
    ui::{FlexDirection, Node, UiRect, percent, px},
    utils::default,
    window::{Window, WindowPlugin},
};

use crate::{
    pane::{PaneLayoutRoot, PanePlugin},
    widget::{WidgetPlugins, dark_theme::create_dark_theme, theme::UiTheme},
    window::{DecoratedWindow, DecoratedWindowPlugin, IsWindowMaximized, PrimaryWindowDecorated},
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
        .add_plugins(DecoratedWindowPlugin)
        .add_plugins(WidgetPlugins)
        .add_plugins(PanePlugin)
        .insert_resource(UiTheme(create_dark_theme()))
        .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryWindowDecorated>,
    decorated_windows: Query<&DecoratedWindow>,
    mut windows: Query<&mut IsWindowMaximized>,
    mut commands: Commands,
) -> Result {
    windows.get_mut(trigger.entity)?.0 = true;

    let decorated_window = decorated_windows.get(trigger.entity)?;

    commands
        .entity(decorated_window.content())
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

    Ok(())
}
