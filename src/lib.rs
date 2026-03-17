pub mod pane;
pub mod theme;
pub mod widget;
pub mod window;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    color::Color,
    ecs::{
        error::Result,
        observer::On,
        system::{Commands, Query, ResMut},
    },
    input_focus::{InputFocus, tab_navigation::TabIndex},
    picking::hover::Hovered,
    text::TextColor,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, OverrideClip,
        PositionType, UiRect, percent, px,
        widget::{Text, TextShadow},
    },
    ui_widgets::{
        MenuItem, MenuLayout, MenuPopup,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
    utils::default,
    window::{Window, WindowPlugin},
};

use crate::{
    pane::{PaneLayoutRoot, PanePlugin},
    theme::{
        ThemeBackgroundColor, ThemeBorderColor, EditorThemePlugin, Theme,
        constants::fonts::REGULAR,
        tokens::{BORDER, WINDOW_BG},
    },
    widget::WidgetPlugins,
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
        .add_plugins(WidgetPlugins)
        .add_plugins(PanePlugin)
        .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryWindowConfigured>,
    editor_windows: Query<&EditorWindow>,
    mut windows: Query<&mut IsWindowMaximized>,
    mut commands: Commands,
) -> Result {
    windows.get_mut(trigger.entity)?.0 = true;

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

    Ok(())
}
