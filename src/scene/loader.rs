use bevy::{
    asset::{
        AssetLoader, AsyncWriteExt, LoadContext, io::Reader, processor::LoadTransformAndSave,
        saver::AssetSaver, transformer::IdentityAssetTransformer,
    },
    ecs::{
        reflect::AppTypeRegistry,
        world::{FromWorld, World},
    },
    reflect::{TypePath, TypeRegistryArc},
};
use serde::de::DeserializeSeed;
use thiserror::Error;

use crate::scene::{
    AssetScene,
    serde::{AssetSceneDeserializer, AssetSceneSerializer},
};

#[derive(Debug, Error)]
pub enum AssetSceneLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Error while trying to read the scene file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

#[derive(Debug, TypePath)]
pub struct AssetSceneLoader {
    type_registry: TypeRegistryArc,
}

impl FromWorld for AssetSceneLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.resource::<AppTypeRegistry>();
        AssetSceneLoader {
            type_registry: type_registry.0.clone(),
        }
    }
}

impl AssetLoader for AssetSceneLoader {
    type Asset = AssetScene;

    type Settings = ();

    type Error = AssetSceneLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes)?;
        let type_registry = self.type_registry.read();
        let scene_deserializer =
            AssetSceneDeserializer::new(&type_registry, load_context.asset_server());
        Ok(scene_deserializer
            .deserialize(&mut deserializer)
            .map_err(|e| deserializer.span_error(e))?)
    }

    fn extensions(&self) -> &[&str] {
        &["asn"]
    }
}
