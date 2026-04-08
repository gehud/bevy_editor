pub mod asset;
pub mod gizmo;
pub mod pane;
pub mod selection;
pub mod theme;
pub mod widget;
pub mod window;

mod assets;
mod menu;
mod panes;

use std::env;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    asset::AssetPlugin,
    ecs::{
        error::Result,
        observer::On,
        system::{Commands, Query},
    },
    picking::Pickable,
    ui::{FlexDirection, JustifyContent, Node, UiRect, percent, px},
    utils::default,
    window::WindowPlugin,
};

use crate::{
    asset::EditorAssetPlugin,
    assets::EditorAssetsPlugin,
    gizmo::EditorGizmoPlugin,
    menu::{EditorMenuPlugin, EditorMenuRoot},
    pane::PaneLayoutRoot,
    panes::EditorPanePlugins,
    selection::EditorSelectionPlugin,
    theme::EditorThemePlugin,
    widget::EditorWidgetPlugins,
    window::{EditorWindowPlugin, EditorWindowStructure, PrimaryEditorWindowConfigured},
};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorWindowPlugin)
            .add_plugins(EditorAssetPlugin)
            .add_plugins(
                DefaultPlugins
                    .build()
                    .disable::<WindowPlugin>()
                    .disable::<AssetPlugin>(),
            )
            .add_plugins(EditorGizmoPlugin)
            .add_plugins(EditorAssetsPlugin)
            .add_plugins(EditorSelectionPlugin)
            .add_plugins(EditorThemePlugin)
            .add_plugins(EditorWidgetPlugins)
            .add_plugins(EditorPanePlugins)
            .add_plugins(EditorMenuPlugin)
            .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryEditorWindowConfigured>,
    mut editor_windows: Query<&EditorWindowStructure>,
    mut commands: Commands,
    // mut ui_scale: bevy::ecs::system::ResMut<bevy::ui::UiScale>,
) -> Result {
    // ui_scale.0 = 1.5;

    let editor_window = editor_windows.get_mut(trigger.entity)?;

    commands
        .entity(editor_window.titlebar())
        .with_children(|commands| {
            commands.spawn((
                EditorMenuRoot,
                Pickable::IGNORE,
                Node {
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
            ));
        });

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
                        flex_shrink: 0.0,
                        padding: UiRect::horizontal(px(8)),
                        ..default()
                    });
                });
        });

    Ok(())
}

pub const PLAY_MODE_VAR: &'static str = "BEVY_EDITOR_PLAY";

pub fn is_play_mode() -> bool {
    let Ok(var) = env::var(PLAY_MODE_VAR) else {
        return false;
    };

    let Ok(value) = var.parse::<bool>() else {
        return false;
    };

    value
}
