pub mod loader;
pub mod serde;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, UntypedAssetId, VisitAssetDependencies},
    ecs::world::FromWorld,
    platform::collections::HashSet,
    reflect::TypePath,
    scene::DynamicScene,
    utils::default,
};

use loader::AssetSceneLoader;

#[derive(TypePath)]
pub struct AssetScene {
    pub scene: DynamicScene,
    pub(crate) dependencies: HashSet<UntypedAssetId>,
}

impl Asset for AssetScene {}

impl VisitAssetDependencies for AssetScene {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        for dependency in &self.dependencies {
            visit(*dependency);
        }
    }
}

pub struct AssetScenePlugin;

impl Plugin for AssetScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AssetScene>()
            .init_asset_loader::<AssetSceneLoader>();
    }
}
