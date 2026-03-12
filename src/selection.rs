use std::any::{Any, TypeId};

use bevy::{ecs::resource::Resource, platform::collections::HashMap};

pub trait Selectable: Send + Sync + Eq + 'static {
    const NAMING: &'static str;
}

#[derive(Default, Resource)]
pub struct Selection {
    by_type: HashMap<TypeId, (&'static str, Vec<Box<dyn Any + Send + Sync>>)>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.by_type.clear();
    }

    pub fn count(&self) -> usize {
        self.by_type.values().map(|values| values.1.len()).sum()
    }

    pub fn naming(&self, type_id: TypeId) -> Option<&'static str> {
        self.by_type.get(&type_id).map(|entry| entry.0)
    }

    pub fn select<S: Selectable>(&mut self, value: S) {
        if self.is_selected(&value) {
            return;
        }

        self.values::<S>().push(Box::new(value));
    }

    pub fn select_typed<S: Selectable>(&mut self, value: S) {
        self.by_type.retain(|key, _| key == &TypeId::of::<S>());
        self.select(value);
    }

    pub fn deselect<S: Selectable>(&mut self, value: &S) {
        let mut should_free = false;

        let values = self.values::<S>();

        if let Some(position) = values
            .iter()
            .position(|selected| selected.downcast_ref::<S>().unwrap() == value)
        {
            values.remove(position);

            if values.len() == 0 {
                should_free = true;
            }
        }

        if should_free {
            self.by_type.remove(&TypeId::of::<S>());
        }
    }

    pub fn deselect_all<S: Selectable>(&mut self) {
        self.by_type.remove(&TypeId::of::<S>());
    }

    pub fn is_selected<S: Selectable>(&self, value: &S) -> bool {
        self.by_type.get(&TypeId::of::<S>()).is_some_and(|values| {
            values
                .1
                .iter()
                .any(|selected| selected.downcast_ref::<S>().unwrap() == value)
        })
    }

    pub fn of_type_id(
        &self,
        type_id: TypeId,
    ) -> Option<impl ExactSizeIterator<Item = &(dyn Any + Send + Sync)>> {
        self.by_type
            .get(&type_id)
            .map(|values| values.1.iter().map(|value| value.as_ref()))
    }

    pub fn of_type<S: Selectable>(&self) -> Option<impl ExactSizeIterator<Item = &S>> {
        self.of_type_id(TypeId::of::<S>())
            .map(|values| values.map(|value| value.downcast_ref::<S>().unwrap()))
    }

    pub fn type_ids(&self) -> impl ExactSizeIterator<Item = &TypeId> {
        self.by_type.keys()
    }

    pub fn single_type(&self) -> Option<TypeId> {
        (self.by_type.len() == 1).then(|| self.by_type.iter().next().unwrap().0.clone())
    }

    pub fn is_type<S: Selectable>(&self) -> bool {
        self.single_type()
            .is_some_and(|type_id| type_id == TypeId::of::<S>())
    }

    fn values<S: Selectable>(&mut self) -> &mut Vec<Box<dyn Any + Send + Sync>> {
        &mut self
            .by_type
            .entry(TypeId::of::<S>())
            .or_insert_with(|| (S::NAMING, Vec::new()))
            .1
    }
}
