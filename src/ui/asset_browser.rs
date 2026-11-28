//! The Assets pane: two views of the same project and embedded asset sources.

mod catalog;

use std::{collections::BTreeSet, path::Path};

use bevy::{
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, Button, ScrollArea},
};

use self::catalog::{Catalog, Entry, Folder, refresh_catalog};
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
    filter: String,
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
    let mut ancestor = Some(folder.clone());
    while let Some(folder) = ancestor {
        ancestor = folder.parent();
        state.expanded.insert(folder);
    }
}

fn sync_browser(
    mut state: ResMut<AssetBrowserState>,
    catalog: Res<Catalog>,
    breadcrumbs: Single<(Entity, Option<&Children>), With<Breadcrumbs>>,
    entries: Single<
        (Entity, &mut Node, &mut ScrollPosition, Option<&Children>),
        With<AssetEntries>,
    >,
    mut previous: Local<Option<(AssetView, Folder)>>,
    mut commands: Commands,
) {
    if !state.is_changed() && !catalog.is_changed() && previous.is_some() {
        return;
    }
    while let Some(parent) = state.folder.parent() {
        if catalog.0.contains_key(&state.folder) {
            break;
        }
        state.folder = parent;
    }

    let (crumbs, old_crumbs) = breadcrumbs.into_inner();
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

        for &child in old_crumbs.into_iter().flatten() {
            commands.entity(child).despawn();
        }
        let crumbs_list = breadcrumb_scenes(&state.folder);
        commands
            .entity(crumbs)
            .apply_scene(bsn! { Children [ {crumbs_list} ] });
        *previous = Some(current);
    }

    let mut scenes = Vec::new();
    match state.view {
        AssetView::Tree => tree_rows(&Folder::Sources, &state, &catalog, 0, &mut scenes),
        AssetView::ActiveFolder => folder_tiles(&state, &catalog, &mut scenes),
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
        let expanded = entry
            .folder
            .as_ref()
            .is_some_and(|folder| filtering || state.expanded.contains(folder));
        let matches = !filtering || entry.name.to_lowercase().contains(&state.filter);
        let index = scenes.len();
        scenes.push(Box::new(tree_row(entry, state, depth, expanded, matches)));
        if expanded && let Some(folder) = &entry.folder {
            tree_rows(folder, state, catalog, depth + 1, scenes);
        }
        if !matches && scenes.len() == index + 1 {
            scenes.pop();
        }
    }
}

fn tree_row(
    entry: &Entry,
    state: &AssetBrowserState,
    depth: usize,
    expanded: bool,
    matches: bool,
) -> impl Scene {
    let button = entry
        .folder
        .clone()
        .map(|folder| folder_button(folder, true));
    let background = if entry.folder.as_ref() == Some(&state.folder) {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let chevron = match &entry.folder {
        Some(_) if expanded => Some(icons::CHEVRON_DOWN),
        Some(_) => Some(icons::CHEVRON_RIGHT),
        None => None,
    }
    .map(|path| icon_sq(path, 15.0, ink::C8C8));
    let left = depth as f32 * 16.0 + if chevron.is_some() { 6.0 } else { 26.0 };
    let color = if matches { ink::E6 } else { ink::DIM };
    let name = entry.name.clone();
    let icon = asset_icon(entry, false);
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
            @{chevron}
            --
            @icon_sq(icon, 15.0, color) Pickable::IGNORE
            --
            @label(name, color) Pickable::IGNORE
        ]
    }
}

fn folder_tiles(state: &AssetBrowserState, catalog: &Catalog, scenes: &mut Vec<Box<dyn Scene>>) {
    match listing(catalog, &state.folder) {
        Ok(entries) => scenes.extend(
            entries
                .iter()
                .filter(|entry| {
                    state.filter.is_empty() || entry.name.to_lowercase().contains(&state.filter)
                })
                .map(|entry| Box::new(asset_tile(entry)) as Box<dyn Scene>),
        ),
        Err(message) => scenes.push(Box::new(label(message, ink::DIM))),
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

fn asset_tile(entry: &Entry) -> impl Scene {
    let button = entry
        .folder
        .clone()
        .map(|folder| folder_button(folder, false));
    let icon = asset_icon(entry, true);
    let name = entry.name.clone();
    bsn! {
        @{button}
        Node {
            width: px(108),
            height: px(88),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            padding: UiRect::all(px(5)),
            border_radius: BorderRadius::all(px(5)),
            overflow: Overflow::clip(),
        }
        Children [
            @icon_sq(icon, 30.0, ink::C8C8) Pickable::IGNORE
            --
            @label(name, ink::C8C8)
            TextLayout { linebreak: LineBreak::WordOrCharacter, justify: Justify::Center }
            Node { max_width: percent(100) }
            Pickable::IGNORE
        ]
    }
}

fn asset_icon(entry: &Entry, large: bool) -> &'static str {
    if entry.folder.is_some() {
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
