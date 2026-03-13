mod window;

use bevy::{
    feathers::{
        FeathersPlugins,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens::WINDOW_BG,
    },
    prelude::*,
    winit::WinitPlugin,
};

use crate::window::{DecoratedWindow, DecoratedWindowPlugin, PrimaryWindowDecorated};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                transparent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DecoratedWindowPlugin)
        .add_plugins(FeathersPlugins)
        .insert_resource(UiTheme(create_dark_theme()))
        .add_observer(setup);
    }
}

fn setup(
    trigger: On<PrimaryWindowDecorated>,
    decorated_windows: Query<&DecoratedWindow>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
) -> Result {
    windows.get_mut(trigger.entity)?.set_maximized(true);

    let decorated_window = decorated_windows.get(trigger.entity)?;

    commands
        .entity(decorated_window.content())
        .with_children(|commands| {

        });

    Ok(())
}
