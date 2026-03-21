pub mod constants;
mod dark_theme;
pub mod palette;
mod rounded_corners;
pub mod tokens;

pub use rounded_corners::*;

use std::fmt::{self, Debug, Display, Formatter};

use bevy::{
    app::{App, Plugin, PostUpdate},
    asset::{Handle, embedded_asset},
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        error::Result,
        lifecycle::Insert,
        observer::On,
        query::{Changed, Or, With},
        reflect::{ReflectComponent, ReflectResource},
        resource::Resource,
        system::{Query, Res},
        world::FromWorld,
    },
    platform::collections::HashMap,
    reflect::Reflect,
    text::{Font, TextColor, TextFont},
    ui::{BackgroundColor, BorderColor, widget::ImageNode},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct ThemeToken(&'static str);

impl ThemeToken {
    pub const fn new(token: &'static str) -> Self {
        Self(token)
    }
}

impl Display for ThemeToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for ThemeToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ThemeToken({:?})", self.0)
    }
}

#[derive(Clone, Reflect, Debug)]
#[reflect(Debug)]
pub struct ThemeProps {
    pub color: HashMap<ThemeToken, Color>,
    pub fonts: HashMap<ThemeToken, Handle<Font>>,
    pub font_sizes: HashMap<ThemeToken, f32>,
}

#[derive(Resource, FromWorld, Reflect, Debug)]
#[reflect(Resource, Debug)]
pub struct Theme(pub ThemeProps);

impl Theme {
    pub fn color(&self, token: &ThemeToken) -> Result<Color> {
        self.0
            .color
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme color {} not found", token).into())
    }

    pub fn font(&self, token: &ThemeToken) -> Result<Handle<Font>> {
        self.0
            .fonts
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme font {} not found", token).into())
    }

    pub fn font_size(&self, token: &ThemeToken) -> Result<f32> {
        self.0
            .font_sizes
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme font size {} not found", token).into())
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, PartialEq, Eq)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor {
    pub top: ThemeToken,
    pub right: ThemeToken,
    pub bottom: ThemeToken,
    pub left: ThemeToken,
}

impl<T: Into<ThemeToken>> From<T> for ThemeBorderColor {
    fn from(token: T) -> Self {
        Self::all(token.into())
    }
}

impl ThemeBorderColor {
    pub fn all(token: impl Into<ThemeToken>) -> Self {
        let token = token.into();
        Self {
            top: token.clone(),
            bottom: token.clone(),
            left: token.clone(),
            right: token.clone(),
        }
    }
}

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeTextColor(pub ThemeToken);

#[derive(Component, Clone, Reflect)]
#[component(immutable)]
#[reflect(Component, Clone)]
#[require(ImageNode)]
pub struct ThemeImageColor(pub ThemeToken);

#[derive(Component, Clone)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(TextFont)]
pub struct ThemeTextFont(pub ThemeToken);

#[derive(Component, Clone)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(TextFont)]
pub struct ThemeTextFontSize(pub ThemeToken);

fn update_theme(
    theme: Res<Theme>,
    background_colors: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    border_colors: Query<(&mut BorderColor, &ThemeBorderColor)>,
    text_colors: Query<(&mut TextColor, &ThemeTextColor)>,
    text_fonts: Query<(Entity, &mut TextFont), Or<(With<ThemeTextFont>, With<ThemeTextFontSize>)>>,
    theme_text_fonts: Query<&ThemeTextFont>,
    theme_text_font_sizes: Query<&ThemeTextFontSize>,
    images: Query<(&mut ImageNode, &ThemeImageColor)>,
) -> Result {
    if theme.is_changed() {
        for (mut background_color, themed) in background_colors {
            background_color.0 = theme.color(&themed.0)?;
        }

        for (mut border_color, themed) in border_colors {
            border_color.top = theme.color(&themed.top)?;
            border_color.right = theme.color(&themed.right)?;
            border_color.bottom = theme.color(&themed.bottom)?;
            border_color.left = theme.color(&themed.left)?;
        }

        for (mut text_color, themed) in text_colors {
            text_color.0 = theme.color(&themed.0)?;
        }

        for (entity, mut text_font) in text_fonts {
            if let Ok(theme_text_font) = theme_text_fonts.get(entity) {
                text_font.font = theme.font(&theme_text_font.0)?;
            }

            if let Ok(theme_text_font_size) = theme_text_font_sizes.get(entity) {
                text_font.font_size = theme.font_size(&theme_text_font_size.0)?;
            }
        }

        for (mut image, themed) in images {
            image.color = theme.color(&themed.0)?;
        }
    }

    Ok(())
}

fn on_changed_background_color(
    insert: On<Insert, ThemeBackgroundColor>,
    mut background_colors: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<Theme>,
) -> Result {
    let (mut background_color, themed) = background_colors.get_mut(insert.entity)?;
    background_color.0 = theme.color(&themed.0)?;
    Ok(())
}

fn on_changed_border_color(
    insert: On<Insert, ThemeBorderColor>,
    mut border_colors: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<Theme>,
) -> Result {
    let (mut border_color, themed) = border_colors.get_mut(insert.entity)?;
    border_color.top = theme.color(&themed.top)?;
    border_color.right = theme.color(&themed.right)?;
    border_color.bottom = theme.color(&themed.bottom)?;
    border_color.left = theme.color(&themed.left)?;
    Ok(())
}

fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut text_colors: Query<(&mut TextColor, &ThemeTextColor), Changed<ThemeTextColor>>,
    theme: Res<Theme>,
) -> Result {
    let (mut text_color, themed) = text_colors.get_mut(insert.entity)?;
    text_color.0 = theme.color(&themed.0)?;
    Ok(())
}

fn on_changed_text_font(
    insert: On<Insert, ThemeTextFont>,
    mut text_fonts: Query<(&mut TextFont, &ThemeTextFont), Changed<ThemeTextFont>>,
    theme: Res<Theme>,
) -> Result {
    let (mut text_font, themed) = text_fonts.get_mut(insert.entity)?;
    text_font.font = theme.font(&themed.0)?;
    Ok(())
}

fn on_changed_text_font_size(
    insert: On<Insert, ThemeTextFont>,
    mut text_fonts: Query<(&mut TextFont, &ThemeTextFont), Changed<ThemeTextFont>>,
    theme: Res<Theme>,
) -> Result {
    let (mut text_font, themed) = text_fonts.get_mut(insert.entity)?;
    text_font.font_size = theme.font_size(&themed.0)?;
    Ok(())
}

fn on_changed_image_color(
    insert: On<Insert, ThemeImageColor>,
    mut image_color: Query<(&mut ImageNode, &ThemeImageColor), Changed<ThemeImageColor>>,
    theme: Res<Theme>,
) -> Result {
    let (mut image_node, image_color) = image_color.get_mut(insert.entity)?;
    image_node.color = theme.color(&image_color.0)?;
    Ok(())
}

pub struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        // Embedded font
        embedded_asset!(app, "src/theme", "assets/theme/fonts/FiraSans-Bold.ttf");
        embedded_asset!(
            app,
            "src/theme",
            "assets/theme/fonts/FiraSans-BoldItalic.ttf"
        );
        embedded_asset!(app, "src/theme", "assets/theme/fonts/FiraSans-Regular.ttf");
        embedded_asset!(app, "src/theme", "assets/theme/fonts/FiraSans-Italic.ttf");
        embedded_asset!(app, "src/theme", "assets/theme/fonts/FiraMono-Medium.ttf");

        embedded_asset!(app, "src/theme", "assets/theme/icons/bevy.png");
        embedded_asset!(app, "src/theme", "assets/theme/icons/pause.png");
        embedded_asset!(app, "src/theme", "assets/theme/icons/play.png");
        embedded_asset!(app, "src/theme", "assets/theme/icons/stop.png");

        app.init_resource::<Theme>()
            .add_systems(PostUpdate, update_theme)
            .add_observer(on_changed_background_color)
            .add_observer(on_changed_border_color)
            .add_observer(on_changed_text_color)
            .add_observer(on_changed_text_font)
            .add_observer(on_changed_text_font_size)
            .add_observer(on_changed_image_color);
    }
}
