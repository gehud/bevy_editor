//! Turns the [`DockLayout`] into UI entities.
//!
//! Only the chrome (splits, splitters, leaf frames, tab bars) is rebuilt when the layout
//! changes. Panel contents are spawned once and re-parented into whichever leaf shows them, so
//! their state (viewport cameras, typed text, scroll positions) survives layout changes.
//! Contents not on screen wait in a hidden "stash" node.

use bevy::{
    feathers::controls::FeathersScrollbar,
    input_focus::tab_navigation::TabIndex,
    math::BVec2,
    picking::cursor::EntityCursor,
    platform::collections::HashMap,
    prelude::*,
    ui::IgnoreScroll,
    ui_widgets::{Button, ControlOrientation},
    window::SystemCursorIcon,
};

use crate::ui::icons;

use super::{
    interact,
    layout::{Axis, DockLayout, DockNode, NodeId, PanelId},
    tab_menu,
};
use crate::ui::{
    panels::{PanelKind, Panels},
    style::{HoverBackground, colors, icon_sq, ink, label},
};

/// Space between docked areas; also the grab area of the splitters.
pub const GAP: f32 = 4.0;

/// The node the dock tree is built into.
#[derive(Component, Clone, Default)]
pub struct DockRoot;

/// Hidden holder for panel contents that are not currently displayed.
#[derive(Component, Clone, Default)]
pub struct DockStash;

/// Root entity of a panel's contents.
#[derive(Component, Clone, Default)]
pub struct PanelContent(pub PanelId);

/// The frame of a leaf; used to find drop targets.
#[derive(Component, Clone, Default)]
pub struct DockLeafArea(pub NodeId);

/// Where a leaf's active panel contents are placed.
#[derive(Component, Clone, Default)]
pub struct LeafContentSlot(pub NodeId);

/// The container of a split's children.
#[derive(Component, Clone, Default)]
pub struct DockSplitArea {
    pub split: NodeId,
    pub axis: Axis,
}

/// A child of a split; its `flex_grow` is the child's size.
#[derive(Component, Clone, Default)]
pub struct DockSplitChild {
    pub split: NodeId,
    pub index: usize,
}

/// The draggable gap between children `index` and `index + 1` of a split.
#[derive(Component, Clone, Default)]
pub struct Splitter {
    pub split: NodeId,
    pub index: usize,
    pub axis: Axis,
}

/// A tab in a leaf's tab bar.
#[derive(Component, Clone, Default)]
pub struct DockTab {
    pub leaf: NodeId,
    pub index: usize,
}

#[derive(Component, Clone, Default)]
pub struct DockTabStrip(pub NodeId);

/// Dragging this tab moves the panel to another place.
#[derive(Component, Clone, Default)]
pub struct DragsPanel(pub PanelId);

/// Creates the stash and spawns every panel's contents into it.
pub(super) fn setup_dock(mut commands: Commands, panels: Res<Panels>) {
    let stash = commands
        .spawn((
            Name::new("Dock Stash"),
            DockStash,
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    for id in panels.ids() {
        commands
            .spawn((
                Name::new(panels.get(id).title()),
                PanelContent(id),
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ChildOf(stash),
            ))
            .queue_apply_scene(panels.get(id).content());
    }
}

/// Rebuilds the dock chrome whenever the layout changes.
pub(super) fn rebuild_dock(
    mut commands: Commands,
    layout: Res<DockLayout>,
    panels: Res<Panels>,
    root: Single<(Entity, Option<&Children>), With<DockRoot>>,
    stash: Single<Entity, With<DockStash>>,
    contents: Query<Entity, With<PanelContent>>,
    tab_strips: Query<(&DockTabStrip, &ScrollPosition)>,
) {
    if !layout.is_changed() {
        return;
    }
    let tab_offsets = tab_strips
        .iter()
        .map(|(strip, scroll)| (strip.0, scroll.x))
        .collect();
    // Move contents to safety first: despawning the chrome despawns its descendants.
    for content in &contents {
        commands.entity(content).insert(ChildOf(*stash));
    }
    let (root, children) = *root;
    for &child in children.into_iter().flatten() {
        commands.entity(child).despawn();
    }
    // Spawn immediately rather than queued: a queued spawn can land a frame later, which
    // would show the dock empty for a frame (visible as flicker on every tab switch or drop).
    let tree = vec![node_scene(&layout, &panels, layout.root(), &tab_offsets)];
    commands
        .entity(root)
        .apply_scene(bsn! { Children [ {tree} ] });
}

/// Puts each leaf's active panel into the leaf, and every other panel into the stash.
pub(super) fn place_panel_contents(
    mut commands: Commands,
    layout: Res<DockLayout>,
    slots: Query<(Entity, &LeafContentSlot)>,
    contents: Query<(Entity, &PanelContent, &ChildOf)>,
    stash: Single<Entity, With<DockStash>>,
) {
    for (content, PanelContent(panel), child_of) in &contents {
        let wanted_parent = layout
            .leaf_of(*panel)
            .filter(|leaf| layout.active_panel(*leaf) == Some(*panel))
            .and_then(|leaf| {
                slots
                    .iter()
                    .find(|(_, slot)| slot.0 == leaf)
                    .map(|(entity, _)| entity)
            })
            .unwrap_or(*stash);
        if child_of.parent() != wanted_parent {
            commands.entity(content).insert(ChildOf(wanted_parent));
        }
    }
}

fn node_scene(
    layout: &DockLayout,
    panels: &Panels,
    id: NodeId,
    tab_offsets: &HashMap<NodeId, f32>,
) -> Box<dyn Scene> {
    match layout.node(id) {
        DockNode::Leaf { tabs, active } => {
            let offset = tab_offsets.get(&id).copied().unwrap_or_default();
            Box::new(leaf_scene(panels, id, tabs, *active, offset))
        }
        DockNode::Split {
            axis,
            children,
            sizes,
        } => {
            let mut items: Vec<Box<dyn Scene>> = Vec::new();
            for (index, (child, size)) in children.iter().zip(sizes).enumerate() {
                if index > 0 {
                    items.push(Box::new(splitter(id, index - 1, *axis)));
                }
                let child = node_scene(layout, panels, *child, tab_offsets);
                let size = *size;
                items.push(Box::new(bsn! {
                    @{child}
                    Node {
                        flex_grow: size,
                        flex_shrink: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        min_height: px(0),
                    }
                    DockSplitChild { split: id, index: index }
                }));
            }
            let direction = match axis {
                Axis::Horizontal => FlexDirection::Row,
                Axis::Vertical => FlexDirection::Column,
            };
            let axis = *axis;
            Box::new(bsn! {
                Node {
                    flex_grow: 1.0,
                    flex_direction: direction,
                    min_width: px(0),
                    min_height: px(0),
                }
                DockSplitArea { split: id, axis: axis }
                Children [ {items} ]
            })
        }
    }
}

fn splitter(split: NodeId, index: usize, axis: Axis) -> impl Scene {
    let (width, height, cursor) = match axis {
        Axis::Horizontal => (px(GAP), Val::Auto, SystemCursorIcon::EwResize),
        Axis::Vertical => (Val::Auto, px(GAP), SystemCursorIcon::NsResize),
    };
    bsn! {
        Node {
            width: width,
            height: height,
            flex_shrink: 0.0,
            align_self: AlignSelf::Stretch,
        }
        Splitter { split: split, index: index, axis: axis }
        EntityCursor::System(cursor)
        on(interact::drag_splitter)
    }
}

fn leaf_scene(
    panels: &Panels,
    leaf: NodeId,
    tabs: &[PanelId],
    active: usize,
    scroll_offset: f32,
) -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(px(6)),
        }
        BackgroundColor(colors::PANE_BG)
        DockLeafArea(leaf)
        Children [
            @tab_bar(panels, leaf, tabs, active, scroll_offset)
            --
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                flex_direction: FlexDirection::Column,
            }
            LeafContentSlot(leaf)
        ]
    }
}

/// Makes a tab: switches to its panel on click, and moves the panel when dragged.
fn tab(look: Box<dyn Scene>, leaf: NodeId, index: usize, panel: PanelId) -> impl Scene {
    let handle = DragsPanel(panel);
    bsn! {
        @{look}
        Button
        DockTab { leaf: leaf, index: index }
        handle
        on(interact::click_tab)
        on(tab_menu::open)
        on(interact::start_panel_drag)
        on(interact::drag_panel)
        on(interact::end_panel_drag)
    }
}

/// The grip opens pane options when the active pane provides them.
fn grip(has_options: bool) -> impl Scene {
    let options = has_options.then(|| {
        bsn! {
            Name::new("Assets View Options")
            Button
            TabIndex(0)
            tab_menu::AssetsOptionsButton
            EntityCursor::System(SystemCursorIcon::Pointer)
            BackgroundColor(Color::NONE)
            HoverBackground { normal: Color::NONE, hover: colors::ITEM_BG_ACTIVE }
            on(tab_menu::open_assets_options)
        }
    });
    let pickable = if has_options {
        Pickable::default()
    } else {
        Pickable::IGNORE
    };
    bsn! {
        @{options}
        Node {
            width: px(12),
            height: px(22),
            padding: UiRect::axes(px(1), px(3)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px(3)),
        }
        pickable
        Children [ @icon_sq(icons::GRIP_VERTICAL, 12.0, ink::B3B3) Pickable::IGNORE ]
    }
}

fn add_tab_button(leaf: NodeId) -> impl Scene {
    let button = tab_menu::AddTabButton(leaf);
    bsn! {
        Name::new("Add Pane")
        Button
        TabIndex(0)
        button
        EntityCursor::System(SystemCursorIcon::Pointer)
        Node {
            width: px(18),
            height: px(18),
            flex_shrink: 0.0,
            margin: UiRect::left(px(5)),
            border_radius: BorderRadius::all(px(3)),
        }
        BackgroundColor(Color::NONE)
        HoverBackground { normal: Color::NONE, hover: colors::ITEM_BG_ACTIVE }
        on(tab_menu::open_add)
        Children [ @icon_sq(icons::PLUS, 18.0, ink::A8A8) Pickable::IGNORE ]
    }
}

/// Titled tabs along the top of the leaf.
fn tab_bar(
    panels: &Panels,
    leaf: NodeId,
    tabs: &[PanelId],
    active: usize,
    scroll_offset: f32,
) -> impl Scene {
    let tab_items = tabs
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            let title = panels.get(*panel).title();
            let look: Box<dyn Scene> = if index == active {
                Box::new(active_tab(title))
            } else {
                Box::new(inactive_tab(title))
            };
            tab(look, leaf, index, *panel)
        })
        .collect::<Vec<_>>();
    let grip = tabs
        .get(active)
        .map(|panel| grip(panels.get(*panel) == PanelKind::Assets))
        .into_iter()
        .collect::<Vec<_>>();
    let scroll = ScrollPosition(Vec2::new(scroll_offset, 0.0));

    bsn! {
        Node {
            height: px(30),
            width: percent(100),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            padding: UiRect { right: px(8), top: px(1) },
            border: UiRect { left: px(1), right: px(1), top: px(1) },
            border_radius: BorderRadius::top(px(6)),
        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor { top: colors::PANEL_BORDER, left: colors::PANEL_BORDER, right: colors::PANEL_BORDER }
        Children [
            #strip
            Node {
                height: percent(100),
                min_width: px(0),
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                padding: UiRect::left(px(6)),
                overflow: Overflow::scroll_x(),
            }
            DockTabStrip(leaf)
            scroll
            on(interact::scroll_tabs)
            Children [
                {tab_items}
                --
                @add_tab_button(leaf)
                --
                @FeathersScrollbar {
                    @target: #strip,
                    @orientation: {ControlOrientation::Horizontal},
                }
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    bottom: px(0),
                    height: px(4),
                }
                // Overlay the strip without scrolling along with the tabs.
                IgnoreScroll(BVec2::new(true, false))
                Visibility::Hidden
                ZIndex(1)
            ]
            --
            {grip}
        ]
    }
}

fn active_tab(title: String) -> impl Scene {
    bsn! {
        Node {
            height: percent(100),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(8)),
            border: UiRect { top: px(2) },
            border_radius: BorderRadius::top(px(2)),
        }
        BackgroundColor(colors::PANE_BG)
        BorderColor { top: colors::ACCENT }
        Children [
            @label(title, ink::F2F2)
            --
            // Concave corners that blend the tab into the panel below it.
            @tab_corner()
            Node { position_type: PositionType::Absolute, bottom: px(0), right: px(-4) }
            --
            @tab_corner()
            Node { position_type: PositionType::Absolute, bottom: px(0), left: px(-4) }
            UiTransform { scale: Vec2::new(-1.0, 1.0) }
        ]
    }
}

fn tab_corner() -> impl Scene {
    bsn! {
        @icon_sq(icons::TAB_CORNER, 4.0, ink::TAB_CORNER)
        Pickable::IGNORE
    }
}

fn inactive_tab(title: String) -> impl Scene {
    bsn! {
        Node {
            height: percent(100),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(8)),
        }
        Children [ @label(title, ink::A8A8) ]
    }
}
