//! The project being edited.

use std::path::{Path, PathBuf};

use bevy::{
    asset::io::{AssetSourceBuilder, file::FileAssetReader},
    prelude::*,
};

pub(crate) const PROJECT_ASSET_SOURCE: &str = "project";

/// Keep the project's loader separate from the editor's own default asset source.
pub(crate) fn register_asset_source(app: &mut App, root: &Path) {
    let path = root.join("assets");
    let Ok(path) = std::path::absolute(path) else {
        return;
    };
    app.register_asset_source(
        PROJECT_ASSET_SOURCE,
        AssetSourceBuilder::new(move || Box::new(FileAssetReader::new(&path))),
    );
}

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
