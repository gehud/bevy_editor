use bevy::{
    app::{App, Plugin},
    asset::AssetPath,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::serde::de::EditorDeserializerProcessor;

pub mod de;
#[cfg(feature = "editor")]
pub mod ser;

#[derive(TypePath, Serialize, Deserialize)]
pub enum AssetRef {
    Empty,
    Db(AssetPath<'static>),
    Uuid(Uuid),
}
