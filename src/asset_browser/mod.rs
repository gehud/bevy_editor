use bevy::{
    app::{App, Plugin},
    ecs::{error::Result, world::World},
};
use egui::{Margin, Ui};

use crate::pane::{Pane, RegisterPane};

pub struct AssetBrowser;

impl Pane for AssetBrowser {
    fn name(&self) -> &str {
        "Asset Browser"
    }

    fn padding(&self) -> Option<Margin> {
        Some(Margin::ZERO)
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        Ok(())
    }
}

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(AssetBrowser);
    }
}
