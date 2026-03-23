use bevy::{
    app::{App, Plugin},
    ecs::system::In,
};

use crate::pane::{PaneStructure, RegisterPane};

pub struct PropertiesPanePlugin;

impl Plugin for PropertiesPanePlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Properties", setup);
    }
}

fn setup(In(pane_structure): In<PaneStructure>) {}
