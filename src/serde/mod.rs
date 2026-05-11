use bevy::{asset::AssetPath, reflect::TypePath};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod de;
#[cfg(feature = "editor")]
pub mod ser;

#[derive(TypePath, Serialize, Deserialize)]
pub enum AssetRef {
    Empty,
    Db(AssetPath<'static>),
    Uuid(Uuid),
}
