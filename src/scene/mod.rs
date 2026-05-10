pub mod loader;
pub mod serde;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, UntypedAssetId, VisitAssetDependencies},
    ecs::{reflect::ReflectResource, resource::Resource},
    platform::collections::HashSet,
    reflect::{
        Reflect, ReflectDeserialize, ReflectSerialize, TypePath, std_traits::ReflectDefault,
    },
    scene::{DynamicScene, Scene},
};

use loader::EditorSceneLoader;

use crate::{
    asset::{EditorAssetApp, id::EditorAssetId},
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
    pub scenes: Vec<EditorAssetId<EditorScene>>,
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
            .register_type::<std::ops::Range<f32>>()
            .register_type_data::<std::ops::Range<f32>, ReflectSerialize>()
            .register_type_data::<std::ops::Range<f32>, ReflectDeserialize>()
            .register_type::<std::ops::Range<f64>>()
            .register_type_data::<std::ops::Range<f64>, ReflectSerialize>()
            .register_type_data::<std::ops::Range<f64>, ReflectDeserialize>()
            .register_type::<std::ops::RangeInclusive<f32>>()
            .register_type_data::<std::ops::RangeInclusive<f32>, ReflectSerialize>()
            .register_type_data::<std::ops::RangeInclusive<f32>, ReflectDeserialize>()
            .register_type::<std::ops::RangeInclusive<f64>>()
            .register_type_data::<std::ops::RangeInclusive<f64>, ReflectSerialize>()
            .register_type_data::<std::ops::RangeInclusive<f64>, ReflectDeserialize>()
            .init_asset_loader::<EditorSceneLoader>();
    }
}
