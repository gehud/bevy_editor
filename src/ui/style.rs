//! Colors, fonts and small building blocks shared by every editor panel.
//!
//! Values come from the "Bevy Editor" Figma design. Colors that already exist in the
//! `bevy_feathers` palette are taken from there so the editor and the widgets stay in sync.

use bevy::{
    feathers::{constants::fonts, palette},
    picking::hover::Hovered,
    prelude::*,
    text::{FontSourceTemplate, FontWeight},
};

/// Builds an opaque color from a `0xRRGGBB` literal, so values can be copied from the design.
pub const fn hex(rgb: u32) -> Color {
    Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Builds a translucent color from a `0xRRGGBB` literal and an alpha in `0..=255`.
pub const fn hexa(rgb: u32, alpha: u8) -> Color {
    Color::srgba_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, alpha)
}

/// Surface and border colors.
pub mod colors {
    use super::*;

    /// Window background, panel top bars, viewport toolbars.
    pub const WINDOW_BG: Color = palette::GRAY_0; // #1F1F24
    /// Panel bodies, active panel tab.
    pub const PANE_BG: Color = palette::GRAY_1; // #2A2A2E
    /// Inputs, buttons, hovered / expanded rows.
    pub const ITEM_BG: Color = palette::GRAY_2; // #36373B
    /// Active document tab, axis label boxes.
    pub const ITEM_BG_ACTIVE: Color = palette::GRAY_3; // #46474D
    /// Active toolbar toggle.
    pub const TOGGLE_ACTIVE_BG: Color = hex(0x505050);
    /// Selection outline, active tab marker.
    pub const ACCENT: Color = palette::ACCENT; // #206EC9
    /// Selected bottom-bar button indicator.
    pub const SELECTOR: Color = hex(0x0F6CBD);
    /// Marker of the "Schedule Explorer" document tab.
    pub const TAB_MARKER_YELLOW: Color = hex(0xFFCA39);

    /// Outer window border.
    pub const WINDOW_BORDER: Color = hex(0x404040);
    /// Border of the left panels and viewport toolbars.
    pub const PANEL_BORDER: Color = hex(0x303030);
    /// Border of the right panel and the bottom bar.
    pub const PANEL_BORDER_ALT: Color = hex(0x373737);
    /// Inputs and component cards.
    pub const ITEM_BORDER: Color = palette::WARM_GRAY_1; // #414142
    /// Tree guide lines.
    pub const TREE_GUIDE: Color = hex(0x414141);
    /// Thin separators inside toolbars.
    pub const DIVIDER: Color = hexa(0xFFFFFF, 26);
    /// Border of the active document tab.
    pub const TAB_BORDER: Color = hexa(0xFFFFFF, 13);
    /// Border of the number fields.
    pub const FIELD_BORDER: Color = hexa(0x000000, 20);

    /// Caption button hover.
    pub const CAPTION_HOVER: Color = hexa(0xFFFFFF, 15);
    /// Close caption button hover (Windows standard).
    pub const CAPTION_CLOSE_HOVER: Color = hex(0xC42B1C);
}

/// Text and icon colors. Each one is used exactly where the design uses it.
pub mod ink {
    use super::*;

    pub const E6: Color = hex(0xE6E6E6);
    pub const ECEC: Color = hex(0xECECEC);
    pub const DDD: Color = hex(0xDDDDDD);
    pub const DADA: Color = hex(0xDADADA);
    pub const DBDB: Color = hex(0xDBDBDB);
    pub const DCDC: Color = hex(0xDCDCDC);
    pub const D1D1: Color = hex(0xD1D1D1);
    pub const C8C8: Color = hex(0xC8C8C8);
    pub const C4C4: Color = hex(0xC4C4C4);
    pub const C2C2: Color = hex(0xC2C2C2);
    pub const B9B9: Color = hex(0xB9B9B9);
    pub const B3B3: Color = hex(0xB3B3B3);
    pub const B1B1: Color = hex(0xB1B1B2);
    pub const AFAF: Color = hex(0xAFAFAF);
    pub const A8A8: Color = hex(0xA8A8A8);
    pub const A4A4: Color = hex(0xA4A4A6);
    pub const DIM: Color = palette::LIGHT_GRAY_2; // #838385
    pub const F2F2: Color = hex(0xF2F2F2);
    pub const EBEB: Color = hex(0xEBEBEB);
    pub const D9D9: Color = hex(0xD9D9D9);
    pub const TAB_CORNER: Color = hex(0x2A2A2A);
    pub const SHADOW: Color = hexa(0x000000, 128);
}

/// Fira Sans weights used by the design.
#[derive(Clone, Copy)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

impl Weight {
    fn font(self) -> &'static str {
        match self {
            Weight::Regular => fonts::REGULAR,
            Weight::Medium => fonts::MEDIUM,
            Weight::SemiBold => fonts::SEMIBOLD,
            Weight::Bold => fonts::BOLD,
        }
    }

    fn font_weight(self) -> FontWeight {
        match self {
            Weight::Regular => FontWeight::NORMAL,
            Weight::Medium => FontWeight::MEDIUM,
            Weight::SemiBold => FontWeight::SEMIBOLD,
            Weight::Bold => FontWeight::BOLD,
        }
    }
}

/// A single-style text entity.
pub fn text(value: impl Into<String>, size: f32, weight: Weight, color: Color) -> impl Scene {
    let font = weight.font();
    let font_weight = weight.font_weight();
    bsn! {
        Text(value)
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: FontSize::Px(size),
            weight: font_weight,
        }
        TextColor(color)
        TextLayout {
            linebreak: LineBreak::NoWrap,
        }
    }
}

/// 12px regular text: the default for labels throughout the editor.
pub fn label(value: impl Into<String>, color: Color) -> impl Scene {
    text(value, 12.0, Weight::Regular, color)
}

/// An icon image of a fixed size, tinted with `color`.
///
/// Icons are white masks (see `tools/rasterize_svg`), so the tint is the icon's color.
pub fn icon(path: &'static str, width: f32, height: f32, color: Color) -> impl Scene {
    bsn! {
        Node {
            width: px(width),
            height: px(height),
            flex_shrink: 0.0,
        }
        ImageNode {
            image: path,
            color: color,
        }
    }
}

/// A square icon.
pub fn icon_sq(path: &'static str, size: f32, color: Color) -> impl Scene {
    icon(path, size, size, color)
}

/// Changes an entity's background while the pointer is over it.
#[derive(Component, Clone, Default)]
#[require(Hovered)]
pub struct HoverBackground {
    pub normal: Color,
    pub hover: Color,
}

pub(super) fn update_hover_backgrounds(
    mut q: Query<(&Hovered, &HoverBackground, &mut BackgroundColor), Changed<Hovered>>,
) {
    for (hovered, colors, mut background) in &mut q {
        background.0 = if hovered.0 {
            colors.hover
        } else {
            colors.normal
        };
    }
}
