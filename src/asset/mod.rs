mod database;

use bevy::{
    app::{App, Plugin, PreStartup, Startup},
    asset::{AssetMetaCheck, AssetMode, AssetPlugin},
    ecs::{error::Result, system::Commands},
    utils::default,
};

use crate::asset::database::AssetDatabase;

const DATABASE_PATH: &'static str = ".bevy/assets";

pub struct EditorAssetPlugin;

impl Plugin for EditorAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPlugin {
            file_path: "assets".into(),
            processed_file_path: ".bevy/imported".into(),
            watch_for_changes_override: Some(true),
            meta_check: AssetMetaCheck::Always,
            mode: AssetMode::Processed,
            ..default()
        })
        .add_systems(PreStartup, open_database)
        .add_systems(Startup, check_db);
    }
}

fn open_database(mut commands: Commands) -> Result {
    commands.insert_resource(AssetDatabase::open(DATABASE_PATH)?);
    Ok(())
}

fn check_db() {}
