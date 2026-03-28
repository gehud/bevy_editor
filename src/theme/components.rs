use bevy::{
    ecs::{component::Component, reflect::ReflectComponent},
    reflect::Reflect,
    text::{TextColor, TextFont},
    ui::{BackgroundColor, BorderColor, widget::ImageNode},
};

use crate::theme::EditorThemeToken;

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(BackgroundColor)]
pub struct ThemedBackgroundColor(pub EditorThemeToken);

impl ThemedBackgroundColor {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(BorderColor)]
pub struct ThemedBorderColor {
    pub top: EditorThemeToken,
    pub right: EditorThemeToken,
    pub bottom: EditorThemeToken,
    pub left: EditorThemeToken,
}

impl ThemedBorderColor {
    pub fn all(token: impl Into<EditorThemeToken>) -> Self {
        let token = token.into();
        Self {
            top: token.clone(),
            bottom: token.clone(),
            left: token.clone(),
            right: token.clone(),
        }
    }
}

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(TextColor)]
pub struct ThemedTextColor(pub EditorThemeToken);

impl ThemedTextColor {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(ImageNode)]
pub struct ThemedImageColor(pub EditorThemeToken);

impl ThemedImageColor {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(TextFont)]
pub struct ThemedTextFont(pub EditorThemeToken);

impl ThemedTextFont {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}

#[derive(Clone, Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Clone, Component, Debug, PartialEq)]
#[require(TextFont)]
pub struct ThemedTextSize(pub EditorThemeToken);

impl ThemedTextSize {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}
