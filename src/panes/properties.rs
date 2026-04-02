use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        hierarchy::ChildOf,
        message::MessageReader,
        observer::On,
        query::With,
        system::{Commands, In, Query, Res},
        world::Ref,
    },
    picking::{
        Pickable,
        events::{Click, Pointer},
    },
    platform::collections::HashSet,
    ui::{
        AlignContent, AlignItems, FlexDirection, Node, Overflow, PositionType, UiRect,
        auto_directional_navigation::AutoDirectionalNavigator, percent, px, widget::Text,
    },
    utils::default,
};

use crate::{
    pane::{PaneApp, PaneStructure},
    selection::{Selection, SelectionChanged, SelectionMap},
    theme::{RoundedCorners, ThemedBackgroundColor, ThemedBorderColor, tokens::BUTTON_BG},
    widget::{EditorText, ScrollArea, TextField, TextFieldSettings, ValueChange},
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
                let root = commands.target_entity();
                if selection_map.0.len() > 1 {
                    commands.spawn((Text::new("Selection:"), EditorText::body()));
                    for (selection, entities) in selection_map.0.iter() {
                        commands.spawn((
                            Text::new(format!("\t{} x{}", selection, entities.len())),
                            EditorText::body(),
                        ));
                    }
                } else if let Some((selection, entities)) = selection_map.0.iter().next() {
                    match selection {
                        Selection::Entity => {
                            commands.commands_mut().run_system_cached_with(
                                setup_entities_properties,
                                (root, entities.clone()),
                            );
                        }
                        Selection::File(path_buf) => todo!(),
                    }
                }
            });
    }
}

fn setup_entities_properties(
    In((root, entities)): In<(Entity, HashSet<Entity>)>,
    mut commands: Commands,
) {
    if entities.len() != 1 {
        return;
    }

    let entity = *entities.iter().next().unwrap();

    commands
        .spawn((
            ChildOf(root),
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
        ))
        .with_children(|commands| {
            commands.spawn((Text::new("Name:"), EditorText::body()));

            commands
                .spawn(TextField::new().build())
                .observe(|trigger: On<ValueChange<String>>| {
                    bevy::log::info!("{} -> {}", trigger.previous, trigger.new);
                });
        });
}
