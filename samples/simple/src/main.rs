#[cfg(feature = "editor")]
mod editor;

use bevy::{
    app::{App, Plugin, Startup},
    asset::AssetServer,
    ecs::{
        error::Result,
        system::{Commands, Res},
    },
    log::info,
    prelude::bevy_main,
};
use bevy_editor::{
    EditorApp,
    asset::EditorAssetIdResolver,
    scene::{EditorScene, EditorSceneRoot, SceneList},
};

#[derive(Default)]
struct MyRuntimePlugin;

impl Plugin for MyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start);
    }
}

fn start(
    scene_list: Res<SceneList>,
    id_resolver: Res<EditorAssetIdResolver>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) -> Result {
    info!("The game begins");

    let scene_id = scene_list.scenes[0].clone();
    let scene_path = id_resolver.get_asset_path(scene_id)?;
    let scene = asset_server.load::<EditorScene>(scene_path);
    commands.spawn(EditorSceneRoot(scene));

    Ok(())
}

#[derive(Default)]
struct MySharedPlugin;

impl Plugin for MySharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, greet);
    }
}

fn greet() {
    info!("Hello, Bevy!");
}

#[bevy_main]
fn main() {
    let mut app = EditorApp::new()
        .shared_plugin(MySharedPlugin)
        .shared_plugin(MyRuntimePlugin);

    #[cfg(feature = "editor")]
    {
        app = app.editor_plugin(editor::MyEditorPlugin);
    }

    app.run();
}
