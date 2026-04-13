use std::any::{Any, TypeId, type_name};

use bevy::ecs::error::Result;
use bevy::math::{DMat2, DMat3, DMat4, DVec2, DVec3, DVec4, Mat3A, Vec3A, prelude::*};
use bevy::reflect::PartialReflect;

use crate::inspection::inspector_egui_impls::Inspector;
use crate::inspection::inspector_options::std_options::{NumberOptions, QuatDisplay, QuatOptions};
use crate::inspection::reflect_inspector::InspectorUi;

macro_rules! vec_ui {
    ($ty:ty > $elem:ty: $count:literal $($component:ident)*) => {
        impl Inspector for $ty {
            fn ui(
                ui: &mut egui::Ui,
                options: &dyn Any,
                id: egui::Id,
                mut env: InspectorUi<'_, '_>,
                value: &mut dyn PartialReflect,
            ) -> Result<bool> {
                let value = value.try_downcast_mut::<Self>().unwrap();

                let options = options
                    .downcast_ref::<NumberOptions<Self>>()
                    .cloned()
                    .unwrap_or_default();

                let mut changed = false;
                ui.scope(|ui| -> Result {
                    ui.style_mut().spacing.item_spacing = egui::Vec2::new(4.0, 0.);

                    ui.columns($count, |ui| -> Result {
                        match ui {
                            [$($component),*] => {
                                $(
                                    changed |= env.ui_for_reflect_with_options(
                                        &mut value.$component,
                                        $component,
                                        id.with(stringify!($component)),
                                        &options.map(|vec| vec.$component)
                                    )?;
                                )*
                                Ok(())
                            }
                            _ => unreachable!(),
                        }
                    })
                }).inner?;

                Ok(changed)
            }

            fn ui_readonly(
                ui: &mut egui::Ui,
                _: &dyn Any,
                _: egui::Id,
                mut env: InspectorUi<'_, '_>,
                value: &dyn PartialReflect,
            ) -> Result {
                let value = value.try_downcast_ref::<Self>().unwrap();

                ui.scope(|ui| -> Result {
                    ui.style_mut().spacing.item_spacing = egui::Vec2::new(4.0, 0.);

                    ui.columns($count, |ui| -> Result {
                        match ui {
                            [$($component),*] => {
                                $(env.ui_for_reflect_readonly(&value.$component, $component)?;)*
                                Ok(())
                            }
                            _ => unreachable!(),
                        }
                    })
                }).inner?;

                Ok(())
            }

            fn ui_many(
                ui: &mut egui::Ui,
                options: &dyn Any,
                id: egui::Id,
                mut env: InspectorUi<'_, '_>,
                values: &mut [&mut dyn PartialReflect],
            ) -> Result<bool> {
                let mut changed = false;

                let options = options
                    .downcast_ref::<NumberOptions<Self>>()
                    .cloned()
                    .unwrap_or_default();

                ui.scope(|ui| -> Result {
                    ui.style_mut().spacing.item_spacing = egui::Vec2::new(4.0, 0.);

                    ui.columns($count, |ui| -> Result {
                        match ui {
                            [$($component),*] => {
                                $(
                                    changed |= env.ui_for_reflect_many_with_options(
                                        TypeId::of::<$elem>(),
                                        type_name::<$elem>(),
                                        $component,
                                        id.with(stringify!($component)),
                                        &options.map(|vec| vec.$component),
                                        values
                                            .iter_mut()
                                            .map(|value| value.try_downcast_mut::<Self>().unwrap().$component.as_partial_reflect_mut())
                                            .collect::<Vec<_>>().as_mut_slice()
                                    )?;
                                )*

                                Ok(())
                            }
                            _ => unreachable!(),
                        }
                    })
                }).inner?;

                Ok(changed)
            }
        }
    };
}

macro_rules! mat_ui {
    ($ty:ty: $($component:ident)*) => {
        impl Inspector for $ty {
            fn ui(
                ui: &mut egui::Ui,
                _: &dyn Any,
                _: egui::Id,
                mut env: InspectorUi<'_, '_>,
                value: &mut dyn PartialReflect,
            ) -> Result<bool> {
                let value = value.try_downcast_mut::<Self>().unwrap();

                let mut changed = false;
                ui.vertical(|ui| -> Result {
                    $(changed |= env.ui_for_reflect(&mut value.$component, ui)?;)*
                    Ok(())
                }).inner?;

                Ok(changed)
            }

            fn ui_readonly(
                ui: &mut egui::Ui,
                _: &dyn Any,
                _: egui::Id,
                mut env: InspectorUi<'_, '_>,
                value: &dyn PartialReflect,
            ) -> Result {
                let value = value.try_downcast_ref::<Self>().unwrap();

                ui.vertical(|ui| -> Result {
                    $(env.ui_for_reflect_readonly(&value.$component, ui)?;)*
                    Ok(())
                }).inner?;

                Ok(())
            }
        }
    };
}

vec_ui!(Vec2 > f32: 2 x y);
vec_ui!(Vec3 > f32: 3 x y z);
vec_ui!(Vec3A > f32: 3 x y z);
vec_ui!(Vec4 > f32: 4 x y z w);
vec_ui!(UVec2 > u32: 2 x y);
vec_ui!(UVec3 > u32: 3 x y z);
vec_ui!(UVec4 > u32: 4 x y z w);
vec_ui!(IVec2 > i32: 2 x y);
vec_ui!(IVec3 > i32: 3 x y z);
vec_ui!(IVec4 > i32: 4 x y z w);
vec_ui!(DVec2 > f64: 2 x y);
vec_ui!(DVec3 > f64: 3 x y z);
vec_ui!(DVec4 > f64: 4 x y z w);
vec_ui!(BVec2 > bool: 2 x y);
vec_ui!(BVec3 > bool: 3 x y z);
vec_ui!(BVec4 > bool: 4 x y z w);

mat_ui!(Mat2: x_axis y_axis);
mat_ui!(Mat3: x_axis y_axis z_axis);
mat_ui!(Mat3A: x_axis y_axis z_axis);
mat_ui!(Mat4: x_axis y_axis z_axis w_axis);
mat_ui!(DMat2: x_axis y_axis);
mat_ui!(DMat3: x_axis y_axis z_axis);
mat_ui!(DMat4: x_axis y_axis z_axis w_axis);

impl Inspector for Quat {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        _id: egui::Id,
        mut env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let options = options
            .downcast_ref::<QuatOptions>()
            .cloned()
            .unwrap_or_default();

        ui.vertical(|ui| -> Result<bool> {
            match options.display {
                QuatDisplay::Raw => {
                    let mut vec4 = Vec4::from(*value);
                    let changed = env.ui_for_reflect(&mut vec4, ui)?;
                    if changed {
                        *value = Quat::from_vec4(vec4).normalize();
                    }
                    Ok(changed)
                }
                QuatDisplay::Euler => quat_ui_kind::<Euler>(value, ui, env),
                QuatDisplay::YawPitchRoll => quat_ui_kind::<YawPitchRoll>(value, ui, env),
                QuatDisplay::AxisAngle => quat_ui_kind::<AxisAngle>(value, ui, env),
            }
        })
        .inner
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = *value.try_downcast_ref::<Self>().unwrap();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        })
        .inner?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Euler(Vec3);
#[derive(Clone, Copy)]
struct YawPitchRoll((f32, f32, f32));
#[derive(Clone, Copy)]
struct AxisAngle((Vec3, f32));

trait RotationEdit {
    fn from_quat(quat: Quat) -> Self;
    fn to_quat(self) -> Quat;

    fn ui(&mut self, ui: &mut egui::Ui, env: InspectorUi<'_, '_>) -> Result<bool>;
}

impl RotationEdit for Euler {
    fn from_quat(quat: Quat) -> Self {
        Euler(quat.to_euler(EulerRot::XYZ).into())
    }

    fn to_quat(self) -> Quat {
        Quat::from_euler(EulerRot::XYZ, self.0.x, self.0.y, self.0.z)
    }

    fn ui(&mut self, ui: &mut egui::Ui, mut env: InspectorUi<'_, '_>) -> Result<bool> {
        env.ui_for_reflect(&mut self.0, ui)
    }
}

impl RotationEdit for YawPitchRoll {
    fn from_quat(quat: Quat) -> Self {
        YawPitchRoll(quat.to_euler(EulerRot::YXZ))
    }

    fn to_quat(self) -> Quat {
        let (y, p, r) = self.0;
        Quat::from_euler(EulerRot::YXZ, y, p, r)
    }

    fn ui(&mut self, ui: &mut egui::Ui, _env: InspectorUi<'_, '_>) -> Result<bool> {
        let (yaw, pitch, roll) = &mut self.0;

        let mut changed = false;
        ui.vertical(|ui| {
            egui::Grid::new("ypr grid").show(ui, |ui| {
                ui.label("Yaw");
                changed |= ui.drag_angle(yaw).changed();
                ui.end_row();
                ui.label("Pitch").changed();
                changed |= ui.drag_angle(pitch).changed();
                ui.end_row();
                ui.label("Roll");
                changed |= ui.drag_angle(roll).changed();
                ui.end_row();
            });
        });
        Ok(changed)
    }
}

impl RotationEdit for AxisAngle {
    fn from_quat(quat: Quat) -> Self {
        AxisAngle(quat.to_axis_angle())
    }

    fn to_quat(self) -> Quat {
        let (axis, angle) = self.0;
        let axis = axis.normalize();
        if axis.is_nan() {
            Quat::IDENTITY
        } else {
            Quat::from_axis_angle(axis.normalize(), angle)
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, mut env: InspectorUi<'_, '_>) -> Result<bool> {
        let (axis, angle) = &mut self.0;

        let mut changed = false;
        ui.vertical(|ui| -> Result {
            egui::Grid::new("axis-angle quat")
                .show(ui, |ui| -> Result {
                    ui.label("Axis");
                    changed |= env.ui_for_reflect(axis, ui)?;
                    ui.end_row();
                    ui.label("Angle");
                    changed |= ui.drag_angle(angle).changed();
                    ui.end_row();
                    Ok(())
                })
                .inner
        })
        .inner?;

        Ok(changed)
    }
}

fn quat_ui_kind<T: Send + Sync + 'static + Copy + RotationEdit>(
    val: &mut Quat,
    ui: &mut egui::Ui,
    env: InspectorUi<'_, '_>,
) -> Result<bool> {
    let id = ui.id();
    let mut intermediate = ui.memory_mut(|memory| {
        *memory
            .data
            .get_temp_mut_or_insert_with(id, || T::from_quat(*val))
    });

    let externally_changed = !intermediate.to_quat().abs_diff_eq(*val, f32::EPSILON);
    if externally_changed {
        intermediate = T::from_quat(*val);
    }

    let changed = intermediate.ui(ui, env)?;

    if changed || externally_changed {
        *val = intermediate.to_quat();
        ui.memory_mut(|memory| memory.data.insert_temp(id, intermediate));
    }

    Ok(changed)
}
