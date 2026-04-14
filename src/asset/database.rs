use std::{
    fs::create_dir_all,
    path::Path,
    str::FromStr,
    sync::{Mutex, MutexGuard, PoisonError},
};

use bevy::{
    asset::{AssetPath, uuid::Uuid},
    ecs::{error::Result, resource::Resource},
};
use rusqlite::{Connection, Error as SqliteError, params};

#[derive(Resource)]
pub(crate) struct AssetDatabase(Mutex<Connection>);

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

    pub fn normalize_path(path: impl AsRef<Path>) -> String {
        path.as_ref()
            .to_string_lossy()
            .to_string()
            .replace('\\', "/")
    }

    pub fn connection(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn init(&self) -> Result {
        self.connection().execute(
            "create table if not exists assets (
                    uuid text not null primary key,
                    path text not null unique,
                    type_path text not null,
                    deleted bool not null default false,
                    modified_at datetime nut null
                )",
            params![],
        )?;

        self.connection().execute(
            "create table if not exists labels (
                    uuid text not null,
                    label text not null
                )",
            params![],
        )?;

        Ok(())
    }

    pub fn get_uuid<'a>(&self, path: impl Into<AssetPath<'a>>) -> Result<Option<Uuid>> {
        let path = path.into();
        let path = Self::normalize_path(path.path());

        match self.connection().query_one(
            "select uuid from assets where not deleted and path = ?1",
            params![path],
            |row| {
                let uuid: String = row.get(0)?;
                Ok(Uuid::from_str(&uuid))
            },
        ) {
            Ok(uuid) => Ok(Some(uuid?)),
            Err(err) => match err {
                SqliteError::QueryReturnedNoRows => Ok(None),
                err => Err(err.into()),
            },
        }
    }

    pub fn get_labels<'a>(&self, uuid: &Uuid) -> Result<Option<Vec<String>>> {
        if self.get_path(uuid)?.is_none() {
            return Ok(None);
        }

        let connection = self.connection();

        let mut stmt = connection.prepare("select label from labels where uuid = ?1")?;

        let iter = stmt.query_map(params![uuid.to_string()], |row| {
            let label: String = row.get(0)?;
            Ok(label)
        })?;

        let mut labels = Vec::new();

        for label in iter {
            labels.push(label?);
        }

        Ok(Some(labels))
    }

    pub fn get_path(&self, uuid: &Uuid) -> Result<Option<AssetPath<'_>>> {
        let path = match self.connection().query_one(
            "select path from assets where not deleted and uuid = ?1",
            params![uuid.to_string()],
            |row| {
                let path: String = row.get(0)?;
                Ok(path)
            },
        ) {
            Ok(value) => value,
            Err(err) => match err {
                SqliteError::QueryReturnedNoRows => {
                    return Ok(None);
                }
                err => return Err(err.into()),
            },
        };

        Ok(Some(AssetPath::from(path)))
    }
}
