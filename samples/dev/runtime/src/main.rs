use bevy::{
    DefaultPlugins,
    app::{App, AppExit, Plugin},
    prelude::bevy_main,
};
use shared::SharedPlugin;

pub struct RuntimePlugin;

impl Plugin for RuntimePlugin {
    fn build(&self, _app: &mut App) {}
}

#[bevy_main]
fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SharedPlugin)
        .add_plugins(RuntimePlugin)
        .run()
}
