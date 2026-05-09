pub mod loader;
pub mod serde;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, Handle, UntypedAssetId, VisitAssetDependencies},
    ecs::{reflect::ReflectResource, resource::Resource},
    platform::collections::HashSet,
    reflect::{Reflect, TypePath, std_traits::ReflectDefault},
    scene::{DynamicScene, Scene},
};

use loader::EditorSceneLoader;

use crate::{
    asset::{EditorAssetApp, EditorAssetId},
    settings::{ReflectSettings, Settings},
};

#[derive(TypePath)]
pub struct EditorScene {
    pub scene: DynamicScene,
    pub(crate) dependencies: HashSet<UntypedAssetId>,
}

impl Asset for EditorScene {}

impl VisitAssetDependencies for EditorScene {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for dependency in &self.dependencies {
            visit(*dependency);
        }
    }
}

#[derive(Default, Resource, Reflect)]
#[reflect(Default, Resource, Settings)]
pub struct SceneList {
    scenes: Vec<EditorAssetId<EditorScene>>,
}

impl Settings for SceneList {
    fn title(&self) -> &str {
        "Scene List"
    }
}

pub struct EditorScenePlugin;

impl Plugin for EditorScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EditorScene>()
            .register_editor_asset::<Scene>()
            .register_editor_asset::<DynamicScene>()
            .register_editor_asset::<EditorScene>()
            .init_asset_loader::<EditorSceneLoader>();
    }
}
