use std::{
    fs::{self},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use bevy::{
    app::{App, Plugin, PreStartup, Update},
    asset::{
        AssetApp, AssetServer,
        io::{AssetSource, AssetSourceBuilder, AssetSourceId, file::FileAssetReader},
    },
    ecs::{error::Result, resource::Resource, system::Res},
    log::{info, warn},
    reflect::TypePath,
    tasks::block_on,
};
use ron::ser::PrettyConfig;
use rusqlite::{Connection, Error as SqliteError, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Resource, Debug, TypePath)]
pub struct AssetDatabase(Arc<Mutex<Connection>>);

impl AssetDatabase {
    fn path() -> PathBuf {
        FileAssetReader::get_base_path()
            .join("imported_assets")
            .join("index.db")
    }

    fn normalize_path(path: impl Into<PathBuf>) -> String {
        path.into().to_string_lossy().to_string().replace('\\', "/")
    }

    fn open() -> Result<Self> {
        fs::create_dir_all(Self::path().parent().unwrap())?;
        let db = Self(Arc::new(Mutex::new(Connection::open(Self::path())?)));
        db.connection().execute(
            "create table if not exists assets (
            uuid blob not null primary key,
            path string not null unique
        )",
            params![],
        )?;
        Ok(db)
    }

    pub fn get_uuid(&self, path: impl Into<PathBuf>) -> Result<Option<Uuid>> {
        match self.connection().query_one(
            "select uuid from assets where path = ?1",
            params![Self::normalize_path(path.into())],
            |row| {
                let uuid: Uuid = row.get(0)?;
                Ok(uuid)
            },
        ) {
            Ok(uuid) => Ok(Some(uuid)),
            Err(error) => match error {
                SqliteError::QueryReturnedNoRows => Ok(None),
                error => Err(error.into()),
            },
        }
    }

    pub fn get_path(&self, uuid: &Uuid) -> Result<Option<PathBuf>> {
        match self.connection().query_one(
            "select path from assets where uuid = ?1",
            params![uuid],
            |row| {
                let path: String = row.get(0)?;
                Ok(path)
            },
        ) {
            Ok(path) => Ok(Some(path.into())),
            Err(error) => match error {
                SqliteError::QueryReturnedNoRows => Ok(None),
                error => Err(error.into()),
            },
        }
    }

    fn insert_path(&self, uuid: &Uuid, path: impl Into<PathBuf>) -> Result<()> {
        let path = Self::normalize_path(path);
        self.connection().execute(
            "insert or replace into assets (uuid, path) values (?1, ?2)",
            params![uuid, path],
        )?;
        Ok(())
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Serialize, Deserialize)]
pub struct AssetDatabaseMeta {
    pub version: String,
    pub uuid: Uuid,
}

impl Default for AssetDatabaseMeta {
    fn default() -> Self {
        Self {
            version: "0.1.0".into(),
            uuid: Uuid::nil(),
        }
    }
}

const DB_ASSET_SOUCE: &'static str = "db";

fn watch(asset_server: Res<AssetServer>) -> Result {
    let source = asset_server.get_source(DB_ASSET_SOUCE)?;

    let receiver = source.event_receiver().unwrap();
    while let Ok(event) = receiver.try_recv() {
        info!("{event:?}");
    }

    Ok(())
}

fn refresh(asset_database: Res<AssetDatabase>, asset_server: Res<AssetServer>) -> Result {
    refresh_recurse(&asset_database, &asset_server, "assets".into())?;
    Ok(())
}

fn refresh_recurse(
    asset_database: &AssetDatabase,
    asset_server: &AssetServer,
    path: PathBuf,
) -> Result {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();

        if path.is_dir() {
            refresh_recurse(asset_database, asset_server, path)?;
            continue;
        }

        let Some(extension) = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_string())
        else {
            continue;
        };

        if block_on(asset_server.get_asset_loader_with_extension(&extension)).is_err() {
            continue;
        }

        let meta_path = path.with_added_extension("dbm");

        let asset_path = path.strip_prefix("assets")?;

        let mut meta_updated = false;
        let meta = if meta_path.exists() && meta_path.is_file() {
            let meta_text = fs::read_to_string(&meta_path)?;
            ron::de::from_str::<AssetDatabaseMeta>(&meta_text)
                .inspect_err(|error| {
                    warn!("Failed to load 'AssetDatabaseMeta': {}", error);
                })
                .ok()
        } else {
            None
        };

        let mut meta = meta.unwrap_or_else(|| {
            meta_updated = true;
            AssetDatabaseMeta::default()
        });

        if let Some(uuid) = asset_database.get_uuid(asset_path)? {
            if meta.uuid != uuid {
                meta_updated = true;
            }

            meta.uuid = uuid;
        } else {
            meta.uuid = Uuid::new_v4();
            meta_updated = true;
        }

        asset_database.insert_path(&meta.uuid, asset_path)?;

        if meta_updated {
            let meta_text = ron::ser::to_string_pretty(&meta, PrettyConfig::default())?;
            fs::write(&meta_path, meta_text)?;
        }
    }

    Ok(())
}

// TODO: Reload scene asset handles on souce asset change.
pub struct EditorAssetDatabasePlugin;

impl Plugin for EditorAssetDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Name(DB_ASSET_SOUCE.into()),
            AssetSourceBuilder::new(AssetSource::get_default_reader("assets".into()))
                .with_watcher(AssetSource::get_default_watcher(
                    "assets".into(),
                    Duration::from_millis(300),
                ))
                .with_watch_warning(AssetSource::get_default_watch_warning()),
        )
        .insert_resource(AssetDatabase::open().unwrap())
        .add_systems(PreStartup, refresh)
        .add_systems(Update, watch);
    }
}
