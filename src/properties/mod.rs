use std::{any::TypeId, f32, ops::DerefMut, path::Path};

use bevy::{
    app::{App, Plugin},
    ecs::{
        change_detection::{DetectChanges, DetectChangesMut},
        component::ComponentId,
        entity::Entity,
        error::Result,
        hierarchy::Children,
        name::Name,
        reflect::AppTypeRegistry,
        world::{CommandQueue, World},
    },
    reflect::TypeRegistry,
};
use egui::{
    Color32, Frame, Id, InnerResponse, Response, TextEdit, TextureId, Ui, Vec2, Window,
    collapsing_header::{CollapsingState, paint_default_icon},
};

use crate::{
    inspection::{
        self, error,
        reflect_inspector::{Context, InspectorUi},
        restricted_world_view::{ReflectBorrow, RestrictedWorldView},
        utils::{self, pretty_type_name, pretty_type_name_str},
    },
    pane::{Pane, RegisterPane},
    selection::{EntitySelection, SelectionMap},
};

pub struct PropertiesPane;

impl Pane for PropertiesPane {
    fn name(&self) -> &str {
        "Properties"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let selection_map = world.resource::<SelectionMap>();

        if selection_map.type_ids().len() > 1 {
            ui.heading("Selected");
            ui.separator();

            for (_, items) in selection_map.iter() {
                ui.label(format!("{} x{}", items.label(), items.len()));
            }
        } else if selection_map.type_ids().len() == 1 {
            if let Some(selected) = selection_map
                .of_type::<EntitySelection>()
                .map(|selected| selected.map(|item| item.entity).collect::<Vec<_>>())
            {
                match selected.as_slice() {
                    &[entity] => ui_for_entity(ui, world, entity)?,
                    entities => ui_for_entities(ui, world, entities)?,
                }
            }
        }

        Ok(())
    }
}

fn ui_for_entity_name(ui: &mut Ui, world: &mut World, entity: Entity) -> Result {
    ui.horizontal(|ui| {
        let mut entity_mut = world.entity_mut(entity);

        if entity_mut.contains::<Name>() {
            if ui.small_button("-").clicked() {
                entity_mut.remove::<Name>();
            }
        } else {
            if ui.small_button("+").clicked() {
                entity_mut.insert(Name::new("Entity"));
            }
        }

        ui.label("Name: ");

        if let Some(mut name) = entity_mut.get_mut::<Name>() {
            name.mutate(|name| {
                TextEdit::singleline(name)
                    .desired_width(f32::INFINITY)
                    .show(ui);
            });
        } else {
            ui.label("Entity");
        }
    });

    Ok(())
}

fn ui_for_entity(ui: &mut Ui, world: &mut World, entity: Entity) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    ui_for_entity_name(ui, world, entity)?;
    ui.separator();

    let mut queue = CommandQueue::default();
    ui_for_entity_components(&mut world.into(), &mut queue, entity, ui, &type_registry)?;

    queue.apply(world);

    Ok(())
}

fn ui_for_entities(ui: &mut Ui, world: &mut World, entities: &[Entity]) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let Some(&first) = entities.first() else {
        return Ok(());
    };

    let Ok(mut components) = components_of_entity(&mut world.into(), first) else {
        error::nonexistent_entity(ui, first);
        return Ok(());
    };

    for &entity in entities.iter().skip(1) {
        components.retain(|(_, id, _, _)| {
            world
                .get_entity(entity)
                .map_or(true, |entity| entity.contains_id(*id))
        })
    }

    let (resources_view, components_view) = RestrictedWorldView::resources_components(world);
    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: resources_view,
        queue: &mut queue,
    };
    let mut env = InspectorUi::new(&type_registry, &mut cx);

    let id = egui::Id::NULL;
    for (name, component_id, component_type_id, size) in components {
        let id = id.with(component_id);
        let inner = egui::CollapsingHeader::new(&name)
            .id_salt(id)
            .show(ui, |ui| -> Result {
                if size == 0 {
                    return Ok(());
                }

                let mut values = Vec::with_capacity(entities.len());

                for (i, &entity) in entities.iter().enumerate() {
                    // skip duplicate entities
                    if entities[0..i].contains(&entity) {
                        continue;
                    };

                    // SAFETY: entities are distinct, env has a context with just resources
                    match unsafe {
                        components_view.get_entity_component_reflect_unchecked(
                            entity,
                            component_type_id,
                            &type_registry,
                        )
                    } {
                        Ok(value) => {
                            values.push(value);
                        }
                        Err(error) => {
                            error::no_access(error, ui, &name);
                            return Ok(());
                        }
                    }
                }

                let mut values_reflect: Vec<_> = values
                    .iter_mut()
                    .map(|value| value.bypass_change_detection().as_partial_reflect_mut())
                    .collect();
                let changed = env.ui_for_reflect_many_with_options(
                    component_type_id,
                    &name,
                    ui,
                    id.with(component_id),
                    &(),
                    values_reflect.as_mut_slice(),
                )?;
                if changed {
                    for value in values.iter_mut() {
                        value.set_changed();
                    }
                }

                Ok(())
            })
            .body_returned;

        if let Some(inner) = inner {
            inner?;
        }
    }

    queue.apply(world);

    Ok(())
}

fn ui_for_entity_components(
    world: &mut RestrictedWorldView<'_>,
    mut queue: &mut CommandQueue,
    entity: Entity,
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
) -> Result {
    let Ok(components) = components_of_entity(world, entity) else {
        error::nonexistent_entity(ui, entity);
        return Ok(());
    };

    ui.push_id(Id::new(entity), |ui| -> Result {
        for (name, component_id, component_type_id, size) in components {
            let id = Id::new(component_id);

            #[cfg(feature = "documentation")]
            let type_docs = type_registry
                .get_type_info(component_type_id)
                .and_then(|info| info.docs());

            if size == 0 {
                ui.indent(id, |ui| {
                    let _response = ui.label(&name);
                    #[cfg(feature = "documentation")]
                    crate::inspection::egui_utils::show_docs(_response, type_docs);
                });
                continue;
            }

            let mut collapsing_state = CollapsingState::load_with_default_open(ui.ctx(), id, true);

            // create a context with access to the world except for the currently viewed component
            let (mut component_view, world) =
                world.split_off_component((entity, component_type_id));
            let mut cx = Context {
                world: world,
                queue: queue,
            };

            let value = match component_view.get_entity_component_reflect(
                entity,
                component_type_id,
                type_registry,
            ) {
                Ok(value) => value,
                Err(e) => {
                    ui.indent(id, |ui| {
                        let response = ui.label(egui::RichText::new(&name).underline());
                        response.on_hover_ui(|ui| error::no_access(e, ui, &name));
                    });
                    continue;
                }
            };

            let changed_by = match &value {
                ReflectBorrow::Mutable(val) => val.changed_by().into_option(),
                ReflectBorrow::Immutable(_) => None,
            };

            let InnerResponse {
                inner: header_response,
                ..
            } = Frame::new()
                .stroke(ui.style().visuals.widgets.open.bg_stroke)
                .corner_radius(ui.style().visuals.widgets.active.corner_radius)
                .show(ui, |ui| -> Result<Response> {
                    let InnerResponse {
                        response: header_response,
                        ..
                    } = Frame::new()
                        .fill(ui.style().visuals.widgets.active.bg_fill)
                        .inner_margin(ui.style().spacing.button_padding)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                collapsing_state.show_toggle_button(ui, paint_default_icon);
                                ui.vertical_centered(|ui| {
                                    ui.label(name);
                                });
                            });
                        });

                    let inner = collapsing_state.show_body_unindented(ui, |ui| -> Result {
                        Frame::new()
                            .inner_margin(ui.style().spacing.button_padding)
                            .show(ui, |ui| -> Result {
                                let mut env = InspectorUi::new(type_registry, &mut cx);
                                let id = id.with(component_id);
                                let options = &();

                                match value {
                                    ReflectBorrow::Mutable(mut value) => {
                                        let changed = env.ui_for_reflect_with_options(
                                            value
                                                .bypass_change_detection()
                                                .as_partial_reflect_mut(),
                                            ui,
                                            id,
                                            options,
                                        )?;

                                        if changed {
                                            value.set_changed();
                                        }
                                    }
                                    ReflectBorrow::Immutable(value) => env
                                        .ui_for_reflect_readonly_with_options(
                                            value.as_partial_reflect(),
                                            ui,
                                            id,
                                            options,
                                        )?,
                                };

                                Ok(())
                            })
                            .inner
                    });

                    if let Some(inner) = inner {
                        inner.inner?;
                    }

                    Ok(header_response)
                });

            let response = header_response?;

            if let Some(location) = changed_by {
                response.context_menu(|ui| {
                    ui.label("Last change:");
                    let path = Path::new(location.file());
                    let pretty = utils::trim_cargo_registry_path(path);

                    if ui
                        .button(format!(
                            "{}:{}:{}",
                            pretty.as_deref().unwrap_or(path).display(),
                            location.line(),
                            location.column()
                        ))
                        .clicked()
                    {
                        if let Err(e) = utils::open_file_at(location) {
                            bevy::log::error!("Failed to open last change location: {}", e);
                        } else {
                            bevy::log::info!("Successfully opened {location}");
                        }
                    }
                });
            }

            #[cfg(feature = "documentation")]
            crate::inspection::egui_utils::show_docs(response, type_docs);
        }

        Ok(())
    })
    .inner
}

fn components_of_entity(
    world: &mut RestrictedWorldView<'_>,
    entity: Entity,
) -> Result<Vec<(String, ComponentId, TypeId, usize)>> {
    let entity_ref = world.world().get_entity(entity)?;

    let archetype = entity_ref.archetype();
    let mut components: Vec<_> = archetype
        .components()
        .iter()
        .map(|component_id| {
            let info = world.world().components().get_info(*component_id).unwrap();
            let name = pretty_type_name_str(&info.name().to_string());

            (
                name,
                *component_id,
                info.type_id().unwrap(),
                info.layout().size(),
            )
        })
        .collect();
    components.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    Ok(components)
}

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(PropertiesPane);
    }
}
