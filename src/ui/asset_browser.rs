//! The Assets panel: a browser for the project's asset folder.
//!
//! The browser shows the design's sample folder; reading the real `assets/` directory is
//! a follow-up.

use bevy::{prelude::*, ui_widgets::Button};

use crate::ui::icons;

use super::{
    style::{HoverBackground, Weight, colors, icon_sq, ink, text},
    widgets::search_field,
};

/// Assets panel contents: breadcrumb, search and the current folder's contents.
pub fn asset_browser_content() -> impl Scene {
    let entries = [
        ("animation", false),
        ("audio", false),
        ("materials", false),
        ("models", false),
        ("scenes", false),
    ]
    .into_iter()
    .map(|(name, _)| bsn! { @asset_tile(icons::FOLDER_FILLED, name, false) })
    .chain(std::iter::once(
        bsn! { @asset_tile(icons::BOX_LARGE, "Stanford...", true) },
    ))
    .collect::<Vec<_>>();

    bsn! {
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            height: percent(100),
            flex_direction: FlexDirection::Column,
        }
        Children [
            // Header: breadcrumb and search.
            Node {
                height: px(34),
                width: percent(100),
                flex_shrink: 0.0,
                padding: UiRect::axes(px(5), px(4)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                Node { padding: UiRect::horizontal(px(9)), align_items: AlignItems::Center }
                Children [
                    @text("project / assets / ", 12.0, Weight::Regular, ink::C8C8)
                    --
                    @text("Stanford Dragon.gltf", 12.0, Weight::SemiBold, ink::C8C8)
                ]
                --
                @search_field("Search...", icons::SEARCH, 15.0, ink::B1B1)
                Node { width: px(200), flex_grow: 0.0 }
            ]
            --
            // Folder contents.
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                padding: UiRect::axes(px(12), px(8)),
                border: UiRect::top(px(1)),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
            }
            BorderColor { top: colors::PANE_BG }
            Children [ {entries} ]
        ]
    }
}

fn asset_tile(icon_path: &'static str, name: &'static str, selected: bool) -> impl Scene {
    let background = if selected {
        colors::ITEM_BG
    } else {
        Color::NONE
    };
    let text_color = if selected { ink::EBEB } else { ink::C8C8 };
    bsn! {
        Button
        Node {
            width: px(74),
            height: px(80),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(10),
            padding: UiRect { left: px(5), right: px(5), top: px(3) },
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(background)
        HoverBackground { normal: background, hover: colors::ITEM_BG }
        Children [
            @icon_sq(icon_path, 30.0, ink::C8C8)
            --
            @text(name, 12.0, Weight::Regular, text_color)
        ]
    }
}
