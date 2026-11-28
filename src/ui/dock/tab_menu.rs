//! Context menu for a dock tab.

use bevy::{
    feathers::{
        controls::{FeathersMenuItem, FeathersMenuPopup},
        display::caption,
    },
    input_focus::tab_navigation::NavAction,
    picking::events::PointerClick,
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::{Activate, MenuAction, MenuEvent, MenuFocusState},
};

use super::{
    layout::{DockLayout, PanelId},
    view::{DockRoot, DragsPanel},
};

#[derive(Component)]
pub(super) struct TabMenu;

#[derive(Component, Clone)]
struct CloseTab(PanelId);

pub(super) fn open(
    mut click: On<PointerClick>,
    tabs: Query<&DragsPanel>,
    root: Single<(Entity, &ComputedNode, &UiGlobalTransform), With<DockRoot>>,
    menus: Query<Entity, With<TabMenu>>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok(&DragsPanel(panel)) = tabs.get(click.entity) else {
        return;
    };
    click.propagate(false);
    for menu in &menus {
        commands.entity(menu).try_despawn();
    }

    // Anchor the menu at the click in dock-local coordinates, outside the clipped tab strip.
    let (root, node, transform) = *root;
    let origin = (transform.translation - node.size() * 0.5) * node.inverse_scale_factor;
    let position = click.pointer.position - origin;
    let close = CloseTab(panel);
    commands
        .spawn((
            Name::new("Tab Context Menu"),
            TabMenu,
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                ..default()
            },
            ChildOf(root),
            Pickable::IGNORE,
        ))
        .observe(dismiss)
        .apply_scene(bsn! {
            Children [
                @FeathersMenuPopup
                Node { min_width: px(100) }
                Visibility::Visible
                MenuFocusState::Opening(NavAction::First)
                Children [
                    @FeathersMenuItem {
                        @caption: bsn! { @caption("Close") Pickable::IGNORE }
                    }
                    close
                    on(close_tab)
                ]
            ]
        });
}

fn dismiss(mut event: On<MenuEvent>, mut commands: Commands) {
    if matches!(event.action, MenuAction::CloseAll) {
        event.propagate(false);
        commands.entity(event.source).try_despawn();
    }
}

fn close_tab(activate: On<Activate>, items: Query<&CloseTab>, mut layout: ResMut<DockLayout>) {
    if let Ok(&CloseTab(panel)) = items.get(activate.entity) {
        layout.remove_panel(panel);
    }
}
