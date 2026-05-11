use std::path::PathBuf;

use bevy::{
    asset::{AssetPath, ReflectHandle, UntypedHandle},
    ecs::{
        entity::Entity,
        world::{FromWorld, World},
    },
    platform::collections::HashMap,
    reflect::{PartialReflect, TypePath, TypeRegistry, serde::ReflectSerializerProcessor},
};
use serde::{Serialize, Serializer, ser::Error};

use crate::{asset::database::AssetDatabase, serde::AssetRef};

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
                return Ok(Ok(map[entity].serialize(serializer)?));
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
                if let Some(path) = untyped_handle.path() {
                    let label = path.label().map(|label| label.to_string());
                    let path = path.path();
                    let extension = path
                        .extension()
                        .map(|extension| extension.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let uuid = self
                        .asset_database
                        .get_uuid(path)
                        .map_err(|error| Error::custom(error))?;

                    if let Some(uuid) = uuid {
                        let mut asset_path = AssetPath::from_path_buf(
                            PathBuf::from(uuid.to_string()).with_extension(extension),
                        );
                        if let Some(label) = label {
                            asset_path = asset_path.with_label(label);
                        }
                        AssetRef::Db(asset_path)
                    } else {
                        AssetRef::Empty
                    }
                } else {
                    AssetRef::Empty
                }
            }
            UntypedHandle::Uuid { uuid, .. } => AssetRef::Uuid(uuid),
        };

        Ok(Ok(asset_ref.serialize(serializer)?))
    }
}
