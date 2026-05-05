use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    fs::{self, File},
    marker::PhantomData,
    ops::Index,
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::{
        Arc, LazyLock, Mutex, MutexGuard, PoisonError, RwLock,
        mpsc::{Receiver, channel},
    },
    time::Duration,
};

use async_channel::Sender;
use async_fs::ReadDir;
use bevy::{
    app::{App, Plugin, PreStartup, Startup, Update},
    asset::{
        AssetApp, AssetMetaCheck, AssetMode, AssetPlugin, AssetServer,
        io::{
            AssetReader, AssetReaderError, AssetSource, AssetSourceBuilder, AssetSourceEvent,
            AssetSourceId, AssetWatcher, AssetWriter, PathStream, Reader,
            file::{FileAssetReader, FileAssetWriter, FileWatcher},
        },
    },
    ecs::{
        error::Result,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res},
    },
    log::{info, warn},
    platform::collections::HashSet,
    tasks::{IoTaskPool, Task, block_on, futures, futures_lite::stream},
    utils::default,
};
use ron::ser::PrettyConfig;
use rusqlite::{Connection, Error as SqliteError, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

thread_local! {
    static READING_PATHS: RefCell<HashSet<PathBuf>> = RefCell::default();
}

struct AssetDatabaseReader {
    db: AssetDatabase,
    inner: FileAssetReader,
}

impl AssetDatabaseReader {
    pub fn new() -> Self {
        Self {
            db: AssetDatabase::open().unwrap(),
            inner: FileAssetReader::new("assets"),
        }
    }

    fn get_asset_path<'a>(&'a self, path: PathBuf) -> Option<PathBuf> {
        let uuid = Uuid::from_str(&path.to_string_lossy().to_string()).ok()?;
        self.db.get_path(&uuid).ok()?
    }
}

impl AssetReader for AssetDatabaseReader {
    async fn read<'a>(&'a self, path: PathBuf) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = self
            .get_asset_path(path.clone())
            .ok_or_else(|| AssetReaderError::NotFound(path))?;
        self.inner.read(asset_path).await
    }

    async fn read_meta<'a>(&'a self, path: PathBuf) -> Result<impl Reader + 'a, AssetReaderError> {
        let asset_path = self
            .get_asset_path(path.clone())
            .ok_or_else(|| AssetReaderError::NotFound(path))?;
        self.inner
            .read(asset_path.with_added_extension("meta"))
            .await
    }

    async fn read_directory<'a>(
        &'a self,
        _path: PathBuf,
    ) -> Result<Box<PathStream>, bevy::asset::io::AssetReaderError> {
        Ok(Box::new(stream::empty()))
    }

    async fn is_directory<'a>(&'a self, _path: PathBuf) -> Result<bool, AssetReaderError> {
        Ok(false)
    }
}

pub struct AssetDatabaseWatcher(#[allow(unused)] FileWatcher);

impl AssetDatabaseWatcher {
    pub fn new(sender: Sender<AssetSourceEvent>) -> Self {
        Self(FileWatcher::new("assets".into(), sender, Duration::from_millis(300)).unwrap())
    }
}

impl AssetWatcher for AssetDatabaseWatcher {}

fn watch(asset_server: Res<AssetServer>) -> Result {
    let source = asset_server.get_source(SOURCE_NAME)?;

    let receiver = source.event_receiver().unwrap();
    while let Ok(event) = receiver.try_recv() {}

    Ok(())
}

pub const SOURCE_NAME: &'static str = "db";

#[derive(Resource)]
pub struct AssetDatabase(Connection);

unsafe impl Send for AssetDatabase {}
unsafe impl Sync for AssetDatabase {}

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
        let db = Self(Connection::open(Self::path())?);
        db.connection().execute(
            "create table if not exists assets (
            uuid blob not null primary key,
            path string not null
        )",
            params![],
        )?;
        Ok(db)
    }

    fn get_uuid(&self, path: impl Into<PathBuf>) -> Result<Option<Uuid>> {
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

    fn get_path(&self, uuid: &Uuid) -> Result<Option<PathBuf>> {
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

    fn connection(&self) -> &Connection {
        &self.0
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

fn setup(mut commands: Commands) -> Result {
    commands.insert_resource(AssetDatabase::open()?);
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

        let mut meta = meta.unwrap_or_default();

        if let Some(uuid) = asset_database.get_uuid(asset_path)? {
            meta.uuid = uuid;
        } else {
            meta.uuid = Uuid::new_v4()
        }

        asset_database.insert_path(&meta.uuid, asset_path)?;
        let meta_text = ron::ser::to_string_pretty(&meta, PrettyConfig::default())?;
        fs::write(&meta_path, meta_text)?;
    }

    Ok(())
}

pub struct AssetDatabasePlugin;

impl Plugin for AssetDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Name(SOURCE_NAME.into()),
            AssetSourceBuilder::new(move || Box::new(AssetDatabaseReader::new()))
                .with_watcher(|sender| Some(Box::new(AssetDatabaseWatcher::new(sender)))),
        )
        .add_plugins(AssetPlugin {
            watch_for_changes_override: Some(true),
            use_asset_processor_override: Some(true),
            mode: AssetMode::Processed,
            meta_check: AssetMetaCheck::Always,
            ..default()
        })
        .add_systems(PreStartup, (setup, refresh).chain())
        .add_systems(Update, watch);
    }
}
