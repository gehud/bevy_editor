//! Menus for closing and adding dock tabs and configuring the active pane.

use bevy::{
    feathers::{
        controls::{FeathersMenuItem, FeathersMenuPopup},
        display::caption,
    },
    input_focus::{FocusGained, tab_navigation::NavAction},
    picking::events::PointerClick,
    prelude::*,
    ui::{InteractionDisabled, UiGlobalTransform},
    ui_widgets::{
        Activate, MenuAction, MenuEvent, MenuFocusState, ScrollArea, ScrollIntoView,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
};

use crate::ui::{
    asset_browser::{AssetBrowserState, AssetView},
    panels::Panels,
};

use super::{
    layout::{DockLayout, NodeId, PanelId},
    view::{DockRoot, DockTabStrip, DragsPanel},
};

#[derive(Component)]
pub(super) struct TabMenu;

#[derive(Component, Clone)]
struct CloseTab(PanelId);

#[derive(Component, Clone)]
pub(super) struct AddTabButton(pub NodeId);

#[derive(Component, Clone, Default)]
pub(super) struct AssetsOptionsButton;

#[derive(Component, Clone, Copy)]
struct SetAssetView(AssetView);

#[derive(Component, Clone)]
struct AddTab {
    leaf: NodeId,
    panel: PanelId,
}

#[derive(Message)]
pub(super) struct PaneAdded(NodeId);

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
    let cursor = click.pointer.position;
    let close = CloseTab(panel);
    let item = bsn! {
        @FeathersMenuItem {
            @caption: bsn! { @caption("Close") Pickable::IGNORE }
        }
        close
        on(close_tab)
    };
    show_menu(
        Rect::from_corners(cursor, cursor),
        *root,
        &menus,
        vec![Box::new(item)],
        &mut commands,
    );
}

pub(super) fn open_add(
    activate: On<Activate>,
    buttons: Query<(&AddTabButton, &ComputedNode, &UiGlobalTransform)>,
    root: Single<(Entity, &ComputedNode, &UiGlobalTransform), With<DockRoot>>,
    menus: Query<Entity, With<TabMenu>>,
    panels: Res<Panels>,
    layout: Res<DockLayout>,
    mut commands: Commands,
) {
    let Ok((&AddTabButton(leaf), node, transform)) = buttons.get(activate.entity) else {
        return;
    };
    let mut items: Vec<Box<dyn Scene>> = panels
        .ids()
        .filter(|panel| layout.leaf_of(*panel).is_none())
        .map(|panel| {
            let title = panels.get(panel).title();
            let action = AddTab { leaf, panel };
            Box::new(bsn! {
                @FeathersMenuItem {
                    @caption: bsn! { @caption(title) Pickable::IGNORE }
                }
                Node { flex_shrink: 0.0 }
                action
                on(add_tab)
                on(scroll_to_focused_item)
            }) as Box<dyn Scene>
        })
        .collect();
    if items.is_empty() {
        items.push(Box::new(bsn! {
            @FeathersMenuItem {
                @caption: bsn! { @caption("No panes to add") Pickable::IGNORE }
            }
            InteractionDisabled
        }));
    }
    let anchor = Rect::from_center_size(
        transform.translation * node.inverse_scale_factor,
        node.size() * node.inverse_scale_factor,
    );
    show_menu(anchor, *root, &menus, items, &mut commands);
}

pub(super) fn open_assets_options(
    activate: On<Activate>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform), With<AssetsOptionsButton>>,
    root: Single<(Entity, &ComputedNode, &UiGlobalTransform), With<DockRoot>>,
    menus: Query<Entity, With<TabMenu>>,
    browser: Res<AssetBrowserState>,
    mut commands: Commands,
) {
    let Ok((node, transform)) = buttons.get(activate.entity) else {
        return;
    };
    let items = [
        (AssetView::Tree, "Tree"),
        (AssetView::ActiveFolder, "Active Folder"),
    ]
    .into_iter()
    .map(|(view, title)| {
        let title = if browser.view == view {
            format!("✓  {title}")
        } else {
            format!("    {title}")
        };
        let action = SetAssetView(view);
        Box::new(bsn! {
            @FeathersMenuItem {
                @caption: bsn! { @caption(title) Pickable::IGNORE }
            }
            action
            on(set_asset_view)
        }) as Box<dyn Scene>
    })
    .collect();
    let anchor = Rect::from_center_size(
        transform.translation * node.inverse_scale_factor,
        node.size() * node.inverse_scale_factor,
    );
    show_menu(anchor, *root, &menus, items, &mut commands);
}

fn set_asset_view(
    activate: On<Activate>,
    items: Query<&SetAssetView>,
    mut browser: ResMut<AssetBrowserState>,
) {
    if let Ok(&SetAssetView(view)) = items.get(activate.entity)
        && browser.view != view
    {
        browser.view = view;
    }
}

fn show_menu(
    anchor: Rect,
    (root, node, transform): (Entity, &ComputedNode, &UiGlobalTransform),
    menus: &Query<Entity, With<TabMenu>>,
    items: Vec<Box<dyn Scene>>,
    commands: &mut Commands,
) {
    for menu in menus {
        commands.entity(menu).try_despawn();
    }

    // Place the anchor in dock-local coordinates, outside the clipped tab strip.
    let origin = (transform.translation - node.size() * 0.5) * node.inverse_scale_factor;
    let position = anchor.min - origin;
    let placements = [
        (PopoverSide::Bottom, PopoverAlign::Start),
        (PopoverSide::Bottom, PopoverAlign::End),
        (PopoverSide::Top, PopoverAlign::Start),
        (PopoverSide::Top, PopoverAlign::End),
    ]
    .map(|(side, align)| PopoverPlacement {
        side,
        align,
        gap: 2.0,
    })
    .to_vec();
    commands
        .spawn((
            Name::new("Dock Tab Menu"),
            TabMenu,
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                width: px(anchor.width()),
                height: px(anchor.height()),
                ..default()
            },
            ChildOf(root),
            Pickable::IGNORE,
        ))
        .observe(dismiss)
        .apply_scene(bsn! {
            Children [
                @FeathersMenuPopup
                Node {
                    min_width: px(100),
                    max_height: Val::Vh(42.0),
                    overflow: Overflow::scroll_y(),
                }
                Popover { positions: placements }
                ScrollArea
                Visibility::Visible
                MenuFocusState::Opening(NavAction::First)
                Children [ {items} ]
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

fn add_tab(
    activate: On<Activate>,
    items: Query<&AddTab>,
    mut layout: ResMut<DockLayout>,
    mut added: MessageWriter<PaneAdded>,
) {
    if let Ok(action) = items.get(activate.entity)
        && layout.add_panel(action.panel, action.leaf)
    {
        added.write(PaneAdded(action.leaf));
    }
}

fn scroll_to_focused_item(event: On<FocusGained>, mut commands: Commands) {
    commands.trigger(ScrollIntoView {
        entity: event.entity,
    });
}

pub(super) fn reveal_added_tab(
    mut added: MessageReader<PaneAdded>,
    mut strips: Query<(&DockTabStrip, &ComputedNode, &mut ScrollPosition)>,
) {
    for &PaneAdded(leaf) in added.read() {
        if let Some((_, node, mut offset)) = strips.iter_mut().find(|(strip, _, _)| strip.0 == leaf)
        {
            // The new tab and the plus button are at the end of the strip.
            offset.x = (node.content_size().x - node.size().x).max(0.0) * node.inverse_scale_factor;
        }
    }
}
