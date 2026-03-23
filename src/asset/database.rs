use std::{fs::create_dir_all, path::Path, sync::Mutex};

use bevy::ecs::{error::Result, resource::Resource};
use rusqlite::Connection;

#[derive(Resource)]
pub(super) struct AssetDatabase(Mutex<Connection>);

impl AssetDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        create_dir_all(
            path.as_ref()
                .parent()
                .ok_or_else(|| "Invalid asset database path.")?,
        )?;
        Ok(Self(Mutex::new(Connection::open(path.as_ref())?)))
    }
}
