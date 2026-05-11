use bevy::{asset::AssetPath, reflect::TypePath};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod de;
#[cfg(feature = "editor")]
pub mod ser;

#[derive(TypePath, Serialize, Deserialize)]
pub enum EditorAssetHandle {
    Runtime,
    AssetPath(AssetPath<'static>),
    Db(Uuid),
    Internal(Uuid),
}
