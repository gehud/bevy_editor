use bevy::{
    ecs::bundle::Bundle,
    reflect::{Reflect, prelude::ReflectDefault},
};

use crate::theme::{EditorThemeToken, ThemedTextColor, ThemedTextFont, ThemedTextSize};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorTextFont {
    Regular,
    Bold,
    Mono,
}

impl Into<EditorThemeToken> for EditorTextFont {
    fn into(self) -> EditorThemeToken {
        match self {
            EditorTextFont::Regular => "editor.text.font.regular",
            EditorTextFont::Bold => "editor.text.font.bold",
            EditorTextFont::Mono => "editor.text.font.mono",
        }
        .into()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorTextSize {
    Lower,
    Normal,
    Raised,
}

impl Into<EditorThemeToken> for EditorTextSize {
    fn into(self) -> EditorThemeToken {
        match self {
            EditorTextSize::Lower => "editor.text.size.lower",
            EditorTextSize::Normal => "editor.text.size.normal",
            EditorTextSize::Raised => "editor.text.size.raised",
        }
        .into()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorTextColor {
    Weak,
    Dimmed,
    Bright,
}

impl Into<EditorThemeToken> for EditorTextColor {
    fn into(self) -> EditorThemeToken {
        match self {
            EditorTextColor::Weak => "editor.text.color.normal",
            EditorTextColor::Dimmed => "editor.text.color.dimmed",
            EditorTextColor::Bright => "editor.text.color.bright",
        }
        .into()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorTextStyle {
    Body,
    Heading,
}

#[derive(Bundle, Clone, Debug, Reflect)]
#[reflect(Clone, Debug, Default)]
pub struct EditorText {
    theme_text_font: ThemedTextFont,
    theme_text_size: ThemedTextSize,
    theme_text_color: ThemedTextColor,
}

impl EditorText {
    pub fn body() -> Self {
        Self {
            theme_text_font: ThemedTextFont::new(EditorTextFont::Regular),
            theme_text_size: ThemedTextSize::new(EditorTextSize::Normal),
            theme_text_color: ThemedTextColor::new(EditorTextColor::Bright),
        }
    }

    pub fn heading() -> Self {
        Self {
            theme_text_font: ThemedTextFont::new(EditorTextFont::Bold),
            theme_text_size: ThemedTextSize::new(EditorTextSize::Raised),
            theme_text_color: ThemedTextColor::new(EditorTextColor::Bright),
        }
    }

    pub fn with_font(mut self, font: EditorTextFont) -> Self {
        self.theme_text_font.0 = font.into();
        self
    }

    pub fn with_size(mut self, size: EditorTextSize) -> Self {
        self.theme_text_size.0 = size.into();
        self
    }

    pub fn with_color(mut self, color: EditorTextColor) -> Self {
        self.theme_text_color.0 = color.into();
        self
    }
}

impl Default for EditorText {
    fn default() -> Self {
        Self::body()
    }
}
