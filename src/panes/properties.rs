use bevy::{
    app::{App, Plugin},
    color::Color,
    ecs::system::{Commands, In},
    ui::{BackgroundColor, Node, Overflow, percent, px},
    utils::default,
};

use crate::pane::{PaneApp, PaneStructure};

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Properties", setup);
    }
}

fn setup(In(pane): In<PaneStructure>, mut commands: Commands) {

}
