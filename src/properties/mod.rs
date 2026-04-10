use bevy::{
    app::{App, Plugin},
    ecs::{entity::Entity, error::Result, world::World},
};
use egui::Ui;

use crate::{
    inspection,
    pane::{Pane, RegisterPane},
    selection::{EntitySelection, Selection, SelectionMap},
};

pub struct PropertiesPane;

impl Pane for PropertiesPane {
    fn name(&self) -> &str {
        "Properties"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let selection_map = world.resource::<SelectionMap>();

        if selection_map.selections().len() > 1 {
            ui.heading("Selected");
            ui.separator();

            for selection in selection_map.selections() {
                let items = selection_map.of_selection(selection).unwrap();
                ui.label(format!("{} x{}", selection, items.count()));
            }
        } else if selection_map.selections().len() == 1 {
            if let Some(selected) = selection_map
                .of_type::<EntitySelection>()
                .map(|selected| selected.map(|item| item.entity).collect::<Vec<_>>())
            {
                match selected.as_slice() {
                    &[entity] => self.ui_for_entity_with_children(ui, world, entity),
                    entities => self.ui_for_entities_shared_components(ui, world, entities),
                }
            }
        }

        Ok(())
    }
}

impl PropertiesPane {
    fn ui_for_entity_with_children(&mut self, ui: &mut Ui, world: &mut World, entity: Entity) {
        inspection::bevy_inspector::ui_for_entity_with_children(world, entity, ui);
    }

    fn ui_for_entities_shared_components(
        &mut self,
        ui: &mut Ui,
        world: &mut World,
        entities: &[Entity],
    ) {
        inspection::bevy_inspector::ui_for_entities_shared_components(world, entities, ui);
    }
}

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(PropertiesPane);
    }
}
