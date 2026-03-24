use std::{
    fs::{create_dir_all, read_dir},
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
};

use bevy::ecs::{error::Result, resource::Resource};
use rusqlite::{Connection, params};

#[derive(Resource)]
pub(super) struct AssetDatabase(Mutex<Connection>);

impl AssetDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        create_dir_all(
            path.as_ref()
                .parent()
                .ok_or_else(|| "Invalid asset database path.")?,
        )?;

        let db = Self(Mutex::new(Connection::open(path.as_ref())?));
        db.init()?;

        Ok(db)
    }

    pub fn connection(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn init(&self) -> Result {
        self.connection().execute(
            "create table if not exists assets (
                    uuid text not null primary key,
                    path text not null,
                    label text default null,
                    deleted bool not null default false,
                    modified_at datetime nut null
                )",
            (),
        )?;
        Ok(())
    }
}
