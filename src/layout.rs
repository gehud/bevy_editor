use bevy::{
    app::{App, Plugin, Startup},
    ecs::{
        lifecycle::Add,
        observer::On,
        query::With,
        system::{Commands, Single},
    },
    window::PrimaryWindow,
};

use crate::window::{EditorWindowStructure, PrimaryEditorWindowConfigured};

fn setup(
    trigger: On<PrimaryEditorWindowConfigured>,
    primary_window: Single<&EditorWindowStructure, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let root = primary_window.root();

    commands.entity(root).with_children(|commands| {});
}

pub struct EditorLayoutPlugin;

impl Plugin for EditorLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(setup);
    }
}
