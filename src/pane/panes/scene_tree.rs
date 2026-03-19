use bevy::{
    app::{App, Plugin, Startup},
    ecs::system::{Commands, In},
    ui::widget::Text,
};

use crate::{
    pane::{PaneStructure, RegisterPane},
    theme::{ThemeTextColor, ThemeTextFont, ThemeTextFontSize, tokens::TEXT_MAIN},
};

pub struct SceneTreePanePlugin;

impl Plugin for SceneTreePanePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_scene)
            .register_pane("Scene Tree", setup);
    }
}

fn spawn_scene() {}

fn setup(In(pane_structure): In<PaneStructure>, mut commands: Commands) {
    commands
        .entity(pane_structure.content)
        .with_children(|commands| {
            commands.spawn((
                Text::new("Scene tree content"),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_MAIN),
                ThemeTextColor(TEXT_MAIN),
            ));
        });
}
