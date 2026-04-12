use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, embedded_asset},
    ecs::{
        error::{BevyError, Result},
        event::EntityEvent,
        observer::On,
        resource::Resource,
        system::{Res, ResMut},
    },
    platform::collections::HashMap,
};
use bevy_egui::{EguiContexts, EguiTextureHandle, EguiUserTextures};
use egui::{
    FontData, FontFamily, TextureId,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

use crate::PrimaryEguiContextConfigured;

pub mod icons {
    pub const CHEVRON_DOWN: &'static str = "chevron_down";
    pub const BOX: &'static str = "box";
}

#[derive(Default, Resource)]
pub struct Icons(HashMap<String, TextureId>);

impl Icons {
    pub fn get(&self, name: impl AsRef<str>) -> Result<TextureId> {
        self.0
            .get(name.as_ref())
            .cloned()
            .ok_or_else(|| BevyError::from("Invalid icon name. Please use 'icons' constants"))
    }

    fn insert(&mut self, name: impl Into<String>, id: TextureId) {
        self.0.insert(name.into(), id);
    }
}

fn setup(
    trigger: On<PrimaryEguiContextConfigured>,
    mut contexts: EguiContexts,
    mut icons: ResMut<Icons>,
    asset_server: Res<AssetServer>,
) -> Result {
    let ctx = contexts.ctx_for_entity_mut(trigger.event_target())?;

    ctx.add_font(FontInsert::new(
        "fira_regular",
        FontData::from_static(include_bytes!("fonts/fira_sans/FiraSans-Regular.ttf")),
        vec![InsertFontFamily {
            family: FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));

    fn insert_icon(
        icons: &mut Icons,
        contexts: &mut EguiContexts,
        asset_server: &AssetServer,
        path: &'static str,
        name: &'static str,
    ) {
        icons.insert(
            name,
            contexts.add_image(EguiTextureHandle::Strong(
                asset_server.load(format!("embedded://bevy_editor/icons{}{}.png", path, name)),
            )),
        );
    }

    insert_icon(
        &mut icons,
        &mut contexts,
        &asset_server,
        "/",
        icons::CHEVRON_DOWN,
    );

    insert_icon(
        &mut icons,
        &mut contexts,
        &asset_server,
        "/",
        icons::BOX,
    );

    Ok(())
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "src/assets", "icons/chevron_down.png");
        embedded_asset!(app, "src/assets", "icons/box.png");

        app.init_resource::<Icons>().add_observer(setup);
    }
}
