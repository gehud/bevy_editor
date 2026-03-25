mod layout;
mod registry;
mod tab;
mod window;

use layout::*;
pub use registry::*;
pub use tab::*;
pub use window::*;

use bevy::{
    app::{App, Plugin},
    asset::AssetServer,
    ecs::{
        hierarchy::ChildOf,
        lifecycle::Add,
        observer::On,
        system::{Commands, Res, ResMut},
    },
    input_focus::InputFocus,
};

fn setup_default_layout(
    trigger: On<Add, PaneLayoutRoot>,
    asset_server: Res<AssetServer>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let root = trigger.entity;

    let divider = spawn_divider(&mut commands, Divider::Horizontal, 1.)
        .insert(ChildOf(root))
        .id();

    let sub_divider = spawn_divider(&mut commands, Divider::Vertical, 0.2)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(
        &mut commands,
        &mut focus,
        &asset_server,
        0.4,
        vec!["Scene Tree".into()],
        false,
    )
    .insert(ChildOf(sub_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(sub_divider));
    spawn_pane(
        &mut commands,
        &mut focus,
        &asset_server,
        0.6,
        vec!["Properties".into()],
        false,
    )
    .insert(ChildOf(sub_divider));

    spawn_resize_handle(&mut commands, Divider::Horizontal).insert(ChildOf(divider));

    let asset_browser_divider = spawn_divider(&mut commands, Divider::Vertical, 0.8)
        .insert(ChildOf(divider))
        .id();

    spawn_pane(
        &mut commands,
        &mut focus,
        &asset_server,
        0.70,
        vec!["Viewport".into()],
        false,
    )
    .insert(ChildOf(asset_browser_divider));
    spawn_resize_handle(&mut commands, Divider::Vertical).insert(ChildOf(asset_browser_divider));
    spawn_pane(
        &mut commands,
        &mut focus,
        &asset_server,
        0.30,
        vec!["Asset Browser".into()],
        false,
    )
    .insert(ChildOf(asset_browser_divider));
}

pub struct EditorPanePlugin;

impl Plugin for EditorPanePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PaneRegistryPlugin)
            .add_plugins(PaneLayoutPlugin)
            .add_plugins(PaneWindowPlugin)
            .add_plugins(PaneTabPlugin)
            .add_observer(setup_default_layout);
    }
}
