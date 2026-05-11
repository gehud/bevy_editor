use std::{
    fs::{self},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use bevy::{
    app::{App, Plugin, PreStartup, Update},
    asset::{
        AssetApp, AssetPath, AssetServer,
        io::{AssetSource, AssetSourceBuilder, AssetSourceId, file::FileAssetReader},
        processor::AssetProcessor,
    },
    ecs::{
        error::Result,
        reflect::AppTypeRegistry,
        resource::Resource,
        system::{Commands, Res},
    },
    log::{error, info},
    reflect::{TypePath, TypeRegistry},
    tasks::{IoTaskPool, Task, TaskPool, futures_lite::StreamExt},
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Error as SqliteError, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::asset::FILE_PATH;

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
            path string not null,
            label string default null,
            type_path string,
            modified_at datetime default null,
            deleted bool not null default false,
            unique(path, label)
        )",
            params![],
        )?;
        Ok(db)
    }

    pub fn get_uuid<'a>(&self, asset_path: impl Into<AssetPath<'a>>) -> Result<Option<Uuid>> {
        let asset_path = asset_path.into();

        let label_query = match asset_path.label() {
            Some(label) => format!("label = {}", label),
            None => "label is null".into(),
        };

        let path = Self::normalize_path(asset_path.path());

        match self.connection().query_one(
            &format!(
                "select uuid from assets where not deleted and path = ?1 and {}",
                label_query
            ),
            params![path],
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

    pub fn get_asset_path<'a>(&self, uuid: &Uuid) -> Result<Option<AssetPath<'a>>> {
        match self.connection().query_one(
            "select path, label from assets where not deleted and uuid = ?1",
            params![uuid],
            |row| {
                let path: String = row.get(0)?;
                let label: Option<String> = row.get(1)?;
                Ok((path, label))
            },
        ) {
            Ok((path, label)) => {
                let mut asset_path = AssetPath::from(path);
                if let Some(label) = label {
                    asset_path = asset_path.with_label(label);
                }
                Ok(Some(asset_path))
            }
            Err(error) => match error {
                SqliteError::QueryReturnedNoRows => Ok(None),
                error => Err(error.into()),
            },
        }
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Serialize, Deserialize)]
pub struct AssetDatabaseMeta {
    pub meta_format_version: String,
    pub uuid: Uuid,
    pub dependencies: Vec<Uuid>,
}

impl Default for AssetDatabaseMeta {
    fn default() -> Self {
        Self {
            meta_format_version: "1.0".into(),
            uuid: Uuid::nil(),
            dependencies: Vec::new(),
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

#[derive(Resource)]
pub(crate) struct RefreshTask(pub Task<Result>);

fn start_refresh(
    type_registry: Res<AppTypeRegistry>,
    asset_database: Res<AssetDatabase>,
    asset_processor: Res<AssetProcessor>,
    mut commands: Commands,
) {
    let type_registry = type_registry.clone();
    let asset_database = asset_database.clone();
    let asset_processor = asset_processor.clone();
    let task = IoTaskPool::get().spawn(async move {
        refresh(
            type_registry.clone(),
            asset_database.clone(),
            asset_processor.clone(),
        )
        .await
    });
    commands.insert_resource(RefreshTask(task));
}

async fn refresh(
    type_registry: AppTypeRegistry,
    asset_database: AssetDatabase,
    asset_processor: AssetProcessor,
) -> Result {
    asset_database
        .connection()
        .execute("update assets set deleted = true", params![])?;
    refresh_recurse(&type_registry, &asset_database, &asset_processor, "".into()).await?;
    Ok(())
}

async fn refresh_recurse(
    type_registry: &AppTypeRegistry,
    asset_database: &AssetDatabase,
    asset_processor: &AssetProcessor,
    path: PathBuf,
) -> Result {
    let souce = asset_processor.get_source(AssetSourceId::Default)?;

    let mut stream = souce.reader().read_directory(&path).await?;

    while let Some(path) = stream.next().await {
        let souce_meta =
            async_fs::metadata(FileAssetReader::get_base_path().join(FILE_PATH).join(&path))
                .await?;

        if souce.reader().is_directory(&path).await? {
            Box::pin(refresh_recurse(
                type_registry,
                asset_database,
                asset_processor,
                path,
            ))
            .await?;
            continue;
        }

        let asset_path = AssetPath::from_path_buf(path);

        let Some(extension) = asset_path.get_full_extension() else {
            continue;
        };

        if asset_processor
            .server()
            .get_asset_loader_with_extension(&extension)
            .await
            .is_err()
        {
            continue;
        }

        let _ = asset_processor
            .write_default_meta_file_for_path(&asset_path)
            .await;

        let path = AssetDatabase::normalize_path(asset_path.path());
        let modified_at: DateTime<Utc> = souce_meta.modified()?.into();

        let modified = match asset_database.connection().query_one(
            "select uuid, modified_at from assets where path = ?1 and label is null",
            params![path],
            |row| {
                let uuid: Uuid = row.get(0)?;
                let modified_at: DateTime<Utc> = row.get(1)?;
                Ok((uuid, modified_at))
            },
        ) {
            Ok((uuid, mut last_modified_at)) => {
                if modified_at > last_modified_at {
                    last_modified_at = modified_at;
                    Some((uuid, last_modified_at))
                } else {
                    None
                }
            }
            Err(error) => match error {
                SqliteError::QueryReturnedNoRows => Some((Uuid::new_v4(), modified_at)),
                error => return Err(error.into()),
            },
        };

        if let Some((uuid, modified_at)) = modified {
            let untyped = asset_processor
                .server()
                .load_untyped_async(&asset_path)
                .await?;

            let type_path = type_registry
                .read()
                .get(untyped.type_id())
                .map(|registration| registration.type_info().type_path());

            asset_database.connection().execute(
                "insert or replace into assets (uuid, path, type_path, modified_at) values (?1, ?2, ?3, ?4)",
                params![uuid, path, type_path, modified_at],
            )?;

            if let Some(labels) = asset_processor
                .server()
                .get_living_labeled_assets(&asset_path)
            {
                for label in labels {
                    let label = label.to_string();
                    let untyped = asset_processor
                        .server()
                        .load_untyped_async(asset_path.clone().with_label(&label))
                        .await?;

                    let type_path = type_registry
                        .read()
                        .get(untyped.type_id())
                        .map(|registration| registration.type_info().type_path());

                    let uuid = match asset_database.connection().query_one(
                        "select uuid from assets where path = ?1 and label = ?2",
                        params![path, label],
                        |row| {
                            let uuid: Uuid = row.get(0)?;
                            Ok(uuid)
                        },
                    ) {
                        Ok(uuid) => uuid,
                        Err(error) => match error {
                            SqliteError::QueryReturnedNoRows => Uuid::new_v4(),
                            error => return Err(error.into()),
                        },
                    };

                    asset_database.connection().execute(
                        "insert or replace into assets (uuid, path, label, type_path) values (?1, ?2, ?3, ?4)",
                        params![uuid, path, label, type_path],
                    )?;
                }
            }
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
            AssetSourceBuilder::new(AssetSource::get_default_reader(FILE_PATH.into()))
                .with_watcher(AssetSource::get_default_watcher(
                    FILE_PATH.into(),
                    Duration::from_millis(300),
                ))
                .with_watch_warning(AssetSource::get_default_watch_warning()),
        )
        .insert_resource(AssetDatabase::open().unwrap())
        .add_systems(PreStartup, start_refresh)
        .add_systems(Update, watch);
    }
}
