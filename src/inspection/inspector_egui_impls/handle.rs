use bevy::{
    asset::{AssetServer, ReflectHandle},
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
    asset::database::AssetDatabase,
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
                let db = env.context.world.get_resource_mut::<AssetDatabase>()?;
                let mut asset_path = db
                    .get_path(&payload.uuid)?
                    .ok_or_else(|| BevyError::from("Invalid asset payload"))?
                    .clone_owned();

                if let Some(label) = &payload.label {
                    asset_path = asset_path.with_label(label);
                }

                let asset_server = env.context.world.get_resource_mut::<AssetServer>()?;
                let untyped = block_on(asset_server.load_untyped_async(asset_path))?;
                untyped.type_id() == reflect_handle.asset_type_id()
            } else {
                false
            };

        let stroke_style = if is_payload_suitable {
            ui.style().visuals.widgets.hovered.bg_stroke
        } else {
            ui.style().visuals.widgets.inactive.bg_stroke
        };

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
                    ui.label("asset");
                });
            })
            .response;

        if let Some(payload) = response.dnd_release_payload::<AssetPayload>() {
            if is_payload_suitable {
                let db = env.context.world.get_resource_mut::<AssetDatabase>()?;
                let mut asset_path = db
                    .get_path(&payload.uuid)?
                    .ok_or_else(|| BevyError::from("Invalid asset payload"))?
                    .clone_owned();

                if let Some(label) = &payload.label {
                    asset_path = asset_path.with_label(label);
                }

                let asset_server = env.context.world.get_resource_mut::<AssetServer>()?;
                let untyped = block_on(asset_server.load_untyped_async(asset_path))?;
                value.apply(reflect_handle.typed(untyped).as_partial_reflect());
            }
        }

        Ok(changed)
    }
}
