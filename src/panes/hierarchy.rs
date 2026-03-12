use bevy::{
    asset::Assets,
    ecs::{
        component::Component, entity::Entity, hierarchy::Children, name::Name, query::With,
        resource::Resource, world::World,
    },
    scene::{Scene, SceneInstance, SceneRoot},
};
use egui::{CollapsingHeader, Id, Key, Sense, Ui, collapsing_header::CollapsingState};

use crate::{Pane, Selectable, Selection};

#[derive(PartialEq, Eq)]
pub struct EntitySelection(pub Entity);

impl Selectable for EntitySelection {
    const NAMING: &'static str = "Entity";
}

#[derive(Resource)]
pub(crate) struct InspectedScene {
    pub root: Entity,
}

pub struct HierarchyPane;

impl Pane for HierarchyPane {
    fn name(&self) -> &str {
        "Hierarchy"
    }

    fn ui(&mut self, world: &mut World, ui: &mut Ui) {
        world.resource_scope::<Selection, _>(|world, mut selection| {
            let mut any_selected = false;

            let root = world.resource::<InspectedScene>().root;
            let name = world
                .entity(root)
                .get::<Name>()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "Scene".into());

            ui.collapsing(name.clone(), |ui| {
                self.ui_recurse(
                    world,
                    ui,
                    &mut selection,
                    root,
                    ui.make_persistent_id(name),
                    &mut any_selected,
                );
            });

            let background = ui.interact(
                ui.available_rect_before_wrap(),
                Id::new("entity_menu"),
                Sense::click(),
            );

            background.context_menu(|ui| {
                self.context_menu(world, ui, world.resource::<InspectedScene>().root);
            });

            if !any_selected && background.clicked() {
                selection.clear();
            }
        });
    }
}

impl HierarchyPane {
    fn context_menu(&mut self, world: &mut World, ui: &mut Ui, entity: Entity) {
        if ui.button("Create Empty").clicked() {
            let new_entity = world.spawn_empty().id();
            world.entity_mut(entity).add_child(new_entity);
        }
    }

    fn ui_recurse(
        &mut self,
        world: &mut World,
        ui: &mut Ui,
        selection: &mut Selection,
        entity: Entity,
        id: Id,
        any_selected: &mut bool,
    ) {
        let Some(children) = world.entity(entity).get::<Children>() else {
            return;
        };

        let children = children.iter().cloned().collect::<Vec<_>>();

        for (i, entity) in children.iter().enumerate() {
            let name = world
                .entity(*entity)
                .get::<Name>()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "Entity".into());

            let id = id.with(&name).with(i);
            if world.entity(*entity).contains::<Children>() {
                let state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
                state
                    .show_header(ui, |ui| {
                        self.ui_label(world, ui, selection, *entity, &name, any_selected);
                    })
                    .body(|ui| {
                        self.ui_recurse(world, ui, selection, *entity, id, any_selected);
                    });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    self.ui_label(world, ui, selection, *entity, &name, any_selected);
                });
            }
        }
    }

    fn ui_label(
        &mut self,
        world: &mut World,
        ui: &mut Ui,
        selection: &mut Selection,
        entity: Entity,
        name: &str,
        any_selected: &mut bool,
    ) {
        let selected = selection.is_selected(&EntitySelection(entity));

        let response = ui.selectable_label(selected, name);

        response.context_menu(|ui| {
            self.context_menu(world, ui, entity);
        });

        if response.clicked() {
            *any_selected = true;

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
}
