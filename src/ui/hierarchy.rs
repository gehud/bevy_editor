//! The Scene Tree panel: the edited scene's entity hierarchy.
//!
//! Each frame a [`Row`] snapshot of the visible tree is computed from the world; the rows are
//! rebuilt only when that snapshot changes (an entity spawned, despawned, renamed, reparented,
//! hidden, expanded, selected, …). Rebuilds wait while a row is renamed or dragged, so the text
//! field and the dragged row stay alive.
//!
//! - Click a row to select its entity ([`Selection`]); Ctrl+click adds or removes it, Shift+click
//!   selects the range from the last clicked row.
//! - Type in the filter field to show only entities whose name contains the text (ignoring
//!   case), with their ancestors (dimmed) and everything expanded.
//! - Click the chevron to collapse or expand its children.
//! - Click the eye to hide or show the entity ([`Visibility`]).
//! - Delete removes the selected entities (with their descendants).
//! - F2 or a double-click renames in place: Enter (or clicking elsewhere) applies the name to
//!   every selected entity, Escape cancels.
//! - Drag rows to reorder or reparent them: the top or bottom quarter of a row drops before or
//!   after it, the middle drops into it as the last child, and the empty space below the rows
//!   moves to the end of the root list. Dragging a selected row moves the whole selection;
//!   reparented entities keep their world transform.

use bevy::{
    camera::Camera,
    ecs::system::SystemParam,
    feathers::{
        controls::{FeathersTextInput, FeathersTextInputContainer},
        theme::ThemeBackgroundColor,
        tokens,
    },
    input::keyboard::{Key, KeyboardInput},
    input_focus::{AutoFocus, FocusLost, FocusedInput, InputFocus},
    light::{DirectionalLight, PointLight, SpotLight},
    picking::events::{PointerClick, PointerDrag, PointerDragEnd, PointerDragStart},
    platform::collections::HashSet,
    prelude::*,
    text::{EditableText, TextEdit},
    ui::UiGlobalTransform,
    ui_widgets::{Button, ScrollArea, TextInput},
};

use super::{
    icons,
    style::{HoverBackground, Weight, colors, icon_sq, ink, text},
    widgets::{filter_bar, wide_button},
};
use crate::editing::{EditorOnly, Selection};

/// Indentation per hierarchy level, in pixels.
const INDENT: f32 = 8.0;
/// Vertical gap between rows, in pixels.
const ROW_GAP: f32 = 3.0;
/// How far the pointer must move before pressing a row turns into dragging it, in pixels.
const DRAG_THRESHOLD: f32 = 4.0;
/// Longest interval between the two clicks of a double-click, in seconds.
const DOUBLE_CLICK_SECS: f64 = 0.4;

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
                SceneTreeFilter
                --
                @wide_button(icons::PACKAGE_PLUS, "Add Entity")
                on(add_entity)
            ]
            --
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP),
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
            }
            ScrollArea
            SceneTreeRows
        ]
    }
}

/// The container the rows are rebuilt into.
#[derive(Component, Clone, Default)]
pub struct SceneTreeRows;

/// Marks the Scene Tree's filter bar (the text input is inside it).
#[derive(Component, Clone, Default)]
pub(super) struct SceneTreeFilter;

/// Scene Tree state that is not part of the scene.
#[derive(Resource, Default)]
pub struct SceneTreeState {
    /// Entities whose children are hidden (everything else is expanded).
    collapsed: HashSet<Entity>,
    /// Order of the root entities, which have no `Children` list to keep one. New roots go last.
    root_order: Vec<Entity>,
    /// The entity whose name is being edited.
    renaming: Option<Entity>,
    /// The row of the last plain or Ctrl click: where a Shift+click range starts.
    anchor: Option<Entity>,
    /// The last plain click and when it happened (in real seconds), to detect double-clicks.
    /// (Rows are rebuilt on selection changes, so picking's own click count doesn't work here.)
    last_click: Option<(Entity, f64)>,
    /// The row being dragged, if any.
    drag: Option<RowDrag>,
    /// The filter text, trimmed and lowercased; empty shows the whole tree.
    filter: String,
    /// Row snapshot of the last rebuild, top to bottom.
    rows: Vec<Row>,
    /// The last rebuild showed "No matches" (the filter hid every row).
    no_matches: bool,
}

/// A row drag in progress.
struct RowDrag {
    /// The row node the drag started on.
    row: Entity,
    /// The entity of that row.
    grabbed: Entity,
    /// The entities the drop moves, in tree order; empty until the pointer passes
    /// [`DRAG_THRESHOLD`].
    entities: Vec<Entity>,
    /// Where they would land.
    target: Option<DropTarget>,
}

/// Where dragged entities land.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DropTarget {
    /// Right before this entity, under the same parent.
    Before(Entity),
    /// Right after this entity, under the same parent.
    After(Entity),
    /// Into `parent`, as its first or last child.
    Into { parent: Entity, first: bool },
    /// At the end of the root list.
    RootEnd,
}

/// What a row reads from its entity.
type RowData = (
    Entity,
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
    /// The entity is the primary (most recently selected) one.
    active: bool,
    /// The row shows a text field to rename its entity.
    renaming: bool,
    /// Shown only as an ancestor of filter matches: the name is drawn dimmed.
    unmatched: bool,
}

/// The name a row shows for an entity.
fn display_name(entity: Entity, name: Option<&Name>) -> String {
    name.map_or_else(|| format!("Entity {entity}"), |n| n.as_str().to_owned())
}

/// Copies the Scene Tree filter field's text into [`SceneTreeState::filter`].
pub(super) fn read_scene_tree_filter(
    inputs: Query<(Entity, &EditableText), Changed<EditableText>>,
    filter_bars: Query<(), With<SceneTreeFilter>>,
    parents: Query<&ChildOf>,
    mut state: ResMut<SceneTreeState>,
) {
    for (input, text) in &inputs {
        if !parents
            .iter_ancestors(input)
            .any(|ancestor| filter_bars.contains(ancestor))
        {
            continue;
        }
        let filter = text.value().to_string().trim().to_lowercase();
        if state.filter != filter {
            state.filter = filter;
        }
    }
}

#[expect(clippy::too_many_arguments, reason = "a Bevy system")]
pub(super) fn sync_scene_tree(
    mut commands: Commands,
    container: Single<Entity, With<SceneTreeRows>>,
    roots: Query<Entity, (SceneFilter, Without<ChildOf>)>,
    scene: Query<RowData, SceneFilter>,
    tree_rows: Query<(), With<TreeRow>>,
    parents: Query<&ChildOf>,
    mut state: ResMut<SceneTreeState>,
    selection: Res<Selection>,
) {
    let state = &mut *state;

    // Keep the dragged row alive until the drop. If it vanished anyway, give up the drag.
    if let Some(drag) = &state.drag {
        if tree_rows.contains(drag.row) {
            return;
        }
        state.drag = None;
    }

    // Known roots keep their order; new roots go last, in spawn order.
    let current: HashSet<Entity> = roots.iter().collect();
    state.root_order.retain(|root| current.contains(root));
    let known: HashSet<Entity> = state.root_order.iter().copied().collect();
    let mut new_roots: Vec<Entity> = current.into_iter().filter(|e| !known.contains(e)).collect();
    new_roots.sort();
    state.root_order.extend(new_roots);

    // With a filter, only matches and their ancestors are shown, all expanded.
    let filtering = !state.filter.is_empty();
    let mut matched = HashSet::new();
    let mut shown = HashSet::new();
    if filtering {
        for (entity, name, ..) in &scene {
            if display_name(entity, name)
                .to_lowercase()
                .contains(&state.filter)
            {
                matched.insert(entity);
                shown.insert(entity);
                shown.extend(parents.iter_ancestors(entity));
            }
        }
    }
    let is_shown = |entity: &Entity| !filtering || shown.contains(entity);

    let mut rows = Vec::new();
    let mut stack: Vec<(Entity, usize)> = (state.root_order.iter().rev())
        .filter(|e| is_shown(e))
        .map(|e| (*e, 0))
        .collect();
    while let Some((entity, depth)) = stack.pop() {
        let Ok((_, name, children, visibility, inherited, camera, dir_light, point, spot, mesh)) =
            scene.get(entity)
        else {
            continue;
        };
        let scene_children: Vec<Entity> = children
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| scene.contains(*child) && is_shown(child))
            .collect();
        let expanded = filtering || !state.collapsed.contains(&entity);
        let visible = visibility != Some(&Visibility::Hidden);
        rows.push(Row {
            entity,
            depth,
            name: display_name(entity, name),
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
            selected: selection.contains(entity),
            active: selection.primary() == Some(entity),
            renaming: state.renaming == Some(entity),
            unmatched: filtering && !matched.contains(&entity),
        });
        if expanded {
            stack.extend(scene_children.into_iter().rev().map(|c| (c, depth + 1)));
        }
    }

    if let Some(renaming) = state.renaming {
        // A rename whose row is gone (despawned, or inside a collapsed parent) is dropped.
        if !rows.iter().any(|row| row.entity == renaming) {
            state.renaming = None;
        // While the text field is up, keep it (and the rows around it) as they are.
        } else if state
            .rows
            .iter()
            .any(|row| row.renaming && row.entity == renaming)
        {
            return;
        }
    }

    let no_matches = filtering && rows.is_empty();
    if rows == state.rows && no_matches == state.no_matches {
        return;
    }
    let mut scenes: Vec<Box<dyn Scene>> = rows
        .iter()
        .map(|row| -> Box<dyn Scene> { Box::new(row_scene(row)) })
        .collect();
    if no_matches {
        scenes.push(Box::new(bsn! {
            @text("No matches", 12.0, Weight::Regular, ink::DIM)
            Node { margin: UiRect::axes(px(6), px(4)) }
        }));
    }
    commands
        .entity(*container)
        .despawn_related::<Children>()
        .apply_scene(bsn! { Children [ {scenes} ] });
    state.rows = rows;
    state.no_matches = no_matches;
}

/// Identifies the entity a row (or a button in it) acts on.
#[derive(Component, Clone, Copy)]
struct RowEntity(Entity);

impl Default for RowEntity {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Marks a row node (as opposed to the buttons inside it).
#[derive(Component, Clone, Copy, Default)]
pub(super) struct TreeRow;

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
    // Selected rows get a background; only the active (primary) one is outlined.
    let background = if row.selected {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let (border, border_width) = if row.active {
        (colors::ACCENT, 2.0)
    } else {
        (Color::NONE, 0.0)
    };
    let weight = if row.selected {
        Weight::Medium
    } else {
        Weight::Regular
    };
    let name_color = if row.unmatched { ink::DIM } else { ink::DDD };
    let indent = row.depth as f32 * INDENT;
    let name = row.name.clone();
    let label: Box<dyn Scene> = if row.renaming {
        Box::new(rename_field(row.entity, name))
    } else {
        Box::new(bsn! {
            @text(name, 12.0, weight, name_color.with_alpha(alpha))
            Pickable::IGNORE
        })
    };

    bsn! {
        Button
        Node {
            height: px(24),
            flex_shrink: 0.0,
            margin: UiRect::left(px(indent)),
            padding: UiRect { left: px(6), right: px(8), top: px(4), bottom: px(4) },
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(4),
            border: UiRect::all(px(border_width)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG }
        BorderColor { top: border, bottom: border, left: border, right: border }
        target
        TreeRow
        on(select_row)
        on(start_row_drag)
        on(drag_row)
        on(end_row_drag)
        Children [
            Node {
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_grow: 1.0,
                min_width: px(0),
            }
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
                @{label}
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
    rows: Query<&RowEntity, With<TreeRow>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    mut state: ResMut<SceneTreeState>,
    mut selection: ResMut<Selection>,
) {
    let Ok(RowEntity(entity)) = rows.get(click.entity) else {
        return;
    };
    click.propagate(false);
    // The release that ends a drag is not a click.
    let dragging = state.drag.as_ref().is_some_and(|d| !d.entities.is_empty());
    if click.button != PointerButton::Primary || dragging {
        return;
    }
    let entity = *entity;
    let ctrl = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let state = &mut *state;

    if shift && let Some(range) = state.anchor.and_then(|a| row_range(&state.rows, a, entity)) {
        if !ctrl {
            selection.clear();
        }
        for e in range {
            selection.add(e);
        }
        // The clicked row becomes the primary selection.
        selection.add(entity);
    } else if ctrl {
        selection.toggle(entity);
        state.anchor = Some(entity);
    } else {
        selection.select(entity);
        state.anchor = Some(entity);
        let now = time.elapsed_secs_f64();
        let double = state
            .last_click
            .is_some_and(|(last, at)| last == entity && now - at < DOUBLE_CLICK_SECS);
        if double {
            state.renaming = Some(entity);
            state.last_click = None;
        } else {
            state.last_click = Some((entity, now));
        }
    }
}

/// The entities of the rows from `a` to `b` (inclusive, in either order).
fn row_range(rows: &[Row], a: Entity, b: Entity) -> Option<Vec<Entity>> {
    let a = rows.iter().position(|row| row.entity == a)?;
    let b = rows.iter().position(|row| row.entity == b)?;
    Some(
        rows[a.min(b)..=a.max(b)]
            .iter()
            .map(|row| row.entity)
            .collect(),
    )
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

/// "Add Entity": spawns an empty entity (child of the primary selection, if any) and selects it.
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
    if let Some(parent) = selection.primary().filter(|parent| scene.contains(*parent)) {
        entity.insert(ChildOf(parent));
    }
    selection.select(entity.id());
}

/// The entities of `entities` that have no ancestor in `entities`.
fn topmost(entities: &[Entity], parents: &Query<&ChildOf>) -> Vec<Entity> {
    entities
        .iter()
        .copied()
        .filter(|entity| {
            !parents
                .iter_ancestors(*entity)
                .any(|ancestor| entities.contains(&ancestor))
        })
        .collect()
}

/// Keyboard shortcuts: Delete removes the selection, F2 renames it.
#[expect(clippy::too_many_arguments, reason = "a Bevy system")]
pub(super) fn scene_tree_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    text_inputs: Query<(), With<TextInput>>,
    mut commands: Commands,
    mut state: ResMut<SceneTreeState>,
    selection: Res<Selection>,
    parents: Query<&ChildOf>,
    scene: Query<(), SceneFilter>,
) {
    // Keys typed into a text field are not shortcuts.
    if focus
        .get()
        .is_some_and(|entity| text_inputs.contains(entity))
    {
        return;
    }
    if keys.just_pressed(KeyCode::Delete) {
        // Despawning an entity takes its descendants along.
        for entity in topmost(selection.entities(), &parents) {
            if scene.contains(entity) {
                commands.entity(entity).try_despawn();
            }
        }
    }
    if keys.just_pressed(KeyCode::F2)
        && let Some(primary) = selection.primary()
        && scene.contains(primary)
    {
        state.renaming = Some(primary);
    }
}

// --- Renaming ---------------------------------------------------------------------------------

/// The text field that renames `target`; it starts out holding `initial`.
#[derive(Component, Clone)]
pub(super) struct RenameField {
    target: Entity,
    initial: String,
}

impl Default for RenameField {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            initial: String::new(),
        }
    }
}

fn rename_field(target: Entity, initial: String) -> impl Scene {
    let field = RenameField { target, initial };
    bsn! {
        @FeathersTextInputContainer
        Node {
            height: px(18),
            flex_grow: 1.0,
            min_width: px(0),
            padding: UiRect::horizontal(px(4)),
            border_radius: BorderRadius::all(px(3)),
        }
        // The theme owns the container's background, so the darker color is set as a token.
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            @FeathersTextInput
            field
            AutoFocus
            on(rename_keys)
            on(rename_focus_lost)
            on(keep_click)
            on(keep_drag_start)
        ]
    }
}

/// Fills newly spawned rename fields with the current name, all selected.
pub(super) fn init_rename_fields(
    mut fields: Query<(&RenameField, &mut EditableText), Added<RenameField>>,
) {
    for (field, mut text) in &mut fields {
        text.editor_mut().set_text(&field.initial);
        text.queue_edit(TextEdit::SelectAll);
    }
}

/// Ends a rename, applying the typed name or not.
#[derive(SystemParam)]
struct Rename<'w, 's> {
    commands: Commands<'w, 's>,
    fields: Query<'w, 's, (&'static RenameField, &'static EditableText)>,
    state: ResMut<'w, SceneTreeState>,
    selection: Res<'w, Selection>,
    focus: ResMut<'w, InputFocus>,
}

impl Rename<'_, '_> {
    fn finish(&mut self, field: Entity, apply: bool) {
        let Ok((RenameField { target, initial }, text)) = self.fields.get(field) else {
            return;
        };
        // Enter and the focus loss it causes both end up here; only the first one counts.
        if self.state.renaming != Some(*target) {
            return;
        }
        self.state.renaming = None;
        if self.focus.get() == Some(field) {
            self.focus.clear();
        }
        let typed = text.value().to_string();
        let name = typed.trim();
        if !apply || name.is_empty() || name == initial {
            return;
        }
        // Renaming one of the selected entities renames all of them.
        let targets = if self.selection.contains(*target) {
            self.selection.entities().to_vec()
        } else {
            vec![*target]
        };
        for entity in targets {
            self.commands
                .entity(entity)
                .try_insert(Name::new(name.to_owned()));
        }
    }
}

/// Enter applies the new name, Escape cancels.
fn rename_keys(mut key: On<FocusedInput<KeyboardInput>>, mut rename: Rename) {
    if !key.input.state.is_pressed() {
        return;
    }
    let apply = match key.input.logical_key {
        Key::Enter => true,
        Key::Escape => false,
        _ => return,
    };
    key.propagate(false);
    rename.finish(key.focused_entity, apply);
}

/// Clicking elsewhere applies the new name.
fn rename_focus_lost(lost: On<FocusLost>, mut rename: Rename) {
    rename.finish(lost.entity, true);
}

/// Clicks in the rename field must not reach the row (which would change the selection).
fn keep_click(mut click: On<PointerClick>) {
    click.propagate(false);
}

/// Selecting text in the rename field must not drag the row.
fn keep_drag_start(mut drag: On<PointerDragStart>) {
    drag.propagate(false);
}

// --- Drag and drop ----------------------------------------------------------------------------

/// Line (or outline) showing where dragged rows will land.
#[derive(Component, Clone, Default)]
pub(super) struct DropIndicator;

pub(super) fn setup_drop_indicator(mut commands: Commands) {
    commands.spawn((
        Name::new("Scene Tree Drop Indicator"),
        DropIndicator,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(colors::ACCENT),
        BorderColor::all(colors::ACCENT),
        GlobalZIndex(i32::MAX - 1),
        Pickable::IGNORE,
    ));
}

/// Drops a row drag whose `PointerDragEnd` never arrived (the button is already up), so the
/// tree, which doesn't rebuild during a drag, can't stay frozen.
pub(super) fn cancel_lost_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<SceneTreeState>,
    mut indicator: Single<&mut Node, With<DropIndicator>>,
) {
    if state.drag.is_some() && !mouse.pressed(MouseButton::Left) {
        state.drag = None;
        indicator.display = Display::None;
    }
}

fn start_row_drag(
    mut drag: On<PointerDragStart>,
    rows: Query<&RowEntity, With<TreeRow>>,
    mut state: ResMut<SceneTreeState>,
) {
    let Ok(RowEntity(entity)) = rows.get(drag.entity) else {
        return;
    };
    drag.propagate(false);
    // Only drags that start on the row itself move it. The chevron and eye are `Button`s, which
    // stop `PointerDragEnd` from bubbling, so a drag started on them would never end here.
    let on_row = drag.original_event_target() == drag.entity;
    if on_row && drag.button == PointerButton::Primary && state.renaming.is_none() {
        state.drag = Some(RowDrag {
            row: drag.entity,
            grabbed: *entity,
            entities: Vec::new(),
            target: None,
        });
    }
}

/// A node's rectangle in logical pixels.
fn logical_rect(node: &ComputedNode, transform: &UiGlobalTransform) -> Rect {
    let scale = node.inverse_scale_factor;
    Rect::from_center_size(transform.translation * scale, node.size * scale)
}

fn drag_row(
    mut drag: On<PointerDrag>,
    row_nodes: Query<(&RowEntity, &ComputedNode, &UiGlobalTransform), With<TreeRow>>,
    container: Single<(&ComputedNode, &UiGlobalTransform), With<SceneTreeRows>>,
    mut state: ResMut<SceneTreeState>,
    selection: Res<Selection>,
    parents: Query<&ChildOf>,
    mut indicator: Single<(&mut Node, &mut BackgroundColor), With<DropIndicator>>,
) {
    if !row_nodes.contains(drag.entity) {
        return;
    }
    drag.propagate(false);
    let state = &mut *state;
    let Some(row_drag) = state.drag.as_mut().filter(|d| d.row == drag.entity) else {
        return;
    };
    if row_drag.entities.is_empty() {
        if drag.distance.length() < DRAG_THRESHOLD {
            return;
        }
        // A selected row drags the whole selection; an unselected one drags just itself.
        let mut entities = if selection.contains(row_drag.grabbed) {
            topmost(selection.entities(), &parents)
        } else {
            vec![row_drag.grabbed]
        };
        entities.sort_by_key(|e| state.rows.iter().position(|row| row.entity == *e));
        row_drag.entities = entities;
    }

    let pointer = drag.pointer.position;
    let area = logical_rect(container.0, container.1);
    let mut rows: Vec<(Entity, Rect)> = row_nodes
        .iter()
        .map(|(RowEntity(entity), node, transform)| (*entity, logical_rect(node, transform)))
        .collect();
    rows.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));

    // The target, and the indicator: a line at `y` from `x` to the right edge, or an outline.
    let hit = if !area.contains(pointer) {
        None
    } else if let Some(&(entity, rect)) = rows
        .iter()
        .find(|(_, rect)| pointer.y < rect.max.y + ROW_GAP / 2.0)
    {
        let row = state.rows.iter().find(|row| row.entity == entity);
        let local = (pointer.y - rect.min.y) / rect.height();
        if local < 0.25 {
            Some((
                DropTarget::Before(entity),
                Indicator::Line(rect.min.x, rect.min.y - ROW_GAP / 2.0),
            ))
        } else if local > 0.75 {
            // Below an expanded parent is where its first child goes.
            if row.is_some_and(|row| row.has_children && row.expanded) {
                let target = DropTarget::Into {
                    parent: entity,
                    first: true,
                };
                Some((
                    target,
                    Indicator::Line(rect.min.x + INDENT, rect.max.y + ROW_GAP / 2.0),
                ))
            } else {
                Some((
                    DropTarget::After(entity),
                    Indicator::Line(rect.min.x, rect.max.y + ROW_GAP / 2.0),
                ))
            }
        } else {
            let target = DropTarget::Into {
                parent: entity,
                first: false,
            };
            Some((target, Indicator::Outline(rect)))
        }
    } else {
        // Empty space below the rows.
        let y = rows
            .last()
            .map_or(area.min.y, |(_, rect)| rect.max.y + ROW_GAP / 2.0);
        Some((DropTarget::RootEnd, Indicator::Line(area.min.x, y)))
    };
    let hit = hit.filter(|(target, _)| drop_allowed(*target, &row_drag.entities, &parents));

    row_drag.target = hit.map(|(target, _)| target);
    let (node, background) = &mut *indicator;
    match hit.map(|(_, indicator)| indicator) {
        None => node.display = Display::None,
        Some(Indicator::Line(x, y)) => {
            node.display = Display::Flex;
            node.left = px(x);
            node.top = px(y - 1.0);
            node.width = px((area.max.x - x).max(0.0));
            node.height = px(2);
            node.border = UiRect::ZERO;
            background.0 = colors::ACCENT;
        }
        Some(Indicator::Outline(rect)) => {
            node.display = Display::Flex;
            node.left = px(rect.min.x);
            node.top = px(rect.min.y);
            node.width = px(rect.width());
            node.height = px(rect.height());
            node.border = UiRect::all(px(2));
            background.0 = colors::ACCENT.with_alpha(0.2);
        }
    }
}

/// How the drop indicator looks.
#[derive(Clone, Copy)]
enum Indicator {
    /// A horizontal line starting at `(x, y)`.
    Line(f32, f32),
    /// An outline around a row.
    Outline(Rect),
}

/// Entities can't be dropped next to or into themselves or their descendants.
fn drop_allowed(target: DropTarget, dragged: &[Entity], parents: &Query<&ChildOf>) -> bool {
    let inside_dragged = |entity: Entity| {
        dragged.contains(&entity)
            || parents
                .iter_ancestors(entity)
                .any(|ancestor| dragged.contains(&ancestor))
    };
    match target {
        DropTarget::Before(entity) | DropTarget::After(entity) => !inside_dragged(entity),
        DropTarget::Into { parent, .. } => !inside_dragged(parent),
        DropTarget::RootEnd => true,
    }
}

fn end_row_drag(
    mut drag: On<PointerDragEnd>,
    mut commands: Commands,
    mut state: ResMut<SceneTreeState>,
    mut indicator: Single<&mut Node, With<DropIndicator>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    transforms: Query<&GlobalTransform>,
) {
    if state.drag.as_ref().is_none_or(|d| d.row != drag.entity) {
        return;
    }
    drag.propagate(false);
    indicator.display = Display::None;
    if let Some(RowDrag {
        entities,
        target: Some(target),
        ..
    }) = state.drag.take()
        && !entities.is_empty()
    {
        drop_entities(
            &mut commands,
            &mut state,
            target,
            &entities,
            &parents,
            &children,
            &transforms,
        );
    }
}

/// Moves `entities` (none an ancestor of another, in tree order) to `target`, keeping their
/// world transforms.
fn drop_entities(
    commands: &mut Commands,
    state: &mut SceneTreeState,
    target: DropTarget,
    entities: &[Entity],
    parents: &Query<&ChildOf>,
    children: &Query<&Children>,
    transforms: &Query<&GlobalTransform>,
) {
    let siblings_of = |parent: Option<Entity>| -> Vec<Entity> {
        match parent {
            Some(parent) => children.get(parent).map(|c| c.to_vec()).unwrap_or_default(),
            None => state.root_order.clone(),
        }
    };
    // The new parent (`None`: the root list), its current child list, and the insertion index.
    let (parent, siblings, index) = match target {
        DropTarget::Before(entity) | DropTarget::After(entity) => {
            let parent = parents.get(entity).ok().map(ChildOf::parent);
            let siblings = siblings_of(parent);
            let position = siblings.iter().position(|e| *e == entity);
            let index = position.map_or(siblings.len(), |p| {
                p + usize::from(matches!(target, DropTarget::After(_)))
            });
            (parent, siblings, index)
        }
        DropTarget::Into { parent, first } => {
            let siblings = siblings_of(Some(parent));
            let index = if first { 0 } else { siblings.len() };
            (Some(parent), siblings, index)
        }
        DropTarget::RootEnd => {
            let siblings = siblings_of(None);
            let index = siblings.len();
            (None, siblings, index)
        }
    };
    // Moved entities that sit before the insertion point leave a gap behind.
    let index = index
        - siblings[..index.min(siblings.len())]
            .iter()
            .filter(|e| entities.contains(e))
            .count();

    for &entity in entities {
        let old_parent = parents.get(entity).ok().map(ChildOf::parent);
        if old_parent == parent {
            continue;
        }
        if let Ok(global) = transforms.get(entity) {
            let transform = match parent.and_then(|p| transforms.get(p).ok()) {
                Some(parent_global) => global.reparented_to(parent_global),
                None => global.compute_transform(),
            };
            commands.entity(entity).insert(transform);
        }
    }

    match parent {
        Some(parent) => {
            // Show where the entities went.
            state.collapsed.remove(&parent);
            commands.entity(parent).insert_children(index, entities);
        }
        None => {
            for &entity in entities {
                commands.entity(entity).remove::<ChildOf>();
            }
            state.root_order.retain(|e| !entities.contains(e));
            let index = index.min(state.root_order.len());
            state
                .root_order
                .splice(index..index, entities.iter().copied());
        }
    }
}
