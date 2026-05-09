use std::{any::TypeId, marker::PhantomData, path::PathBuf};

use bevy::{
    asset::{Asset, AssetPath},
    reflect::{FromType, Reflect},
};
use uuid::Uuid;

#[derive(Reflect)]
#[reflect(Clone)]
pub struct EditorAssetId<A: Asset> {
    pub uuid: Uuid,
    pub extension: String,
    pub label: Option<String>,
    #[reflect(ignore)]
    _phantom_data: PhantomData<A>,
}

impl<A: Asset> EditorAssetId<A> {
    pub fn untyped(self) -> UntypedEditorAssetId {
        UntypedEditorAssetId {
            type_id: TypeId::of::<A>(),
            uuid: self.uuid,
            extension: self.extension,
            label: self.label,
        }
    }

    pub fn asset_path<'a>(self) -> AssetPath<'a> {
        let mut asset_path = AssetPath::from_path_buf(
            PathBuf::from(self.uuid.to_string()).with_extension(self.extension),
        );
        if let Some(label) = self.label {
            asset_path = asset_path.with_label(label);
        }
        asset_path
    }
}

impl<A: Asset> Clone for EditorAssetId<A> {
    fn clone(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            extension: self.extension.clone(),
            label: self.label.clone(),
            _phantom_data: self._phantom_data.clone(),
        }
    }
}

impl<A: Asset> Into<UntypedEditorAssetId> for EditorAssetId<A> {
    fn into(self) -> UntypedEditorAssetId {
        self.untyped()
    }
}

impl<'a, A: Asset> Into<AssetPath<'a>> for EditorAssetId<A> {
    fn into(self) -> AssetPath<'a> {
        self.asset_path()
    }
}

#[derive(Clone, Reflect)]
#[reflect(Clone)]
pub struct UntypedEditorAssetId {
    pub type_id: TypeId,
    pub uuid: Uuid,
    pub extension: String,
    pub label: Option<String>,
}

impl UntypedEditorAssetId {
    pub fn asset_path<'a>(self) -> AssetPath<'a> {
        let mut asset_path = AssetPath::from_path_buf(
            PathBuf::from(self.uuid.to_string()).with_extension(self.extension),
        );
        if let Some(label) = self.label {
            asset_path = asset_path.with_label(label);
        }
        asset_path
    }
}

#[derive(Clone, Reflect)]
pub struct ReflectEditorAssetId {}

impl<A: Asset> FromType<EditorAssetId<A>> for ReflectEditorAssetId {
    fn from_type() -> Self {
        Self {}
    }
}

#[cfg(feature = "editor")]
pub(crate) mod inspector {
    use bevy::reflect::Reflect;

    use crate::inspection::inspector_egui_impls::Inspector;

    #[derive(Reflect)]
    pub(crate) struct EditorAssetIdInspector;

    impl Inspector for EditorAssetIdInspector {}
}
