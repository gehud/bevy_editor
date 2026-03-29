use std::{fmt, path::PathBuf};

use bevy::{
    app::{App, Last, Plugin},
    asset::Deferred,
    ecs::{
        component::Component,
        entity::Entity,
        event::EntityEvent,
        lifecycle::{Add, HookContext, Remove},
        message::{Message, MessageReader},
        observer::On,
        resource::Resource,
        system::{Commands, Query, ResMut},
        world::DeferredWorld,
    },
    platform::collections::{HashMap, HashSet},
};

#[derive(Clone, Component, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) enum Selection {
    #[default]
    Entity,
    File(PathBuf),
}

impl fmt::Display for Selection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selection::Entity => write!(f, "Entity"),
            Selection::File(path_buf) => {
                if path_buf.is_dir() {
                    write!(f, "Folder")
                } else {
                    write!(f, "File")
                }
            }
        }
    }
}

#[derive(Clone, Component, Debug)]
#[require(Selection)]
#[component(on_remove = on_selected_remove)]
pub struct Selected;

fn on_selected_remove(mut world: DeferredWorld, ctx: HookContext) {
    world.commands().entity(ctx.entity).remove::<Selection>();
}

#[derive(Default, Resource)]
pub(crate) struct SelectionMap(pub HashMap<Selection, HashSet<Entity>>);

#[derive(Message)]
pub struct SelectionChanged;

fn on_add_selection(_: On<Add, Selection>, mut commands: Commands) {
    commands.write_message(SelectionChanged);
}

fn on_remove_selection(_: On<Remove, Selection>, mut commands: Commands) {
    commands.write_message(SelectionChanged);
}

fn update_selection_map(
    mut selection_changed_messages: MessageReader<SelectionChanged>,
    mut selection_map: ResMut<SelectionMap>,
    selections: Query<(&Selection, Entity)>,
) {
    if selection_changed_messages.is_empty() {
        return;
    }

    selection_changed_messages.clear();

    selection_map.0.clear();

    for (selection, entity) in selections {
        selection_map
            .0
            .entry(selection.clone())
            .or_default()
            .insert(entity);
    }
}

pub struct EditorSelectionPlugin;

impl Plugin for EditorSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SelectionChanged>()
            .init_resource::<SelectionMap>()
            .add_observer(on_add_selection)
            .add_observer(on_remove_selection)
            .add_systems(Last, update_selection_map);
    }
}
