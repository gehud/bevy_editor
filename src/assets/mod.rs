pub mod icons;

use bevy::app::{App, Plugin};

pub const LUCIDE_FONT_FAMILY: &'static str = "lucide_regular";
pub const ICON_TEXT_STYLE: &'static str = "icon";

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, _app: &mut App) {}
}
