use bevy::asset::{AssetPath, AssetServer, ReflectHandle, UntypedAssetId, UntypedHandle};
use bevy::platform::collections::HashSet;
use bevy::reflect::prelude::ReflectDefault;
use bevy::reflect::{
    PartialReflect, TypeRegistry,
    serde::{ReflectDeserializerProcessor, ReflectSerializerProcessor},
};
use bevy::reflect::{TypePath, TypeRegistration};
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

#[derive(TypePath, Serialize, Deserialize)]
pub enum AssetRef {
    Empty,
    AssetPath(AssetPath<'static>),
    Uuid(Uuid),
}

#[derive(Default)]
pub struct AssetHandleSerializerProcessor;

impl AssetHandleSerializerProcessor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ReflectSerializerProcessor for AssetHandleSerializerProcessor {
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
                    AssetRef::AssetPath(path.clone())
                } else {
                    AssetRef::Empty
                }
            }
            UntypedHandle::Uuid { uuid, .. } => AssetRef::Uuid(uuid),
        };

        Ok(Ok(asset_ref.serialize(serializer)?))
    }
}

pub struct AssetHandleDeserializerProcessor<'a> {
    pub asset_server: &'a AssetServer,
    pub collector: &'a mut HashSet<UntypedAssetId>,
}

impl<'a> AssetHandleDeserializerProcessor<'a> {
    pub fn new(asset_server: &'a AssetServer, collector: &'a mut HashSet<UntypedAssetId>) -> Self {
        Self {
            asset_server,
            collector,
        }
    }
}

impl ReflectDeserializerProcessor for AssetHandleDeserializerProcessor<'_> {
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
            AssetRef::AssetPath(asset_path) => reflect_handle.load(self.asset_server, asset_path),
            AssetRef::Uuid(uuid) => reflect_handle.typed(UntypedHandle::Uuid {
                type_id: reflect_handle.asset_type_id(),
                uuid,
            }),
        };

        self.collector.insert(
            reflect_handle
                .downcast_handle_untyped(handle.as_any())
                .unwrap()
                .id(),
        );

        Ok(Ok(handle))
    }
}
