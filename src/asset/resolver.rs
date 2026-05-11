use bevy::{
    app::{App, Plugin},
    asset::AssetPath,
    ecs::{
        error::Result,
        resource::Resource,
        world::{FromWorld, World},
    },
    reflect::TypePath,
};

#[cfg(feature = "editor")]
use crate::asset::database::AssetDatabase;
use crate::asset::id::UntypedEditorAssetId;

#[derive(Clone, TypePath, Resource)]
pub struct EditorAssetIdResolver {
    #[cfg(feature = "editor")]
    asset_database: AssetDatabase,
}

impl FromWorld for EditorAssetIdResolver {
    fn from_world(world: &mut World) -> Self {
        Self {
            #[cfg(feature = "editor")]
            asset_database: world.resource::<AssetDatabase>().clone(),
        }
    }
}

impl EditorAssetIdResolver {
    pub fn get_asset_path<'a>(&self, id: impl Into<UntypedEditorAssetId>) -> Result<AssetPath<'a>> {
        let id = id.into();
        #[cfg(feature = "editor")]
        {
            Ok(self
                .asset_database
                .get_asset_path(&id.uuid)?
                .unwrap_or_default())
        }
        #[cfg(not(feature = "editor"))]
        {
            use std::fs;

            use bevy::asset::io::file::FileAssetReader;

            let map_dir = FileAssetReader::get_base_path().join(MAP_PATH);
            let asset_path = fs::read_to_string(map_dir.join(id.uuid.to_string()))?;
            Ok(AssetPath::from(asset_path))
        }
    }
}

pub(crate) const MAP_PATH: &'static str = "imported_assets/Map";

pub struct EditorAssetResolverPlugin;

impl Plugin for EditorAssetResolverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorAssetIdResolver>();
    }
}
