use std::{
    env::{consts::EXE_EXTENSION, current_dir},
    fs::{DirEntry, read_dir},
    path::PathBuf,
    time::{Duration, Instant},
};

use bevy::{
    app::{App, Plugin, Update},
    asset::{AssetLoader, AssetPath, AssetServer, LoadContext, io::{AssetReader, AssetSource, AssetSourceId, file::FileAssetReader}},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::ChildOf,
        lifecycle::Add,
        observer::On,
        query::Changed,
        system::{Commands, In, Query, Res},
    },
    log::info,
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer},
    },
    tasks::block_on,
    ui::{
        AlignContent, AlignItems, FlexDirection, FlexWrap, JustifyContent, Node, Overflow,
        OverflowAxis, UiRect, percent, px,
        widget::{ImageNode, Text},
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
                            padding: UiRect::horizontal(px(12))
                                .with_top(px(8))
                                .with_bottom(px(8)),
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
                                overflow: Overflow {
                                    y: OverflowAxis::Scroll,
                                    x: OverflowAxis::Hidden,
                                },
                                flex_wrap: FlexWrap::Wrap,
                                align_content: AlignContent::Start,
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
                        inspected_path: "./assets".into(),
                    });
                });
        });
}

fn update_browser(
    browsers: Query<(Entity, &AssetBrowser), Changed<AssetBrowser>>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) -> Result {
    for (browser_entity, browser) in browsers {
        if !browser.inspected_path.exists() {
            continue;
        }

        commands
            .entity(browser.path_root)
            .despawn_children()
            .with_children(|commands| {
                let container = commands.target_entity();

                let mut path = Some(browser.inspected_path.as_path());
                while let Some(path_entry) = path {
                    if !spawn_path_component(
                        commands.commands_mut(),
                        browser_entity,
                        container,
                        path_entry.to_path_buf(),
                    ) {
                        break;
                    }

                    spawn_path_separator(commands.commands_mut(), container);
                    path = path_entry.parent();
                }
            });

        commands.entity(browser.content_root).despawn_children();

        for entry in browser.inspected_path.read_dir()? {
            let path = entry?.path();
            spawn_dir_entry(
                &mut commands,
                &assets,
                path,
                browser_entity,
                browser.content_root,
            );
        }
    }

    Ok(())
}

#[derive(Component)]
struct DirEntryButton {
    last_click: Instant,
}

const DOUBLE_CLICK_SUBSEC_MILLIS: u32 = 250;

fn spawn_dir_entry(
    commands: &mut Commands,
    assets: &AssetServer,
    path: PathBuf,
    browser: Entity,
    container: Entity,
) {
    let is_dir = path.is_dir();

    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default();

    commands
        .spawn((
            DirEntryButton {
                last_click: Instant::now(),
            },
            ChildOf(container),
            Node {
                width: px(74),
                height: px(80),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(10),
                border_radius: RoundedCorners::All.to_border_radius(5.0),
                padding: UiRect::horizontal(px(5)).with_top(px(3)),
                ..default()
            },
            ThemeBackgroundColor(PANE_BG),
        ))
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
            move |trigger: On<Pointer<Click>>,
                  mut buttons: Query<&mut DirEntryButton>,
                  mut browsers: Query<&mut AssetBrowser>,
                  assets: Res<AssetServer>|
                  -> Result {
                let mut button = buttons.get_mut(trigger.entity)?;

                let now = Instant::now();
                let last_click = button.last_click;
                let is_double_click =
                    (now - last_click).subsec_millis() <= DOUBLE_CLICK_SUBSEC_MILLIS;

                if is_double_click {
                    if is_dir {
                        let mut browser = browsers.get_mut(browser)?;
                        browser.inspected_path = path.clone();
                    } else {
                    }
                }

                button.last_click = now;

                Ok(())
            },
        )
        .with_children(|commands| {
            commands.spawn((
                Pickable::IGNORE,
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
                ImageNode::new(assets.load(if is_dir {
                    "embedded://bevy_editor/assets/pane/icons/folder.png"
                } else {
                    "embedded://bevy_editor/assets/pane/icons/file.png"
                })),
            ));

            commands.spawn((
                Pickable::IGNORE,
                Text::new(file_name),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_MAIN),
                ThemeTextColor(TEXT_MAIN),
            ));
        });
}

fn spawn_path_component(
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

fn spawn_path_separator(commands: &mut Commands, container: Entity) {
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
