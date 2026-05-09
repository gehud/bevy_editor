use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    path::PathBuf,
};

use bevy::{
    asset::{Asset, AssetPath},
    reflect::{FromType, Reflect, std_traits::ReflectDefault},
    utils::default,
};
use uuid::Uuid;

#[derive(Reflect)]
#[reflect(Clone, Default)]
pub struct EditorAssetId<A: Asset> {
    pub uuid: Uuid,
    pub extension: String,
    pub label: Option<String>,
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
            extension: value.extension,
            label: value.label,
            _phantom_data: default(),
        })
    }

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

impl<A: Asset> Default for EditorAssetId<A> {
    fn default() -> Self {
        Self {
            uuid: default(),
            extension: default(),
            label: default(),
            _phantom_data: default(),
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
    pub fn type_id(&self) -> TypeId {
        self.type_id
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

    pub fn typed_unchecked<A: Asset>(self) -> EditorAssetId<A> {
        EditorAssetId {
            uuid: self.uuid,
            extension: self.extension,
            label: self.label,
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

#[cfg(feature = "editor")]
pub(crate) mod inspector {
    use bevy::{
        asset::AssetPath,
        reflect::{Reflect, std_traits::ReflectDefault},
    };
    use egui::Frame;
    use lucide_icons::Icon;

    use crate::{
        asset::{AssetDatabase, ReflectEditorAssetId, UntypedEditorAssetId},
        asset_browser::AssetPayload,
        assets::icons::MaterialIcon,
        inspection::inspector_egui_impls::Inspector,
    };

    #[derive(Reflect)]
    pub(crate) struct EditorAssetIdInspector;

    impl Inspector for EditorAssetIdInspector {
        fn ui(
            ui: &mut egui::Ui,
            _options: &dyn std::any::Any,
            _id: egui::Id,
            env: crate::inspection::reflect_inspector::InspectorUi<'_, '_>,
            value: &mut dyn bevy::reflect::PartialReflect,
        ) -> bevy::ecs::error::Result<bool> {
            let mut changed = false;

            let registration = value
                .try_as_reflect()
                .map(|reflect| reflect.reflect_type_info().type_id())
                .map(|type_id| env.type_registry.get(type_id).unwrap())
                .unwrap();

            let Some(reflect_id) = registration.data::<ReflectEditorAssetId>() else {
                ui.label("Asset is not reflected (use 'register_editor_asset').");
                return Ok(false);
            };

            let reflect_default = registration.data::<ReflectDefault>().unwrap();

            let is_payload_suitable =
                if let Some(payload) = ui.response().dnd_hover_payload::<AssetPayload>() {
                    payload.0.type_id() == reflect_id.asset_type_id() && payload.0.path().is_some()
                } else {
                    false
                };

            let stroke_style = if is_payload_suitable {
                ui.style().visuals.widgets.hovered.bg_stroke
            } else {
                ui.style().visuals.widgets.inactive.bg_stroke
            };

            let untyped = reflect_id
                .downcast_untyped(value.try_as_reflect().unwrap().as_any())
                .unwrap();

            let asset_database = env.context.world.get_resource_mut::<AssetDatabase>()?;

            let mut path = if untyped.uuid.is_nil() {
                "Empty".into()
            } else {
                asset_database
                    .get_path(&untyped.uuid)?
                    .unwrap_or_else(|| "Missing".into())
            };

            if let Some(parent) = path.parent() {
                path = path.strip_prefix(parent)?.to_path_buf();
            }

            let mut path = AssetPath::from(path);

            if let Some(label) = &untyped.label {
                path = path.with_label(label);
            }

            let response = Frame::new()
                .inner_margin(ui.spacing().button_padding)
                .corner_radius(ui.style().visuals.widgets.inactive.corner_radius)
                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                .stroke(stroke_style)
                .show(ui, |ui| {
                    ui.take_available_width();
                    ui.horizontal(|ui| {
                        if ui.button(MaterialIcon::new(Icon::X)).clicked() {
                            value.apply(reflect_default.default().as_partial_reflect());
                            changed = true;
                        }

                        ui.label(path.to_string());
                    });
                })
                .response;

            if let Some(payload) = response.dnd_release_payload::<AssetPayload>() {
                if is_payload_suitable {
                    let asset_path = payload.0.path().unwrap().to_owned();

                    if let Some(uuid) = asset_database.get_uuid(asset_path.path())? {
                        let untyped = UntypedEditorAssetId {
                            type_id: reflect_id.asset_type_id(),
                            uuid,
                            extension: asset_path
                                .path()
                                .extension()
                                .map(|extension| extension.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            label: asset_path.label().map(|label| label.to_string()),
                        };
                        value.apply(reflect_id.typed(untyped).as_partial_reflect());
                        changed = true;
                    }
                }
            }

            Ok(changed)
        }
    }
}
