mod components;
pub mod constants;
mod dark_theme;
pub mod palette;
mod rounded_corners;
mod theme;
mod token;
pub mod tokens;

pub use components::*;
pub use rounded_corners::*;
pub use theme::*;
pub use token::*;

use std::fmt::Debug;

use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{
        change_detection::DetectChanges,
        entity::Entity,
        error::Result,
        query::{Or, With},
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Query, Res},
        world::Ref,
    },
    reflect::Reflect,
    text::{TextColor, TextFont},
    ui::{BackgroundColor, BorderColor, UiSystems, widget::ImageNode},
};

fn update_theme(
    theme: Res<EditorTheme>,
    themed_background_colors: Query<(&mut BackgroundColor, Ref<ThemedBackgroundColor>)>,
    themed_border_colors: Query<(&mut BorderColor, Ref<ThemedBorderColor>)>,
    themed_text_colors: Query<(&mut TextColor, Ref<ThemedTextColor>)>,
    text_fonts: Query<
        (Entity, &mut TextFont),
        Or<(With<ThemedTextFont>, With<ThemedTextSize>)>,
    >,
    themed_text_fonts: Query<Ref<ThemedTextFont>>,
    themed_text_sizes: Query<Ref<ThemedTextSize>>,
    themed_image_colors: Query<(&mut ImageNode, Ref<ThemedImageColor>)>,
) -> Result {
    for (mut background_color, themed) in themed_background_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        background_color.0 = theme.color(&themed.0)?;
    }

    for (mut border_color, themed) in themed_border_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        border_color.top = theme.color(&themed.top)?;
        border_color.right = theme.color(&themed.right)?;
        border_color.bottom = theme.color(&themed.bottom)?;
        border_color.left = theme.color(&themed.left)?;
    }

    for (mut text_color, themed) in themed_text_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        text_color.0 = theme.color(&themed.0)?;
    }

    for (entity, mut text_font) in text_fonts {
        if let Ok(themed) = themed_text_fonts.get(entity) {
            if theme.is_changed() || themed.is_changed() {
                text_font.font = theme.font(&themed.0)?;
            }
        }

        if let Ok(themed) = themed_text_sizes.get(entity) {
            if theme.is_changed() || themed.is_changed() {
                text_font.font_size = theme.font_size(&themed.0)?;
            }
        }
    }

    for (mut image, themed) in themed_image_colors {
        if !(theme.is_changed() || themed.is_changed()) {
            continue;
        }

        image.color = theme.color(&themed.0)?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect, SystemSet)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub enum EditorThemeSystems {
    Update,
}

pub struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorTheme>()
            .configure_sets(
                PostUpdate,
                EditorThemeSystems::Update.in_set(UiSystems::Prepare),
            )
            .add_systems(PostUpdate, update_theme.in_set(EditorThemeSystems::Update));
    }
}
