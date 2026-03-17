pub mod constants;
mod dark_theme;
mod handle_or_path;
pub mod palette;
mod rounded_corners;
pub mod tokens;

pub use dark_theme::*;
pub use handle_or_path::*;
pub use rounded_corners::*;

use bevy::{
    app::{App, Plugin, PostUpdate},
    asset::{AssetServer, Handle, embedded_asset},
    color::{Color, palettes},
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        lifecycle::Insert,
        observer::On,
        query::Changed,
        reflect::{ReflectComponent, ReflectResource},
        resource::Resource,
        system::{Query, Res},
    },
    log::warn_once,
    platform::collections::HashMap,
    reflect::{Reflect, prelude::ReflectDefault},
    text::{Font, TextColor, TextFont},
    ui::{BackgroundColor, BorderColor, widget::ImageNode},
};
use smol_str::SmolStr;

/// A design token for the theme. This serves as the lookup key for the theme properties.
#[derive(Clone, PartialEq, Eq, Hash, Reflect)]
pub struct ThemeToken(SmolStr);

impl ThemeToken {
    /// Construct a new [`ThemeToken`] from a [`SmolStr`].
    pub const fn new(text: SmolStr) -> Self {
        Self(text)
    }

    /// Construct a new [`ThemeToken`] from a static string.
    pub const fn new_static(text: &'static str) -> Self {
        Self(SmolStr::new_static(text))
    }
}

impl core::fmt::Display for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ThemeToken({:?})", self.0)
    }
}

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<ThemeToken, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color(&self, token: &ThemeToken) -> Color {
        let color = self.0.color.get(token);
        match color {
            Some(c) => *c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given color.
    pub fn set_color(&mut self, token: &str, color: Color) {
        self.0
            .color
            .insert(ThemeToken::new(SmolStr::new(token)), color);
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
#[derive(Component, Clone)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

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

#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
#[require(TextFont)]
pub struct ThemeTextFont {
    /// The font handle or path.
    pub font: HandleOrPath<Font>,
    /// The desired font size.
    pub font_size: f32,
}

impl ThemeTextFont {
    pub fn from_handle(handle: Handle<Font>) -> Self {
        Self {
            font: HandleOrPath::Handle(handle),
            font_size: 16.0,
        }
    }

    pub fn from_path(path: &str) -> Self {
        Self {
            font: HandleOrPath::Path(path.to_string()),
            font_size: 16.0,
        }
    }
}

fn update_theme(
    mut q_background: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor)>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, theme_bg) in q_background.iter_mut() {
            bg.0 = theme.color(&theme_bg.0);
        }

        // Update all border colors
        for (mut border, theme_border) in q_border.iter_mut() {
            border.set_all(theme.color(&theme_border.0));
        }
    }
}

fn on_changed_background(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut text_colors: Query<(&mut TextColor, &ThemeTextColor), Changed<ThemeTextColor>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut text_color, theme_text_color)) = text_colors.get_mut(insert.entity) {
        text_color.0 = theme.color(&theme_text_color.0);
    }
}

fn on_changed_image_color(
    insert: On<Insert, ThemeImageColor>,
    mut image_color: Query<(&mut ImageNode, &ThemeImageColor), Changed<ThemeImageColor>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut image_node, image_color)) = image_color.get_mut(insert.entity) {
        image_node.color = theme.color(&image_color.0);
    }
}

fn on_changed_text_font(
    insert: On<Insert, ThemeTextFont>,
    mut text_fonts: Query<(&mut TextFont, &ThemeTextFont), Changed<ThemeTextFont>>,
    assets: Res<AssetServer>,
) {
    if let Ok((mut text_font, theme_text_font)) = text_fonts.get_mut(insert.entity) {
        text_font.font_size = theme_text_font.font_size;
        text_font.font = match &theme_text_font.font {
            HandleOrPath::Handle(handle) => handle.clone(),
            HandleOrPath::Path(path) => assets.load::<Font>(path),
        };
    }
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
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

        app.init_resource::<UiTheme>()
            .add_systems(PostUpdate, update_theme)
            .add_observer(on_changed_image_color)
            .add_observer(on_changed_background)
            .add_observer(on_changed_border)
            .add_observer(on_changed_text_color)
            .add_observer(on_changed_text_font);
    }
}
