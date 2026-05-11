#[cfg(feature = "editor")]
mod editor;

use bevy::{
    app::{App, Plugin, Startup, Update}, asset::{AssetEvent, AssetId, AssetServer, Assets, Handle}, ecs::{
        error::Result,
        message::MessageReader,
        resource::Resource,
        system::{Commands, Res, ResMut},
    }, image::{Image, ImageLoaderSettings, ImageSamplerDescriptor}, log::info, platform::collections::HashMap, prelude::bevy_main, scene::{DynamicScene, DynamicSceneRoot}
};
use bevy_editor::{
    EditorApp,
    asset::resolver::EditorAssetIdResolver,
    scene::{EditorScene, SceneList},
};

#[derive(Default)]
struct MyRuntimePlugin;

impl Plugin for MyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenesToSpawn>()
            .add_systems(Startup, start)
            .add_systems(Update, spawn_scene);
    }
}

fn spawn_scene(
    mut editor_scene_events: MessageReader<AssetEvent<EditorScene>>,
    mut editor_scenes: ResMut<Assets<EditorScene>>,
    mut scenes: ResMut<Assets<DynamicScene>>,
    mut scenes_to_spawn: ResMut<ScenesToSpawn>,
    mut commands: Commands,
) {
    for event in editor_scene_events.read() {
        match event {
            AssetEvent::Added { id } => {
                let editor_scene = editor_scenes.remove(*id).unwrap();
                commands.spawn(DynamicSceneRoot(scenes.add(editor_scene.scene)));
                scenes_to_spawn.0.remove(id);
                info!("Scene loaded");
            }
            _ => {}
        }
    }
}

#[derive(Default, Resource)]
struct ScenesToSpawn(#[allow(unused)] HashMap<AssetId<EditorScene>, Handle<EditorScene>>);

fn start(
    scene_list: Res<SceneList>,
    id_resolver: Res<EditorAssetIdResolver>,
    asset_server: Res<AssetServer>,
    mut scenes_to_spawn: ResMut<ScenesToSpawn>,
) -> Result {
    info!("The game begins");

    let scene_id = scene_list.scenes[0].clone();
    let scene_path = id_resolver.get_asset_path(scene_id)?;
    let scene_handle = asset_server.load::<EditorScene>(scene_path);
    scenes_to_spawn.0.insert(scene_handle.id(), scene_handle);

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
        .runtime_plugin(MyRuntimePlugin);

    #[cfg(feature = "editor")]
    {
        app = app.editor_plugin(editor::MyEditorPlugin);
    }

    app.run();
}
