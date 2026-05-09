use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy::{
    asset::Asset,
    reflect::{FromType, Reflect},
};
use uuid::Uuid;

#[derive(Reflect)]
#[reflect(Clone)]
pub struct EditorAssetId<A: Asset> {
    uuid: Uuid,
    extension: String,
    #[reflect(ignore)]
    _phantom_data: PhantomData<A>,
}

impl<A: Asset> Clone for EditorAssetId<A> {
    fn clone(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            extension: self.extension.clone(),
            _phantom_data: self._phantom_data.clone(),
        }
    }
}

#[derive(Clone, Reflect)]
pub struct ReflectEditorAssetId {}

impl<A: Asset> FromType<EditorAssetId<A>> for ReflectEditorAssetId {
    fn from_type() -> Self {
        Self {

        }
    }
}

#[cfg(feature = "editor")]
pub(crate) mod inspector {
    use bevy::reflect::Reflect;

    use crate::{asset::EditorAssetId, inspection::inspector_egui_impls::Inspector};

    #[derive(Reflect)]
    pub(crate) struct EditorAssetIdInspector;

    impl Inspector for EditorAssetIdInspector {}
}
