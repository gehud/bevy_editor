use bevy::{
    app::{App, Plugin},
    ecs::{error::Result, world::World},
};
use egui::Ui;

use crate::{
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

        if selection_map.selections().len() >= 1 {
            ui.heading("Selected");
            ui.separator();

            for selection in selection_map.selections() {
                let items = selection_map.of_selection(selection).unwrap();
                ui.label(format!("{} x{}", selection, items.count()));
            }
        } else if selection_map.selections().len() == 1 {
            if let Some(entities) = selection_map.of_type::<EntitySelection>() {

            }
        }

        Ok(())
    }
}

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(PropertiesPane);
    }
}
