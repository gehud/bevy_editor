//! General-purpose machinery for displaying [`Reflect`] types using [`egui`]
//!
//! # Examples
//! **Basic usage**
//! ```rust
//! use bevy::reflect::{Reflect, TypeRegistry};
//! use bevy_inspector_egui::reflect_inspector::{ui_for_value, InspectorUi, Context};
//!
//! #[derive(Reflect)]
//! struct Data {
//!     value: f32,
//! }
//!
//! fn ui(data: &mut Data, ui: &mut egui::Ui, type_registry: &TypeRegistry) {
//!     let mut cx = Context::default(); // empty context, with no access to the bevy world
//!     let mut env = InspectorUi::new_no_short_circuit(type_registry, &mut cx); // no short circuiting, couldn't display `Handle<StandardMaterial>`
//!
//!     let _changed = env.ui_for_reflect(data, ui);
//!
//!     // alternatively, if you are using an empty `Context`:
//!     let _changed = ui_for_value(data, ui, type_registry);
//! }
//! ```
//!
//!
//! **Bevy specific usage**
//! ```rust
//! use bevy::reflect::{Reflect, TypeRegistry};
//! use bevy_inspector_egui::reflect_inspector::{InspectorUi, Context};
//!
//! use bevy::ecs::prelude::*;
//! use bevy::ecs::world::CommandQueue;
//! use bevy::asset::Handle;
//! use bevy::pbr::StandardMaterial;
//!
//! #[derive(Reflect)]
//! struct Data {
//!     material: Handle<StandardMaterial>,
//! }
//!
//! fn ui(mut data: Mut<Data>, ui: &mut egui::Ui, world: &mut World, type_registry: &TypeRegistry) {
//!     let mut queue = CommandQueue::default();
//!     let mut cx = Context {
//!         world: Some(world.into()),
//!         queue: Some(&mut queue),
//!     };
//!     let mut env = InspectorUi::for_bevy(type_registry, &mut cx);
//!
//!     // alternatively
//!     // use crate::inspection::bevy_inspector::short_circuit;
//!     // let mut env = InspectorUi::new(type_registry, &mut cx, Some(short_circuit::short_circuit), Some(short_circuit::short_circuit_readonly));
//!
//!     let changed = env.ui_for_reflect(data.bypass_change_detection(), ui);
//!     if changed {
//!         data.set_changed();
//!     }
//!
//!     queue.apply(world);
//! }
//! ```

#[cfg(feature = "documentation")]
use crate::inspection::egui_utils::show_docs;

use crate::inspection::error::{self, TypeDataError};
use crate::inspection::inspector_egui_impls::{ReflectInspector, iter_all_eq};
use crate::inspection::inspector_options::{InspectorOptions, ReflectInspectorOptions, Target};
use crate::inspection::restricted_world_view::RestrictedWorldView;
use crate::inspection::{
    egui_utils::{add_button, down_button, remove_button, up_button},
    utils::pretty_type_name_str,
};
use bevy::ecs::error::Result;
use bevy::ecs::world::CommandQueue;
use bevy::reflect::{
    Array, DynamicEnum, DynamicTuple, DynamicTyped, DynamicVariant, Enum, EnumInfo, List, ListInfo,
    Map, Reflect, ReflectMut, ReflectRef, Struct, StructInfo, Tuple, TupleInfo, TupleStruct,
    TupleStructInfo, TypeInfo, TypeRegistry, VariantInfo, VariantType,
};
use bevy::reflect::{DynamicStruct, std_traits::ReflectDefault};
use bevy::reflect::{PartialReflect, Set, SetInfo};
use egui::{Grid, WidgetText};
use std::borrow::Cow;
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
};

/// Display the value without any [`Context`] or short circuiting behaviour.
///
/// This means that for example bevy's `Handle<StandardMaterial>` values cannot be displayed,
/// as they would need to have access to the `World`.
///
/// Use [`InspectorUi::new`] instead to provide context or use one of the methods in [`bevy_inspector`](crate::inspection::bevy_inspector).
pub fn ui_for_value(
    value: &mut dyn PartialReflect,
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
) -> Result<bool> {
    InspectorUi::new(type_registry, &mut Context::default()).ui_for_reflect(value, ui)
}

/// Display the readonly value without any [`Context`] or short circuiting behaviour.
///
/// This means that for example bevy's `Handle<StandardMaterial>` values cannot be displayed,
/// as they would need to have access to the `World`.
///
/// Use [`InspectorUi::new`] instead to provide context or use one of the methods in [`bevy_inspector`](crate::inspection::bevy_inspector).
pub fn ui_for_value_readonly(
    value: &dyn PartialReflect,
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
) -> Result {
    InspectorUi::new(type_registry, &mut Context::default()).ui_for_reflect_readonly(value, ui)
}

#[derive(Default)]
pub struct Context<'a> {
    pub world: Option<RestrictedWorldView<'a>>,
    pub queue: Option<&'a mut CommandQueue>,
}

pub struct InspectorUi<'a, 'c> {
    /// Reference to the [`TypeRegistry`]
    pub type_registry: &'a TypeRegistry,
    /// [`Context`] with additional data that can be used to display values
    pub context: &'a mut Context<'c>,
}

impl<'a, 'c> InspectorUi<'a, 'c> {
    pub fn new(type_registry: &'a TypeRegistry, context: &'a mut Context<'c>) -> Self {
        Self {
            type_registry,
            context,
        }
    }
}

impl InspectorUi<'_, '_> {
    /// Draws the inspector UI for the given value.
    pub fn ui_for_reflect(
        &mut self,
        value: &mut dyn PartialReflect,
        ui: &mut egui::Ui,
    ) -> Result<bool> {
        self.ui_for_reflect_with_options(value, ui, egui::Id::NULL, &())
    }

    /// Draws the inspector UI for the given value in a read-only way.
    pub fn ui_for_reflect_readonly(
        &mut self,
        value: &dyn PartialReflect,
        ui: &mut egui::Ui,
    ) -> Result {
        self.ui_for_reflect_readonly_with_options(value, ui, egui::Id::NULL, &())
    }

    /// Draws the inspector UI for the given value with some options.
    ///
    /// The options can be [`struct@InspectorOptions`] for structs or enums with nested options for their fields,
    /// or other structs like [`NumberOptions`](crate::inspection::inspector_options::std_options::NumberOptions) which are interpreted
    /// by leaf types like `f32` or `Vec3`,
    pub fn ui_for_reflect_with_options(
        &mut self,
        value: &mut dyn PartialReflect,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        let mut options = options;
        if options.is::<()>()
            && let Some(data) = value.try_as_reflect().and_then(|val| {
                self.type_registry
                    .get_type_data::<ReflectInspectorOptions>(val.type_id())
            })
        {
            options = &data.0;
        }
        let reason = match value.try_as_reflect_mut() {
            Some(value) => match get_type_data(self.type_registry, value) {
                Ok(ui_impl) => {
                    return ui_impl.ui(
                        ui,
                        options,
                        id,
                        self.reborrow(),
                        value.as_partial_reflect_mut(),
                    );
                }
                Err(e) => e,
            },
            None => TypeDataError::NotFullyReflected,
        };

        let result = match value.reflect_mut() {
            ReflectMut::Struct(value) => self.ui_for_struct(value, ui, id, options)?,
            ReflectMut::TupleStruct(value) => self.ui_for_tuple_struct(value, ui, id, options)?,
            ReflectMut::Tuple(value) => self.ui_for_tuple(value, ui, id, options)?,
            ReflectMut::List(value) => self.ui_for_list(value, ui, id, options)?,
            ReflectMut::Array(value) => self.ui_for_array(value, ui, id, options)?,
            ReflectMut::Map(value) => self.ui_for_reflect_map(value, ui, id, options)?,
            ReflectMut::Enum(value) => self.ui_for_enum(value, ui, id, options)?,
            ReflectMut::Opaque(value) => {
                error::reflect_value_no_impl(ui, reason, value.reflect_short_type_path());
                false
            }
            ReflectMut::Set(value) => self.ui_for_set(value, ui, id, options)?,
            #[allow(unreachable_patterns)]
            _ => {
                ui.label("unsupported");
                false
            }
        };

        Ok(result)
    }

    /// Draws the inspector UI for the given value with some options in a read-only way.
    ///
    /// The options can be [`struct@InspectorOptions`] for structs or enums with nested options for their fields,
    /// or other structs like [`NumberOptions`](crate::inspection::inspector_options::std_options::NumberOptions) which are interpreted
    /// by leaf types like `f32` or `Vec3`,
    pub fn ui_for_reflect_readonly_with_options(
        &mut self,
        value: &dyn PartialReflect,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        let mut options = options;
        if options.is::<()>()
            && let Some(value_reflect) = value.try_as_reflect()
            && let Some(data) = self
                .type_registry
                .get_type_data::<ReflectInspectorOptions>(value_reflect.type_id())
        {
            options = &data.0;
        }

        let reason = match value.try_as_reflect() {
            Some(value) => match get_type_data(self.type_registry, value) {
                Ok(ui_impl) => {
                    return ui_impl.ui_readonly(
                        ui,
                        options,
                        id,
                        self.reborrow(),
                        value.as_partial_reflect(),
                    );
                }
                Err(e) => e,
            },
            None => TypeDataError::NotFullyReflected,
        };

        match value.reflect_ref() {
            ReflectRef::Struct(value) => self.ui_for_struct_readonly(value, ui, id, options)?,
            ReflectRef::TupleStruct(value) => {
                self.ui_for_tuple_struct_readonly(value, ui, id, options)?
            }
            ReflectRef::Tuple(value) => self.ui_for_tuple_readonly(value, ui, id, options)?,
            ReflectRef::List(value) => self.ui_for_list_readonly(value, ui, id, options)?,
            ReflectRef::Array(value) => self.ui_for_array_readonly(value, ui, id, options)?,
            ReflectRef::Map(value) => self.ui_for_reflect_map_readonly(value, ui, id, options)?,
            ReflectRef::Enum(value) => self.ui_for_enum_readonly(value, ui, id, options)?,
            ReflectRef::Opaque(value) => {
                error::reflect_value_no_impl(ui, reason, value.reflect_short_type_path())
            }
            ReflectRef::Set(value) => self.ui_for_set_readonly(value, ui, id, options)?,
            #[allow(unreachable_patterns)]
            _ => {
                ui.label("unsupported");
            }
        };

        Ok(())
    }

    pub fn ui_for_reflect_many(
        &mut self,
        type_id: TypeId,
        name: &str,
        ui: &mut egui::Ui,
        id: egui::Id,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        self.ui_for_reflect_many_with_options(type_id, name, ui, id, &(), values)
    }

    pub fn ui_for_reflect_many_with_options(
        &mut self,
        type_id: TypeId,
        name: &str,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        let Some(registration) = self.type_registry.get(type_id) else {
            error::not_in_type_registry(ui, name);
            return Ok(false);
        };
        let info = registration.type_info();

        let mut options = options;
        if options.is::<()>()
            && let Some(data) = self
                .type_registry
                .get_type_data::<ReflectInspectorOptions>(type_id)
        {
            options = &data.0;
        }

        let reason = match registration.data::<ReflectInspector>() {
            Some(ui_impl) => {
                return ui_impl.ui_many(ui, options, id, self.reborrow(), values);
            }
            None => TypeDataError::NoTypeData,
        };

        if let Some(s) = self
            .type_registry
            .get_type_data::<ReflectInspector>(type_id)
        {
            return s.ui_many(ui, options, id, self.reborrow(), values);
        }

        let result = match info {
            TypeInfo::Struct(info) => self.ui_for_struct_many(info, ui, id, options, values)?,
            TypeInfo::TupleStruct(info) => {
                self.ui_for_tuple_struct_many(info, ui, id, options, values)?
            }
            TypeInfo::Tuple(info) => self.ui_for_tuple_many(info, ui, id, options, values)?,
            TypeInfo::List(info) => self.ui_for_list_many(info, ui, id, options, values)?,
            TypeInfo::Array(info) => {
                error::no_multiedit(ui, &pretty_type_name_str(info.type_path()));
                false
            }
            TypeInfo::Map(info) => {
                error::no_multiedit(ui, &pretty_type_name_str(info.type_path()));
                false
            }
            TypeInfo::Enum(info) => self.ui_for_enum_many(info, ui, id, options, values)?,
            TypeInfo::Opaque(info) => {
                error::reflect_value_no_impl(ui, reason, info.type_path());
                false
            }
            TypeInfo::Set(info) => self.ui_for_set_many(info, ui, id, options, values)?,
        };

        Ok(result)
    }
}

enum ListOp {
    AddElement(usize),
    RemoveElement(usize),
    MoveElementUp(usize),
    MoveElementDown(usize),
}

enum SetOp {
    RemoveElement(Box<dyn PartialReflect>),
    AddElement(Box<dyn PartialReflect>),
}

fn ui_for_empty_collection(ui: &mut egui::Ui, label: impl Into<WidgetText>) -> bool {
    let mut add = false;
    ui.vertical_centered(|ui| {
        ui.label(label);
        if add_button(ui).on_hover_text("Add element").clicked() {
            add = true;
        }
    });
    add
}

fn ui_for_empty_list(ui: &mut egui::Ui) -> bool {
    ui_for_empty_collection(ui, "(Empty List)")
}

fn ui_for_list_controls(ui: &mut egui::Ui, index: usize, len: usize) -> Option<ListOp> {
    use ListOp::*;
    let mut op = None;
    ui.horizontal_top(|ui| {
        if add_button(ui).on_hover_text("Add element").clicked() {
            op = Some(AddElement(index));
        }
        if remove_button(ui).on_hover_text("Remove element").clicked() {
            op = Some(RemoveElement(index));
        }
        let up_enabled = index > 0;
        ui.add_enabled_ui(up_enabled, |ui| {
            if up_button(ui).on_hover_text("Move element up").clicked() {
                op = Some(MoveElementUp(index));
            }
        });
        let down_enabled = len.checked_sub(1).map(|l| index < l).unwrap_or(false);
        ui.add_enabled_ui(down_enabled, |ui| {
            if down_button(ui).on_hover_text("Move element down").clicked() {
                op = Some(MoveElementDown(index));
            }
        });
    });
    op
}

fn ui_for_empty_set(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| ui.label("(Empty Set)"));
}

struct MapDraftElement {
    key: Box<dyn PartialReflect>,
    value: Box<dyn PartialReflect>,
}
impl Clone for MapDraftElement {
    fn clone(&self) -> Self {
        Self {
            key: self.key.to_dynamic(),
            value: self.value.to_dynamic(),
        }
    }
}

struct SetDraftElement(Box<dyn PartialReflect>);

impl Clone for SetDraftElement {
    fn clone(&self) -> Self {
        Self(self.0.to_dynamic())
    }
}

impl InspectorUi<'_, '_> {
    fn ui_for_struct(
        &mut self,
        value: &mut dyn Struct,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        let Some(TypeInfo::Struct(type_info)) = value.get_represented_type_info() else {
            return Ok(false);
        };

        let mut changed = false;
        Grid::new(id)
            .show(ui, |ui| -> Result {
                for i in 0..value.field_len() {
                    let field_info = type_info.field_at(i).unwrap();

                    let _response = ui.label(field_info.name());
                    #[cfg(feature = "documentation")]
                    show_docs(_response, field_info.docs());

                    let field = value.field_at_mut(i).unwrap();
                    changed |= self.ui_for_reflect_with_options(
                        field,
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                    )?;
                    ui.end_row();
                }

                Ok(())
            })
            .inner?;

        Ok(changed)
    }

    fn ui_for_struct_readonly(
        &mut self,
        value: &dyn Struct,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        let Some(TypeInfo::Struct(type_info)) = value.get_represented_type_info() else {
            return Ok(());
        };

        Grid::new(id)
            .show(ui, |ui| -> Result {
                for i in 0..value.field_len() {
                    let field_info = type_info.field_at(i).unwrap();

                    let _response = ui.label(field_info.name());
                    #[cfg(feature = "documentation")]
                    show_docs(_response, field_info.docs());

                    let field = value.field_at(i).unwrap();
                    self.ui_for_reflect_readonly_with_options(
                        field,
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                    )?;
                    ui.end_row();
                }

                Ok(())
            })
            .inner
    }

    fn ui_for_struct_many(
        &mut self,
        info: &StructInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        let mut changed = false;
        Grid::new(id)
            .show(ui, |ui| -> Result {
                for (i, field) in info.iter().enumerate() {
                    let _response = ui.label(field.name());
                    #[cfg(feature = "documentation")]
                    show_docs(_response, field.docs());

                    let mut values = values
                        .iter_mut()
                        .map(|value| match value.reflect_mut() {
                            ReflectMut::Struct(strukt) => strukt.field_at_mut(i).unwrap(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    changed |= self.ui_for_reflect_many_with_options(
                        field.type_id(),
                        field.type_path(),
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                        &mut values,
                    )?;
                    ui.end_row();
                }

                Ok(())
            })
            .inner?;
        Ok(changed)
    }

    fn ui_for_tuple_struct(
        &mut self,
        value: &mut dyn TupleStruct,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        maybe_grid(value.field_len(), ui, id, |ui, label| {
            (0..value.field_len())
                .map(|i| {
                    if label {
                        ui.label(i.to_string());
                    }
                    let field = value.field_mut(i).unwrap();
                    let changed = self.ui_for_reflect_with_options(
                        field,
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                    )?;
                    ui.end_row();
                    Ok(changed)
                })
                .fold(Ok(false), or)
        })
    }

    fn ui_for_tuple_struct_readonly(
        &mut self,
        value: &dyn TupleStruct,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        maybe_grid_readonly(value.field_len(), ui, id, |ui, label| -> Result {
            for i in 0..value.field_len() {
                if label {
                    ui.label(i.to_string());
                }
                let field = value.field(i).unwrap();
                self.ui_for_reflect_readonly_with_options(
                    field,
                    ui,
                    id.with(i),
                    inspector_options_struct_field(options, i),
                )?;
                ui.end_row();
            }

            Ok(())
        })
    }

    fn ui_for_tuple_struct_many(
        &mut self,
        info: &TupleStructInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        maybe_grid(info.field_len(), ui, id, |ui, label| {
            info.iter()
                .enumerate()
                .map(|(i, field)| {
                    if label {
                        ui.label(i.to_string());
                    }

                    let mut values = values
                        .iter_mut()
                        .map(|value| match value.reflect_mut() {
                            ReflectMut::TupleStruct(s) => s.field_mut(i).unwrap(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    let changed = self.ui_for_reflect_many_with_options(
                        field.type_id(),
                        field.type_path(),
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                        &mut values,
                    )?;

                    ui.end_row();
                    Ok(changed)
                })
                .fold(Ok(false), or)
        })
    }

    fn ui_for_tuple(
        &mut self,
        value: &mut dyn Tuple,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        maybe_grid(value.field_len(), ui, id, |ui, label| {
            (0..value.field_len())
                .map(|i| {
                    if label {
                        ui.label(i.to_string());
                    }
                    let field = value.field_mut(i).unwrap();
                    let changed = self.ui_for_reflect_with_options(
                        field,
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                    )?;
                    ui.end_row();
                    Ok(changed)
                })
                .fold(Ok(false), or)
        })
    }

    fn ui_for_tuple_readonly(
        &mut self,
        value: &dyn Tuple,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        maybe_grid_readonly(value.field_len(), ui, id, |ui, label| {
            for i in 0..value.field_len() {
                if label {
                    ui.label(i.to_string());
                }
                let field = value.field(i).unwrap();
                self.ui_for_reflect_readonly_with_options(
                    field,
                    ui,
                    id.with(i),
                    inspector_options_struct_field(options, i),
                )?;
                ui.end_row();
            }

            Ok(())
        })
    }

    fn ui_for_tuple_many(
        &mut self,
        info: &TupleInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        maybe_grid(info.field_len(), ui, id, |ui, label| {
            info.iter()
                .enumerate()
                .map(|(i, field)| {
                    if label {
                        ui.label(i.to_string());
                    }

                    let mut values = values
                        .iter_mut()
                        .map(|value| match value.reflect_mut() {
                            ReflectMut::Tuple(strukt) => strukt.field_mut(i).unwrap(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();

                    let changed = self.ui_for_reflect_many_with_options(
                        field.type_id(),
                        field.type_path(),
                        ui,
                        id.with(i),
                        inspector_options_struct_field(options, i),
                        &mut values,
                    )?;
                    ui.end_row();
                    Ok(changed)
                })
                .fold(Ok(false), or)
        })
    }

    /// Mutate one or more lists based on a [`ListOp`], generated by some user interaction.
    fn respond_to_list_op<'a>(
        &mut self,
        ui: &mut egui::Ui,
        id: egui::Id,
        lists: impl Iterator<Item = &'a mut dyn List>,
        op: ListOp,
    ) -> Result<bool> {
        use ListOp::*;
        let mut changed = false;
        let error_id = id.with("error");

        for list in lists {
            let Some(TypeInfo::List(info)) = list.get_represented_type_info() else {
                continue;
            };
            match op {
                AddElement(i) => {
                    let default = self
                        .get_default_value_for(info.item_ty().id())
                        .map(|def| def.into_partial_reflect())
                        .or_else(|| list.get(i).map(|v| v.to_dynamic()));
                    if let Some(new_value) = default {
                        list.insert(i, new_value);
                    } else {
                        ui.data_mut(|data| data.insert_temp::<bool>(error_id, true));
                    }
                    changed = true;
                }
                RemoveElement(i) => {
                    list.remove(i);
                    changed = true;
                }
                MoveElementUp(i) => {
                    if let Some(prev_idx) = i.checked_sub(1) {
                        // Clone this element and insert it at its index - 1.
                        if let Some(element) = list.get(i) {
                            let clone = element.to_dynamic();
                            list.insert(prev_idx, clone);
                        }
                        // Remove the original, now at its index + 1.
                        list.remove(i + 1);
                        changed = true;
                    }
                }
                MoveElementDown(i) => {
                    // Clone the next element and insert it at this index.
                    if let Some(next_element) = list.get(i + 1) {
                        let next_clone = next_element.to_dynamic();
                        list.insert(i, next_clone);
                    }
                    // Remove the original, now at i + 2.
                    list.remove(i + 2);
                    changed = true;
                }
            }
        }

        Ok(changed)
    }

    fn ui_for_list(
        &mut self,
        list: &mut dyn List,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        use ListOp::*;
        let mut changed = false;

        ui.vertical(|ui| -> Result {
            let mut op = None;
            let len = list.len();
            if len == 0 && ui_for_empty_list(ui) {
                op = Some(AddElement(0))
            }
            for i in 0..len {
                egui::Grid::new((id, i))
                    .show(ui, |ui| -> Result {
                        ui.label(i.to_string());
                        let val = list.get_mut(i).unwrap();
                        ui.horizontal_top(|ui| -> Result {
                            changed |=
                                self.ui_for_reflect_with_options(val, ui, id.with(i), options)?;
                            Ok(())
                        })
                        .inner?;
                        ui.end_row();

                        let item_op = ui_for_list_controls(ui, i, len);
                        if item_op.is_some() {
                            op = item_op;
                        }

                        Ok(())
                    })
                    .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            let Some(TypeInfo::List(info)) = list.get_represented_type_info() else {
                return Ok(());
            };
            let error_id = id.with("error");

            // Respond to control interaction
            if let Some(op) = op {
                let lists = std::iter::once(list);
                changed |= self.respond_to_list_op(ui, id, lists, op)?;
            }

            let error = ui.data_mut(|data| *data.get_temp_mut_or_default::<bool>(error_id));
            if error {
                error::no_default_value(ui, info.type_path());
            }
            if ui.input(|input| input.pointer.any_down()) {
                ui.data_mut(|data| data.insert_temp::<bool>(error_id, false));
            }

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    fn ui_for_list_readonly(
        &mut self,
        list: &dyn List,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        ui.vertical(|ui| -> Result {
            let len = list.len();
            for i in 0..len {
                let val = list.get(i).unwrap();
                ui.horizontal_top(|ui| -> Result {
                    self.ui_for_reflect_readonly_with_options(val, ui, id.with(i), options)?;
                    Ok(())
                })
                .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            Ok(())
        })
        .inner
    }

    fn ui_for_list_many(
        &mut self,
        info: &ListInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        use ListOp::*;
        let mut changed = false;

        let same_len = iter_all_eq(values.iter_mut().map(|value| match value.reflect_mut() {
            ReflectMut::List(l) => l.len(),
            _ => unreachable!(),
        }));

        let Some(len) = same_len else {
            ui.label("lists have different sizes, cannot multiedit");
            return Ok(changed);
        };

        ui.vertical(|ui| -> Result {
            let mut op = None;

            if len == 0 && ui_for_empty_list(ui) {
                op = Some(AddElement(0));
            }

            for i in 0..len {
                let mut items_at_i: Vec<&mut dyn PartialReflect> = values
                    .iter_mut()
                    .map(|value| match value.reflect_mut() {
                        ReflectMut::List(list) => list.get_mut(i).unwrap(),
                        _ => unreachable!(),
                    })
                    .collect();

                egui::Grid::new((id, i))
                    .show(ui, |ui| -> Result {
                        ui.label(i.to_string());
                        ui.horizontal_top(|ui| -> Result {
                            changed |= self.ui_for_reflect_many_with_options(
                                info.item_ty().id(),
                                info.type_path(),
                                ui,
                                id.with(i),
                                options,
                                items_at_i.as_mut_slice(),
                            )?;

                            Ok(())
                        })
                        .inner?;
                        ui.end_row();
                        let item_op = ui_for_list_controls(ui, i, len);
                        if item_op.is_some() {
                            op = item_op;
                        }

                        Ok(())
                    })
                    .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            let error_id = id.with("error");
            let error = ui.data_mut(|data| *data.get_temp_mut_or_default::<bool>(error_id));
            if error {
                error::no_default_value(ui, info.type_path());
            }
            if ui.input(|input| input.pointer.any_down()) {
                ui.data_mut(|data| data.insert_temp::<bool>(error_id, false));
            }
            if let Some(op) = op {
                let lists = values.iter_mut().map(|l| match l.reflect_mut() {
                    ReflectMut::List(list) => list,
                    _ => unreachable!(),
                });
                changed |= self.respond_to_list_op(ui, id, lists, op)?;
            }

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    fn ui_for_reflect_map(
        &mut self,
        map: &mut dyn Map,
        ui: &mut egui::Ui,
        id: egui::Id,
        _options: &dyn Any,
    ) -> Result<bool> {
        let mut changed = false;
        if map.is_empty() {
            ui.label("(Empty Map)");
            ui.end_row();
        }

        egui::Grid::new(id)
            .show(ui, |ui| -> Result {
                let mut i = 0;

                let mut error = None;

                map.retain(&mut |key, value| {
                    if error.is_some() {
                        return true;
                    }

                    let ui_id = id.with(i);
                    i += 1;

                    if let Err(err) = self.ui_for_reflect_readonly_with_options(key, ui, ui_id, &())
                    {
                        error = Some(err);
                        return true;
                    }

                    match self.ui_for_reflect_with_options(value, ui, ui_id, &()) {
                        Ok(value) => {
                            changed |= value;
                        }
                        Err(err) => {
                            error = Some(err);
                            return true;
                        }
                    }

                    let delete = remove_button(ui).on_hover_text("Remove element").clicked();
                    ui.end_row();

                    !delete
                });

                if let Some(error) = error {
                    return Err(error);
                }

                self.map_add_element_ui(map, ui, id, &mut changed)?;

                Ok(())
            })
            .inner?;

        Ok(changed)
    }

    fn map_add_element_ui(
        &mut self,
        map: &mut (dyn Map + 'static),
        ui: &mut egui::Ui,
        id: egui::Id,
        changed: &mut bool,
    ) -> Result {
        let map_draft_id = id.with("map_draft");
        let draft_clone = ui.data_mut(|data| {
            data.get_temp_mut_or_default::<Option<MapDraftElement>>(map_draft_id)
                .to_owned()
        });

        let Some(map_info) = map.get_represented_map_info() else {
            return Ok(());
        };

        let Some(key_default) = self.get_reflect_default(map_info.key_ty().id()) else {
            return Ok(());
        };

        let Some(value_default) = self.get_reflect_default(map_info.value_ty().id()) else {
            return Ok(());
        };

        ui.separator();
        ui.end_row();
        ui.label("New element");
        match draft_clone {
            None => {
                // If no draft element exists, show a button to create one.
                if add_button(ui).clicked() {
                    // Insert a temporary 'draft' key-value pair into UI state.
                    let key = key_default.default().into_partial_reflect();
                    let value = value_default.default().into_partial_reflect();
                    ui.data_mut(|data| {
                        data.insert_temp(map_draft_id, Some(MapDraftElement { key, value }))
                    });
                }
                ui.end_row();
            }
            Some(MapDraftElement { mut key, mut value }) => {
                ui.end_row();
                // Show controls for editing our draft element.
                let key_changed = self.ui_for_reflect_with_options(key.as_mut(), ui, id, &())?;
                let value_changed =
                    self.ui_for_reflect_with_options(value.as_mut(), ui, id, &())?;

                // If the clone changed, update the data in UI state.
                if key_changed || value_changed {
                    let next_draft = MapDraftElement { key, value };
                    ui.data_mut(|data| data.insert_temp(map_draft_id, Some(next_draft)));
                }

                // Show controls to insert the draft into the map, or remove it.
                if ui.button("Insert").clicked() {
                    let draft = ui
                        .data_mut(|data| data.get_temp::<Option<MapDraftElement>>(map_draft_id))
                        .flatten();
                    if let Some(draft) = draft {
                        map.insert_boxed(draft.key, draft.value);
                        ui.data_mut(|data| data.remove_by_type::<Option<MapDraftElement>>());
                    }
                    *changed = true;
                }

                if ui.button("Cancel").clicked() {
                    ui.data_mut(|data| data.remove_by_type::<Option<MapDraftElement>>());
                    *changed = true;
                }
                ui.end_row();
            }
        }

        Ok(())
    }

    fn ui_for_reflect_map_readonly(
        &mut self,
        map: &dyn Map,
        ui: &mut egui::Ui,
        id: egui::Id,
        _options: &dyn Any,
    ) -> Result {
        egui::Grid::new(id)
            .show(ui, |ui| -> Result {
                for (i, (key, value)) in map.iter().enumerate() {
                    let ui_id = id.with(i);
                    self.ui_for_reflect_readonly_with_options(key, ui, ui_id, &())?;
                    self.ui_for_reflect_readonly_with_options(value, ui, ui_id, &())?;
                    ui.end_row();
                }

                Ok(())
            })
            .inner
    }

    /// Mutate one or more lists based on a [`SetOp`], generated by some user interaction.
    fn respond_to_sets_op<'a>(
        &mut self,
        sets: impl Iterator<Item = &'a mut dyn Set>,
        op: SetOp,
    ) -> Result<bool> {
        let mut changed = false;

        for set in sets {
            changed |= self.respond_to_set_op(set, &op)?;
        }

        Ok(changed)
    }
    fn respond_to_set_op<'a>(&mut self, set: &'a mut dyn Set, op: &SetOp) -> Result<bool> {
        use SetOp::*;
        match &op {
            AddElement(new_value) => {
                set.insert_boxed(new_value.to_dynamic());
            }
            RemoveElement(val) => {
                set.remove(&**val);
            }
        }
        Ok(true)
    }

    fn ui_for_set(
        &mut self,
        set: &mut dyn Set,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        use SetOp::*;
        let mut changed = false;

        ui.vertical(|ui| -> Result {
            let mut op = None;

            let len = set.len();
            if len == 0 {
                ui_for_empty_set(ui);
            }

            for (i, val) in set.iter().enumerate() {
                egui::Grid::new((id, i))
                    .show(ui, |ui| -> Result {
                        ui.horizontal_top(|ui| -> Result {
                            self.ui_for_reflect_readonly_with_options(val, ui, id.with(i), options)
                        })
                        .inner?;
                        ui.horizontal_top(|ui| {
                            if remove_button(ui).on_hover_text("Remove element").clicked() {
                                let copy = val.to_dynamic();
                                op = Some(RemoveElement(copy));
                            }
                        });
                        ui.end_row();
                        Ok(())
                    })
                    .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }
            let Some(TypeInfo::Set(set_info)) = set.get_represented_type_info() else {
                return Ok(());
            };
            let value_type = set_info.value_ty();
            let new_op = self.set_add_element_ui(value_type, ui, id, options, &mut changed)?;
            if new_op.is_some() {
                op = new_op;
            }

            ui.end_row();

            let error_id = id.with("error");

            // Respond to control interaction
            if let Some(op) = op {
                changed |= self.respond_to_set_op(set, &op)?;
            }

            let error = ui.data_mut(|data| *data.get_temp_mut_or_default::<bool>(error_id));
            if error {
                error::no_default_value(ui, set_info.type_path());
            }
            if ui.input(|input| input.pointer.any_down()) {
                ui.data_mut(|data| data.insert_temp::<bool>(error_id, false));
            }

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    #[must_use]
    fn set_add_element_ui(
        &mut self,
        value_type: bevy::reflect::Type,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        changed: &mut bool,
    ) -> Result<Option<SetOp>> {
        let mut op = None;

        let Some(item_default) = self.get_reflect_default(value_type.id()) else {
            return Ok(op);
        };

        let item_default = item_default.clone();

        ui.vertical(|ui| -> Result {
            ui.label("New element");
            let set_draft_id = id.with("set_draft");
            let draft_clone = ui.data_mut(|data| {
                data.get_temp_mut_or_default::<Option<SetDraftElement>>(set_draft_id)
                    .to_owned()
            });
            ui.end_row();
            match draft_clone {
                None => {
                    // If no draft element exists, show a button to create one.
                    if add_button(ui).clicked() {
                        // Insert a temporary 'draft' value into UI state, once inserted, we cannot modify it.
                        let draft = SetDraftElement(item_default.default().into_partial_reflect());
                        ui.data_mut(|data| data.insert_temp(set_draft_id, Some(draft)));
                    }

                    ui.end_row();
                }
                Some(SetDraftElement(mut v)) => {
                    ui.end_row();
                    // Show controls for editing our draft element.
                    // FIXME: is the id passed here correct?
                    let value_changed =
                        self.ui_for_reflect_with_options(v.as_mut(), ui, id, options)?;

                    // If the clone changed, update the data in UI state.
                    if value_changed {
                        let next_draft = SetDraftElement(v);
                        ui.data_mut(|data| data.insert_temp(set_draft_id, Some(next_draft)));
                    }

                    // Show controls to insert the draft into the set, or remove it.
                    if ui.button("Insert").clicked() {
                        let draft = ui
                            .data_mut(|data| data.get_temp::<Option<SetDraftElement>>(set_draft_id))
                            .flatten();
                        if let Some(draft) = draft {
                            op = Some(SetOp::AddElement(draft.0));
                            ui.data_mut(|data| data.remove_by_type::<Option<SetDraftElement>>());
                        }
                        *changed = true;
                    }

                    if ui.button("Cancel").clicked() {
                        ui.data_mut(|data| data.remove_by_type::<Option<SetDraftElement>>());
                        *changed = true;
                    }
                    ui.end_row();
                }
            }

            Ok(())
        })
        .inner?;

        Ok(op)
    }

    fn ui_for_set_readonly(
        &mut self,
        set: &dyn Set,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        let len = set.len();
        ui.vertical(|ui| -> Result {
            for (i, val) in set.iter().enumerate() {
                ui.horizontal_top(|ui| -> Result {
                    self.ui_for_reflect_readonly_with_options(val, ui, id.with(i), options)
                })
                .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            Ok(())
        })
        .inner
    }

    fn ui_for_set_many(
        &mut self,
        info: &SetInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        use SetOp::*;
        let mut changed = false;

        let same_len = iter_all_eq(values.iter_mut().map(|value| match value.reflect_mut() {
            ReflectMut::List(l) => l.len(),
            _ => unreachable!(),
        }));

        let Some(len) = same_len else {
            ui.label("lists have different sizes, cannot multiedit");
            return Ok(changed);
        };

        ui.vertical(|ui| -> Result {
            let mut op = None;

            if len == 0 {
                ui_for_empty_set(ui)
            }

            let set0 = match values[0].reflect_mut() {
                ReflectMut::Set(set) => set,
                _ => unreachable!(),
            };
            let Some(TypeInfo::Set(set_info)) = set0.get_represented_type_info() else {
                return Ok(());
            };
            let value_type = set_info.value_ty();
            let reflected_values: Vec<Box<dyn PartialReflect>> =
                set0.iter().map(|v| v.to_dynamic()).collect();

            for (i, value_to_check) in reflected_values.iter().enumerate() {
                let value_type_id = (**value_to_check).type_id();
                egui::Grid::new((value_type_id, i))
                    .show(ui, |ui| -> Result {
                        // Do all sets contain this value ?
                        if len == 1
                            || values[1..].iter_mut().all(|set_to_compare| {
                                let set_to_compare = match set_to_compare.reflect_mut() {
                                    ReflectMut::Set(set) => set,
                                    _ => unreachable!(),
                                };
                                set_to_compare.iter().any(|value| {
                                    value.reflect_partial_eq(value_to_check.borrow()) == Some(true)
                                })
                            })
                        {
                            // All sets contain this value: Show value
                            ui.horizontal_top(|ui| -> Result {
                                self.ui_for_reflect_readonly_with_options(
                                    value_to_check.borrow(),
                                    ui,
                                    // FIXME: is the id passed here correct?
                                    id.with(i),
                                    options,
                                )?;
                                Ok(())
                            })
                            .inner?;
                            ui.horizontal_top(|ui| {
                                if remove_button(ui).on_hover_text("Remove element").clicked() {
                                    let copy = value_to_check.to_dynamic();
                                    op = Some(RemoveElement(copy));
                                }
                            });
                        } else {
                            ui.label("Different values");
                        }

                        ui.end_row();

                        Ok(())
                    })
                    .inner?;
                if i != len - 1 {
                    ui.separator();
                }
            }
            let op = self.set_add_element_ui(value_type, ui, id, options, &mut changed)?;

            ui.end_row();

            let error_id = id.with("error");
            let error = ui.data_mut(|data| *data.get_temp_mut_or_default::<bool>(error_id));
            if error {
                error::no_default_value(ui, info.type_path());
            }
            if ui.input(|input| input.pointer.any_down()) {
                ui.data_mut(|data| data.insert_temp::<bool>(error_id, false));
            }
            if let Some(op) = op {
                let sets = values.iter_mut().map(|l| match l.reflect_mut() {
                    ReflectMut::Set(list) => list,
                    _ => unreachable!(),
                });
                changed |= self.respond_to_sets_op(sets, op)?;
            }

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    fn ui_for_array(
        &mut self,
        array: &mut dyn Array,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        let mut changed = false;

        ui.vertical(|ui| -> Result {
            let len = array.len();
            for i in 0..len {
                let val = array.get_mut(i).unwrap();
                ui.horizontal_top(|ui| -> Result {
                    changed |= self.ui_for_reflect_with_options(val, ui, id.with(i), options)?;
                    Ok(())
                })
                .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    fn ui_for_array_readonly(
        &mut self,
        array: &dyn Array,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        ui.vertical(|ui| -> Result {
            let len = array.len();
            for i in 0..len {
                let val = array.get(i).unwrap();
                ui.horizontal_top(|ui| -> Result {
                    self.ui_for_reflect_readonly_with_options(val, ui, id.with(i), options)
                })
                .inner?;

                if i != len - 1 {
                    ui.separator();
                }
            }

            Ok(())
        })
        .inner
    }

    fn ui_for_enum(
        &mut self,
        value: &mut dyn Enum,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result<bool> {
        let Some(type_info) = value.get_represented_type_info() else {
            ui.label("Unrepresentable");
            return Ok(false);
        };
        let type_info = match type_info {
            TypeInfo::Enum(info) => info,
            _ => unreachable!("invalid reflect impl: type info mismatch"),
        };

        let mut changed = false;

        ui.vertical(|ui| -> Result {
            let changed_variant =
                self.ui_for_enum_variant_select(id, ui, value.variant_index(), type_info);
            if let Some((_new_variant, dynamic_enum)) = changed_variant {
                changed = true;
                value.apply(&dynamic_enum);
            }
            let variant_index = value.variant_index();

            let always_show_label = matches!(value.variant_type(), VariantType::Struct);
            changed |=
                maybe_grid_label_if(value.field_len(), ui, id, always_show_label, |ui, label| {
                    (0..value.field_len())
                        .map(|i| {
                            if label {
                                #[cfg(feature = "documentation")]
                                let field_docs = type_info.variant_at(variant_index).and_then(
                                    |info| match info {
                                        VariantInfo::Struct(info) => info.field_at(i)?.docs(),
                                        _ => None,
                                    },
                                );

                                let _response = if let Some(name) = value.name_at(i) {
                                    ui.label(name)
                                } else {
                                    ui.label(i.to_string())
                                };
                                #[cfg(feature = "documentation")]
                                show_docs(_response, field_docs);
                            }
                            let field_value = value
                                .field_at_mut(i)
                                .expect("invalid reflect impl: field len");
                            let changed = self.ui_for_reflect_with_options(
                                field_value,
                                ui,
                                id.with(i),
                                inspector_options_enum_variant_field(options, variant_index, i),
                            )?;
                            ui.end_row();
                            Ok(changed)
                        })
                        .fold(Ok(false), or)
                })?;

            Ok(())
        })
        .inner?;

        Ok(changed)
    }

    fn ui_for_enum_many(
        &mut self,
        info: &EnumInfo,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        let mut changed = false;

        let same_variant = iter_all_eq(values.iter_mut().map(|value| match value.reflect_mut() {
            ReflectMut::Enum(info) => info.variant_index(),
            _ => unreachable!(),
        }));

        if let Some(variant_index) = same_variant {
            let mut variant = info.variant_at(variant_index).unwrap();

            ui.vertical(|ui| -> Result {
                let variant_changed = self.ui_for_enum_variant_select(id, ui, variant_index, info);
                if let Some((new_variant_idx, dynamic_enum)) = variant_changed {
                    changed = true;
                    variant = info.variant_at(new_variant_idx).unwrap();

                    for value in values.iter_mut() {
                        value.apply(&dynamic_enum);
                    }
                }

                let field_len = match variant {
                    VariantInfo::Struct(info) => info.field_len(),
                    VariantInfo::Tuple(info) => info.field_len(),
                    VariantInfo::Unit(_) => 0,
                };

                let always_show_label = matches!(variant, VariantInfo::Struct(_));
                changed |=
                    maybe_grid_label_if(field_len, ui, id, always_show_label, |ui, label| {
                        let handle = |(field_index, field_name, field_type_id, field_type_name)| {
                            if label {
                                ui.label(field_name);
                            }

                            let mut variants_across: Vec<&mut dyn PartialReflect> = values
                                .iter_mut()
                                .map(|value| match value.reflect_mut() {
                                    ReflectMut::Enum(value) => {
                                        value.field_at_mut(field_index).unwrap()
                                    }
                                    _ => unreachable!(),
                                })
                                .collect();

                            self.ui_for_reflect_many_with_options(
                                field_type_id,
                                field_type_name,
                                ui,
                                id.with(field_index),
                                inspector_options_enum_variant_field(
                                    options,
                                    variant_index,
                                    field_index,
                                ),
                                variants_across.as_mut_slice(),
                            )?;

                            ui.end_row();

                            Ok(false)
                        };

                        match variant {
                            VariantInfo::Struct(info) => info
                                .iter()
                                .enumerate()
                                .map(|(i, field)| {
                                    (
                                        i,
                                        Cow::Borrowed(field.name()),
                                        field.type_id(),
                                        field.type_path(),
                                    )
                                })
                                .map(handle)
                                .fold(Ok(false), or),
                            VariantInfo::Tuple(info) => info
                                .iter()
                                .enumerate()
                                .map(|(i, field)| {
                                    (
                                        i,
                                        Cow::Owned(i.to_string()),
                                        field.type_id(),
                                        field.type_path(),
                                    )
                                })
                                .map(handle)
                                .fold(Ok(false), or),
                            VariantInfo::Unit(_) => Ok(false),
                        }
                    })?;

                Ok(())
            })
            .inner?;
        } else {
            ui.label("enums have different selected variants, cannot multiedit");
        }

        Ok(changed)
    }

    fn ui_for_enum_variant_select(
        &mut self,
        id: egui::Id,
        ui: &mut egui::Ui,
        active_variant_idx: usize,
        info: &bevy::reflect::EnumInfo,
    ) -> Option<(usize, DynamicEnum)> {
        let mut changed_variant = None;

        ui.horizontal_top(|ui| {
            egui::ComboBox::new(id.with("select"), "")
                .selected_text(info.variant_names()[active_variant_idx])
                .show_ui(ui, |ui| {
                    for (i, variant) in info.iter().enumerate() {
                        let variant_name = variant.name();
                        let is_active_variant = i == active_variant_idx;

                        let variant_is_constructable =
                            variant_constructable(self.type_registry, variant);

                        ui.add_enabled_ui(variant_is_constructable.is_ok(), |ui| {
                            let mut variant_label_response =
                                ui.selectable_label(is_active_variant, variant_name);

                            if let Err(fields) = variant_is_constructable {
                                variant_label_response = variant_label_response
                                    .on_disabled_hover_ui(|ui| {
                                        error::unconstructable_variant(
                                            ui,
                                            info.type_path(),
                                            variant_name,
                                            &fields,
                                        );
                                    });
                            }

                            /*let res = variant_label_response.on_hover_ui(|ui| {
                                if !unconstructable_variants.is_empty() {
                                    error::unconstructable_variants(
                                        ui,
                                        info.type_name(),
                                        &unconstructable_variants,
                                    );
                                }
                            });*/

                            if variant_label_response.clicked()
                                && let Ok(dynamic_enum) =
                                    self.construct_default_variant(variant, ui)
                            {
                                changed_variant = Some((i, dynamic_enum));
                            };
                        });
                    }

                    false
                });
        });

        changed_variant
    }

    fn ui_for_enum_readonly(
        &mut self,
        value: &dyn Enum,
        ui: &mut egui::Ui,
        id: egui::Id,
        options: &dyn Any,
    ) -> Result {
        ui.vertical(|ui| -> Result {
            let active_variant = value.variant_name();
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::new(id, "")
                    .selected_text(active_variant)
                    .show_ui(ui, |_| {})
            });

            let always_show_label = matches!(value.variant_type(), VariantType::Struct);
            maybe_grid_readonly_label_if(
                value.field_len(),
                ui,
                id,
                always_show_label,
                |ui, label| {
                    for i in 0..value.field_len() {
                        if label {
                            if let Some(name) = value.name_at(i) {
                                ui.label(name);
                            } else {
                                ui.label(i.to_string());
                            }
                        }
                        let field_value =
                            value.field_at(i).expect("invalid reflect impl: field len");
                        self.ui_for_reflect_readonly_with_options(
                            field_value,
                            ui,
                            id.with(i),
                            inspector_options_enum_variant_field(options, value.variant_index(), i),
                        )?;
                        ui.end_row();
                    }

                    Ok(())
                },
            )
        })
        .inner
    }
}

impl<'a, 'c> InspectorUi<'a, 'c> {
    pub fn reborrow<'s>(&'s mut self) -> InspectorUi<'s, 'c> {
        InspectorUi {
            type_registry: self.type_registry,
            context: self.context,
        }
    }

    fn get_reflect_default(&self, type_id: TypeId) -> Option<&ReflectDefault> {
        self.type_registry.get_type_data::<ReflectDefault>(type_id)
    }

    fn get_default_value_for(&mut self, type_id: TypeId) -> Option<Box<dyn Reflect>> {
        if let Some(reflect_default) = self.type_registry.get_type_data::<ReflectDefault>(type_id) {
            return Some(reflect_default.default());
        }

        None
    }

    fn construct_default_variant(
        &mut self,
        variant: &VariantInfo,
        ui: &mut egui::Ui,
    ) -> Result<DynamicEnum, ()> {
        let dynamic_variant = match variant {
            VariantInfo::Struct(struct_info) => {
                let mut dynamic_struct = DynamicStruct::default();
                for field in struct_info.iter() {
                    let field_default_value = match self.get_default_value_for(field.type_id()) {
                        Some(value) => value,
                        None => {
                            error::no_default_value(ui, field.type_path());
                            return Err(());
                        }
                    };
                    dynamic_struct.insert_boxed(field.name(), field_default_value.to_dynamic());
                }
                DynamicVariant::Struct(dynamic_struct)
            }
            VariantInfo::Tuple(tuple_info) => {
                let mut dynamic_tuple = DynamicTuple::default();
                for field in tuple_info.iter() {
                    let field_default_value = match self.get_default_value_for(field.type_id()) {
                        Some(value) => value,
                        None => {
                            error::no_default_value(ui, field.type_path());
                            return Err(());
                        }
                    };
                    dynamic_tuple.insert_boxed(field_default_value.to_dynamic());
                }
                DynamicVariant::Tuple(dynamic_tuple)
            }
            VariantInfo::Unit(_) => DynamicVariant::Unit,
        };
        let dynamic_enum = DynamicEnum::new(variant.name(), dynamic_variant);
        Ok(dynamic_enum)
    }
}

#[must_use]
fn maybe_grid(
    i: usize,
    ui: &mut egui::Ui,
    id: egui::Id,
    mut f: impl FnMut(&mut egui::Ui, bool) -> Result<bool>,
) -> Result<bool> {
    match i {
        0 => Ok(false),
        1 => f(ui, false),
        _ => Grid::new(id).show(ui, |ui| f(ui, true)).inner,
    }
}
#[must_use]
fn maybe_grid_label_if(
    i: usize,
    ui: &mut egui::Ui,
    id: egui::Id,
    always_show_label: bool,
    mut f: impl FnMut(&mut egui::Ui, bool) -> Result<bool>,
) -> Result<bool> {
    match i {
        0 => Ok(false),
        1 if !always_show_label => f(ui, false),
        _ => Grid::new(id).show(ui, |ui| f(ui, true)).inner,
    }
}

fn maybe_grid_readonly(
    i: usize,
    ui: &mut egui::Ui,
    id: egui::Id,
    mut f: impl FnMut(&mut egui::Ui, bool) -> Result,
) -> Result {
    match i {
        0 => Ok(()),
        1 => f(ui, false),
        _ => Grid::new(id).show(ui, |ui| f(ui, true)).inner,
    }
}
fn maybe_grid_readonly_label_if(
    i: usize,
    ui: &mut egui::Ui,
    id: egui::Id,
    always_show_label: bool,
    mut f: impl FnMut(&mut egui::Ui, bool) -> Result,
) -> Result {
    match i {
        0 => Ok(()),
        1 if !always_show_label => f(ui, false),
        _ => Grid::new(id).show(ui, |ui| f(ui, true)).inner,
    }
}

fn variant_constructable<'a>(
    type_registry: &TypeRegistry,
    variant: &'a VariantInfo,
) -> Result<(), Vec<&'a str>> {
    let type_id_is_constructable = |type_id: TypeId| {
        type_registry
            .get_type_data::<ReflectDefault>(type_id)
            .is_some()
    };

    let unconstructable_fields: Vec<&'a str> = match variant {
        VariantInfo::Struct(variant) => variant
            .iter()
            .filter_map(|field| {
                (!type_id_is_constructable(field.type_id())).then_some(field.type_path())
            })
            .collect(),
        VariantInfo::Tuple(variant) => variant
            .iter()
            .filter_map(|field| {
                (!type_id_is_constructable(field.type_id())).then_some(field.type_path())
            })
            .collect(),
        VariantInfo::Unit(_) => return Ok(()),
    };

    if unconstructable_fields.is_empty() {
        Ok(())
    } else {
        Err(unconstructable_fields)
    }
}

fn inspector_options_struct_field(options: &dyn Any, field: usize) -> &dyn Any {
    options
        .downcast_ref::<InspectorOptions>()
        .and_then(|options| options.get(Target::Field(field)))
        .unwrap_or(&())
}

fn inspector_options_enum_variant_field<'a>(
    options: &'a dyn Any,
    variant_index: usize,
    field_index: usize,
) -> &'a dyn Any {
    options
        .downcast_ref::<InspectorOptions>()
        .and_then(|options| {
            options.get(Target::VariantField {
                variant_index,
                field_index,
            })
        })
        .unwrap_or(&())
}

fn or(a: Result<bool>, b: Result<bool>) -> Result<bool> {
    Ok(a? || b?)
}

fn get_type_data<'a>(
    type_registry: &'a TypeRegistry,
    type_id: &dyn DynamicTyped,
) -> Result<&'a ReflectInspector, TypeDataError> {
    let registration = type_registry
        .get(type_id.reflect_type_info().type_id())
        .ok_or(TypeDataError::NotRegistered)?;
    let data = registration
        .data::<ReflectInspector>()
        .ok_or(TypeDataError::NoTypeData)?;
    Ok(data)
}
