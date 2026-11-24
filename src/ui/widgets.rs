//! Pieces shared by several panels: panel tab bars, search fields, wide buttons.

use bevy::{
    feathers::{
        constants::icons,
        controls::{FeathersTextInput, FeathersTextInputContainer, text_input_placeholder},
        theme::InheritableThemeTextColor,
    },
    prelude::*,
    ui_widgets::Button,
};

use super::{
    style::{HoverBackground, Weight, colors, icon_sq, ink, label, text},
    tokens,
};

/// One tab in a panel's top bar.
pub struct PanelTab {
    pub title: &'static str,
    pub active: bool,
}

/// Top bar of a side panel: tabs, an "add tab" button and a drag grip.
pub fn panel_top_bar(tabs: Vec<PanelTab>, border: Color) -> impl Scene {
    let tabs = tabs
        .into_iter()
        .map(|tab| -> Box<dyn SceneList> {
            if tab.active {
                Box::new(bsn_list! { @active_panel_tab(tab.title) })
            } else {
                Box::new(bsn_list! { @inactive_panel_tab(tab.title) })
            }
        })
        .collect::<Vec<_>>();

    bsn! {
        Node {
            height: px(30),
            width: percent(100),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect { left: px(6), right: px(8), top: px(1) },
            border: UiRect { left: px(1), right: px(1), top: px(1) },
            border_radius: BorderRadius::top(px(6)),
        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor { top: border, left: border, right: border }
        Children [
            Node {
                height: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(5),
            }
            Children [
                {tabs}
                --
                @icon_sq(icons::PLUS, 18.0, ink::A8A8)
            ]
            --
            @grip()
        ]
    }
}

fn active_panel_tab(title: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: percent(100),
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
            // Concave corners that blend the tab into the bar below it.
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
    icon_sq(icons::TAB_CORNER, 4.0, ink::TAB_CORNER)
}

fn inactive_panel_tab(title: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(8)),
        }
        Children [ @label(title, ink::A8A8) ]
    }
}

/// The vertical grip used to drag panels around.
pub fn grip() -> impl Scene {
    bsn! {
        Node {
            width: px(12),
            padding: UiRect::axes(px(1), px(3)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
        }
        Children [ @icon_sq(icons::GRIP_VERTICAL, 12.0, ink::B3B3) ]
    }
}

/// An editable search / filter field with a placeholder and a trailing icon.
pub fn search_field(
    placeholder: &'static str,
    icon_path: &'static str,
    icon_size: f32,
    icon_color: Color,
) -> impl Scene {
    bsn! {
        @FeathersTextInputContainer
        Node {
            height: px(24),
            // The feathers container grows by default; that would stretch it in column layouts.
            flex_grow: 0.0,
            padding: UiRect { left: px(8), right: px(8) },
            border_radius: BorderRadius::all(px(4)),
            justify_content: JustifyContent::SpaceBetween,
        }
        BackgroundColor(colors::ITEM_BG)
        Children [
            @FeathersTextInput
            --
            @text_input_placeholder(placeholder)
            Node { left: px(8) }
            InheritableThemeTextColor(tokens::PLACEHOLDER)
            --
            @icon_sq(icon_path, icon_size, icon_color)
        ]
    }
}

/// Filter field with a funnel icon plus a trailing "more" button (Scene Tree, Components).
pub fn filter_bar() -> impl Scene {
    bsn! {
        Node {
            height: px(24),
            width: percent(100),
            column_gap: px(3),
            align_items: AlignItems::Center,
        }
        Children [
            @search_field("Filter...", icons::FUNNEL, 24.0, ink::B1B1)
            Node { flex_grow: 1.0, padding: UiRect { left: px(8), right: px(2) } }
            --
            Node { padding: UiRect::horizontal(px(4)) }
            Children [ @icon_sq(icons::ELLIPSIS, 15.0, ink::B1B1) ]
        ]
    }
}

/// Full-width button with an icon and a medium-weight caption ("Add Entity", "Add Component").
pub fn wide_button(icon_path: &'static str, caption: &'static str) -> impl Scene {
    bsn! {
        Button
        Node {
            height: px(24),
            width: percent(100),
            padding: UiRect::axes(px(10), px(5)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(5),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::ITEM_BG)
        HoverBackground { normal: colors::ITEM_BG, hover: colors::ITEM_BG_ACTIVE }
        Children [
            @icon_sq(icon_path, 15.0, ink::E6)
            --
            @text(caption, 12.0, Weight::Medium, ink::E6)
        ]
    }
}

/// A 1px vertical separator spanning its parent's height.
pub fn vertical_divider() -> impl Scene {
    bsn! {
        Node { width: px(1), align_self: AlignSelf::Stretch, flex_shrink: 0.0 }
        BackgroundColor(colors::DIVIDER)
    }
}
