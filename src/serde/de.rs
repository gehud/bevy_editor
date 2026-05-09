use bevy::{
    asset::{AssetServer, ReflectHandle, UntypedAssetId, UntypedHandle},
    ecs::{
        reflect::AppTypeRegistry,
        resource::Resource,
        world::{FromWorld, World},
    },
    platform::collections::HashSet,
    reflect::{
        PartialReflect, TypePath, TypeRegistration, TypeRegistry, serde::ReflectDeserializerProcessor, std_traits::ReflectDefault
    },
};
use serde::Deserialize;

#[cfg(feature = "editor")]
use crate::asset::AssetDatabase;
use crate::serde::AssetRef;

#[derive(Clone, Resource, TypePath)]
pub struct EditorDeserializerProcessor {
    #[cfg(feature = "editor")]
    pub asset_database: AssetDatabase,
    pub asset_server: AssetServer,
    pub type_registry: AppTypeRegistry,
    pub asset_collector: HashSet<UntypedAssetId>,
}

impl FromWorld for EditorDeserializerProcessor {
    fn from_world(world: &mut World) -> Self {
        Self {
            asset_database: world.resource::<AssetDatabase>().clone(),
            asset_server: world.resource::<AssetServer>().clone(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
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

        let asset_ref = AssetRef::deserialize(deserializer)?;

        let handle = match asset_ref {
            AssetRef::Empty => reflect_default.default(),
            AssetRef::Db(asset_path) => {
                #[cfg(feature = "editor")]
                {
                    use std::str::FromStr;

                    use serde::de::Error;
                    use uuid::Uuid;

                    let uuid = asset_path
                        .path()
                        .with_extension("")
                        .to_string_lossy()
                        .to_string();

                    let uuid = Uuid::from_str(&uuid).map_err(|error| Error::custom(error))?;

                    let label = asset_path.label().map(|label| label.to_string());
                    let path = self
                        .asset_database
                        .get_path(&uuid)
                        .map_err(|error| Error::custom(error))?;

                    if let Some(path) = path {
                        use bevy::asset::AssetPath;

                        let mut asset_path = AssetPath::from(path);
                        if let Some(label) = label {
                            asset_path = asset_path.with_label(label);
                        }
                        reflect_handle.load(&self.asset_server, asset_path)
                    } else {
                        reflect_default.default()
                    }
                }
                #[cfg(not(feature = "editor"))]
                {
                    reflect_handle.load(self.asset_server, asset_path)
                }
            }
            AssetRef::Uuid(uuid) => reflect_handle.typed(UntypedHandle::Uuid {
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
