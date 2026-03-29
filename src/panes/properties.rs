use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        hierarchy::ChildOf,
        message::MessageReader,
        query::With,
        system::{Commands, In, Query, Res},
        world::Ref,
    },
    ui::{FlexDirection, Node, Overflow, PositionType, UiRect, percent, px},
    utils::default,
};

use crate::{
    pane::{PaneApp, PaneStructure},
    selection::{Selection, SelectionChanged, SelectionMap},
    widget::{EditorText, ScrollArea},
};

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Properties", setup)
            .add_systems(Update, update);
    }
}

#[derive(Component)]
struct PropertiesRoot;

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {
    commands.entity(pane.content()).with_children(|commands| {
        let scroll = commands
            .spawn(Node {
                width: percent(100),
                height: percent(100),
                margin: UiRect::all(px(6)),
                ..default()
            })
            .id();

        let content = commands
            .commands_mut()
            .spawn((
                PropertiesRoot,
                ChildOf(scroll),
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .id();

        commands.commands_mut().entity(scroll).insert(ScrollArea {
            target: content,
            vertical: true,
            ..default()
        });
    });
}

fn update(
    roots: Query<(Entity, Ref<PropertiesRoot>)>,
    selection_map: Res<SelectionMap>,
    mut commands: Commands,
) {
    for (root_entity, root) in roots {
        if !(root.is_added() || selection_map.is_changed()) {
            continue;
        }

        commands
            .entity(root_entity)
            .despawn_children()
            .with_children(|commands| {
                if selection_map.0.len() > 1 {
                    commands.spawn(EditorText::new("Selection:"));
                    for (selection, entities) in selection_map.0.iter() {
                        commands.spawn(EditorText::new(format!(
                            "\t{} x{}",
                            selection,
                            entities.len()
                        )));
                    }
                } else if let Some((selection, entities)) = selection_map.0.iter().next() {
                    match selection {
                        Selection::Entity => {}
                        Selection::File(path_buf) => todo!(),
                    }
                }
            });
    }
}
