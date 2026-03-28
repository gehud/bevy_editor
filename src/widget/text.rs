use std::borrow::Cow;

use bevy::{
    ecs::bundle::Bundle,
    reflect::{Reflect, prelude::ReflectDefault},
    ui::widget::Text,
};

use crate::theme::{EditorThemeToken, ThemedTextColor, ThemedTextFont, ThemedTextSize};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorTextFont {
    Regular,
    Bold,
    Mono
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
    text: Text,
    theme_text_font: ThemedTextFont,
    theme_text_size: ThemedTextSize,
    theme_text_color: ThemedTextColor,
}

impl EditorText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Text::new(text),
            theme_text_font: ThemedTextFont::new(EditorTextFont::Regular),
            theme_text_size: ThemedTextSize::new(EditorTextSize::Normal),
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

    pub fn styled(self, style: EditorTextStyle) -> Self {
        match style {
            EditorTextStyle::Body => self
                .with_font(EditorTextFont::Regular)
                .with_size(EditorTextSize::Normal)
                .with_color(EditorTextColor::Bright),
            EditorTextStyle::Heading => self
                .with_font(EditorTextFont::Bold)
                .with_size(EditorTextSize::Raised)
                .with_color(EditorTextColor::Bright),
        }
    }

    pub fn body(self) -> Self {
        self.styled(EditorTextStyle::Body)
    }

    pub fn heading(self) -> Self {
        self.styled(EditorTextStyle::Heading)
    }
}

impl Default for EditorText {
    fn default() -> Self {
        Self::new(String::default())
    }
}

impl From<Text> for EditorText {
    fn from(text: Text) -> Self {
        Self::new(text.0)
    }
}

impl From<&str> for EditorText {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<&String> for EditorText {
    fn from(text: &String) -> Self {
        Self::new(text)
    }
}

impl From<&mut String> for EditorText {
    fn from(text: &mut String) -> Self {
        Self::new(text.clone())
    }
}

impl From<String> for EditorText {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&Box<str>> for EditorText {
    fn from(text: &Box<str>) -> Self {
        Self::new(text.clone())
    }
}

impl From<&mut Box<str>> for EditorText {
    fn from(text: &mut Box<str>) -> Self {
        Self::new(text.clone())
    }
}

impl From<Box<str>> for EditorText {
    fn from(text: Box<str>) -> Self {
        Self::new(text)
    }
}

impl From<Cow<'_, str>> for EditorText {
    fn from(text: Cow<'_, str>) -> Self {
        Self::new(text)
    }
}
