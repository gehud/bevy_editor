use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    utils::default,
    window::{Window, WindowPlugin},
};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                ..default()
            }),
            ..default()
        }));
    }
}
