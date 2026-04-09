use std::env;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, Startup},
    camera::Camera2d,
    ecs::{error::Result, system::Commands},
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

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

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins)
            .add_plugins(EguiPlugin::default())
            .add_systems(Startup, setup)
            .add_systems(EguiPrimaryContextPass, ui);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn ui(mut contexts: EguiContexts) -> Result {
    egui::Window::new("Hello").show(contexts.ctx_mut()?, |ui| {
        ui.label("world");
    });

    Ok(())
}
