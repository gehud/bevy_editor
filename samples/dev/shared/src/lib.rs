use bevy::app::{App, Plugin};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, _app: &mut App) {}
}
