use std::any::Any;

use bevy::asset::uuid;
use bevy::camera::visibility::RenderLayers;
use bevy::color::{Color, Hsla, Hsva, LinearRgba, Srgba};
use bevy::ecs::entity::Entity;
use bevy::ecs::error::Result;
use bevy::reflect::PartialReflect;
use egui::Color32;

use crate::inspection::reflect_inspector::InspectorUi;

use super::Inspector;

impl Inspector for uuid::Uuid {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        Self::ui_readonly(ui, options, id, env, value.as_partial_reflect())?;
        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _: &dyn Any,
        _: egui::Id,
        _: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        ui.label(value.to_string());
        Ok(())
    }
}

impl Inspector for Entity {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        Self::ui_readonly(ui, options, id, env, value)?;
        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _: &dyn Any,
        _: egui::Id,
        _: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        ui.label(format!("{value:?}"));
        Ok(())
    }
}

// impl Inspector for Handle<Mesh> {
//     fn ui(
//         &mut self,
//         ui: &mut egui::Ui,
//         _: &dyn Any,
//         _: egui::Id,
//         env: InspectorUi<'_, '_>,
//     ) -> bool {
//         let handle = &*self;
//         let Some(world) = &mut env.context.world else {
//             no_world_in_context(ui, "Handle<Mesh>");
//             return false;
//         };
//         let mut meshes = match world.get_resource_mut::<Assets<Mesh>>() {
//             Ok(meshes) => meshes,
//             Err(error) => {
//                 no_access(error, ui, "Assets<Mesh>");
//                 return false;
//             }
//         };
//         let Some(mesh) = meshes.get_mut(handle) else {
//             nonexistent_asset_handle(ui, handle.id().untyped());
//             return false;
//         };

//         mesh_ui_inner(mesh, ui);

//         ui.add_enabled_ui(mesh.indices().is_some(), |ui| {
//             if ui.button("Duplicate vertices").clicked() {
//                 mesh.duplicate_vertices();
//             }
//         });
//         ui.add_enabled_ui(mesh.indices().is_none(), |ui| {
//             if ui.button("Compute flat normals").clicked() {
//                 mesh.compute_flat_normals();
//             }
//         });
//         if ui.button("Generate tangents").clicked() {
//             let _ = mesh.generate_tangents();
//         }

//         false
//     }

//     fn ui_readonly(&self, ui: &mut egui::Ui, _: &dyn Any, _: egui::Id, env: InspectorUi<'_, '_>) {
//         let Some(world) = &mut env.context.world else {
//             no_world_in_context(ui, "Handle<Mesh>");
//             return;
//         };
//         let meshes = match world.get_resource_mut::<Assets<Mesh>>() {
//             Ok(meshes) => meshes,
//             Err(error) => return no_access(error, ui, "Assets<Mesh>"),
//         };
//         let Some(mesh) = meshes.get(self) else {
//             return nonexistent_asset_handle(ui, self.id().untyped());
//         };

//         mesh_ui_inner(mesh, ui);
//     }
// }

// fn mesh_ui_inner(mesh: &Mesh, ui: &mut egui::Ui) {
//     egui::Grid::new("mesh").show(ui, |ui| {
//         ui.label("primitive_topology");
//         ui.label(format!("{:?}", mesh.primitive_topology()));
//         ui.end_row();

//         ui.label("Vertices");
//         ui.label(mesh.count_vertices().to_string());
//         ui.end_row();

//         if let Some(indices) = mesh.indices() {
//             ui.label("Indices");
//             let len = match indices {
//                 bevy::mesh::Indices::U16(vec) => vec.len(),
//                 bevy::mesh::Indices::U32(vec) => vec.len(),
//             };
//             ui.label(len.to_string());
//             ui.end_row();
//         }

//         ui.label("Vertex Attributes");

//         let builtin_attributes = &[
//             Mesh::ATTRIBUTE_POSITION,
//             Mesh::ATTRIBUTE_COLOR,
//             Mesh::ATTRIBUTE_UV_0,
//             Mesh::ATTRIBUTE_NORMAL,
//             Mesh::ATTRIBUTE_TANGENT,
//             Mesh::ATTRIBUTE_COLOR,
//             Mesh::ATTRIBUTE_JOINT_INDEX,
//             Mesh::ATTRIBUTE_JOINT_WEIGHT,
//         ];

//         ui.vertical(|ui| {
//             for attribute in builtin_attributes {
//                 if mesh.attribute(attribute.id).is_some() {
//                     ui.label(attribute.name);
//                 }
//             }
//         });
//     });
// }

impl Inspector for Srgba {
    fn ui(
        ui: &mut egui::Ui,
        _: &dyn Any,
        _: egui::Id,
        _: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let mut color = Color32::from_rgba_unmultiplied(
            (value.red * 255.) as u8,
            (value.green * 255.) as u8,
            (value.blue * 255.) as u8,
            (value.alpha * 255.) as u8,
        );

        if ui.color_edit_button_srgba(&mut color).changed() {
            let [r, g, b, a] = color.to_srgba_unmultiplied();
            value.red = r as f32 / 255.;
            value.green = g as f32 / 255.;
            value.blue = b as f32 / 255.;
            value.alpha = a as f32 / 255.;
            return Ok(true);
        }

        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        })
        .inner?;
        Ok(())
    }
}

impl Inspector for LinearRgba {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let linerar = value.try_downcast_mut::<Self>().unwrap();

        let mut color = [linerar.red, linerar.green, linerar.blue, linerar.alpha];
        if ui
            .color_edit_button_rgba_premultiplied(&mut color)
            .changed()
        {
            linerar.red = color[0];
            linerar.green = color[1];
            linerar.blue = color[2];
            linerar.alpha = color[3];
            return Ok(true);
        }

        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        })
        .inner?;
        Ok(())
    }
}

impl Inspector for Hsla {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let mut hsva =
            egui::ecolor::Hsva::new(value.hue, value.saturation, value.lightness, value.alpha);
        if ui.color_edit_button_hsva(&mut hsva).changed() {
            value.hue = hsva.h;
            value.saturation = hsva.s;
            value.lightness = hsva.v;
            value.alpha = hsva.a;
            return Ok(true);
        }

        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        });
        Ok(())
    }
}
impl Inspector for Hsva {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        let mut hsva =
            egui::ecolor::Hsva::new(value.hue, value.saturation, value.value, value.alpha);
        if ui.color_edit_button_hsva(&mut hsva).changed() {
            value.hue = hsva.h;
            value.saturation = hsva.s;
            value.value = hsva.v;
            value.alpha = hsva.a;
            return Ok(true);
        }

        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        });
        Ok(())
    }
}

impl Inspector for Color {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let changed = match value {
            Color::Srgba(color) => Srgba::ui(ui, options, id, env, color)?,
            Color::LinearRgba(color) => LinearRgba::ui(ui, options, id, env, color)?,
            Color::Hsla(color) => Hsla::ui(ui, options, id, env, color)?,
            Color::Hsva(color) => Hsva::ui(ui, options, id, env, color)?,
            Color::Hwba(_)
            | Color::Lcha(_)
            | Color::Laba(_)
            | Color::Oklaba(_)
            | Color::Oklcha(_)
            | Color::Xyza(_) => {
                ui.label(format!(
                    "Colorspace of {value:?} is not supported yet. PRs welcome"
                ));
                false
            }
        };

        Ok(changed)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        });
        Ok(())
    }
}

impl Inspector for RenderLayers {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let mut new_value = None;
        egui::Grid::new(id).num_columns(2).show(ui, |ui| {
            for layer in value.iter() {
                let mut layer_copy = layer;
                if ui.add(egui::DragValue::new(&mut layer_copy)).changed() {
                    new_value = Some(value.clone().without(layer).with(layer_copy));
                }

                if ui.button("-").clicked() {
                    new_value = Some(value.clone().without(layer));
                }
                ui.end_row();
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Add").clicked() {
                let new_layer = value.iter().last().map_or(0, |last| last + 1);
                new_value = Some(value.clone().with(new_layer));
            }
        });

        if let Some(new_value) = new_value {
            *value = new_value;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap().clone();
        for layer in value.iter() {
            ui.label(format!("- {layer}"));
        }
        Ok(())
    }
}

impl Inspector for bevy::gizmos::config::GizmoConfigStore {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        id: egui::Id,
        mut env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        for (ty, group, value) in value.iter_mut() {
            use egui::CollapsingHeader;

            let name = env
                .type_registry
                .get(*ty)
                .map(|x| x.type_info().ty().short_path())
                .unwrap_or("<unknown gizmo group>");
            let inner = CollapsingHeader::new(name)
                .id_salt(id.with(ty))
                .show(ui, |ui| -> Result {
                    env.ui_for_reflect(group, ui)?;
                    ui.separator();
                    env.ui_for_reflect_with_options(value, ui, egui::Id::new("data"), &())?;
                    Ok(())
                })
                .body_returned;

            if let Some(inner) = inner {
                inner?;
            }
        }

        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        id: egui::Id,
        mut env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        for (ty, group, value) in value.iter() {
            use egui::CollapsingHeader;

            let name = env
                .type_registry
                .get(*ty)
                .map(|x| x.type_info().ty().short_path())
                .unwrap_or("<unknown gizmo group>");
            let inner = CollapsingHeader::new(name)
                .id_salt(id.with(ty))
                .show(ui, |ui| -> Result {
                    env.ui_for_reflect_readonly(group, ui)?;
                    ui.separator();
                    env.ui_for_reflect_readonly_with_options(
                        value,
                        ui,
                        egui::Id::new("data"),
                        &(),
                    )?;
                    Ok(())
                })
                .body_returned;

            if let Some(inner) = inner {
                inner?;
            }
        }

        Ok(())
    }
}
