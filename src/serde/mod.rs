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

pub struct EditorSerdePlugin;

impl Plugin for EditorSerdePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorDeserializerProcessor>();
        #[cfg(feature = "editor")]
        {
            use crate::serde::ser::EditorSerializerProcessor;

            app.init_resource::<EditorSerializerProcessor>();
        }
    }
}
