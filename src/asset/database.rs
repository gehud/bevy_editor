use std::{
    fs::{create_dir_all, read_dir},
    path::Path,
    str::FromStr,
    sync::{Mutex, MutexGuard, PoisonError},
};

use bevy::{
    asset::{AssetPath, uuid::Uuid},
    ecs::{error::Result, resource::Resource},
};
use rusqlite::{Connection, params};

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
                    path text not null,
                    label text default null,
                    deleted bool not null default false,
                    modified_at datetime nut null,
                    unique (path, label)
                )",
            (),
        )?;
        Ok(())
    }

    pub fn get_asset_uuid<'a>(&self, path: impl Into<AssetPath<'a>>) -> Result<Uuid> {
        let path = path.into();

        let uuid = if let Some(label) = path.label() {
            let path = Self::normalize_path(path.path());
            self.connection().query_one(
                "select uuid from assets where not deleted and path = ?1 and label = ?2",
                params![path, label],
                |row| {
                    let uuid: String = row.get(0)?;
                    Ok(Uuid::from_str(&uuid))
                },
            )?
        } else {
            let path = Self::normalize_path(path.path());
            self.connection().query_one(
                "select uuid from assets where not deleted and path = ?1 and label = null",
                params![path],
                |row| {
                    let uuid: String = row.get(0)?;
                    Ok(Uuid::from_str(&uuid))
                },
            )?
        }?;

        Ok(uuid)
    }

    pub fn get_asset_labeled_uuids<'a>(&self, path: impl Into<AssetPath<'a>>) -> Result<Vec<Uuid>> {
        let path = path.into();

        if path.label().is_some() {
            return Err("'AssetPath' without label expected.".into());
        }

        let path = Self::normalize_path(path.path());

        let connection = self.connection();

        let mut stmt = connection.prepare(
            "select uuid from assets where not deleted and label not null and path = ?1",
        )?;

        let iter = stmt.query_map(params![path], |row| {
            let uuid: String = row.get(0)?;
            Ok(Uuid::from_str(&uuid))
        })?;

        let mut uuids = Vec::new();

        for uuid in iter {
            uuids.push(uuid??);
        }

        Ok(uuids)
    }

    pub fn get_path_by_uuid<'a>(&self, uuid: &Uuid) -> Result<AssetPath<'_>> {
        let (path, label) = self.connection().query_one(
            "select path, label from assets where uuid = ?1",
            params![uuid.to_string()],
            |row| {
                let path: String = row.get(0)?;
                let label: Option<String> = row.get(1).ok();
                Ok((path, label))
            },
        )?;

        let mut path = AssetPath::from(path);

        if let Some(label) = label {
            path = path.with_label(label);
        }

        Ok(path)
    }
}
