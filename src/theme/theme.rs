use bevy::{
    asset::Handle,
    color::Color,
    ecs::{error::Result, reflect::ReflectResource, resource::Resource},
    platform::collections::HashMap,
    reflect::Reflect,
    text::Font,
};

use crate::theme::EditorThemeToken;

#[derive(Clone, Debug, Reflect, Resource)]
#[reflect(Clone, Debug, Resource)]
pub struct EditorTheme {
    pub color: HashMap<EditorThemeToken, Color>,
    pub fonts: HashMap<EditorThemeToken, Handle<Font>>,
    pub font_sizes: HashMap<EditorThemeToken, f32>,
}

impl EditorTheme {
    pub fn color(&self, token: &EditorThemeToken) -> Result<Color> {
        self.color
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme color {} not found", token).into())
    }

    pub fn font(&self, token: &EditorThemeToken) -> Result<Handle<Font>> {
        self.fonts
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme font {} not found", token).into())
    }

    pub fn font_size(&self, token: &EditorThemeToken) -> Result<f32> {
        self.font_sizes
            .get(token)
            .cloned()
            .ok_or_else(|| format!("Theme font size {} not found", token).into())
    }
}
