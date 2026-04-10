use std::{
    any::{Any, TypeId},
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use bevy::{
    app::{App, Last, Plugin, PostUpdate},
    asset::uuid::Uuid,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EntityEvent, Event},
        lifecycle::{Add, Despawn, Remove},
        message::{Message, MessageReader, MessageWriter},
        observer::On,
        query::With,
        resource::Resource,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut, Single},
    },
    pbr::material_uses_bindless_resources,
    picking::{
        events::{Click, Pointer},
        pointer::PointerButton,
    },
    platform::collections::HashMap,
    window::PrimaryWindow,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EntitySelection {
    pub entity: Entity,
}

impl SelectionItem for EntitySelection {
    const SELECTION: Selection = Selection::Entity;
}

impl IntoSelectionItem for Entity {
    type Item = EntitySelection;

    fn into_selection_item(self) -> Self::Item {
        EntitySelection { entity: self }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FileSelection {
    pub path: PathBuf,
}

impl SelectionItem for FileSelection {
    const SELECTION: Selection = Selection::File;
}

impl IntoSelectionItem for PathBuf {
    type Item = FileSelection;

    fn into_selection_item(self) -> Self::Item {
        FileSelection { path: self }
    }
}

pub trait SelectionItem: Eq + Send + Sync + 'static {
    const SELECTION: Selection;
}

pub trait IntoSelectionItem {
    type Item: SelectionItem;

    fn into_selection_item(self) -> Self::Item;
}

impl<S: SelectionItem> IntoSelectionItem for S {
    type Item = S;

    fn into_selection_item(self) -> Self::Item {
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Selection {
    Entity,
    File,
}

impl Display for Selection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Selection::Entity => write!(f, "Entity"),
            Selection::File => write!(f, "File"),
        }
    }
}

pub trait SelectionItems: Any + Send + Sync {
    fn count(&self) -> usize;
}

impl<S: SelectionItem> SelectionItems for Vec<S> {
    fn count(&self) -> usize {
        self.len()
    }
}

#[derive(Component)]
struct Selected;

#[derive(EntityEvent)]
pub struct Select {
    entity: Entity,
}

#[derive(EntityEvent)]
pub struct Deselect {
    entity: Entity,
}

#[derive(Default, Resource)]
pub struct SelectionMap {
    by_type: HashMap<Selection, Box<dyn SelectionItems>>,
}

impl SelectionMap {
    pub fn clear(&mut self) {
        self.by_type.clear();
    }

    pub fn is_selected<S: IntoSelectionItem>(&self, item: S) -> bool {
        self.get::<S::Item>()
            .is_some_and(|items| items.contains(&item.into_selection_item()))
    }

    pub fn selections(&self) -> impl ExactSizeIterator<Item = &Selection> {
        self.by_type.keys()
    }

    pub fn select<S: IntoSelectionItem>(&mut self, item: S) {
        let items = self.get_mut_or_isnert::<S::Item>();

        let item = item.into_selection_item();

        if let Some(position) = items.iter().position(|selected| selected == &item) {
            items.swap_remove(position);
        }

        items.push(item);
    }

    pub fn deselect<S: IntoSelectionItem>(&mut self, item: S) {
        let Some(items) = self.get_mut::<S::Item>() else {
            return;
        };

        let item = item.into_selection_item();

        let Some(position) = items.iter().position(|selected| selected == &item) else {
            return;
        };

        items.remove(position);

        if items.is_empty() {
            self.by_type.remove(&S::Item::SELECTION);
        }
    }

    pub fn of_selection(&self, selection: &Selection) -> Option<&dyn SelectionItems> {
        self.by_type.get(selection).map(|items| items.as_ref())
    }

    pub fn of_type<S: SelectionItem>(&self) -> Option<impl ExactSizeIterator<Item = &S>> {
        self.get::<S>().map(|items| items.iter())
    }

    fn get<S: SelectionItem>(&self) -> Option<&Vec<S>> {
        self.by_type
            .get(&S::SELECTION)
            .map(|items| (items.as_ref() as &dyn Any).downcast_ref().unwrap())
    }

    fn get_mut<S: SelectionItem>(&mut self) -> Option<&mut Vec<S>> {
        self.by_type
            .get_mut(&S::SELECTION)
            .map(|items| (items.as_mut() as &mut dyn Any).downcast_mut().unwrap())
    }

    fn get_mut_or_isnert<S: SelectionItem>(&mut self) -> &mut Vec<S> {
        (self
            .by_type
            .entry(S::SELECTION)
            .or_insert_with(|| Box::new(Vec::<S>::new()))
            .as_mut() as &mut dyn Any)
            .downcast_mut()
            .unwrap()
    }
}

fn mark_entities(
    map: Res<SelectionMap>,
    marked: Query<Entity, With<Selected>>,
    mut commands: Commands,
) {
    let selected = map
        .of_type::<EntitySelection>()
        .map(|items| items.map(|item| item.entity).collect::<Vec<_>>())
        .unwrap_or_default();

    for entity in marked.iter() {
        if !selected.contains(&entity) {
            commands.entity(entity).remove::<Selected>();
        }
    }

    for entity in selected {
        if !marked.contains(entity) {
            commands.entity(entity).insert(Selected);
        }
    }
}

fn on_selected(trigger: On<Add, Selected>, mut commands: Commands) {
    commands.trigger(Select {
        entity: trigger.event_target(),
    });
}

fn on_deselected(trigger: On<Remove, Selected>, mut commands: Commands) {
    commands.trigger(Deselect {
        entity: trigger.event_target(),
    });
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum SelectionSystems {
    Mark,
}

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionMap>()
            .configure_sets(Last, SelectionSystems::Mark)
            .add_systems(Last, mark_entities.in_set(SelectionSystems::Mark))
            .add_observer(on_selected)
            .add_observer(on_deselected);
    }
}
