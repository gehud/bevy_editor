use bevy::{
    app::{App, Plugin},
    utils::default,
    window::{Window, WindowPlugin},
};

pub struct EditorWindowPlugin;

impl Plugin for EditorWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".into(),
                ..default()
            }),
            ..default()
        });
    }
}
