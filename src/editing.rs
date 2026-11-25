//! Editor-wide editing state: what belongs to the edited scene, and what is selected.

use bevy::prelude::*;

pub struct EditingPlugin;

impl Plugin for EditingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .add_systems(Last, clear_despawned_selection);
    }
}

/// Marks entities that belong to the editor itself (editor cameras, the grid, …) rather than to
/// the scene being edited. They are hidden from the Scene Tree and other scene views.
///
/// UI entities (with a [`Node`]) are always treated as editor-only and need no marker.
#[derive(Component, Clone, Copy, Default)]
pub struct EditorOnly;

/// The entities selected in the editor, in the order they were selected.
///
/// The last one is the *primary* selection: the one a single-entity view (such as the inspector)
/// shows, and the one renamed in place.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct Selection(Vec<Entity>);

impl Selection {
    /// All selected entities, oldest first.
    pub fn entities(&self) -> &[Entity] {
        &self.0
    }

    /// The most recently selected entity.
    pub fn primary(&self) -> Option<Entity> {
        self.0.last().copied()
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.0.contains(&entity)
    }

    /// Selects only `entity`.
    pub fn select(&mut self, entity: Entity) {
        self.0.clear();
        self.0.push(entity);
    }

    /// Adds `entity` to the selection (as the primary one), or removes it if it was selected.
    pub fn toggle(&mut self, entity: Entity) {
        if let Some(index) = self.0.iter().position(|e| *e == entity) {
            self.0.remove(index);
        } else {
            self.0.push(entity);
        }
    }

    /// Adds `entity` to the selection and makes it the primary one.
    pub fn add(&mut self, entity: Entity) {
        self.0.retain(|e| *e != entity);
        self.0.push(entity);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

fn clear_despawned_selection(mut selection: ResMut<Selection>, entities: Query<()>) {
    // Check first, so the resource is only marked changed when something was despawned.
    if selection.0.iter().any(|entity| !entities.contains(*entity)) {
        selection.0.retain(|entity| entities.contains(*entity));
    }
}
