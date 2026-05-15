mod dark;
pub mod palette;
pub mod token;
pub mod tokens;

use bevy::{
    app::{App, Plugin},
    color::Color,
    ecs::{error::Result, resource::Resource, schedule::SystemSet},
    platform::collections::HashMap,
};

use crate::theme::token::EditorThemeToken;

#[derive(Clone, Debug, Resource)]
pub struct EditorTheme {
    pub color: HashMap<EditorThemeToken, Color>,
}

impl EditorTheme {
    pub fn color(&self, token: impl Into<EditorThemeToken>) -> Result<Color> {
        let token = token.into();
        self.color
            .get(&token)
            .cloned()
            .ok_or_else(|| format!("Theme color {} not found", token).into())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum EditorThemeSystems {
    Update,
}

pub struct EditorThemePlugin;

impl Plugin for EditorThemePlugin {
    fn build(&self, app: &mut App) {
        let theme = EditorTheme::dark(app.world_mut());
        app.insert_resource(theme);
    }
}
