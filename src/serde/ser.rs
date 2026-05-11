use bevy::{
    asset::{ReflectHandle, UntypedHandle, io::AssetSourceId},
    ecs::{
        entity::Entity,
        world::{FromWorld, World},
    },
    platform::collections::HashMap,
    reflect::{PartialReflect, TypePath, TypeRegistry, serde::ReflectSerializerProcessor},
};
use serde::{Serialize, Serializer, ser::Error};

use crate::{asset::database::AssetDatabase, serde::EditorAssetHandle};

#[derive(Clone, TypePath)]
pub struct EditorSerializerProcessor {
    pub asset_database: AssetDatabase,
    pub scene_entity_map: Option<HashMap<Entity, Entity>>,
}

impl FromWorld for EditorSerializerProcessor {
    fn from_world(world: &mut World) -> Self {
        Self {
            asset_database: world.resource::<AssetDatabase>().clone(),
            scene_entity_map: None,
        }
    }
}

impl ReflectSerializerProcessor for EditorSerializerProcessor {
    fn try_serialize<S>(
        &self,
        value: &dyn PartialReflect,
        registry: &TypeRegistry,
        serializer: S,
    ) -> Result<Result<S::Ok, S>, S::Error>
    where
        S: Serializer,
    {
        let Some(value) = value.try_as_reflect() else {
            // we don't have any info on this type; do the default serialization logic
            return Ok(Err(serializer));
        };

        if let Some(map) = &self.scene_entity_map {
            if let Some(entity) = value.downcast_ref::<Entity>() {
                let mapped = map
                    .get(entity)
                    .cloned()
                    .unwrap_or_else(|| Entity::PLACEHOLDER);
                return Ok(Ok(mapped.serialize(serializer)?));
            }
        }

        let type_id = value.reflect_type_info().type_id();
        let Some(reflect_handle) = registry.get_type_data::<ReflectHandle>(type_id) else {
            // this isn't a `Handle<T>`
            return Ok(Err(serializer));
        };

        let untyped_handle = reflect_handle
            .downcast_handle_untyped(value.as_any())
            .unwrap();

        let asset_ref = match untyped_handle {
            UntypedHandle::Strong(..) => {
                if let Some(asset_path) = untyped_handle.path() {
                    if matches!(asset_path.source(), AssetSourceId::Default) {
                        let uuid = self
                            .asset_database
                            .get_uuid(asset_path)
                            .map_err(|error| Error::custom(error))?
                            .unwrap_or_default();

                        EditorAssetHandle::Db(uuid)
                    } else {
                        EditorAssetHandle::AssetPath(asset_path.clone())
                    }
                } else {
                    EditorAssetHandle::Runtime
                }
            }
            UntypedHandle::Uuid { uuid, .. } => EditorAssetHandle::Internal(uuid),
        };

        Ok(Ok(asset_ref.serialize(serializer)?))
    }
}
