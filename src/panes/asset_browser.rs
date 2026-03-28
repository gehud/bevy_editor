use std::{
    path::PathBuf,
    time::Instant,
};

use bevy::{
    app::{App, Plugin, Update},
    asset::
        AssetServer
    ,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        message::MessageReader,
        observer::On,
        system::{Commands, In, Query, Res},
        world::Ref,
    },
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer},
    },
    text::TextLayout,
    ui::{
        AlignContent, AlignItems, FlexDirection, FlexWrap, JustifyContent, Node, Overflow,
        OverflowAxis, PositionType, UiRect, percent, px,
        widget::ImageNode,
    },
    utils::default,
};

use crate::{
    asset::{DatabaseRefresed, database::AssetDatabase},
    pane::{PaneApp, PaneStructure},
    theme::{
        RoundedCorners, ThemedBackgroundColor, tokens::{BUTTON_BG, PANE_BG, WINDOW_BG},
    },
    widget::{EditorText, ScrollArea},
};

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
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

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {
    commands.entity(pane.content()).with_children(|commands| {
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
                            padding: UiRect::horizontal(px(5)).with_top(px(4)).with_bottom(px(4)),
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
                        margin: UiRect::horizontal(px(12))
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
                            position_type: PositionType::Absolute,
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
    mut database_refresh_events: MessageReader<DatabaseRefresed>,
    browsers: Query<(Entity, Ref<AssetBrowser>)>,
    asset_server: Res<AssetServer>,
    asset_database: Res<AssetDatabase>,
    mut commands: Commands,
) -> Result {
    let database_regreshed = !database_refresh_events.is_empty();
    database_refresh_events.clear();

    for (browser_entity, browser) in browsers {
        if !browser.inspected_path.exists() {
            continue;
        }

        let should_update = database_regreshed || browser.is_changed();
        if !should_update {
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
                &asset_server,
                &asset_database,
                path,
                browser_entity,
                browser.content_root,
            )?;
        }
    }

    Ok(())
}

#[derive(Component)]
struct DirEntryButton {
    asset_path: PathBuf,
    last_click: Instant,
}

const DOUBLE_CLICK_SUBSEC_MILLIS: u32 = 250;

#[derive(Component)]
struct InspectLabeledAssetsButton {
    entry: Entity,
    inspecting: bool,
    indicator: Entity,
    roots: Vec<Entity>,
}

fn spawn_dir_entry(
    commands: &mut Commands,
    asset_server: &AssetServer,
    asset_database: &AssetDatabase,
    path: PathBuf,
    browser: Entity,
    container: Entity,
) -> Result {
    let is_dir = path.is_dir();

    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .unwrap_or_default();

    let asset_path = path.strip_prefix("./assets")?.to_path_buf();
    let has_labels = !asset_database
        .get_asset_labeled_uuids(asset_path.clone())?
        .is_empty();

    let entry = commands
        .spawn((
            DirEntryButton {
                asset_path,
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
                overflow: Overflow::hidden(),
                padding: UiRect::horizontal(px(5)).with_top(px(3)),
                ..default()
            },
            ThemedBackgroundColor::new(PANE_BG),
        ))
        .id();

    commands
        .entity(entry)
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemedBackgroundColor::new(BUTTON_BG));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemedBackgroundColor::new(PANE_BG));
        })
        .observe(
            move |trigger: On<Pointer<Click>>,
                  mut buttons: Query<&mut DirEntryButton>,
                  mut browsers: Query<&mut AssetBrowser>|
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
                    }
                }

                button.last_click = now;

                Ok(())
            },
        )
        .with_children(|commands| {
            commands
                .spawn((
                    Pickable::IGNORE,
                    Node {
                        width: px(30),
                        height: px(30),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::End,
                        ..default()
                    },
                    ImageNode::new(asset_server.load(if is_dir {
                        "embedded://bevy_editor//icons/folder.png"
                    } else {
                        "embedded://bevy_editor//icons/file.png"
                    })),
                ))
                .with_children(|commands| {
                    if has_labels {
                        let button = commands
                            .spawn((
                                Node {
                                    width: px(28),
                                    height: px(28),
                                    margin: UiRect::horizontal(px(-14)),
                                    border_radius: RoundedCorners::All.to_border_radius(14.0),
                                    padding: UiRect::all(px(4)),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                ThemedBackgroundColor::new(WINDOW_BG),
                            ))
                            .observe(on_inspect_labeled_assets_click)
                            .id();

                        let indicator = commands
                            .commands_mut()
                            .spawn((
                                ChildOf(button),
                                Pickable::IGNORE,
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    ..default()
                                },
                                ImageNode::new(
                                    asset_server
                                        .load("embedded://bevy_editor//icons/chevron_right.png"),
                                ),
                            ))
                            .id();

                        commands
                            .commands_mut()
                            .entity(button)
                            .insert(InspectLabeledAssetsButton {
                                entry,
                                inspecting: false,
                                indicator,
                                roots: Vec::new(),
                            });
                    }
                });

            commands.spawn((
                Pickable::IGNORE,
                TextLayout::new_with_no_wrap(),
                EditorText::new(file_name),
            ));
        });

    Ok(())
}

fn on_inspect_labeled_assets_click(
    trigger: On<Pointer<Click>>,
    dir_entry_buttons: Query<&DirEntryButton>,
    mut labeled_asset_buttons: Query<&mut InspectLabeledAssetsButton>,
    asset_database: Res<AssetDatabase>,
    asset_server: Res<AssetServer>,
    mut image_nodes: Query<&mut ImageNode>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut commands: Commands,
) -> Result {
    let mut labeled_asset_button = labeled_asset_buttons.get_mut(trigger.entity)?;
    labeled_asset_button.inspecting = !labeled_asset_button.inspecting;
    let mut indicator = image_nodes.get_mut(labeled_asset_button.indicator)?;

    indicator.image = asset_server.load(if labeled_asset_button.inspecting {
        "embedded://bevy_editor//icons/chevron_left.png"
    } else {
        "embedded://bevy_editor//icons/chevron_right.png"
    });

    if !labeled_asset_button.inspecting {
        for entity in &labeled_asset_button.roots {
            commands.entity(*entity).despawn();
        }

        labeled_asset_button.roots.clear();
    } else {
        let container = parents.get(labeled_asset_button.entry)?.parent();
        let dir_entry_button = dir_entry_buttons.get(labeled_asset_button.entry)?;
        let siblings = children.get(container)?;

        let mut insert_index = siblings
            .iter()
            .position(|sibling| *sibling == labeled_asset_button.entry)
            .unwrap()
            + 1;

        let labeled_assets =
            asset_database.get_asset_labeled_uuids(dir_entry_button.asset_path.clone())?;

        for (i, labeled_asset) in labeled_assets.iter().enumerate() {
            let labeled_path = asset_database.get_path_by_uuid(&labeled_asset)?;
            let label = labeled_path.label().unwrap_or_default();

            let is_last = i == labeled_assets.len() - 1;

            let entity =
                commands
                    .spawn((
                        Pickable::IGNORE,
                        Node {
                            width: px(74),
                            height: px(80),
                            padding: UiRect::vertical(px(5)),
                            ..default()
                        },
                    ))
                    .with_children(|commands| {
                        commands
                            .spawn((
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    row_gap: px(10),
                                    overflow: Overflow::hidden(),
                                    border_radius: RoundedCorners::Right
                                        .to_border_radius(if is_last { 5.0 } else { 0.0 }),
                                    padding: UiRect::horizontal(px(5)).with_top(px(3)),
                                    ..default()
                                },
                                ThemedBackgroundColor::new(WINDOW_BG),
                            ))
                            .with_children(|commands| {
                                commands.spawn((
                                    Pickable::IGNORE,
                                    Node {
                                        width: px(30),
                                        height: px(30),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::End,
                                        ..default()
                                    },
                                    ImageNode::new(
                                        asset_server.load("embedded://bevy_editor//icons/file.png"),
                                    ),
                                ));

                                commands.spawn((
                                    Pickable::IGNORE,
                                    TextLayout::new_with_no_wrap(),
                                    EditorText::new(label),
                                ));
                            });
                    })
                    .id();

            commands
                .entity(container)
                .insert_child(insert_index, entity);
            insert_index += 1;

            labeled_asset_button.roots.push(entity);
        }
    }

    Ok(())
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
            ThemedBackgroundColor::new(PANE_BG),
        ))
        .with_children(|commands| {
            commands.spawn(EditorText::new(file_name));
        })
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemedBackgroundColor::new(BUTTON_BG));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(ThemedBackgroundColor::new(PANE_BG));
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
            ThemedBackgroundColor::new(PANE_BG),
        ))
        .with_children(|commands| {
            commands.spawn((Pickable::IGNORE, EditorText::new("/")));
        });
}
