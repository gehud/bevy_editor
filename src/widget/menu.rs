use bevy::{
    app::{App, Plugin, Update},
    camera::NormalizedRenderTarget,
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        hierarchy::ChildOf,
        message::MessageReader,
        observer::On,
        query::With,
        system::{Commands, Query},
    },
    math::Vec2,
    picking::{
        events::{Click, Over, Pointer},
        pointer::PointerButton,
    },
    ui::{ComputedNode, UiGlobalTransform},
};

use crate::{
    widget::{CloseContextMenu, ContextMenu, ContextMenuAction, SpawnContextMenu},
    window::EditorWindow,
};

#[derive(Component)]
pub struct MenuButton(pub ContextMenu);

#[derive(Component, Default)]
struct MenuBarState {
    selected_button: Option<Entity>,
}

#[derive(Component, Default, Clone, Copy, Debug)]
#[require(MenuBarState)]
pub struct MenuBar;

pub struct MenuBarPlugin;

impl Plugin for MenuBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_menu_button_click)
            .add_observer(on_menu_button_over)
            .add_systems(Update, deselect_menu_bar);
    }
}

fn on_menu_button_click(
    trigger: On<Pointer<Click>>,
    editor_windows: Query<(), With<EditorWindow>>,
    menu_buttons: Query<&MenuButton>,
    parents: Query<&ChildOf>,
    computed_nodes: Query<&ComputedNode>,
    ui_global_transforms: Query<&UiGlobalTransform>,
    mut menu_bars: Query<&mut MenuBarState>,
    mut commands: Commands,
) -> Result {
    if trigger.button != PointerButton::Primary {
        return Ok(());
    }

    let NormalizedRenderTarget::Window(window_ref) = trigger.pointer_location.target else {
        return Ok(());
    };

    let editor_window = window_ref.entity();

    if !editor_windows.contains(editor_window) {
        return Ok(());
    }

    let Ok(menu_button) = menu_buttons.get(trigger.entity) else {
        return Ok(());
    };

    let Ok(parent) = parents.get(trigger.entity) else {
        return Ok(());
    };

    let Ok(mut menu_bar) = menu_bars.get_mut(parent.parent()) else {
        return Ok(());
    };

    spawn_context_menu(
        trigger.entity,
        &computed_nodes,
        &ui_global_transforms,
        &mut commands,
        menu_button,
        &mut menu_bar,
        editor_window,
    )?;

    Ok(())
}

fn on_menu_button_over(
    trigger: On<Pointer<Over>>,
    editor_windows: Query<(), With<EditorWindow>>,
    menu_buttons: Query<&MenuButton>,
    parents: Query<&ChildOf>,
    computed_nodes: Query<&ComputedNode>,
    ui_global_transforms: Query<&UiGlobalTransform>,
    mut menu_bars: Query<&mut MenuBarState>,
    mut commands: Commands,
) -> Result {
    let Ok(menu_button) = menu_buttons.get(trigger.entity) else {
        return Ok(());
    };

    let Ok(parent) = parents.get(trigger.entity) else {
        return Ok(());
    };

    let Ok(mut menu_bar) = menu_bars.get_mut(parent.parent()) else {
        return Ok(());
    };

    let Some(selected_button) = menu_bar.selected_button else {
        return Ok(());
    };

    if selected_button == trigger.entity {
        return Ok(());
    }

    let NormalizedRenderTarget::Window(window_ref) = trigger.pointer_location.target else {
        return Ok(());
    };

    let editor_window = window_ref.entity();

    if !editor_windows.contains(editor_window) {
        return Ok(());
    }

    commands.write_message(CloseContextMenu);

    spawn_context_menu(
        trigger.entity,
        &computed_nodes,
        &ui_global_transforms,
        &mut commands,
        menu_button,
        &mut menu_bar,
        editor_window,
    )?;

    Ok(())
}

fn spawn_context_menu(
    target: Entity,
    computed_nodes: &Query<&ComputedNode>,
    ui_global_transforms: &Query<&UiGlobalTransform>,
    commands: &mut Commands,
    menu_button: &MenuButton,
    menu_bar: &mut MenuBarState,
    editor_window: Entity,
) -> Result {
    let computed_node = computed_nodes.get(target)?;
    let button_position =
        ui_global_transforms.get(target)?.translation * computed_node.inverse_scale_factor();
    let button_half =
        computed_nodes.get(target)?.size() * computed_node.inverse_scale_factor() * 0.5;
    let position = Vec2 {
        x: button_position.x - button_half.x,
        y: button_position.y + button_half.y,
    };
    menu_bar.selected_button = Some(target);
    commands.write_message(SpawnContextMenu {
        editor_window,
        menu: menu_button.0.clone(),
        position,
    });

    Ok(())
}

fn deselect_menu_bar(
    mut actions: MessageReader<ContextMenuAction>,
    mut menu_bars: Query<&mut MenuBarState>,
) {
    for action in actions.read() {
        match action {
            ContextMenuAction::Discard | ContextMenuAction::Select => {
                for mut menu_bar in &mut menu_bars {
                    menu_bar.selected_button = None;
                }
            }
            _ => {}
        }
    }
}
