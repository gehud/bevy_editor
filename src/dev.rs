//! Development helpers.

use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

/// Set this environment variable to a `.png` path to have the editor save a screenshot of its
/// window shortly after startup and then exit. Used to compare the UI against the design.
pub const SCREENSHOT_ENV: &str = "BEVY_EDITOR_SCREENSHOT";

/// Frames to wait before capturing, so assets and layout have settled.
const CAPTURE_FRAME: u32 = 120;
/// Frames to wait after capturing before exiting, so the file is written.
const EXIT_FRAME: u32 = CAPTURE_FRAME + 30;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(path) = std::env::var(SCREENSHOT_ENV) {
            app.insert_resource(ScreenshotRequest(path))
                .add_systems(Update, capture_screenshot);
        }
    }
}

#[derive(Resource)]
struct ScreenshotRequest(String);

fn capture_screenshot(
    mut commands: Commands,
    request: Res<ScreenshotRequest>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    *frame += 1;
    if *frame == CAPTURE_FRAME {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(request.0.clone()));
    } else if *frame == EXIT_FRAME {
        exit.write(AppExit::Success);
    }
}
