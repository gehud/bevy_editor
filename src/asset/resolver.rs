use bevy::{
    app::{App, Plugin},
    asset::AssetPath,
    ecs::{
        error::Result,
        resource::Resource,
        world::{FromWorld, World},
    },
};

#[cfg(feature = "editor")]
use crate::asset::database::AssetDatabase;
use crate::asset::id::UntypedEditorAssetId;

#[derive(Resource)]
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
            let Some(path) = self.asset_database.get_path(&id.uuid)? else {
                return Ok(AssetPath::default());
            };

            if path
                .extension()
                .map(|extension| extension.to_string_lossy().to_string())
                .unwrap_or_default()
                != id.extension
            {
                return Ok(AssetPath::default());
            }

            let mut asset_path = AssetPath::from(path);

            if let Some(label) = id.label {
                asset_path = asset_path.with_label(label);
            }

            Ok(asset_path)
        }
        #[cfg(not(feature = "editor"))]
        {
            Ok(id.asset_path())
        }
    }
}

pub struct EditorAssetResolverPlugin;

impl Plugin for EditorAssetResolverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorAssetIdResolver>();
    }
}
