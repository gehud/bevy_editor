#[cfg(feature = "editor")]
pub mod editor;

use bevy::{
    DefaultPlugins,
    app::{App, AppExit, Plugin},
    prelude::bevy_main,
};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct RuntimePlugin;

impl Plugin for RuntimePlugin {
    fn build(&self, _app: &mut App) {}
}

#[bevy_main]
fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "editor")]
    {
        use bevy_editor::{EditorApp, EditorPlugins};

        use crate::editor::EditorPlugin;

        app.add_editor_plugins((EditorPlugins, EditorPlugin))
            .add_plugins(SharedPlugin)
            .add_runtime_plugins(RuntimePlugin);
    }
    #[cfg(not(feature = "editor"))]
    {
        app.add_plugins(SharedPlugin).add_plugins(RuntimePlugin);
    }

    app.run()
}
