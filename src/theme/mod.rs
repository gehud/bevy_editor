mod dark;
pub mod palette;
mod rounded_corners;
pub mod token;
pub mod tokens;

pub use rounded_corners::*;

use bevy::{
    app::{App, Plugin, PostUpdate},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        error::Result,
        reflect::ReflectComponent,
        resource::Resource,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Query, Res},
        world::Ref,
    },
    platform::collections::HashMap,
    reflect::Reflect,
    text::{Font, TextColor, TextFont},
    ui::{BackgroundColor, BorderColor, widget::ImageNode},
};

use crate::theme::token::EditorThemeToken;

#[derive(Clone, Debug, Resource)]
pub struct EditorTheme {
    pub colors: HashMap<EditorThemeToken, Color>,
    pub fonts: HashMap<EditorThemeToken, TextFont>,
}

impl EditorTheme {
    pub fn color(&self, token: impl Into<EditorThemeToken>) -> Result<Color> {
        let token = token.into();
        self.colors
            .get(&token)
            .cloned()
            .ok_or_else(|| format!("Theme color {} not found", token).into())
    }

    pub fn font(&self, token: impl Into<EditorThemeToken>) -> Result<TextFont> {
        let token = token.into();
        self.fonts
            .get(&token)
            .cloned()
            .ok_or_else(|| format!("Theme color {} not found", token).into())
    }
}

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
#[require(ImageNode)]
pub struct ThemedImageColor(pub EditorThemeToken);

impl ThemedImageColor {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
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
#[require(TextFont)]
pub struct ThemedTextFont(pub EditorThemeToken);

impl ThemedTextFont {
    pub fn new(token: impl Into<EditorThemeToken>) -> Self {
        Self(token.into())
    }
}

pub fn update_theme(
    theme: Res<EditorTheme>,
    themed_background_colors: Query<(&mut BackgroundColor, Ref<ThemedBackgroundColor>)>,
    themed_border_colors: Query<(&mut BorderColor, Ref<ThemedBorderColor>)>,
    themed_image_colors: Query<(&mut ImageNode, Ref<ThemedImageColor>)>,
    themed_text_colors: Query<(&mut TextColor, Ref<ThemedTextColor>)>,
    themed_text_fonts: Query<(&mut TextFont, Ref<ThemedTextFont>)>,
) -> Result {
    for (mut background_color, themed) in themed_background_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        background_color.0 = theme.color(themed.0)?;
    }

    for (mut border_color, themed) in themed_border_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        border_color.top = theme.color(themed.top)?;
        border_color.right = theme.color(themed.right)?;
        border_color.bottom = theme.color(themed.bottom)?;
        border_color.left = theme.color(themed.left)?;
    }

    for (mut image, themed) in themed_image_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        image.color = theme.color(themed.0)?;
    }

    for (mut text_color, themed) in themed_text_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        text_color.0 = theme.color(themed.0)?;
    }

    for (mut text_font, themed) in themed_text_fonts {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        *text_font = theme.font(themed.0)?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum EditorThemeSystems {
    Update,
}

pub struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        let theme = EditorTheme::dark(app.world_mut());
        app.insert_resource(theme)
            .add_systems(PostUpdate, update_theme.in_set(EditorThemeSystems::Update));
    }
}
