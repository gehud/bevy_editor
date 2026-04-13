pub(crate) mod database;

use std::{
    env::current_dir,
    fs::read_dir,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::{App, Plugin, PreStartup, Startup, Update},
    asset::{AssetApp, AssetMetaCheck, AssetMode, AssetPlugin, AssetServer, uuid::Uuid},
    ecs::{
        error::Result,
        message::{Message, MessageReader},
        system::{Commands, Res},
    },
    log::info,
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
    assets: Res<AssetServer>,
    database: Res<AssetDatabase>,
    mut commands: Commands,
) -> Result {
    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    database.connection().execute(
        "update assets set deleted = true where label is null",
        params![],
    )?;

    refresh_recurse("assets", &assets, &database)?;

    commands.write_message(DatabaseRefresed);

    Ok(())
}

fn refresh_recurse(
    path: impl AsRef<Path>,
    assets: &AssetServer,
    database: &AssetDatabase,
) -> Result {
    for entry in read_dir(path)? {
        let path = entry?.path();

        if path.is_dir() {
            refresh_recurse(path, assets, database)?;
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
                "select modified_at from assets where path = ?1 and label is null",
                params![path],
                |row| {
                    let modified_at: DateTime<Utc> = row.get(0)?;
                    Ok(modified_at)
                },
            );

            let last_modified_at = DateTime::<Utc>::from(metadata.modified()?);
            match row {
                Ok(mut modified_at) => {
                    if modified_at > last_modified_at {
                        modified_at = last_modified_at;

                        database.connection().execute(
                            "update assets set deleted = true where path = ?1",
                            params![path],
                        )?;

                        let _ = block_on(assets.load_untyped_async(&path))?;

                        if let Some(labels) = assets.get_living_labeled_assets(&path) {
                            for label in labels {
                                database.connection().execute(
                                "update assets set deleted = false, modified_at = ?1 where path = ?2 and label = ?3",
                                params![modified_at, path, label],
                            )?;
                            }
                        }
                    }

                    database.connection().execute(
                        "update assets set deleted = false, modified_at = ?1 where path = ?2 and label is null",
                        params![modified_at, path],
                    )?;
                }
                Err(error) => match error {
                    SqliteError::QueryReturnedNoRows => {
                        let _ = block_on(assets.load_untyped_async(&path))?;
                        database.connection().execute(
                            "insert into assets (uuid, path, modified_at) values (?1, ?2, ?3)",
                            params![Uuid::new_v4().to_string(), path, last_modified_at],
                        )?;

                        if let Some(labels) = assets.get_living_labeled_assets(&path) {
                            for label in labels {
                                database.connection().execute(
                                    "insert into assets (uuid, path, label, modified_at) values (?1, ?2, ?3, ?4)",
                                    params![Uuid::new_v4().to_string(), path, label, last_modified_at],
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
