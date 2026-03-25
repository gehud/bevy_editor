use bevy::{
    app::{App, Plugin},
    ecs::system::In,
};

use crate::pane::{Pane, PaneApp};

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane("Properties", setup);
    }
}

fn setup(In(pane_structure): In<Pane>) {}
