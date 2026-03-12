use std::any::TypeId;

use bevy::{
    ecs::{
        entity::Entity,
        name::Name,
        reflect::{AppTypeRegistry, ReflectComponent},
        world::{EntityWorldMut, World},
    },
    reflect::prelude::ReflectDefault,
};
use egui::{Id, InnerResponse, Ui};

use crate::{Pane, Selection, panes::EntitySelection};

pub struct PropertiesPane;

impl Pane for PropertiesPane {
    fn name(&self) -> &str {
        "Properties"
    }

    fn ui(&mut self, world: &mut World, ui: &mut Ui) {
        world.resource_scope::<Selection, _>(|world, mut selection| {
            if selection.is_type::<EntitySelection>() {
                self.entity_ui(
                    world,
                    ui,
                    selection
                        .of_type::<EntitySelection>()
                        .unwrap()
                        .map(|value| value.0)
                        .collect::<Vec<_>>()
                        .as_slice(),
                );
            } else {
                let type_ids = selection.type_ids();

                if type_ids.len() != 0 {
                    ui.heading(format!("Selected x{}", selection.count()));
                    ui.separator();

                    for type_id in type_ids.cloned() {
                        let count = selection.of_type_id(type_id).unwrap().len();
                        let naming = selection.naming(type_id).unwrap();
                        ui.label(format!("{naming} x{count}"));
                    }
                }
            }
        });
    }
}

impl PropertiesPane {
    fn entity_ui(&self, world: &mut World, ui: &mut Ui, entities: &[Entity]) -> bool {
        match entities {
            &[entity] => self.single_entity_ui(world, ui, entity),
            entities => self.shared_entity_ui(world, ui, entities),
        }
    }

    fn entity_header_ui(&self, world: &mut World, ui: &mut Ui, entity: Entity) -> bool {
        let InnerResponse { inner: changed, .. } = ui.horizontal(|ui| {
            let mut entity_mut = world.entity_mut(entity);
            let mut changed = false;

            if entity_mut.contains::<Name>() {
                if ui.button("-").clicked() {
                    entity_mut.remove::<Name>();
                    changed |= true;
                }
            } else {
                if ui.button("+").clicked() {
                    entity_mut.insert(Name::new("Entity"));
                    changed |= true;
                }
            }

            ui.label("Name: ");

            if let Some(mut name) = entity_mut.get_mut::<Name>() {
                name.mutate(|name| {
                    changed |= ui.text_edit_singleline(name).changed();
                });
            } else {
                ui.label("Entity");
            }

            changed
        });

        changed
    }

    fn single_entity_ui(&self, world: &mut World, ui: &mut Ui, entity: Entity) -> bool {
        let mut changed = false;

        changed |= self.entity_header_ui(world, ui, entity);
        ui.separator();
        changed |= self.entity_components_ui(world, ui, entity);
        ui.separator();
        changed |= self.entity_add_component_ui(world, ui, entity);

        changed
    }

    fn entity_components_ui(&self, world: &mut World, ui: &mut Ui, entity: Entity) -> bool {
        let mut changed = false;

        let mut type_ids = world
            .inspect_entity(entity)
            .unwrap()
            .filter_map(|info| info.type_id())
            .collect::<Vec<_>>();

        type_ids.sort_by(|a, b| {
            let type_registry = world.resource::<AppTypeRegistry>().read();

            let a_name = type_registry
                .get_type_info(*a)
                .map(|info| info.type_path())
                .unwrap_or_else(|| "z");

            let b_name = type_registry
                .get_type_info(*b)
                .map(|info| info.type_path())
                .unwrap_or_else(|| "z");

            a_name.cmp(b_name)
        });

        for component_id in type_ids {
            changed |= self.entity_component_ui(world, ui, Id::new(entity), entity, component_id);
        }

        changed
    }

    fn entity_component_ui(
        &self,
        world: &mut World,
        ui: &mut Ui,
        id: Id,
        entity: Entity,
        type_id: TypeId,
    ) -> bool {
        let type_registry = world.resource::<AppTypeRegistry>().read();

        let Some(reflect_component) = type_registry.get_type_data::<ReflectComponent>(type_id)
        else {
            return false;
        };

        let mut changed = false;

        let id = id.with(type_id);

        let ident = type_registry
            .get_type_info(type_id)
            .unwrap()
            .type_path_table()
            .short_path();

        let header = egui::CollapsingHeader::new(ident).id_salt(id);

        let _response = header.show(ui, |ui| {
            // TODO: inspection
        });

        changed
    }

    fn entity_add_component_ui(&self, world: &mut World, ui: &mut Ui, entity: Entity) -> bool {
        let type_registry = world.resource::<AppTypeRegistry>().clone();
        let type_registry = type_registry.read();

        let mut changed = false;

        let type_ids = type_registry
            .iter()
            .filter_map(|registration| {
                if registration.data::<ReflectComponent>().is_some()
                    && registration.data::<ReflectDefault>().is_some()
                {
                    Some(registration.type_id())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        ui.vertical_centered_justified(|ui| {
            ui.button("Add Component").context_menu(|ui| {
                for type_id in type_ids {
                    let registration = type_registry.get(type_id).unwrap();
                    let label = registration
                        .type_info()
                        .type_path_table()
                        .short_path()
                        .to_string();

                    if ui.button(label).clicked() {
                        let component = registration.data::<ReflectDefault>().unwrap().default();

                        registration.data::<ReflectComponent>().unwrap().insert(
                            &mut world.entity_mut(entity),
                            component.as_partial_reflect(),
                            &type_registry,
                        );

                        changed = true;

                        ui.close();
                    }
                }
            });
        });

        changed
    }

    fn shared_entity_ui(&self, world: &mut World, ui: &mut Ui, entities: &[Entity]) -> bool {
        false
    }
}
