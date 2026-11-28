//! The Assets pane: two views of the same project and embedded asset sources.

mod catalog;
mod inspection;

use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};

use bevy::{
    asset::AssetPath,
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, Button, ScrollArea},
};

use self::catalog::{Catalog, Entry, EntryKind, Folder, refresh_catalog};
pub(crate) use self::inspection::register_asset_inspection_source;
use self::inspection::{AssetDetails, DetailsLoader, Inspections};
use super::{
    icons,
    style::{HoverBackground, colors, icon_sq, ink, label},
    widgets::search_field,
};

pub(super) struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetBrowserState>()
            .init_resource::<Catalog>()
            .init_asset::<AssetDetails>()
            .init_asset_loader::<DetailsLoader>()
            .add_systems(Update, (refresh_catalog, read_filter, sync_browser).chain());
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum AssetView {
    Tree,
    #[default]
    ActiveFolder,
}

#[derive(Default, Resource)]
pub(super) struct AssetBrowserState {
    pub(super) view: AssetView,
    folder: Folder,
    expanded: BTreeSet<Folder>,
    expanded_files: HashSet<AssetPath<'static>>,
    filter: String,
}

impl AssetBrowserState {
    fn expand_path(&mut self, folder: Folder) {
        let mut ancestor = Some(folder);
        while let Some(folder) = ancestor {
            ancestor = folder.parent();
            self.expanded.insert(folder);
        }
    }
}

#[derive(Component, Clone, Default)]
struct AssetFilter;

#[derive(Component, Clone, Default)]
struct Breadcrumbs;

#[derive(Component, Clone, Default)]
struct AssetEntries;

#[derive(Component, Clone)]
struct OpenFolder {
    folder: Folder,
    toggle: bool,
}

#[derive(Component, Clone)]
struct ToggleFile(AssetPath<'static>);

pub(super) fn asset_browser_content() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            min_width: px(0),
            flex_direction: FlexDirection::Column,
        }
        Children [
            Node {
                height: px(34),
                flex_shrink: 0.0,
                padding: UiRect::axes(px(6), px(4)),
                align_items: AlignItems::Center,
                column_gap: px(6),
            }
            Children [
                Breadcrumbs
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                }
                --
                @search_field("Search...", icons::SEARCH, 15.0, ink::B1B1)
                AssetFilter
                Node { width: percent(35), max_width: px(200), min_width: px(72) }
            ]
            --
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::all(px(6)),
                overflow: Overflow::scroll_y(),
            }
            ScrollArea
            AssetEntries
        ]
    }
}

fn read_filter(
    inputs: Query<(Entity, &EditableText), Changed<EditableText>>,
    parents: Query<&ChildOf>,
    filters: Query<(), With<AssetFilter>>,
    mut state: ResMut<AssetBrowserState>,
) {
    for (input, text) in &inputs {
        if parents
            .iter_ancestors(input)
            .any(|parent| filters.contains(parent))
        {
            let filter = text.value().to_string().trim().to_lowercase();
            if state.filter != filter {
                state.filter = filter;
            }
        }
    }
}

fn open_folder(
    event: On<Activate>,
    actions: Query<&OpenFolder>,
    mut state: ResMut<AssetBrowserState>,
) {
    let Ok(OpenFolder { folder, toggle }) = actions.get(event.entity) else {
        return;
    };
    state.folder = folder.clone();
    if *toggle && state.expanded.remove(folder) {
        return;
    }
    state.expand_path(folder.clone());
}

fn toggle_file(
    event: On<Activate>,
    files: Query<&ToggleFile>,
    mut state: ResMut<AssetBrowserState>,
) {
    let Ok(ToggleFile(path)) = files.get(event.entity) else {
        return;
    };
    if !state.expanded_files.remove(path) {
        state.expanded_files.insert(path.clone());
    }
}

fn sync_browser(
    mut state: ResMut<AssetBrowserState>,
    catalog: Res<Catalog>,
    mut inspections: Inspections,
    breadcrumbs: Single<(Entity, Option<&Children>), With<Breadcrumbs>>,
    entries: Single<
        (Entity, &mut Node, &mut ScrollPosition, Option<&Children>),
        With<AssetEntries>,
    >,
    mut previous: Local<Option<(AssetView, Folder)>>,
    mut commands: Commands,
) {
    if !state.is_changed()
        && !catalog.is_changed()
        && !inspections.is_changed()
        && previous.is_some()
    {
        return;
    }
    if catalog.is_changed() {
        inspections.retain_files(&catalog);
        state
            .expanded_files
            .retain(|path| catalog.contains_file(path));
    }
    while let Some(parent) = state.folder.parent() {
        if catalog.0.contains_key(&state.folder) {
            break;
        }
        state.folder = parent;
    }

    let (body, mut node, mut scroll, old_entries) = entries.into_inner();
    let current = (state.view, state.folder.clone());
    if previous.as_ref() != Some(&current) {
        if previous
            .as_ref()
            .is_none_or(|(view, _)| *view != state.view)
            || state.view == AssetView::ActiveFolder
        {
            scroll.0 = Vec2::ZERO;
        }
        node.flex_direction = match state.view {
            AssetView::Tree => FlexDirection::Column,
            AssetView::ActiveFolder => FlexDirection::Row,
        };
        node.flex_wrap = match state.view {
            AssetView::Tree => FlexWrap::NoWrap,
            AssetView::ActiveFolder => FlexWrap::Wrap,
        };
        node.align_content = AlignContent::FlexStart;
        node.row_gap = px(3);
        node.column_gap = px(3);

        let (crumbs, old_crumbs) = breadcrumbs.into_inner();
        for &child in old_crumbs.into_iter().flatten() {
            commands.entity(child).despawn();
        }
        if state.view == AssetView::ActiveFolder {
            let crumbs_list = breadcrumb_scenes(&state.folder);
            commands
                .entity(crumbs)
                .apply_scene(bsn! { Children [ {crumbs_list} ] });
        }
        *previous = Some(current);
    }

    let mut scenes = Vec::new();
    match state.view {
        AssetView::Tree => tree_rows(
            &Folder::Sources,
            &state,
            &catalog,
            &mut inspections,
            0,
            &mut scenes,
        ),
        AssetView::ActiveFolder => folder_tiles(&state, &catalog, &mut inspections, &mut scenes),
    }
    if scenes.is_empty() {
        let message = if state.filter.is_empty() {
            "This folder is empty"
        } else {
            "No matching assets"
        };
        scenes.push(Box::new(label(message, ink::DIM)));
    }
    for &child in old_entries.into_iter().flatten() {
        commands.entity(child).despawn();
    }
    commands
        .entity(body)
        .apply_scene(bsn! { Children [ {scenes} ] });
}

fn breadcrumb_scenes(path_end: &Folder) -> Vec<Box<dyn Scene>> {
    let mut path = vec![path_end.clone()];
    while let Some(parent) = path.last().and_then(Folder::parent) {
        path.push(parent);
    }
    let mut scenes: Vec<Box<dyn Scene>> = Vec::new();
    for (index, folder) in path.into_iter().rev().enumerate() {
        if index > 0 {
            scenes.push(Box::new(label(" / ", ink::DIM)));
        }
        let name = folder.name();
        let color = if &folder == path_end {
            ink::EBEB
        } else {
            ink::C8C8
        };
        scenes.push(Box::new(bsn! {
            @folder_button(folder, false)
            Node {
                height: px(24),
                padding: UiRect::horizontal(px(4)),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4)),
            }
            Children [ @label(name, color) Pickable::IGNORE ]
        }));
    }
    scenes
}

fn listing<'a>(catalog: &'a Catalog, folder: &Folder) -> Result<&'a [Entry], &'static str> {
    match catalog.0.get(folder) {
        Some(Ok(entries)) => Ok(entries),
        Some(Err(message)) => Err(message),
        None => Err("Loading assets…"),
    }
}

fn tree_rows(
    folder: &Folder,
    state: &AssetBrowserState,
    catalog: &Catalog,
    inspections: &mut Inspections,
    depth: usize,
    scenes: &mut Vec<Box<dyn Scene>>,
) {
    let filtering = !state.filter.is_empty();
    let entries = match listing(catalog, folder) {
        Ok(entries) => entries,
        Err(message) => {
            if !filtering {
                let indent = depth as f32 * 16.0 + 6.0;
                scenes.push(Box::new(bsn! {
                    @label(message, ink::DIM)
                    Node { padding: UiRect::left(px(indent)), flex_shrink: 0.0 }
                }));
            }
            return;
        }
    };
    for entry in entries {
        let folder = entry.folder();
        let matches = !filtering || entry.name.to_lowercase().contains(&state.filter);
        if !matches && folder.is_none() {
            continue;
        }
        let index = scenes.len();
        let active = folder == Some(&state.folder);
        let file = entry.asset_path();
        let details = file.and_then(|path| inspections.get(path));
        let expanded = if let Some(folder) = folder {
            Some(filtering || state.expanded.contains(folder))
        } else {
            details
                .filter(|details| !details.labels.is_empty())
                .zip(file)
                .map(|(_, path)| state.expanded_files.contains(path))
        };
        let root_type = details.map(|details| details.root_type);
        scenes.push(Box::new(tree_row(
            entry, active, depth, expanded, matches, root_type,
        )));
        if expanded == Some(true) {
            if let Some(folder) = folder {
                tree_rows(folder, state, catalog, inspections, depth + 1, scenes);
            } else if let Some(details) = details {
                for (name, kind) in &details.labels {
                    scenes.push(Box::new(label_row(depth + 1, format!("#{name}"), kind)));
                }
            }
        }
        if !matches && scenes.len() == index + 1 {
            scenes.pop();
        }
    }
}

fn tree_row(
    entry: &Entry,
    active: bool,
    depth: usize,
    expanded: Option<bool>,
    matches: bool,
    root_type: Option<&'static str>,
) -> impl Scene {
    let button = entry_button(entry, AssetView::Tree, expanded.is_some());
    let background = if active { colors::ITEM_BG } else { Color::NONE };
    let chevron = if expanded == Some(true) {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };
    let left = depth as f32 * 16.0 + 6.0;
    let color = if matches { ink::E6 } else { ink::DIM };
    let name = entry.name.clone();
    let icon = asset_icon(entry, false);
    let chevron_visibility = if expanded.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let type_label = root_type.map(|kind| bsn! { @label(kind, ink::DIM) Pickable::IGNORE });
    bsn! {
        @{button}
        Node {
            height: px(24),
            flex_shrink: 0.0,
            padding: UiRect { left: px(left), right: px(6) },
            column_gap: px(5),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(5)),
            overflow: Overflow::clip(),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG_ACTIVE }
        Children [
            @icon_sq(chevron, 15.0, ink::C8C8) chevron_visibility Pickable::IGNORE
            --
            @icon_sq(icon, 15.0, color) Pickable::IGNORE
            --
            @label(name, color) Pickable::IGNORE
            Node { flex_grow: 1.0, min_width: px(0), overflow: Overflow::clip() }
            --
            @{type_label}
        ]
    }
}

fn label_row(depth: usize, name: String, kind: &'static str) -> impl Scene {
    let left = depth as f32 * 16.0 + 46.0;
    bsn! {
        Node {
            min_height: px(24),
            flex_shrink: 0.0,
            padding: UiRect { left: px(left), right: px(6) },
            align_items: AlignItems::Center,
            column_gap: px(5),
        }
        Pickable::IGNORE
        Children [
            @label(name, ink::E6)
            TextLayout { linebreak: LineBreak::WordOrCharacter }
            Node { flex_grow: 1.0, min_width: px(0) }
            Pickable::IGNORE
            --
            @label(kind, ink::DIM) Pickable::IGNORE
        ]
    }
}

fn folder_tiles(
    state: &AssetBrowserState,
    catalog: &Catalog,
    inspections: &mut Inspections,
    scenes: &mut Vec<Box<dyn Scene>>,
) {
    let entries = match listing(catalog, &state.folder) {
        Ok(entries) => entries,
        Err(message) => {
            scenes.push(Box::new(label(message, ink::DIM)));
            return;
        }
    };
    for entry in entries {
        if !state.filter.is_empty() && !entry.name.to_lowercase().contains(&state.filter) {
            continue;
        }
        let file = entry.asset_path();
        let details = file
            .and_then(|path| inspections.get(path))
            .filter(|details| !details.labels.is_empty());
        let expanded = details
            .zip(file)
            .map(|(_, path)| state.expanded_files.contains(path));
        let button = entry_button(entry, AssetView::ActiveFolder, expanded.is_some());
        let root = tile(
            entry.name.clone(),
            asset_icon(entry, true),
            button,
            expanded,
            None,
        );
        if let Some(details) = details.filter(|_| expanded == Some(true)) {
            let children: Vec<Box<dyn Scene>> = details
                .labels
                .iter()
                .map(|(name, kind)| {
                    let child = tile(
                        format!("#{name}"),
                        labeled_asset_icon(kind),
                        None,
                        None,
                        Some(kind),
                    );
                    Box::new(child) as Box<dyn Scene>
                })
                .collect();
            scenes.push(Box::new(expanded_tiles(root, children)));
        } else {
            scenes.push(Box::new(root));
        }
    }
}

fn folder_button(folder: Folder, toggle: bool) -> impl Scene {
    let action = OpenFolder { folder, toggle };
    bsn! {
        Button
        TabIndex(0)
        action
        BackgroundColor(Color::NONE)
        HoverBackground { normal: Color::NONE, hover: colors::ITEM_BG }
        on(open_folder)
    }
}

fn entry_button(entry: &Entry, view: AssetView, expandable: bool) -> Option<Box<dyn Scene>> {
    match &entry.kind {
        EntryKind::Folder(folder) => Some(Box::new(folder_button(
            folder.clone(),
            view == AssetView::Tree,
        ))),
        EntryKind::File(path) if expandable => {
            let action = ToggleFile(path.clone());
            Some(Box::new(bsn! {
                Button
                TabIndex(0)
                action
                on(toggle_file)
            }))
        }
        EntryKind::File(_) => None,
    }
}

fn expanded_tiles(root: impl Scene, children: Vec<Box<dyn Scene>>) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_shrink: 0.0,
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(3),
            column_gap: px(3),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::WINDOW_BG)
        Children [
            @{root}
            --
            {children}
        ]
    }
}

fn tile(
    name: String,
    icon: &'static str,
    button: Option<Box<dyn Scene>>,
    expanded: Option<bool>,
    kind: Option<&'static str>,
) -> impl Scene {
    let background = if expanded == Some(true) {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let hover = button.as_ref().map(|_| {
        bsn! { HoverBackground { normal: background, hover: colors::ITEM_BG_ACTIVE } }
    });
    let chevron = expanded.map(|expanded| {
        let icon = if expanded {
            icons::CHEVRON_DOWN
        } else {
            icons::CHEVRON_RIGHT
        };
        bsn! {
            @icon_sq(icon, 15.0, ink::C8C8)
            Node { position_type: PositionType::Absolute, top: px(13), right: px(5) }
            Pickable::IGNORE
        }
    });
    let type_label = kind.map(|kind| bsn! { @label(kind, ink::DIM) Pickable::IGNORE });
    bsn! {
        @{button}
        @{hover}
        Node {
            width: px(108),
            max_width: percent(100),
            height: px(88),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(3),
            padding: UiRect::all(px(5)),
            border_radius: BorderRadius::all(px(5)),
            overflow: Overflow::clip(),
        }
        BackgroundColor(background)
        Children [
            @icon_sq(icon, 30.0, ink::C8C8) Pickable::IGNORE
            --
            @label(name, ink::C8C8)
            TextLayout { linebreak: LineBreak::WordOrCharacter, justify: Justify::Center }
            Node { max_width: percent(100), overflow: Overflow::clip() }
            Pickable::IGNORE
            --
            @{type_label}
            --
            @{chevron}
        ]
    }
}

fn labeled_asset_icon(kind: &str) -> &'static str {
    match kind {
        "WorldAsset" => icons::SQUARES_OVERLAPPING,
        "Mesh" | "GltfMesh" => icons::BOX,
        "Image" => icons::FILE_IMAGE,
        "StandardMaterial" | "GltfMaterial" => icons::CIRCLE_FILLED,
        _ => icons::FILE,
    }
}

fn asset_icon(entry: &Entry, large: bool) -> &'static str {
    if entry.folder().is_some() {
        return icons::FOLDER_FILLED;
    }
    match Path::new(&entry.name)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("gltf" | "glb") if large => icons::BOX_LARGE,
        Some("gltf" | "glb") => icons::BOX,
        Some("png" | "jpg" | "jpeg" | "svg" | "webp" | "ktx2") => icons::FILE_IMAGE,
        _ => icons::FILE,
    }
}
