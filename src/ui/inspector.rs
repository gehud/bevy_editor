//! Right panel: Components / Resources / Systems, showing the selected entity's components.
//!
//! The cards reproduce the design's sample (a Transform and a reflected component). The number
//! fields are live `bevy_feathers` number inputs; wiring them to the selected entity through
//! reflection is the next step.

use bevy::{
    feathers::{
        constants::icons,
        controls::{FeathersNumberInput, NumberInputValue},
        palette, tokens as feathers_tokens,
    },
    prelude::*,
    ui_widgets::Button,
};

use super::{
    style::{Weight, colors, icon_sq, ink, label, text},
    widgets::{PanelTab, filter_bar, panel_top_bar, wide_button},
};

pub fn inspector_panel() -> impl Scene {
    bsn! {
        Node {
            width: px(310),
            height: percent(100),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border_radius: BorderRadius::all(px(6)),
        }
        BackgroundColor(colors::PANE_BG)
        Children [
            @panel_top_bar(
                vec![
                    PanelTab { title: "Components", active: true },
                    PanelTab { title: "Resources", active: false },
                    PanelTab { title: "Systems", active: false },
                ],
                colors::PANEL_BORDER_ALT,
            )
            --
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(6)),
            }
            Children [
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: percent(100) }
                Children [
                    @filter_bar()
                    --
                    @wide_button(icons::PACKAGE_PLUS, "Add Component")
                ]
                --
                Node { flex_direction: FlexDirection::Column, row_gap: px(6), width: percent(100) }
                Children [
                    @transform_card()
                    --
                    @reflected_card()
                ]
            ]
        ]
    }
}

/// A collapsible component card: header with title, body with fields.
fn component_card(title: &'static str) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor(colors::PANE_BG)
        BorderColor { top: colors::ITEM_BORDER, bottom: colors::ITEM_BORDER, left: colors::ITEM_BORDER, right: colors::ITEM_BORDER }
        BoxShadow(vec![ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.88),
            x_offset: px(0),
            y_offset: px(0),
            spread_radius: px(0),
            blur_radius: px(1),
        }])
        Children [
            Node {
                height: px(26),
                width: percent(100),
                padding: UiRect::horizontal(px(8)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border_radius: BorderRadius::top(px(5)),
            }
            BackgroundColor(colors::ITEM_BG)
            Children [
                @icon_sq(icons::CHEVRON_DOWN, 15.0, ink::C4C4)
                --
                Node { column_gap: px(5), align_items: AlignItems::Center }
                Children [
                    @icon_sq(icons::MOVE_3D_SMALL, 13.0, ink::C4C4)
                    --
                    @text(title, 12.0, Weight::Medium, ink::DCDC)
                    TextShadow { offset: Vec2::new(0.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.2) }
                ]
                --
                @icon_sq(icons::ELLIPSIS, 15.0, ink::C4C4)
            ]
        ]
    }
}

fn card_fields() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect { left: px(7), right: px(7), top: px(5), bottom: px(7) },
        }
    }
}

/// Small caption above a field.
fn field_caption(caption: &'static str) -> impl Scene {
    bsn! {
        Node { padding: UiRect::vertical(px(3)) }
        Children [
            @text(caption, 10.0, Weight::Regular, ink::D1D1.with_alpha(0.8))
            TextShadow { offset: Vec2::new(0.0, 1.0), color: ink::SHADOW }
        ]
    }
}

fn transform_card() -> impl Scene {
    bsn! {
        @component_card("Transform")
        Children [
            @card_fields()
            Children [
                @vec3_field("Translation", Vec3::new(42.0, -42.0, 25.0))
                --
                @vec3_field("Rotation", Vec3::new(-1.5, 0.0, 1000.0))
                --
                @vec3_field("Scale", Vec3::new(0.0, 0.0, 0.0))
            ]
        ]
    }
}

fn vec3_field(caption: &'static str, value: Vec3) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(2), width: percent(100) }
        Children [
            @field_caption(caption)
            --
            Node { width: percent(100), column_gap: px(8), padding: UiRect::left(px(1)) }
            Children [
                @axis_input("X", feathers_tokens::TEXT_INPUT_X_AXIS, palette::X_AXIS, value.x)
                --
                @axis_input("Y", feathers_tokens::TEXT_INPUT_Y_AXIS, palette::Y_AXIS, value.y)
                --
                @axis_input("Z", feathers_tokens::TEXT_INPUT_Z_AXIS, palette::Z_AXIS, value.z)
            ]
        ]
    }
}

fn axis_input(
    axis: &'static str,
    sigil: bevy::feathers::theme::ThemeToken,
    sigil_color: Color,
    value: f32,
) -> impl Scene {
    bsn! {
        @FeathersNumberInput {
            @sigil_color: sigil,
            @label_text: axis,
        }
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            height: px(22),
            border: UiRect::left(px(2)),
        }
        BorderColor::all(sigil_color)
        NumberInputValue::F32(value)
    }
}

fn reflected_card() -> impl Scene {
    bsn! {
        @component_card("Reflectable Component")
        Children [
            @card_fields()
            Children [
                Node { flex_direction: FlexDirection::Column, row_gap: px(2), width: percent(100) }
                Children [
                    @field_caption("First Property")
                    --
                    @FeathersNumberInput
                    Node { width: percent(100), height: px(22) }
                    NumberInputValue::F32(42.0)
                ]
                --
                Node { flex_direction: FlexDirection::Column, row_gap: px(2), width: percent(100) }
                Children [
                    @field_caption("Second Property")
                    --
                    @file_field()
                ]
            ]
        ]
    }
}

/// A file-path property: an "Open File" button next to the current file name.
fn file_field() -> impl Scene {
    bsn! {
        Node {
            height: px(22),
            width: percent(100),
            overflow: Overflow::clip(),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            align_items: AlignItems::Center,
        }
        BackgroundColor(colors::ITEM_BG)
        BorderColor { top: colors::FIELD_BORDER, bottom: colors::FIELD_BORDER, left: colors::FIELD_BORDER, right: colors::FIELD_BORDER }
        Children [
            Button
            Node {
                width: px(94),
                height: percent(100),
                padding: UiRect::axes(px(15), px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::left(px(2.5)),
            }
            BackgroundColor(colors::ITEM_BG_ACTIVE)
            Children [ @label("Open File", ink::DCDC) ]
            --
            Node { flex_grow: 1.0, justify_content: JustifyContent::Center }
            Children [ @label("No File", ink::C2C2) ]
        ]
    }
}
