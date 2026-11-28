//! Bevy Editor: a scene editor, asset inspector and plugin host for Bevy.
//!
//! The editor is a library so that the binary (`src/main.rs`) and the examples (`examples/`)
//! can share it: add [`EditorPlugin`] to an empty `App` to turn it into the editor, then spawn
//! whatever scene should be edited.

mod dev;
pub mod editing;
mod editor_camera;
mod grid;
mod project;
mod ui;

use std::path::PathBuf;

use bevy::{prelude::*, window::WindowResolution};

pub use project::ProjectRoot;

/// Everything the editor needs, including Bevy's `DefaultPlugins` and the editor window.
pub struct EditorPlugin {
    /// Spawn the built-in demo scene (a light and a stylized dragon).
    pub demo_scene: bool,
    /// The project folder to open. The Assets pane browses its `assets/` directory.
    pub project: Option<PathBuf>,
}

impl Default for EditorPlugin {
    fn default() -> Self {
        Self {
            demo_scene: true,
            project: None,
        }
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".into(),
                // Sized like the design's window; the editor draws its own title bar.
                resolution: WindowResolution::new(1660, 880),
                decorations: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            ui::EditorUiPlugin,
            editing::EditingPlugin,
            editor_camera::EditorCameraPlugin,
            grid::InfiniteGridPlugin,
            dev::DevPlugin,
        ));
        if self.demo_scene {
            app.add_systems(Startup, ui::spawn_demo_scene);
        }
        if let Some(root) = &self.project {
            if !root.is_dir() {
                warn!("project folder {} does not exist", root.display());
            }
            app.insert_resource(ProjectRoot(root.clone()));
        }
    }
}
