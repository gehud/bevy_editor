use bevy::{
    asset::{AssetServer, ReflectHandle, UntypedAssetId, UntypedHandle},
    ecs::{
        reflect::AppTypeRegistry,
        world::{FromWorld, World},
    },
    platform::collections::HashSet,
    reflect::{
        PartialReflect, TypePath, TypeRegistration, TypeRegistry,
        serde::ReflectDeserializerProcessor, std_traits::ReflectDefault,
    },
};
use serde::{Deserialize, de::Error};

use crate::{
    asset::{id::UntypedEditorAssetId, resolver::EditorAssetIdResolver},
    serde::EditorAssetHandle,
};

#[derive(Clone, TypePath)]
pub struct EditorDeserializerProcessor {
    pub asset_server: AssetServer,
    pub type_registry: AppTypeRegistry,
    pub asset_id_resolver: EditorAssetIdResolver,
    pub asset_collector: HashSet<UntypedAssetId>,
}

impl FromWorld for EditorDeserializerProcessor {
    fn from_world(world: &mut World) -> Self {
        Self {
            asset_server: world.resource::<AssetServer>().clone(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
            asset_id_resolver: world.resource::<EditorAssetIdResolver>().clone(),
            asset_collector: HashSet::new(),
        }
    }
}

impl ReflectDeserializerProcessor for EditorDeserializerProcessor {
    fn try_deserialize<'de, D>(
        &mut self,
        registration: &TypeRegistration,
        _registry: &TypeRegistry,
        deserializer: D,
    ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let Some(reflect_handle) = registration.data::<ReflectHandle>() else {
            // we don't want to deserialize this - give the deserializer back
            return Ok(Err(deserializer));
        };

        let Some(reflect_default) = registration.data::<ReflectDefault>() else {
            // we don't want to deserialize this - give the deserializer back
            return Ok(Err(deserializer));
        };

        let editor_handle = EditorAssetHandle::deserialize(deserializer)?;

        let handle = match editor_handle {
            EditorAssetHandle::Runtime => reflect_default.default(),
            EditorAssetHandle::Db(uuid) => {
                let id = UntypedEditorAssetId {
                    type_id: reflect_handle.asset_type_id(),
                    uuid,
                };

                let asset_path = self
                    .asset_id_resolver
                    .get_asset_path(id)
                    .map_err(|error| Error::custom(error))?;
                reflect_handle.load(&self.asset_server, asset_path)
            }
            EditorAssetHandle::AssetPath(asset_path) => {
                reflect_handle.load(&self.asset_server, asset_path)
            }
            EditorAssetHandle::Internal(uuid) => reflect_handle.typed(UntypedHandle::Uuid {
                type_id: reflect_handle.asset_type_id(),
                uuid,
            }),
        };

        self.asset_collector.insert(
            reflect_handle
                .downcast_handle_untyped(handle.as_any())
                .unwrap()
                .id(),
        );

        Ok(Ok(handle))
    }
}
