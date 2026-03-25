use std::{
    env::{current_dir, current_exe},
    process::Command,
};

use bevy::{
    app::{App, Plugin, Update},
    asset::AssetServer,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::ChildOf,
        lifecycle::Add,
        observer::On,
        query::With,
        schedule::{IntoScheduleConfigs, common_conditions::resource_changed},
        system::{Commands, EntityCommands, Query, Res},
    },
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer},
    },
    ui::{
        AlignItems, JustifyContent, Node, UiRect, px,
        widget::{ImageNode, Text},
    },
    utils::default,
};

use crate::{
    PLAY_MODE_VAR,
    pane::{OpenPane, PaneRegistry},
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeTextFont,
        ThemeTextFontSize,
        tokens::{BORDER, BUTTON_BG, PANE_BG, TEXT_HEADING, TEXT_MAIN, WINDOW_BG},
    },
    widget::{ContextMenu, ContextMenuMark, MenuBar, MenuButton},
};

#[derive(Component)]
pub(crate) struct EditorMenuRoot;

pub struct EditorMenuPlugin;

impl Plugin for EditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_view_menu.run_if(resource_changed::<PaneRegistry>),
        )
        .add_observer(setup);
    }
}

fn setup(trigger: On<Add, EditorMenuRoot>, assets: Res<AssetServer>, mut commands: Commands) {
    commands.entity(trigger.entity).with_children(|commands| {
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
                                assets.load("embedded://bevy_editor/icons/bevy.png"),
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
                    .spawn((
                        MenuBar,
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(4),
                            ..default()
                        },
                    ))
                    .with_children(|commands| {
                        let bar = commands.target_entity();
                        spawn_menu_button(commands.commands_mut(), "View", bar)
                            .insert(ViewMenuButton);
                    });
            });

        commands
            .spawn(Node {
                align_items: AlignItems::Center,
                padding: UiRect::left(px(10)),
                ..default()
            })
            .with_children(|commands| {
                commands
                    .spawn((
                        Node {
                            height: px(22),
                            border: UiRect::all(px(1)),
                            border_radius: RoundedCorners::All.to_border_radius(5.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::horizontal(px(6)),
                            ..default()
                        },
                        ThemeBorderColor::all(BORDER),
                        ThemeBackgroundColor(BUTTON_BG),
                    ))
                    .with_children(|commands| {
                        commands.spawn((
                            Pickable::IGNORE,
                            Node {
                                width: px(13),
                                height: px(13),
                                ..default()
                            },
                            ImageNode::new(
                                assets.load("embedded://bevy_editor/icons/play.png"),
                            ),
                        ));
                    })
                    .observe(|_: On<Pointer<Click>>| -> Result {
                        let exe_path = current_exe()?;
                        let current_dir = current_dir()?;
                        Command::new(exe_path)
                            .current_dir(current_dir)
                            .env(PLAY_MODE_VAR, "true")
                            .spawn()?;
                        Ok(())
                    });
            });
    });
}

#[derive(Component)]
struct ViewMenuButton;

fn update_view_menu(
    pane_registry: Res<PaneRegistry>,
    view_menu_buttons: Query<Entity, With<ViewMenuButton>>,
    mut commands: Commands,
) {
    let Ok(view_menu_button) = view_menu_buttons.single() else {
        return;
    };

    let mut menu = ContextMenu::new();

    for name in pane_registry.names() {
        let name = name.clone();
        menu = menu.with_option(true, ContextMenuMark::None, name.clone(), move |world| {
            world
                .commands()
                .write_message(OpenPane { name: name.clone() });

            Ok(())
        });
    }

    commands.entity(view_menu_button).insert(MenuButton(menu));
}

fn spawn_menu_button<'a>(
    commands: &'a mut Commands,
    label: impl Into<String>,
    bar: Entity,
) -> EntityCommands<'a> {
    let root = commands
        .spawn((
            ChildOf(bar),
            Node {
                padding: UiRect::horizontal(px(8)).with_top(px(3)).with_bottom(px(3)),
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
                Text::new(label.into()),
                ThemeTextColor(TEXT_HEADING),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_HEADING),
            ));
        })
        .id();

    commands.entity(root)
}
