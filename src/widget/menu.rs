use bevy::{app::{App, Plugin}, ecs::component::Component};

use crate::widget::ContextMenu;

#[derive(Component)]
pub struct MenuButton(pub ContextMenu);

pub struct MenuBarPlugin;

impl Plugin for MenuBarPlugin {
    fn build(&self, app: &mut App) {

    }
}
