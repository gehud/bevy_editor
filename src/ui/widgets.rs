//! Pieces shared by several panels: search fields, wide buttons, dividers.

use bevy::{
    feathers::{
        controls::{FeathersTextInput, FeathersTextInputContainer, text_input_placeholder},
        theme::InheritableThemeTextColor,
    },
    prelude::*,
    ui_widgets::Button,
};

use crate::ui::icons;

use super::{
    style::{HoverBackground, Weight, colors, icon_sq, ink, text},
    tokens,
};

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
