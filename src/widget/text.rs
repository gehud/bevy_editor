use std::borrow::Cow;

use bevy::{
    ecs::bundle::Bundle,
    reflect::{Reflect, prelude::ReflectDefault},
    ui::widget::Text,
};

use crate::theme::{ThemeTextColor, ThemeTextFont, ThemeTextFontSize, ThemeToken};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum TextStyle {
    Body,
    Heading,
}

impl Into<ThemeToken> for TextStyle {
    fn into(self) -> ThemeToken {
        match self {
            TextStyle::Body => "editor.text.body",
            TextStyle::Heading => "editor.text.heading",
        }
        .into()
    }
}

#[derive(Bundle, Clone, Debug, Reflect)]
#[reflect(Clone, Debug, Default)]
pub struct EditorText {
    text: Text,
    theme_text_font: ThemeTextFont,
    theme_text_font_size: ThemeTextFontSize,
    theme_text_color: ThemeTextColor,
}

impl EditorText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Text::new(text),
            theme_text_font: ThemeTextFont(TextStyle::Body.into()),
            theme_text_font_size: ThemeTextFontSize(TextStyle::Body.into()),
            theme_text_color: ThemeTextColor(TextStyle::Body.into()),
        }
    }

    pub fn styled(mut self, style: TextStyle) -> Self {
        self.theme_text_font.0 = style.clone().into();
        self.theme_text_font_size.0 = style.clone().into();
        self.theme_text_color.0 = style.clone().into();
        self
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
