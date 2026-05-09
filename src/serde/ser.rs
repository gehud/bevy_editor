use std::path::PathBuf;

use bevy::{
    asset::{AssetPath, ReflectHandle, UntypedHandle},
    ecs::{
        resource::Resource,
        world::{FromWorld, World},
    },
    reflect::{PartialReflect, TypePath, TypeRegistry, serde::ReflectSerializerProcessor},
};
use serde::{Serialize, Serializer, ser::Error};

use crate::{asset::AssetDatabase, serde::AssetRef};

#[derive(Clone, TypePath)]
pub struct EditorSerializerProcessor {
    asset_database: AssetDatabase,
}

impl FromWorld for EditorSerializerProcessor {
    fn from_world(world: &mut World) -> Self {
        Self {
            asset_database: world.resource::<AssetDatabase>().clone(),
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
