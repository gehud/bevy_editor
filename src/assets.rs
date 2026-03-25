use bevy::{
    app::{App, Plugin},
    asset::embedded_asset,
};

pub struct EditorAssetsPlugin;

impl Plugin for EditorAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fonts/FiraMono-Medium.ttf");
        embedded_asset!(app, "fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "fonts/FiraSans-BoldItalic.ttf");
        embedded_asset!(app, "fonts/FiraSans-Italic.ttf");
        embedded_asset!(app, "fonts/FiraSans-Regular.ttf");

        embedded_asset!(app, "icons/bevy.png");
        embedded_asset!(app, "icons/check.png");
        embedded_asset!(app, "icons/chevron_left.png");
        embedded_asset!(app, "icons/chevron_right.png");
        embedded_asset!(app, "icons/close.png");
        embedded_asset!(app, "icons/file.png");
        embedded_asset!(app, "icons/folder.png");
        embedded_asset!(app, "icons/maximize.png");
        embedded_asset!(app, "icons/minimize.png");
        embedded_asset!(app, "icons/pause.png");
        embedded_asset!(app, "icons/play.png");
        embedded_asset!(app, "icons/restore.png");
        embedded_asset!(app, "icons/stop.png");

        embedded_asset!(app, "shaders/alpha_pattern.wgsl");
        embedded_asset!(app, "shaders/color_plane.wgsl");
    }
}
