use std::path::PathBuf;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::ChildOf,
        lifecycle::Add,
        observer::On,
        query::Changed,
        system::{Commands, In, Query},
    },
    log::info,
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer},
    },
    ui::{
        AlignItems, FlexDirection, JustifyContent, Node, Overflow, UiRect, percent, px,
        widget::Text,
    },
    utils::default,
};

use crate::{
    pane::{PaneStructure, RegisterPane},
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeTextColor, ThemeTextFont, ThemeTextFontSize,
        tokens::{BUTTON_BG, PANE_BG, TEXT_MAIN},
    },
    widget::ScrollArea,
};

pub struct AssetBrowserPanePlugin;

impl Plugin for AssetBrowserPanePlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Asset Browser", setup)
            .add_systems(Update, update_browser);
    }
}

#[derive(Component)]
struct AssetBrowser {
    path_root: Entity,
    content_root: Entity,
    inspected_path: PathBuf,
}

fn setup(In(pane_structure): In<PaneStructure>, mut commands: Commands) {
    commands
        .entity(pane_structure.content)
        .with_children(|commands| {
            commands
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                ))
                .with_children(|commands| {
                    let root = commands.target_entity();

                    let path_container = commands
                        .spawn((
                            Pickable::IGNORE,
                            Node {
                                width: percent(100),
                                height: px(34),
                                padding: UiRect::horizontal(px(5))
                                    .with_top(px(4))
                                    .with_bottom(px(4)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .id();

                    let path = commands
                        .commands_mut()
                        .spawn((
                            Pickable::IGNORE,
                            ChildOf(path_container),
                            Node {
                                flex_direction: FlexDirection::RowReverse,
                                ..default()
                            },
                        ))
                        .id();

                    let area = commands
                        .spawn(Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        })
                        .id();

                    let content = commands
                        .commands_mut()
                        .spawn((
                            Pickable::IGNORE,
                            ChildOf(area),
                            Node {
                                width: percent(100),
                                height: percent(100),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ))
                        .id();

                    commands.commands_mut().entity(area).insert(ScrollArea {
                        target: content,
                        vertical: true,
                        ..default()
                    });

                    commands.commands_mut().entity(root).insert(AssetBrowser {
                        path_root: path,
                        content_root: content,
                        inspected_path: "./assets/a/b".into(),
                    });
                });
        });
}

fn update_browser(
    browsers: Query<(Entity, &mut AssetBrowser), Changed<AssetBrowser>>,
    mut commands: Commands,
) -> Result {
    for (browser_entity, mut browser) in browsers {
        commands
            .entity(browser.path_root)
            .despawn_children()
            .with_children(|commands| {
                let container = commands.target_entity();

                let mut path = Some(browser.inspected_path.as_path());
                while let Some(path_entry) = path {
                    if !spawn_dir_button(
                        commands.commands_mut(),
                        browser_entity,
                        container,
                        path_entry.to_path_buf(),
                    ) {
                        break;
                    }

                    spawn_dir_separator(commands.commands_mut(), container);
                    path = path_entry.parent();
                }
            });
    }

    Ok(())
}

fn spawn_dir_button(
    commands: &mut Commands,
    browser: Entity,
    container: Entity,
    path: PathBuf,
) -> bool {
    let Some(file_name) = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy())
    else {
        return false;
    };

    commands
        .spawn((
            ChildOf(container),
            Node {
                padding: UiRect::horizontal(px(6)).with_top(px(4)).with_bottom(px(4)),
                border_radius: RoundedCorners::All.to_border_radius(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ThemeBackgroundColor(PANE_BG),
        ))
        .with_children(|commands| {
            commands.spawn((
                Text::new(file_name),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_MAIN),
                ThemeTextColor(TEXT_MAIN),
            ));
        })
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(BUTTON_BG));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemeBackgroundColor(PANE_BG));
        })
        .observe(
            move |_: On<Pointer<Click>>, mut browsers: Query<&mut AssetBrowser>| -> Result {
                browsers.get_mut(browser)?.inspected_path = path.clone();
                Ok(())
            },
        );

    true
}

fn spawn_dir_separator(commands: &mut Commands, container: Entity) {
    commands
        .spawn((
            ChildOf(container),
            Node {
                padding: UiRect::horizontal(px(4)),
                border_radius: RoundedCorners::All.to_border_radius(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ThemeBackgroundColor(PANE_BG),
        ))
        .with_children(|commands| {
            commands.spawn((
                Pickable::IGNORE,
                Text::new("/"),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_MAIN),
                ThemeTextColor(TEXT_MAIN),
            ));
        });
}
