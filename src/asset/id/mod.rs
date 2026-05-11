#[cfg(feature = "editor")]
pub(crate) mod inspector;

use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use bevy::{
    asset::Asset,
    reflect::{FromType, Reflect, std_traits::ReflectDefault},
    utils::default,
};
use uuid::Uuid;

// TODO: Maybe allow serializing asset path with non-default souce (embedded).
#[derive(Debug, Eq, Hash, PartialEq, Reflect)]
#[reflect(Clone, Default)]
pub struct EditorAssetId<A: Asset> {
    pub(crate) uuid: Uuid,
    #[reflect(ignore)]
    _phantom_data: PhantomData<A>,
}

impl<A: Asset> EditorAssetId<A> {
    fn try_from(value: UntypedEditorAssetId) -> Option<Self> {
        let found = value.type_id();
        let expected = TypeId::of::<A>();

        if found != expected {
            return None;
        }

        Some(Self {
            uuid: value.uuid,
            _phantom_data: default(),
        })
    }

    pub fn untyped(self) -> UntypedEditorAssetId {
        UntypedEditorAssetId {
            type_id: TypeId::of::<A>(),
            uuid: self.uuid,
        }
    }
}

impl<A: Asset> Clone for EditorAssetId<A> {
    fn clone(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            _phantom_data: self._phantom_data.clone(),
        }
    }
}

impl<A: Asset> Default for EditorAssetId<A> {
    fn default() -> Self {
        Self {
            uuid: default(),
            _phantom_data: default(),
        }
    }
}

impl<A: Asset> Into<UntypedEditorAssetId> for EditorAssetId<A> {
    fn into(self) -> UntypedEditorAssetId {
        self.untyped()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Reflect)]
#[reflect(Clone)]
pub struct UntypedEditorAssetId {
    pub(crate) type_id: TypeId,
    pub(crate) uuid: Uuid,
}

impl UntypedEditorAssetId {
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn typed_unchecked<A: Asset>(self) -> EditorAssetId<A> {
        EditorAssetId {
            uuid: self.uuid,
            _phantom_data: default(),
        }
    }

    pub fn typed_debug_checked<A: Asset>(self) -> EditorAssetId<A> {
        debug_assert_eq!(
            self.type_id(),
            TypeId::of::<A>(),
            "The target EditorAssetId<A>'s TypeId does not match the TypeId of this UntypedEditorAssetId"
        );
        self.typed_unchecked()
    }

    pub fn typed<A: Asset>(self) -> EditorAssetId<A> {
        let Some(id) = self.try_typed() else {
            panic!(
                "The target EditorAssetId<{}>'s TypeId does not match the TypeId of this UntypedEditorAssetId",
                core::any::type_name::<A>()
            )
        };

        id
    }

    pub fn try_typed<A: Asset>(self) -> Option<EditorAssetId<A>> {
        EditorAssetId::try_from(self)
    }
}

#[derive(Clone)]
pub struct ReflectEditorAssetId {
    asset_type_id: TypeId,
    downcast_untyped: fn(&dyn Any) -> Option<UntypedEditorAssetId>,
    typed: fn(UntypedEditorAssetId) -> Box<dyn Reflect>,
}

impl ReflectEditorAssetId {
    pub fn asset_type_id(&self) -> TypeId {
        self.asset_type_id.clone()
    }

    pub fn downcast_untyped(&self, id: &dyn Any) -> Option<UntypedEditorAssetId> {
        (self.downcast_untyped)(id)
    }

    pub fn typed(&self, id: UntypedEditorAssetId) -> Box<dyn Reflect> {
        (self.typed)(id)
    }
}

impl<A: Asset> FromType<EditorAssetId<A>> for ReflectEditorAssetId {
    fn from_type() -> Self {
        Self {
            asset_type_id: TypeId::of::<A>(),
            downcast_untyped: |id: &dyn Any| {
                id.downcast_ref::<EditorAssetId<A>>()
                    .map(|id| id.clone().untyped())
            },
            typed: |id: UntypedEditorAssetId| Box::new(id.typed_debug_checked::<A>()),
        }
    }
}
