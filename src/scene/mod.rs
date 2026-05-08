pub mod loader;
pub mod serde;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, UntypedAssetId, VisitAssetDependencies},
    platform::collections::HashSet,
    reflect::TypePath,
    scene::DynamicScene,
};

use loader::EditorSceneLoader;

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

pub struct EditorScenePlugin;

impl Plugin for EditorScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<EditorScene>()
            .init_asset_loader::<EditorSceneLoader>();
    }
}
