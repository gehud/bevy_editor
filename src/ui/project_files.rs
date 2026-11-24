//! The Project Files panel.
//!
//! The tree currently shows the sample content from the design; reading the project directory is
//! a next step.

use bevy::{prelude::*, ui_widgets::Button};

use crate::ui::icons;

use super::{
    style::{HoverBackground, colors, icon_sq, ink, label},
    widgets::search_field,
};

fn panel_body() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(6)),
        }
        BackgroundColor(colors::PANE_BG)
    }
}

/// Project Files panel contents: search and the file tree.
pub fn project_files_content() -> impl Scene {
    bsn! {
        @panel_body()
        Children [
            @search_field("Search...", icons::SEARCH, 15.0, ink::B1B1)
            Node { width: percent(100) }
            --
            Node { flex_direction: FlexDirection::Column, row_gap: px(3), width: percent(100) }
            Children [
                @file_row(FileRow::folder(".cargo", false))
                --
                @file_row(FileRow::folder("assets", true))
                --
                // Contents of `assets`, with a guide line along their left edge.
                Node { flex_direction: FlexDirection::Column, row_gap: px(3), padding: UiRect::left(px(14)) }
                Children [
                    @tree_guide(0.0)
                    --
                    @file_row(FileRow { highlighted: true, ..FileRow::folder("models", true) })
                    --
                    Node { flex_direction: FlexDirection::Column, row_gap: px(3), padding: UiRect::left(px(14)) }
                    Children [
                        @tree_guide(-3.0)
                        --
                        @file_row(FileRow::file(icons::BOX, "stanford-dragon.gltf"))
                        --
                        @file_row(FileRow::file(icons::BOX, "teapot.gltf"))
                    ]
                    --
                    @file_row(FileRow::file(icons::FILE_IMAGE, "sky-texture.png"))
                ]
                --
                @file_row(FileRow::folder("src", false))
                --
                @file_row(FileRow::file(icons::FILE, "Cargo.lock"))
                --
                @file_row(FileRow::file(icons::FILE, "Cargo.toml"))
            ]
        ]
    }
}

/// Vertical guide line marking the extent of an expanded folder.
///
/// It sits 1px left of the indented container, i.e. under the parent folder's chevron.
fn tree_guide(top: f32) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(-1),
            top: px(top),
            bottom: px(0),
            width: px(1),
        }
        BackgroundColor(colors::TREE_GUIDE)
    }
}

#[derive(Clone)]
struct FileRow {
    icon: &'static str,
    name: &'static str,
    highlighted: bool,
}

impl FileRow {
    fn folder(name: &'static str, expanded: bool) -> Self {
        let icon = if expanded {
            icons::CHEVRON_DOWN
        } else {
            icons::CHEVRON_RIGHT
        };
        Self {
            icon,
            name,
            highlighted: false,
        }
    }

    fn file(icon: &'static str, name: &'static str) -> Self {
        Self {
            icon,
            name,
            highlighted: false,
        }
    }
}

fn file_row(row: FileRow) -> impl Scene {
    let background = if row.highlighted {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    bsn! {
        Button
        Node {
            height: px(24),
            width: percent(100),
            padding: UiRect::axes(px(6), px(4)),
            column_gap: px(5),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG }
        Children [
            @icon_sq(row.icon, 15.0, ink::E6)
            --
            @label(row.name, ink::E6)
        ]
    }
}
