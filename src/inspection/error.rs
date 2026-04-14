use std::{any::TypeId, borrow::Cow};

use bevy::asset::UntypedAssetId;
use bevy::ecs::entity::Entity;
use bevy::reflect::TypeRegistry;
use egui::FontId;

use crate::inspection::{
    egui_utils::layout_job, restricted_world_view::Error, utils::pretty_type_name_str,
};

/// Lacking access on restricted world view
pub fn no_access(error: Error, ui: &mut egui::Ui, name_of_type: &str) {
    match error {
        Error::NoAccessToResource(_) => no_access_resource(ui, name_of_type),
        Error::NoAccessToComponent((entity, _)) => no_access_component(ui, entity, name_of_type),
        Error::ComponentDoesNotExist((entity, _)) => {
            nonexistent_component(ui, entity, name_of_type)
        }
        Error::ResourceDoesNotExist(_) => nonexistent_resource(ui, name_of_type),
        Error::NoComponentId(_) => missing_componentid(ui, name_of_type),
        Error::NoTypeRegistration(_) => {
            crate::inspection::error::not_in_type_registry(ui, name_of_type)
        }
        Error::NoTypeData(_, data) => missing_typedata(ui, name_of_type, data),
    }
}

/// The inspector has no access to the given resource
pub fn no_access_resource(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "No access to resource "),
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

/// The inspector has no access to the given component
pub fn no_access_component(ui: &mut egui::Ui, entity: Entity, type_name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "No access to component "),
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " on entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

/// A resource necessary for the UI does not exist
pub fn nonexistent_resource(ui: &mut egui::Ui, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Resource "),
        (FontId::monospace(12.0), name),
        (FontId::proportional(13.0), " does not exist in the world."),
    ]);

    ui.label(job);
}

/// An entity does not exist
pub fn nonexistent_entity(ui: &mut egui::Ui, entity: Entity) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), " does not exist."),
    ]);

    ui.label(job);
}

/// A component necessary for the UI does not exist
pub fn nonexistent_component(ui: &mut egui::Ui, entity: Entity, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Component "),
        (FontId::monospace(12.0), name),
        (FontId::proportional(13.0), " does not exist on entity "),
        (FontId::monospace(12.0), &format!("{entity:?}")),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

/// The type has no associated component id
pub fn missing_componentid(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no associated "),
        (FontId::monospace(12.0), "ComponentId"),
        (FontId::proportional(13.0), "."),
    ]);

    ui.label(job);
}

/// The component is not backed by a rust type
pub fn missing_type_id(ui: &mut egui::Ui, component_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), component_name),
        (
            FontId::proportional(13.0),
            " is not backed by a rust type, so it cannot be displayed.",
        ),
    ]);

    ui.label(job);
}

/// The type has no required type data for displaying the UI
pub fn missing_typedata(ui: &mut egui::Ui, type_name: &str, type_data: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no "),
        (FontId::monospace(12.0), type_data),
        (
            FontId::proportional(13.0),
            " type data, so it cannot be displayed",
        ),
    ]);

    ui.label(job);
}

/// The UI requires access to the bevy world, but it is not present in the inspector context
pub fn no_world_in_context(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " needs the bevy world in the "),
        (FontId::monospace(12.0), "InspectorUi"),
        (
            FontId::proportional(13.0),
            " context to provide meaningful information.",
        ),
    ]);

    ui.label(job);
}

/// The handle does not exist.
pub fn nonexistent_asset_handle(ui: &mut egui::Ui, handle: UntypedAssetId) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "Handle "),
        (FontId::monospace(12.0), &format!("{handle:?}")),
        (FontId::proportional(13.0), " points to no asset."),
    ]);

    ui.label(job);
}

/// The state does not exist
pub fn nonexistent_state(ui: &mut egui::Ui, name: &str) {
    let job = layout_job(&[
        (FontId::proportional(13.0), "State "),
        (FontId::monospace(12.0), name),
        (
            FontId::proportional(13.0),
            " does not exist. Did you forget to call ",
        ),
        (
            FontId::monospace(12.0),
            &format!(".add_state::<{name}>(..)"),
        ),
        (FontId::proportional(13.0), "?"),
    ]);

    ui.label(job);
}

/// Yields a human-readable version of the type id if possible
pub fn typeid_name(type_id: TypeId, type_registry: &TypeRegistry) -> Cow<'_, str> {
    type_registry
        .get(type_id)
        .map(|registration| Cow::Borrowed(registration.type_info().type_path_table().short_path()))
        .unwrap_or_else(|| Cow::Owned(format!("{type_id:?}")))
}

pub enum TypeDataError {
    NoTypeData,
    NotRegistered,
    NotFullyReflected,
}

pub fn reflect_value_no_impl(ui: &mut egui::Ui, reason: TypeDataError, type_name: &str) {
    let text = match reason {
        TypeDataError::NotRegistered | TypeDataError::NoTypeData => &[
            (FontId::monospace(12.0), type_name),
            (FontId::proportional(13.0), " is "),
            (FontId::monospace(12.0), "#[reflect(opaque)],\n"),
            (FontId::proportional(13.0), "you need to register an "),
            (FontId::monospace(12.0), "InspectorEguiImpl"),
            (FontId::proportional(13.0), " in the "),
            (FontId::monospace(12.0), "TypeRegistry"),
            (FontId::proportional(13.0), " .\n\n"),
            (FontId::proportional(13.0), "Try implementing "),
            (
                FontId::monospace(12.0),
                &format!("InspectorPrimitive for {}", pretty_type_name_str(type_name)),
            ),
            (FontId::proportional(13.0), "\nand call "),
            (
                FontId::monospace(12.0),
                &format!(
                    "app.register_type_data::<{}, InspectorEguiImpl>",
                    pretty_type_name_str(type_name)
                ),
            ),
            (FontId::proportional(13.0), "."),
        ] as &[_],
        TypeDataError::NotFullyReflected => &[
            (FontId::monospace(12.0), type_name),
            (FontId::proportional(13.0), " is "),
            (FontId::monospace(12.0), "#[reflect(opaque)],\n"),
            (
                FontId::proportional(13.0),
                "but not backed by a real rust type.",
            ),
        ],
    };
    let job = layout_job(text);

    ui.label(job).on_hover_ui(|ui| {
        ui.set_max_width(ui.spacing().tooltip_width);
        ui.label("If you see this message for a primitive type that should already have an implementation,\nmake sure you have the DefaultInspectorConfigPlugin added or open an issue on github.");
    });
}
pub fn no_default_value(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " has no "),
        (FontId::monospace(12.0), "ReflectDefault"),
        (
            FontId::proportional(13.0),
            " type data, so no value of it can be constructed.",
        ),
    ]);

    ui.label(job);
}

pub fn unconstructable_variant(
    ui: &mut egui::Ui,
    type_name: &str,
    variant: &str,
    unconstructable_field_types: &[&str],
) {
    let mut vec = Vec::with_capacity(2 + unconstructable_field_types.len() * 2 + 4);

    let qualified_variant = format!("{}::{}", pretty_type_name_str(type_name), variant);
    vec.extend([
        (FontId::monospace(12.0), qualified_variant.as_str()),
        (
            FontId::proportional(13.0),
            " has unconstructable fields.\nConsider adding ",
        ),
        (FontId::monospace(12.0), "#[reflect(Default)]"),
        (FontId::proportional(13.0), " to\n\n"),
    ]);
    vec.extend(unconstructable_field_types.iter().flat_map(|variant| {
        [
            (FontId::proportional(13.0), "- "),
            (FontId::monospace(12.0), *variant),
        ]
    }));

    let job = layout_job(&vec);

    ui.label(job);
}

pub fn not_in_type_registry(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (FontId::proportional(13.0), " is not registered in the "),
        (FontId::monospace(12.0), "TypeRegistry"),
    ]);

    ui.label(job);
}

pub fn no_multiedit(ui: &mut egui::Ui, type_name: &str) {
    let job = layout_job(&[
        (FontId::monospace(12.0), type_name),
        (
            FontId::proportional(13.0),
            " doesn't support multi-editing.",
        ),
    ]);

    ui.label(job);
}
