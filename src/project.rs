//! The project being edited.

use std::path::PathBuf;

use bevy::prelude::*;

/// The folder of the project open in the editor. Absent when no project is open.
#[derive(Resource, Clone, Debug)]
pub struct ProjectRoot(pub PathBuf);

impl ProjectRoot {
    /// The project's display name: the folder name.
    pub fn name(&self) -> String {
        // `canonicalize` resolves relative paths such as `.` to a real folder name.
        let path = self.0.canonicalize().unwrap_or_else(|_| self.0.clone());
        path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }
}
