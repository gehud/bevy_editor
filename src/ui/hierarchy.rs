//! The Scene Tree panel: the edited scene's entity hierarchy.
//!
//! Each frame a [`Row`] snapshot of the visible tree is computed from the world; the rows are
//! rebuilt only when that snapshot changes (an entity spawned, despawned, renamed, reparented,
//! hidden, expanded, selected, …).
//!
//! - Click a row to select its entity ([`Selection`]).
//! - Click the chevron to collapse or expand its children.
//! - Click the eye to hide or show the entity ([`Visibility`]).

use bevy::{
    camera::Camera,
    light::{DirectionalLight, PointLight, SpotLight},
    picking::events::PointerClick,
    platform::collections::HashSet,
    prelude::*,
    ui_widgets::Button,
};

use super::{
    icons,
    style::{HoverBackground, Weight, colors, icon_sq, ink, text},
    widgets::{filter_bar, wide_button},
};
use crate::editing::{EditorOnly, Selection};

/// Indentation per hierarchy level, in pixels.
const INDENT: f32 = 8.0;

/// Entities that are part of the edited scene.
type SceneFilter = (With<Transform>, Without<EditorOnly>, Without<Node>);

/// Scene Tree panel contents: filter, "Add Entity" and the entity hierarchy.
pub fn scene_tree_content() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(6)),
        }
        BackgroundColor(colors::PANE_BG)
        Children [
            Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: percent(100) }
            Children [
                @filter_bar()
                --
                @wide_button(icons::PACKAGE_PLUS, "Add Entity")
                on(add_entity)
            ]
            --
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
            }
            SceneTreeRows
        ]
    }
}

/// The container the rows are rebuilt into.
#[derive(Component, Clone, Default)]
pub struct SceneTreeRows;

/// Which entities are collapsed in the Scene Tree (everything else is expanded).
#[derive(Resource, Default)]
pub struct SceneTreeState {
    collapsed: HashSet<Entity>,
}

/// What a row reads from its entity.
type RowData = (
    Option<&'static Name>,
    Option<&'static Children>,
    Option<&'static Visibility>,
    Option<&'static InheritedVisibility>,
    Has<Camera>,
    Has<DirectionalLight>,
    Has<PointLight>,
    Has<SpotLight>,
    Has<Mesh3d>,
);

/// What kind of icon a row shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowKind {
    Camera,
    Light,
    Mesh,
    Empty,
}

/// Everything a row displays; the tree is rebuilt when any row changes.
#[derive(Clone, PartialEq, Debug)]
struct Row {
    entity: Entity,
    depth: usize,
    name: String,
    kind: RowKind,
    has_children: bool,
    expanded: bool,
    /// The entity's own visibility setting is not `Hidden`.
    visible: bool,
    /// Hidden itself or by an ancestor: the row is drawn faded.
    faded: bool,
    selected: bool,
}

/// Row snapshot of the last rebuild.
#[derive(Default)]
pub(super) struct LastRows(Vec<Row>);

pub(super) fn sync_scene_tree(
    mut commands: Commands,
    container: Single<Entity, With<SceneTreeRows>>,
    roots: Query<Entity, (SceneFilter, Without<ChildOf>)>,
    scene: Query<RowData, SceneFilter>,
    state: Res<SceneTreeState>,
    selection: Res<Selection>,
    mut last: Local<LastRows>,
) {
    let mut rows = Vec::new();
    let mut roots: Vec<Entity> = roots.iter().collect();
    roots.sort();
    let mut stack: Vec<(Entity, usize)> = roots.into_iter().rev().map(|e| (e, 0)).collect();

    while let Some((entity, depth)) = stack.pop() {
        let Ok((name, children, visibility, inherited, camera, dir_light, point, spot, mesh)) =
            scene.get(entity)
        else {
            continue;
        };
        let scene_children: Vec<Entity> = children
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| scene.contains(*child))
            .collect();
        let expanded = !state.collapsed.contains(&entity);
        let visible = visibility != Some(&Visibility::Hidden);
        rows.push(Row {
            entity,
            depth,
            name: name.map_or_else(|| format!("Entity {entity}"), |n| n.as_str().to_owned()),
            kind: if camera {
                RowKind::Camera
            } else if dir_light || point || spot {
                RowKind::Light
            } else if mesh {
                RowKind::Mesh
            } else {
                RowKind::Empty
            },
            has_children: !scene_children.is_empty(),
            expanded,
            visible,
            faded: !visible || inherited.is_some_and(|v| !v.get()),
            selected: selection.0 == Some(entity),
        });
        if expanded {
            stack.extend(scene_children.into_iter().rev().map(|c| (c, depth + 1)));
        }
    }

    if rows == last.0 {
        return;
    }
    let scenes = rows.iter().map(row_scene).collect::<Vec<_>>();
    commands
        .entity(*container)
        .despawn_related::<Children>()
        .apply_scene(bsn! { Children [ {scenes} ] });
    last.0 = rows;
}

/// Identifies the entity a row (or a button in it) acts on.
#[derive(Component, Clone, Copy)]
struct RowEntity(Entity);

impl Default for RowEntity {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Marks a row's expand/collapse chevron.
#[derive(Component, Clone, Copy, Default)]
struct ExpandToggle;

/// Marks a row's visibility (eye) button.
#[derive(Component, Clone, Copy, Default)]
struct VisibilityToggle;

fn row_scene(row: &Row) -> impl Scene + use<> {
    let target = RowEntity(row.entity);
    let alpha = if row.faded { 0.6 } else { 1.0 };
    let (expander_icon, expander_color) = match (row.has_children, row.expanded) {
        (false, _) => (icons::DOT, ink::DDD),
        (true, false) => (icons::CHEVRON_RIGHT, ink::E6),
        (true, true) => (icons::CHEVRON_DOWN, ink::E6),
    };
    let (kind_icon, kind_color) = match row.kind {
        RowKind::Camera => (icons::VIDEO, ink::DDD),
        RowKind::Light => (icons::LIGHTBULB, ink::E6),
        RowKind::Mesh | RowKind::Empty => (icons::BOX, ink::E6),
    };
    let (visibility_icon, visibility_color) = if row.visible {
        (icons::EYE, ink::E6)
    } else {
        (icons::EYE_OFF, ink::DDD)
    };
    // Selected rows are outlined; expanded parents keep a background, as in the design.
    let background = if row.selected || (row.has_children && row.expanded) {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let (border, border_width) = if row.selected {
        (colors::ACCENT, 2.0)
    } else {
        (Color::NONE, 0.0)
    };
    let weight = if row.selected {
        Weight::Medium
    } else {
        Weight::Regular
    };
    let indent = row.depth as f32 * INDENT;
    let name = row.name.clone();

    bsn! {
        Button
        Node {
            height: px(24),
            flex_shrink: 0.0,
            margin: UiRect::left(px(indent)),
            padding: UiRect { left: px(6), right: px(8), top: px(4), bottom: px(4) },
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::all(px(border_width)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG }
        BorderColor { top: border, bottom: border, left: border, right: border }
        target
        on(select_row)
        Children [
            Node { column_gap: px(4), align_items: AlignItems::Center, min_width: px(0) }
            Pickable::IGNORE
            Children [
                Button
                Node { flex_shrink: 0.0 }
                target
                ExpandToggle
                on(toggle_expanded)
                Children [ @icon_sq(expander_icon, 15.0, expander_color.with_alpha(alpha)) Pickable::IGNORE ]
                --
                @icon_sq(kind_icon, 15.0, kind_color.with_alpha(alpha))
                Pickable::IGNORE
                --
                @text(name, 12.0, weight, ink::DDD.with_alpha(alpha))
                Pickable::IGNORE
            ]
            --
            Button
            Node { flex_shrink: 0.0 }
            target
            VisibilityToggle
            on(toggle_visibility)
            Children [ @icon_sq(visibility_icon, 15.0, visibility_color.with_alpha(alpha)) Pickable::IGNORE ]
        ]
    }
}

fn select_row(
    mut click: On<PointerClick>,
    rows: Query<&RowEntity, (Without<ExpandToggle>, Without<VisibilityToggle>)>,
    mut selection: ResMut<Selection>,
) {
    let Ok(RowEntity(entity)) = rows.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button == PointerButton::Primary {
        selection.0 = Some(*entity);
    }
}

fn toggle_expanded(
    mut click: On<PointerClick>,
    toggles: Query<&RowEntity, With<ExpandToggle>>,
    mut state: ResMut<SceneTreeState>,
) {
    let Ok(RowEntity(entity)) = toggles.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button == PointerButton::Primary && !state.collapsed.remove(entity) {
        state.collapsed.insert(*entity);
    }
}

fn toggle_visibility(
    mut click: On<PointerClick>,
    toggles: Query<&RowEntity, With<VisibilityToggle>>,
    mut visibilities: Query<&mut Visibility>,
) {
    let Ok(RowEntity(entity)) = toggles.get(click.entity) else {
        return;
    };
    click.propagate(false);
    if click.button == PointerButton::Primary
        && let Ok(mut visibility) = visibilities.get_mut(*entity)
    {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

/// "Add Entity": spawns an empty entity (child of the selection, if any) and selects it.
fn add_entity(
    click: On<PointerClick>,
    mut commands: Commands,
    mut selection: ResMut<Selection>,
    scene: Query<(), SceneFilter>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let mut entity = commands.spawn((
        Name::new("Entity"),
        Transform::default(),
        Visibility::default(),
    ));
    if let Some(parent) = selection.0.filter(|parent| scene.contains(*parent)) {
        entity.insert(ChildOf(parent));
    }
    selection.0 = Some(entity.id());
}
