use bevy::ecs::{component::Component, entity::Entity, name::Name, query::With, world::World};
use egui::{Id, Key, Sense, Ui};

use crate::{Pane, Selectable, Selection};

#[derive(PartialEq, Eq)]
pub struct EntitySelection(pub Entity);

impl Selectable for EntitySelection {
    const NAMING: &'static str = "Entity";
}

#[derive(Component)]
pub struct InspectedEntity;

pub struct HierarchyPane;

impl Pane for HierarchyPane {
    fn name(&self) -> &str {
        "Hierarchy"
    }

    fn ui(&mut self, world: &mut World, ui: &mut Ui) {
        let mut selection_performed = false;

        world.resource_scope::<Selection, _>(|world, mut selection| {
            for entity in world
                .query_filtered::<Entity, With<InspectedEntity>>()
                .iter(world)
            {
                let selected = selection.is_selected(&EntitySelection(entity));

                let entity_ref = world.entity(entity);
                let name = entity_ref
                    .get::<Name>()
                    .map(|name| name.as_str())
                    .unwrap_or_else(|| "Entity");

                if ui.selectable_label(selected, name).clicked() {
                    selection_performed = true;

                    if !ui.input(|i| i.modifiers.ctrl) {
                        selection.clear();
                        selection.select(EntitySelection(entity));
                    } else {
                        if selected {
                            selection.deselect(&EntitySelection(entity));
                        } else {
                            selection.select(EntitySelection(entity));
                        }
                    }
                }
            }

            let background = ui.interact(
                ui.available_rect_before_wrap(),
                Id::new("entity_menu"),
                Sense::click(),
            );

            background.context_menu(|ui| {
                if ui.button("Create Empty").clicked() {
                    world.spawn(InspectedEntity);
                }
            });

            if !selection_performed && background.clicked() {
                selection.clear();
            }
        });
    }
}
