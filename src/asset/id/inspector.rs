use bevy::{
    asset::AssetPath,
    reflect::{Reflect, std_traits::ReflectDefault},
};
use egui::Frame;
use lucide_icons::Icon;

use crate::{
    asset::{AssetDatabase, ReflectEditorAssetId, id::UntypedEditorAssetId},
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
