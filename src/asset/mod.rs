use std::{
    fs::File,
    ops::Index,
    path::Path,
    sync::{
        Mutex, PoisonError,
        mpsc::{Receiver, channel},
    },
    time::Duration,
};

use async_channel::Sender;
use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::{
        AssetApp, AssetMetaCheck, AssetMode, AssetPlugin, AssetServer,
        io::{
            AssetReader, AssetSource, AssetSourceBuilder, AssetSourceEvent, AssetSourceId, AssetWatcher, AssetWriter, file::{FileAssetReader, FileAssetWriter, FileWatcher}
        },
    },
    ecs::{
        error::Result,
        resource::Resource,
        system::{Commands, Res},
    },
    log::{info, warn},
    utils::default,
};

struct IndexedAssetReader(FileAssetReader);

impl IndexedAssetReader {
    pub fn new() -> Self {
        Self(FileAssetReader::new("assets"))
    }
}

impl AssetReader for IndexedAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::asset::io::AssetReaderFuture<Value: bevy::asset::io::Reader + 'a> {
        self.0.read(path)
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::asset::io::AssetReaderFuture<Value: bevy::asset::io::Reader + 'a> {
        self.0.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<
        Output = std::result::Result<
            Box<bevy::asset::io::PathStream>,
            bevy::asset::io::AssetReaderError,
        >,
    > {
        self.0.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<
        Output = std::result::Result<bool, bevy::asset::io::AssetReaderError>,
    > {
        self.0.is_directory(path)
    }
}

pub struct IndexAssetWatcher(#[allow(unused)] FileWatcher);

impl IndexAssetWatcher {
    pub fn new(sender: Sender<AssetSourceEvent>) -> Self {
        Self(FileWatcher::new("assets".into(), sender, Duration::from_millis(300)).unwrap())
    }
}

impl AssetWatcher for IndexAssetWatcher {}

pub struct IndexAssetWriter(FileAssetWriter);

impl IndexAssetWriter {
    pub fn new(create_root: bool) -> Self {
        Self(FileAssetWriter::new("assets", create_root))
    }
}

impl AssetWriter for IndexAssetWriter {
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<Box<bevy::asset::io::Writer>, bevy::asset::io::AssetWriterError>> {
        self.0.write(path)
    }

    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<Box<bevy::asset::io::Writer>, bevy::asset::io::AssetWriterError>> {
        self.0.write_meta(path)
    }

    fn remove<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.remove(path)
    }

    fn remove_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.remove_meta(path)
    }

    fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.rename(old_path, new_path)
    }

    fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.rename_meta(old_path, new_path)
    }

    fn create_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.create_directory(path)
    }

    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.remove_directory(path)
    }

    fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.remove_empty_directory(path)
    }

    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<(), bevy::asset::io::AssetWriterError>> {
        self.0.remove_assets_in_directory(path)
    }
}

fn watch(asset_server: Res<AssetServer>) -> Result {
    let source = asset_server.get_source("index")?;

    let receiver = source.event_receiver().unwrap();
    while let Ok(event) = receiver.try_recv() {
        info!("{:?}", event);
    }

    Ok(())
}

pub struct AssetIndexPlugin;

impl Plugin for AssetIndexPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Name("index".into()),
            AssetSourceBuilder::new(|| Box::new(IndexedAssetReader::new()))
                .with_writer(|create_root| Some(Box::new(IndexAssetWriter::new(create_root))))
                .with_watcher(|sender| Some(Box::new(IndexAssetWatcher::new(sender)))),
        )
        .add_plugins(AssetPlugin {
            watch_for_changes_override: Some(true),
            use_asset_processor_override: Some(true),
            mode: AssetMode::Processed,
            meta_check: AssetMetaCheck::Always,
            ..default()
        })
        .add_systems(Update, watch);
    }
}
