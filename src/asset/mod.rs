#[cfg(feature = "editor")]
pub mod database;
pub mod id;
pub mod resolver;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetMetaCheck, AssetMode, AssetPlugin, Handle, ReflectHandle},
    utils::default,
};

#[cfg(feature = "editor")]
use crate::asset::database::{AssetDatabase, EditorAssetDatabasePlugin};
use crate::asset::id::{EditorAssetId, ReflectEditorAssetId};
use crate::asset::resolver::EditorAssetResolverPlugin;

pub trait EditorAssetApp {
    fn register_editor_asset<A: Asset>(&mut self) -> &mut Self;
}

impl EditorAssetApp for App {
    fn register_editor_asset<A: Asset>(&mut self) -> &mut Self {
        self.register_type::<Handle<A>>()
            .register_type::<EditorAssetId<A>>()
            .register_type_data::<Handle<A>, ReflectHandle>()
            .register_type_data::<EditorAssetId<A>, ReflectEditorAssetId>()
    }
}

pub struct EditorAssetPlugin;

impl Plugin for EditorAssetPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "editor")]
        {
            app.add_plugins(EditorAssetDatabasePlugin);
            app.add_plugins(AssetPlugin {
                mode: AssetMode::Processed,
                meta_check: AssetMetaCheck::Always,
                ..default()
            });
        }
        #[cfg(not(feature = "editor"))]
        {
            app.add_plugins(AssetPlugin {
                mode: AssetMode::Processed,
                meta_check: AssetMetaCheck::Never,
                ..default()
            });
        }

        app.add_plugins(EditorAssetResolverPlugin);
    }
}
