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

/// The entity selected in the editor, if any.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection(pub Option<Entity>);

fn clear_despawned_selection(mut selection: ResMut<Selection>, entities: Query<()>) {
    if let Some(entity) = selection.0
        && !entities.contains(entity)
    {
        selection.0 = None;
    }
}
