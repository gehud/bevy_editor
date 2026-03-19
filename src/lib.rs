pub mod pane;
pub mod theme;
pub mod widget;
pub mod window;

use std::fmt::Debug;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    asset::{AssetServer, embedded_asset},
    camera::NormalizedRenderTarget,
    color::Color,
    ecs::{
        entity::ContainsEntity,
        error::Result,
        hierarchy::ChildOf,
        observer::On,
        system::{Commands, EntityCommands, Query, Res, ResMut, SystemParam},
    },
    input_focus::{InputFocus, tab_navigation::TabIndex},
    picking::{
        events::{Out, Over, Pointer},
        hover::Hovered,
    },
    reflect::Reflect,
    render::RenderPlugin,
    text::TextColor,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, OverrideClip,
        PositionType, UiRect, UiScale, percent, px,
        widget::{ImageNode, Text, TextShadow},
    },
    ui_widgets::{
        MenuItem, MenuLayout, MenuPopup,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
    utils::default,
    window::{Window, WindowPlugin},
};

use crate::{
    pane::{
        EditorPanePlugin, PaneLayoutRoot,
        panes::{SceneTreePanePlugin, ViewportPanePlugin},
    },
    theme::{
        EditorThemePlugin, RoundedCorners, Theme, ThemeBackgroundColor, ThemeBorderColor,
        ThemeTextColor, ThemeTextFont, ThemeTextFontSize,
        constants::fonts::REGULAR,
        tokens::{BORDER, PANE_BG, TEXT_HEADING, TEXT_MAIN, WINDOW_BG},
    },
    widget::{ContextMenu, EditorWidgetPlugins, MenuButton},
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
        .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryWindowConfigured>,
    editor_windows: Query<&EditorWindow>,
    assets: Res<AssetServer>,
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
            commands
                .spawn(Node {
                    column_gap: px(15),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|commands| {
                    commands
                        .spawn(Node {
                            padding: UiRect::left(px(12)),
                            align_items: AlignItems::Center,
                            column_gap: px(6),
                            ..default()
                        })
                        .with_children(|commands| {
                            commands.spawn((
                                Node {
                                    width: px(20),
                                    height: px(20),
                                    ..default()
                                },
                                ImageNode::new(
                                    assets
                                        .load("embedded://bevy_editor/assets/theme/icons/bevy.png"),
                                ),
                            ));

                            commands.spawn((
                                Text::new("Bevy"),
                                ThemeTextColor(TEXT_HEADING),
                                ThemeTextFont(TEXT_HEADING),
                                ThemeTextFontSize(TEXT_HEADING),
                            ));
                        });

                    // Menu
                    commands
                        .spawn(Node {
                            align_items: AlignItems::Center,
                            column_gap: px(4),
                            ..default()
                        })
                        .with_children(|commands| {
                            let menu = commands.target_entity();
                            spawn_menu_button(commands.commands_mut(), "File", ContextMenu::new())
                                .insert(ChildOf(menu));
                            spawn_menu_button(commands.commands_mut(), "Edit", ContextMenu::new())
                                .insert(ChildOf(menu));
                        });
                });
        });

    Ok(())
}

fn spawn_menu_button<'a>(
    commands: &'a mut Commands,
    label: impl Into<String>,
    menu: ContextMenu,
) -> EntityCommands<'a> {
    let root = commands
        .spawn((
            Node {
                padding: UiRect::horizontal(px(8)).with_top(px(4)).with_bottom(px(4)),
                border_radius: RoundedCorners::All.to_border_radius(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ThemeBackgroundColor(WINDOW_BG),
        ))
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(PANE_BG));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(WINDOW_BG));
        })
        .with_children(|commands| {
            commands.spawn((
                MenuButton(menu),
                Text::new(label.into()),
                ThemeTextColor(TEXT_HEADING),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_HEADING),
            ));
        })
        .id();

    commands.entity(root)
}
