use bevy::{
    asset::{AssetPath, AssetServer, ReflectHandle, UntypedHandle, processor::AssetProcessor},
    ecs::{
        error::{BevyError, Result},
        resource,
    },
    log::info,
    reflect::{FromType, GetTypeRegistration, Reflect, TypePath, prelude::ReflectDefault},
    tasks::block_on,
};
use egui::{DragValue, Frame, InnerResponse, Response, Widget};
use lucide_icons::Icon;

use crate::{
    asset_browser::AssetPayload,
    assets::icons::MaterialIcon,
    inspection::inspector_egui_impls::{Inspector, ReflectInspector},
};

#[derive(Reflect)]
pub(super) struct HandleInspector;

impl Inspector for HandleInspector {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn std::any::Any,
        _id: egui::Id,
        env: crate::inspection::reflect_inspector::InspectorUi<'_, '_>,
        value: &mut dyn bevy::reflect::PartialReflect,
    ) -> bevy::ecs::error::Result<bool> {
        ui.scope(|ui| {

        });
        let mut changed = false;

        let registration = value
            .try_as_reflect()
            .map(|reflect| reflect.reflect_type_info().type_id())
            .map(|type_id| env.type_registry.get(type_id).unwrap())
            .unwrap();

        let Some(reflect_handle) = registration.data::<ReflectHandle>() else {
            ui.label("Asset is not reflected.");
            return Ok(false);
        };

        let reflect_default = registration.data::<ReflectDefault>().unwrap();

        let is_payload_suitable =
            if let Some(payload) = ui.response().dnd_hover_payload::<AssetPayload>() {
                payload.0.type_id() == reflect_handle.asset_type_id()
            } else {
                false
            };

        let stroke_style = if is_payload_suitable {
            ui.style().visuals.widgets.hovered.bg_stroke
        } else {
            ui.style().visuals.widgets.inactive.bg_stroke
        };

        let untyped = reflect_handle
            .downcast_handle_untyped(value.try_as_reflect().unwrap().as_any())
            .unwrap();

        let path = untyped
            .path()
            .map(|path| path.to_owned())
            .unwrap_or_else(|| AssetPath::from("internal"));

        let label = path.label().map(|label| label.to_string());
        let mut path = path.path().to_path_buf();

        if let Some(parent) = path.parent() {
            path = path.strip_prefix(parent)?.to_path_buf();
        }

        let mut path = AssetPath::from(path);

        if let Some(label) = label {
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
                info!("{:?}", payload.0.path());
                value.apply(reflect_handle.typed(payload.0.clone()).as_partial_reflect());
            }
        }

        Ok(changed)
    }
}
