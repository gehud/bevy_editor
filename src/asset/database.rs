use std::{
    fs::{create_dir_all, read_dir},
    path::Path,
    str::FromStr,
    sync::{Mutex, MutexGuard, PoisonError},
};

use bevy::{
    asset::{AssetPath, uuid::Uuid},
    ecs::{error::Result, resource::Resource},
    utils::default,
};
use rusqlite::{Connection, Error as SqliteError, ToSql, params, types::Null};

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

    pub fn get_asset_uuid<'a>(&self, path: impl Into<AssetPath<'a>>) -> Result<Option<Uuid>> {
        let mut path = path.into();

        let label_query = if let Some(label) = path.take_label() {
            format!("label = '{}'", label)
        } else {
            "label is null".into()
        };

        let path = Self::normalize_path(path.path());

        match self.connection().query_one(
            &format!(
                "select uuid from assets where not deleted and path = ?1 and {}",
                label_query
            ),
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

    pub fn get_asset_labeled_uuids<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
    ) -> Result<Option<Vec<Uuid>>> {
        let path = path.into();

        if path.label().is_some() {
            return Err("'AssetPath' without label expected.".into());
        }

        if self.get_asset_uuid(&path)?.is_none() {
            return Ok(None);
        }

        let path = Self::normalize_path(path.path());

        let connection = self.connection();

        let mut stmt = connection.prepare(
            "select uuid from assets where not deleted and label is not null and path = ?1",
        )?;

        let iter = stmt.query_map(params![path], |row| {
            let uuid: String = row.get(0)?;
            Ok(Uuid::from_str(&uuid))
        })?;

        let mut uuids = Vec::new();

        for uuid in iter {
            uuids.push(uuid??);
        }

        Ok(Some(uuids))
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
