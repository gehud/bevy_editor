pub mod serde;

use std::{
    path::Path,
    sync::{
        Mutex, PoisonError,
        mpsc::{Receiver, channel},
    },
};

use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::{AssetMetaCheck, AssetMode, AssetPlugin},
    ecs::{
        error::Result,
        resource::Resource,
        system::{Commands, Res},
    },
    log::{info, warn},
    utils::default,
};
use notify::{
    Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
    recommended_watcher,
};

const ASSETS_ROOT: &'static str = "assets";

#[derive(Resource)]
struct DirectoryWatcher {
    _watcher: RecommendedWatcher,
    receiver: Mutex<Receiver<Result<Event, Error>>>,
}

fn setup(mut commands: Commands) {
    let (sender, receiver) = channel();
    let watcher = recommended_watcher(sender);

    match watcher {
        Ok(mut watcher) => {
            if watcher
                .watch(Path::new(ASSETS_ROOT), RecursiveMode::Recursive)
                .is_ok()
            {
                commands.insert_resource(DirectoryWatcher {
                    _watcher: watcher,
                    receiver: Mutex::new(receiver),
                });
            } else {
                warn!("Failed to watch directory: {:?}", ASSETS_ROOT);
            }
        }
        Err(e) => {
            warn!("Failed to create directory watcher: {}", e);
        }
    }
}

fn watch(watcher: Res<DirectoryWatcher>) -> Result {
    let receiver = watcher
        .receiver
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    while let Ok(event) = receiver.try_recv() {
        let event = event?;

        match event.kind {
            EventKind::Modify(modify_kind) => match modify_kind {
                ModifyKind::Name(rename_mode) => match rename_mode {
                    RenameMode::Both => {
                        info!(
                            "Asset renamed from {:?} to {:?}",
                            event.paths[0], event.paths[1]
                        );
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

pub struct AssetWatcherPlugin;

impl Plugin for AssetWatcherPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetPlugin {
            watch_for_changes_override: Some(true),
            use_asset_processor_override: Some(true),
            mode: AssetMode::Processed,
            meta_check: AssetMetaCheck::Always,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, watch);
    }
}
