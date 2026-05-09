//! Custom UI implementations for specific types. Check [`InspectorPrimitive`] for an example.

use crate::{
    asset::{ReflectEditorAssetId, inspector::EditorAssetIdInspector},
    inspection::{inspector_egui_impls::handle::HandleInspector, reflect_inspector::InspectorUi},
};
use bevy::{
    asset::{Handle, ReflectHandle, uuid},
    mesh::{Mesh, Mesh3d},
    pbr::StandardMaterial,
    platform::time::Instant,
    transform::components::Transform,
};
use bevy::{
    ecs::error::Result,
    reflect::{FromType, PartialReflect, Reflect, TypeRegistry},
};
use disqualified::ShortName;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

mod bevy_impls;
mod glam_impls;
mod handle;
mod image;
mod std_impls;

/// Custom UI implementation for a concrete type.
///
/// # Example Usage
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_inspector_egui::inspector_egui_impls::{Inspector, ReflectInspector};
/// use bevy_inspector_egui::quick::ResourceInspectorPlugin;
/// use bevy_inspector_egui::reflect_inspector::InspectorUi;
///
/// #[derive(Reflect, Default)]
/// struct ToggleOption(bool);
///
/// impl Inspector for ToggleOption {
///     fn ui(&mut self, ui: &mut egui::Ui, _: &dyn std::any::Any, _: egui::Id, _: InspectorUi<'_, '_>) -> bool {
///         let mut changed = ui.radio_value(&mut self.0, false, "Disabled").changed();
///         changed |= ui.radio_value(&mut self.0, true, "Enabled").changed();
///         changed
///     }
///
///     fn ui_readonly(&self, ui: &mut egui::Ui, _: &dyn std::any::Any, _: egui::Id, _: InspectorUi<'_, '_>) {
///         let mut copy = self.0;
///         ui.add_enabled_ui(false, |ui| {
///             ui.radio_value(&mut copy, false, "Disabled").changed();
///             ui.radio_value(&mut copy, true, "Enabled").changed();
///         });
///     }
/// }
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         // ...
///         .register_type_data::<ToggleOption, ReflectInspector>()
///         .run();
/// }
/// ```
#[allow(unused)]
pub trait Inspector: Reflect {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        Ok(())
    }

    fn ui_many(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        ui.label(format!(
            "{}, doesn't support multi-editing",
            ShortName::of::<Self>()
        ));

        Ok(false)
    }

    fn ui_many_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        values: &[&dyn PartialReflect],
    ) -> Result {
        ui.label(format!(
            "{}, doesn't support multi-inspection",
            ShortName::of::<Self>()
        ));
        Ok(())
    }
}

/// Function pointers for displaying a concrete type, to be registered in the [`TypeRegistry`].
///
/// This can used for leaf types like `u8` or `String`, as well as people who want to completely customize the way
/// to display a certain type. You can use [`Inspector`] to avoid manually writing the function pointers with correct downcasting.
#[derive(Clone)]
pub struct ReflectInspector {
    fn_ui: fn(
        &mut egui::Ui,
        &dyn Any,
        egui::Id,
        InspectorUi<'_, '_>,
        &mut dyn PartialReflect,
    ) -> Result<bool>,

    fn_ui_readonly:
        fn(&mut egui::Ui, &dyn Any, egui::Id, InspectorUi<'_, '_>, &dyn PartialReflect) -> Result,

    fn_ui_many: fn(
        &mut egui::Ui,
        &dyn Any,
        egui::Id,
        InspectorUi<'_, '_>,
        &mut [&mut dyn PartialReflect],
    ) -> Result<bool>,

    fn_ui_many_readonly: fn(
        &mut egui::Ui,
        &dyn Any,
        egui::Id,
        InspectorUi<'_, '_>,
        &[&dyn PartialReflect],
    ) -> Result,
}

impl<T: Inspector> FromType<T> for ReflectInspector {
    fn from_type() -> Self {
        ReflectInspector::of::<T>()
    }
}

impl ReflectInspector {
    pub fn of<T: Inspector>() -> Self {
        ReflectInspector {
            fn_ui: T::ui,
            fn_ui_readonly: T::ui_readonly,
            fn_ui_many: T::ui_many,
            fn_ui_many_readonly: T::ui_many_readonly,
        }
    }

    pub fn ui(
        &self,
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        (self.fn_ui)(ui, options, id, env, value)
    }

    pub fn ui_readonly(
        &self,
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        (self.fn_ui_readonly)(ui, options, id, env, value)
    }

    pub fn ui_many(
        &self,
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        (self.fn_ui_many)(ui, options, id, env, values)
    }

    pub fn ui_many_readonly(
        &self,
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        values: &[&dyn PartialReflect],
    ) -> Result {
        (self.fn_ui_many_readonly)(ui, options, id, env, values)
    }
}

/// Register [`ReflectInspector`]s for primitive rust types as well as standard library types
#[rustfmt::skip]
pub fn register_std_impls(type_registry: &mut TypeRegistry) {
    type_registry.register_type_data::<f32, ReflectInspector>();
    type_registry.register_type_data::<f64, ReflectInspector>();
    type_registry.register_type_data::<i8, ReflectInspector>();
    type_registry.register_type_data::<i16, ReflectInspector>();
    type_registry.register_type_data::<i32, ReflectInspector>();
    type_registry.register_type_data::<i64, ReflectInspector>();
    type_registry.register_type_data::<isize, ReflectInspector>();
    type_registry.register_type_data::<u8, ReflectInspector>();
    type_registry.register_type_data::<u16, ReflectInspector>();
    type_registry.register_type_data::<u32, ReflectInspector>();
    type_registry.register_type_data::<u64, ReflectInspector>();
    type_registry.register_type_data::<usize, ReflectInspector>();

    type_registry.register_type_data::<bool, ReflectInspector>();
    type_registry.register_type_data::<String, ReflectInspector>();
    type_registry.register_type_data::<std::ops::Range<f32>, ReflectInspector>();

    type_registry.register::<std::ops::Range<f64>>();
    type_registry.register_type_data::<std::ops::Range<f64>, ReflectInspector>();

    type_registry.register_type_data::<std::ops::RangeInclusive<f32>, ReflectInspector>();

    type_registry.register::<std::ops::RangeInclusive<f64>>();
    type_registry.register_type_data::<std::ops::RangeInclusive<f64>, ReflectInspector>();

    type_registry.register_type_data::<TypeId, ReflectInspector>();

    type_registry.register_type_data::<std::time::Duration, ReflectInspector>();

    type_registry.register_type_data::<Instant, ReflectInspector>();
}

/// Register [`ReflectInspector`]s for [`bevy::math`]/`glam` types
#[rustfmt::skip]
pub fn register_glam_impls(type_registry: &mut TypeRegistry) {
    type_registry.register_type_data::<bevy::math::Vec2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Vec3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Vec3A, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Vec4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::UVec2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::UVec3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::UVec4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::IVec2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::IVec3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::IVec4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DVec2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DVec3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DVec4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::BVec2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::BVec3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::BVec4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Mat2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Mat3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Mat3A, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Mat4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DMat2, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DMat3, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::DMat4, ReflectInspector>();
    type_registry.register_type_data::<bevy::math::Quat, ReflectInspector>();
}

/// Register [`ReflectInspector`]s for `bevy` types
#[rustfmt::skip]
pub fn register_bevy_impls(type_registry: &mut TypeRegistry) {
    type_registry.register_type_data::<bevy::ecs::entity::Entity, ReflectInspector>();
    type_registry.register_type_data::<Transform, ReflectInspector>();
    type_registry.register_type_data::<bevy::color::Color, ReflectInspector>();
    type_registry.register_type_data::<bevy::color::Srgba, ReflectInspector>();
    type_registry.register_type_data::<bevy::color::LinearRgba, ReflectInspector>();
    type_registry.register_type_data::<bevy::color::Hsla, ReflectInspector>();
    type_registry.register_type_data::<bevy::color::Hsva, ReflectInspector>();
    type_registry.register_type_data::<bevy::gizmos::config::GizmoConfigStore, ReflectInspector>();
    type_registry.register_type_data::<uuid::Uuid, ReflectInspector>();

    for registration in type_registry.iter_mut() {
        if registration.data::<ReflectHandle>().is_some() {
            registration.insert(ReflectInspector::of::<HandleInspector>());
        }

        if registration.data::<ReflectEditorAssetId>().is_some() {
            registration.insert(ReflectInspector::of::<EditorAssetIdInspector>());
        }
    }
}

pub(crate) fn iter_all_eq<T: PartialEq>(mut iter: impl Iterator<Item = T>) -> Option<T> {
    let first = iter.next()?;
    iter.all(|elem| elem == first).then_some(first)
}
