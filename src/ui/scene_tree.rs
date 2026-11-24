//! Left column: the Scene Tree panel and the Project Files panel.
//!
//! Both trees currently show the sample content from the design. Binding the Scene Tree to the
//! edited world and Project Files to the file system are the next steps.

use bevy::{feathers::constants::icons, prelude::*, ui_widgets::Button};

use super::{
    style::{HoverBackground, Weight, colors, icon_sq, ink, label, text},
    widgets::{PanelTab, filter_bar, panel_top_bar, search_field, wide_button},
};

/// Width of the left column.
const COLUMN_WIDTH: f32 = 266.0;

pub fn left_column() -> impl Scene {
    bsn! {
        Node {
            width: px(COLUMN_WIDTH),
            height: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
        }
        Children [
            @scene_tree_panel()
            --
            @project_files_panel()
        ]
    }
}

fn side_panel_shell() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(px(6)),
        }
        BackgroundColor(colors::PANE_BG)
    }
}

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

// ---------------------------------------------------------------------------------------------
// Scene Tree
// ---------------------------------------------------------------------------------------------

fn scene_tree_panel() -> impl Scene {
    bsn! {
        @side_panel_shell()
        Node { flex_grow: 1.0, min_height: px(0) }
        Children [
            @panel_top_bar(
                vec![
                    PanelTab { title: "Scene Tree", active: true },
                    PanelTab { title: "Import", active: false },
                ],
                colors::PANEL_BORDER,
            )
            --
            @panel_body()
            Children [
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: percent(100) }
                Children [
                    @filter_bar()
                    --
                    @wide_button(icons::PACKAGE_PLUS, "Add Entity")
                ]
                --
                Node { flex_direction: FlexDirection::Column, row_gap: px(3), width: percent(100) }
                Children [
                    @entity_row(EntityRow {
                        expander: Expander::Collapsed,
                        icon: icons::VIDEO,
                        icon_color: ink::DDD,
                        name: "Camera 3D",
                        visible: true,
                        ..default()
                    })
                    --
                    @entity_row(EntityRow {
                        expander: Expander::Leaf,
                        icon: icons::LIGHTBULB,
                        icon_color: ink::E6,
                        name: "Directional Light",
                        visible: false,
                        ..default()
                    })
                    Node { margin: UiRect::bottom(px(3)) }
                    --
                    @entity_row(EntityRow {
                        expander: Expander::Expanded,
                        icon: icons::BOX,
                        icon_color: ink::E6,
                        name: "Stanford Dragon",
                        visible: true,
                        highlighted: true,
                        ..default()
                    })
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(2),
                        padding: UiRect { left: px(8), top: px(2) },
                        width: percent(100),
                    }
                    Children [
                        @entity_row(EntityRow { name: "Head", ..EntityRow::child() })
                        --
                        @entity_row(EntityRow { name: "Body", selected: true, ..EntityRow::child() })
                        --
                        @entity_row(EntityRow { name: "Tail", ..EntityRow::child() })
                    ]
                ]
            ]
        ]
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Expander {
    #[default]
    Leaf,
    Collapsed,
    Expanded,
}

#[derive(Clone)]
struct EntityRow {
    expander: Expander,
    icon: &'static str,
    icon_color: Color,
    name: &'static str,
    visible: bool,
    /// Expanded parent rows keep a background.
    highlighted: bool,
    selected: bool,
}

impl Default for EntityRow {
    fn default() -> Self {
        Self {
            expander: Expander::Leaf,
            icon: icons::BOX,
            icon_color: ink::E6,
            name: "",
            visible: true,
            highlighted: false,
            selected: false,
        }
    }
}

impl EntityRow {
    fn child() -> Self {
        Self {
            expander: Expander::Collapsed,
            ..default()
        }
    }
}

fn entity_row(row: EntityRow) -> impl Scene {
    let (expander_icon, expander_color) = match row.expander {
        Expander::Leaf => (icons::DOT, ink::DDD),
        Expander::Collapsed => (icons::CHEVRON_RIGHT, ink::E6),
        Expander::Expanded => (icons::CHEVRON_DOWN, ink::E6),
    };
    let (visibility_icon, visibility_color) = if row.visible {
        (icons::EYE, ink::E6)
    } else {
        (icons::EYE_OFF, ink::DDD)
    };
    let background = if row.highlighted || row.selected {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let border = if row.selected {
        colors::ACCENT
    } else {
        Color::NONE
    };
    let border_width = if row.selected { 2.0 } else { 0.0 };
    let weight = if row.selected {
        Weight::Medium
    } else {
        Weight::Regular
    };
    // Hidden entities are drawn faded.
    let alpha = if row.visible { 1.0 } else { 0.6 };

    bsn! {
        Button
        Node {
            height: px(24),
            width: percent(100),
            padding: UiRect { left: px(6), right: px(8), top: px(4), bottom: px(4) },
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect::all(px(border_width)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG }
        BorderColor { top: border, bottom: border, left: border, right: border }
        Children [
            Node { column_gap: px(4), align_items: AlignItems::Center }
            Children [
                @icon_sq(expander_icon, 15.0, expander_color.with_alpha(alpha))
                --
                Node { column_gap: px(4), align_items: AlignItems::Center }
                Children [
                    @icon_sq(row.icon, 15.0, row.icon_color.with_alpha(alpha))
                    --
                    @text(row.name, 12.0, weight, ink::DDD.with_alpha(alpha))
                ]
            ]
            --
            @icon_sq(visibility_icon, 15.0, visibility_color.with_alpha(alpha))
        ]
    }
}

// ---------------------------------------------------------------------------------------------
// Project Files
// ---------------------------------------------------------------------------------------------

fn project_files_panel() -> impl Scene {
    bsn! {
        @side_panel_shell()
        Node { height: px(442), flex_shrink: 0.0 }
        Children [
            @panel_top_bar(vec![PanelTab { title: "Project Files", active: true }], colors::PANEL_BORDER)
            --
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
