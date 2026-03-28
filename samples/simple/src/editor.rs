use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, In},
};

use bevy_editor::{
    pane::{PaneApp, PaneStructure},
    widget::EditorText,
};

pub struct MyEditorPlugin;

impl Plugin for MyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Sample", setup);
    }
}

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {
    commands.entity(pane.content()).with_children(|commands| {
        commands.spawn(EditorText::new("Hello, Bevy Editor!"));
    });
}
