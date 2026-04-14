pub(crate) mod database;

use std::{
    env::current_dir,
    fs::read_dir,
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::{App, Plugin, PreStartup, Startup, Update},
    asset::{
        AssetApp, AssetMetaCheck, AssetMode, AssetPlugin, AssetServer,
        io::{AssetSource, AssetSourceBuilder},
        uuid::Uuid,
    },
    ecs::{
        error::Result,
        message::{Message, MessageReader},
        reflect::AppTypeRegistry,
        system::{Commands, Res},
    },
    log::info,
    reflect::TypeRegistry,
    tasks::block_on,
    utils::default,
};
use chrono::{DateTime, Utc};
use rusqlite::{Error as SqliteError, params};

use crate::asset::database::AssetDatabase;

const DATABASE_PATH: &'static str = "imported_assets/index.db";

#[derive(Message)]
pub struct RefreshDatabase;

#[derive(Message)]
pub struct DatabaseRefresed;

fn open_database(mut commands: Commands) -> Result {
    commands.insert_resource(AssetDatabase::open(DATABASE_PATH)?);
    commands.write_message(RefreshDatabase);
    Ok(())
}

fn refresh(
    mut requests: MessageReader<RefreshDatabase>,
    type_registry: Res<AppTypeRegistry>,
    assets: Res<AssetServer>,
    database: Res<AssetDatabase>,
    mut commands: Commands,
) -> Result {
    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    database
        .connection()
        .execute("update assets set deleted = true", params![])?;

    let type_registry = type_registry.read();
    refresh_recurse("assets", &type_registry, &assets, &database)?;

    commands.write_message(DatabaseRefresed);

    Ok(())
}

fn refresh_recurse(
    path: impl AsRef<Path>,
    type_registry: &TypeRegistry,
    assets: &AssetServer,
    database: &AssetDatabase,
) -> Result {
    for entry in read_dir(path)? {
        let path = entry?.path();

        if path.is_dir() {
            refresh_recurse(path, type_registry, assets, database)?;
        } else {
            let extension = path
                .extension()
                .map(|extension| extension.to_string_lossy().to_string())
                .unwrap_or_default();

            if !block_on(assets.get_asset_loader_with_extension(&extension)).is_ok() {
                continue;
            }

            let metadata = path.metadata()?;
            let path = AssetDatabase::normalize_path(path.strip_prefix("assets")?);

            let row = database.connection().query_one(
                "select uuid, modified_at from assets where path = ?1",
                params![path],
                |row| {
                    let uuid: String = row.get(0)?;
                    let modified_at: DateTime<Utc> = row.get(1)?;
                    Ok((uuid, modified_at))
                },
            );

            let new_modified_at = DateTime::<Utc>::from(metadata.modified()?);
            match row {
                Ok((uuid, mut modified_at)) => {
                    if new_modified_at > modified_at {
                        modified_at = new_modified_at;
                    }

                    let untyped = block_on(assets.load_untyped_async(&path))?;

                    let type_path = type_registry
                        .get_type_info(untyped.type_id())
                        .map(|info| info.type_path())
                        .unwrap_or_default();

                    database.connection().execute(
                        "update assets set deleted = false, type_path = ?1, modified_at = ?2 where path = ?3",
                        params![type_path, modified_at, path],
                    )?;

                    database
                        .connection()
                        .execute("delete from labels where uuid = ?1", params![uuid])?;

                    if let Some(labels) = assets.get_living_labeled_assets(&path) {
                        for label in labels {
                            database.connection().execute(
                                "insert into labels (uuid, label) values (?1, ?2)",
                                params![uuid, label],
                            )?;
                        }
                    }
                }
                Err(error) => match error {
                    SqliteError::QueryReturnedNoRows => {
                        let untyped = block_on(assets.load_untyped_async(&path))?;

                        let type_path = type_registry
                            .get_type_info(untyped.type_id())
                            .map(|info| info.type_path())
                            .unwrap_or_default();

                        let uuid = Uuid::new_v4().to_string();

                        database.connection().execute(
                            "insert into assets (uuid, path, type_path, modified_at) values (?1, ?2, ?3, ?4)",
                            params![uuid, path, type_path, new_modified_at],
                        )?;

                        if let Some(labels) = assets.get_living_labeled_assets(&path) {
                            for label in labels {
                                database.connection().execute(
                                    "insert into labels (uuid, label) values (?1, ?2)",
                                    params![uuid, label],
                                )?;
                            }
                        }
                    }
                    _ => return Err(error.into()),
                },
            };
        }
    }

    Ok(())
}

pub struct AssetDatabasePlugin;

impl Plugin for AssetDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPlugin {
            watch_for_changes_override: Some(true),
            meta_check: AssetMetaCheck::Never,
            use_asset_processor_override: Some(true),
            mode: AssetMode::Processed,
            ..default()
        })
        .add_message::<RefreshDatabase>()
        .add_message::<DatabaseRefresed>()
        .add_systems(PreStartup, open_database)
        .add_systems(Update, refresh);
    }
}
