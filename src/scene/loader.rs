use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::{
        reflect::AppTypeRegistry,
        world::{FromWorld, World},
    },
    reflect::{TypePath, TypeRegistryArc},
};
use serde::de::DeserializeSeed;
use thiserror::Error;

use crate::{
    scene::{EditorScene, serde::de::SceneDeserializer},
    serde::de::EditorDeserializerProcessor,
};

#[derive(Debug, Error)]
pub enum EditorSceneLoaderError {
    /// An [IO Error](std::io::Error)
    #[error("Error while trying to read the scene file: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON Error](ron::error::SpannedError)
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

#[derive(TypePath)]
pub struct EditorSceneLoader {
    type_registry: TypeRegistryArc,
    processor: EditorDeserializerProcessor,
}

impl FromWorld for EditorSceneLoader {
    fn from_world(world: &mut World) -> Self {
        EditorSceneLoader {
            processor: EditorDeserializerProcessor::from_world(world),
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl AssetLoader for EditorSceneLoader {
    type Asset = EditorScene;

    type Settings = ();

    type Error = EditorSceneLoaderError;

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
        let mut processor = self.processor.clone();
        let scene_deserializer = SceneDeserializer::new(&type_registry, &mut processor);
        Ok(scene_deserializer
            .deserialize(&mut deserializer)
            .map_err(|e| deserializer.span_error(e))?)
    }

    fn extensions(&self) -> &[&str] {
        &["asn"]
    }
}
