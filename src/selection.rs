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

impl EntitySelection {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

impl From<Entity> for EntitySelection {
    fn from(entity: Entity) -> Self {
        Self::new(entity)
    }
}

impl SelectionItem for EntitySelection {
    const LABEL: &'static str = "Entity";
}

pub trait SelectedItem: Any + Send + Sync + 'static {
    fn clone(&self) -> Box<dyn SelectedItem>;

    fn eq(&self, other: &dyn SelectedItem) -> Option<bool>;
}

pub trait SelectionItem: SelectedItem + Clone + Eq {
    const LABEL: &'static str;
}

impl<T: Any + Clone + PartialEq + Send + Sync + 'static> SelectedItem for T {
    fn clone(&self) -> Box<dyn SelectedItem> {
        Box::new(Clone::clone(self))
    }

    fn eq(&self, other: &dyn SelectedItem) -> Option<bool> {
        let Some(other) = (other as &dyn Any).downcast_ref::<T>() else {
            return None;
        };

        Some(PartialEq::eq(self, other))
    }
}

pub trait SelectionItems: Any + Send + Sync {
    fn label(&self) -> &'static str;

    fn len(&self) -> usize;

    fn clone(&self) -> Box<dyn SelectionItems>;
}

impl<S: SelectionItem> SelectionItems for Vec<S> {
    fn label(&self) -> &'static str {
        S::LABEL
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn clone(&self) -> Box<dyn SelectionItems> {
        Box::new(Clone::clone(self))
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
    by_type: HashMap<TypeId, Box<dyn SelectionItems>>,
}

impl Clone for SelectionMap {
    fn clone(&self) -> Self {
        Self {
            by_type: self
                .by_type
                .iter()
                .map(|(type_id, items)| (*type_id, items.as_ref().clone()))
                .collect(),
        }
    }
}

impl SelectionMap {
    pub fn clear(&mut self) {
        self.by_type.clear();
    }

    pub fn is_selected<S: SelectionItem>(&self, item: &S) -> bool {
        self.get::<S>().is_some_and(|items| items.contains(item))
    }

    pub fn type_ids(&self) -> impl ExactSizeIterator<Item = &TypeId> {
        self.by_type.keys()
    }

    pub fn select<S: SelectionItem>(&mut self, item: S) {
        let items = self.get_mut_or_isnert::<S>();

        if let Some(position) = items.iter().position(|selected| selected == &item) {
            items.swap_remove(position);
        }

        items.push(item);
    }

    pub fn deselect<S: SelectionItem>(&mut self, item: &S) {
        let Some(items) = self.get_mut::<S>() else {
            return;
        };

        let Some(position) = items.iter().position(|selected| selected == item) else {
            return;
        };

        items.remove(position);

        if items.is_empty() {
            self.by_type.remove(&TypeId::of::<S>());
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&TypeId, &dyn SelectionItems)> {
        self.by_type
            .iter()
            .map(|(type_id, items)| (type_id, items.as_ref()))
    }

    pub fn of_type<S: SelectionItem>(&self) -> Option<impl ExactSizeIterator<Item = &S>> {
        self.get::<S>().map(|items| items.iter())
    }

    pub fn of_type_id(&self, type_id: &TypeId) -> Option<&dyn SelectionItems> {
        self.by_type.get(type_id).map(|items| items.as_ref())
    }

    fn get<S: SelectionItem>(&self) -> Option<&Vec<S>> {
        self.by_type
            .get(&TypeId::of::<S>())
            .map(|items| (items.as_ref() as &dyn Any).downcast_ref().unwrap())
    }

    fn get_mut<S: SelectionItem>(&mut self) -> Option<&mut Vec<S>> {
        self.by_type
            .get_mut(&TypeId::of::<S>())
            .map(|items| (items.as_mut() as &mut dyn Any).downcast_mut().unwrap())
    }

    fn get_mut_or_isnert<S: SelectionItem>(&mut self) -> &mut Vec<S> {
        (self
            .by_type
            .entry(TypeId::of::<S>())
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

fn on_deselected(
    trigger: On<Remove, Selected>,
    mut map: ResMut<SelectionMap>,
    mut commands: Commands,
) {
    commands.trigger(Deselect {
        entity: trigger.event_target(),
    });

    // In case the entity was despawned
    map.deselect(&EntitySelection::new(trigger.event_target()));
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
