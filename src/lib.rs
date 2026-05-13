use std::env;

use bevy::app::{App, PluginGroup, PluginGroupBuilder, Plugins};

pub const PLAY_MODE_VAR: &'static str = "BEVY_EDITOR_PLAY";

pub fn is_play_mode() -> bool {
    let Ok(var) = env::var(PLAY_MODE_VAR) else {
        return false;
    };

    let Ok(value) = var.parse::<bool>() else {
        return false;
    };

    value
}

pub trait EditorApp {
    fn add_editor_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self;

    fn add_runtime_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self;
}

impl EditorApp for App {
    fn add_editor_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        if !is_play_mode() {
            self.add_plugins(plugins)
        } else {
            self
        }
    }

    fn add_runtime_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        if is_play_mode() {
            self.add_plugins(plugins)
        } else {
            self
        }
    }
}

pub struct EditorPlugins;

impl PluginGroup for EditorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<EditorPlugins>().build()
    }
}
