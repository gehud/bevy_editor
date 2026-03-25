use bevy::{
    app::{App, Plugin},
    ecs::system::{Commands, In},
    ui::widget::Text,
};

use bevy_editor::{
    pane::{PaneStructure, PaneApp},
    theme::{ThemeTextColor, ThemeTextFont, ThemeTextFontSize, tokens::TEXT_MAIN},
};

pub struct MyEditorPlugin;

impl Plugin for MyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Sample", setup);
    }
}

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {
    commands
        .entity(pane.content())
        .with_children(|commands| {
            commands.spawn((
                Text::new("Hello, Bevy Editor!"),
                ThemeTextFont(TEXT_MAIN),
                ThemeTextFontSize(TEXT_MAIN),
                ThemeTextColor(TEXT_MAIN),
            ));
        });
}
