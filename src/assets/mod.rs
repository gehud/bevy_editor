use bevy::{
    app::{App, Plugin},
    asset::embedded_asset,
};

pub const FONT_BOLD: &'static str = "embedded://bevy_editor/fonts/Fira_Sans/FiraSans-Bold.ttf";
pub const FONT_REGULAR: &'static str = "embedded://bevy_editor/fonts/Fira_Sans/FiraSans-Regular.ttf";

pub struct EditorAssetsPlugin;

impl Plugin for EditorAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "src/assets", "fonts/Fira_Sans/FiraSans-Bold.ttf");
        embedded_asset!(app, "src/assets", "fonts/Fira_Sans/FiraSans-Regular.ttf");

        embedded_asset!(app, "src/assets", "icons/branding/bevy.png");
    }
}
