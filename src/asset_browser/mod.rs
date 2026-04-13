use std::{
    f32::consts::TAU,
    fs::read_dir,
    path::{Path, PathBuf},
};

use bevy::{
    app::{App, Plugin},
    asset::{AssetPath, AssetServer},
    ecs::{
        error::{BevyError, Result},
        resource::Resource,
        world::World,
    },
    log::info_once,
    tasks::block_on,
};
use egui::{
    Align2, Color32, FontId, FontSelection, Frame, Id, InnerResponse, Label, Margin, RichText,
    ScrollArea, Shape, Stroke, TextureOptions, Ui, UiBuilder,
    collapsing_header::{CollapsingState, paint_default_icon},
    emath::Rot2,
    epaint::{PathShape, PathStroke, TextShape},
    load::SizedTexture,
    text::LayoutJob,
};
use lucide_icons::Icon;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    asset::database::AssetDatabase,
    assets::icons::MaterialIcon,
    pane::{Pane, RegisterPane},
    prefs::RegisterPref,
    utils::paint_collapsing_button,
};

pub struct AssetBrowser;

impl Pane for AssetBrowser {
    fn name(&self) -> &str {
        "Asset Browser"
    }

    fn padding(&self) -> Option<Margin> {
        Some(Margin::ZERO)
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        Frame::new()
            .inner_margin(Margin {
                top: 4,
                right: 5,
                bottom: 4,
                left: 5,
            })
            .show(ui, |ui| {
                let _ = ui.button("assets");
            });

        ui.separator();

        let InnerResponse { inner, .. } = Frame::new()
            .inner_margin(Margin {
                top: 8,
                right: 12,
                bottom: 8,
                left: 12,
            })
            .show(ui, |ui| -> Result {
                ScrollArea::vertical()
                    .show(ui, |ui| -> Result {
                        for entry in read_dir("assets")? {
                            let asset_path = entry?.path().strip_prefix("assets")?.to_owned();
                            ui_for_asset(ui, world, asset_path)?;
                        }

                        Ok(())
                    })
                    .inner
            });

        inner?;

        Ok(())
    }
}

pub(crate) struct AssetPayload(pub Uuid);

enum AssetBrowserEntry {
    Directory {
        file_name: String,
        path: PathBuf,
    },
    File {
        file_name: String,
        path: PathBuf,
    },
    Asset {
        uuid: Uuid,
        file_name: String,
        path: PathBuf,
        labels: Vec<Uuid>,
    },
    LabeledAsset {
        uuid: Uuid,
        path: PathBuf,
        label: String,
    },
}

impl AssetBrowserEntry {
    fn new<'a>(path: impl Into<AssetPath<'a>>, world: &mut World) -> Result<Self> {
        let asset_path = path.into();
        let path = Path::new("assets").join(asset_path.path().to_path_buf());
        let db = world.resource::<AssetDatabase>();

        if let Some(label) = asset_path.label() {
            let uuid = db
                .get_asset_uuid(asset_path.clone())?
                .ok_or_else(|| BevyError::from("Invalid asset path"))?;

            Ok(Self::LabeledAsset {
                uuid,
                path,
                label: label.to_string(),
            })
        } else {
            let file_name = path
                .file_name()
                .map(|file_name| file_name.to_string_lossy().to_string())
                .ok_or_else(|| BevyError::from("Wrong path"))?;
            if path.is_dir() {
                Ok(Self::Directory { file_name, path })
            } else {
                if let Some(uuid) = db.get_asset_uuid(asset_path.clone())? {
                    let labels = world
                        .resource::<AssetDatabase>()
                        .get_asset_labeled_uuids(asset_path)?
                        .unwrap_or_default();

                    Ok(Self::Asset {
                        uuid,
                        file_name,
                        path,
                        labels,
                    })
                } else {
                    Ok(Self::File { file_name, path })
                }
            }
        }
    }

    fn header(&self, ui: &mut Ui, state: &mut CollapsingState, _: &mut World) -> Result {
        match self {
            AssetBrowserEntry::Directory { file_name, .. } => {
                state.show_toggle_button(ui, paint_collapsing_button);
                ui.label(file_name);
            }
            AssetBrowserEntry::File { file_name, .. } => {
                ui.label(MaterialIcon::new(Icon::File).rich_text().size(15.0));
                ui.label(file_name);
            }
            AssetBrowserEntry::Asset {
                file_name,
                labels,
                uuid,
                path,
            } => {
                if !labels.is_empty() {
                    state.show_toggle_button(ui, paint_collapsing_button);
                }
                ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));
                ui.dnd_drag_source(
                    Id::new(path).with("dnd_drag_source"),
                    AssetPayload(*uuid),
                    |ui| ui.label(file_name),
                );
            }
            AssetBrowserEntry::LabeledAsset {
                label, uuid, path, ..
            } => {
                ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));
                ui.dnd_drag_source(
                    Id::new(path).with(label).with("dnd_drag_source"),
                    AssetPayload(*uuid),
                    |ui| ui.label(label),
                );
            }
        }

        Ok(())
    }

    fn body(&self, ui: &mut Ui, world: &mut World) -> Result {
        match self {
            AssetBrowserEntry::Directory { path, .. } => {
                for entry in read_dir(path)? {
                    let asset_path = entry?.path().strip_prefix("assets")?.to_owned();
                    ui_for_asset(ui, world, asset_path)?;
                }
            }
            AssetBrowserEntry::Asset { labels, .. } => {
                for uuid in labels {
                    let path = world
                        .resource::<AssetDatabase>()
                        .get_path_by_uuid(uuid)?
                        .ok_or_else(|| BevyError::from("Invalid labeled uuid"))?
                        .into_owned();

                    ui_for_asset(ui, world, path)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

fn ui_for_asset<'a>(
    ui: &mut Ui,
    world: &mut World,
    asset_path: impl Into<AssetPath<'a>>,
) -> Result {
    let asset_path = asset_path.into();

    let mut collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), Id::new(&asset_path), false);

    let entry = AssetBrowserEntry::new(asset_path, world)?;

    let InnerResponse { inner, response } =
        ui.scope_builder(UiBuilder::new().id_salt("frame"), |ui| -> Result {
            let response = ui.response();

            ui.set_height(24.0);
            let mut frame = Frame::new()
                .inner_margin(Margin {
                    top: 4,
                    right: 6,
                    bottom: 4,
                    left: 6,
                })
                .corner_radius(ui.style().visuals.widgets.noninteractive.corner_radius);

            if ui.rect_contains_pointer(response.rect) {
                frame = frame.fill(ui.style().visuals.widgets.hovered.bg_fill);
            }

            frame
                .show(ui, |ui| -> Result {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| -> Result { entry.header(ui, &mut collapsing_state, world) })
                        .inner
                })
                .inner
        });

    inner?;

    let inner = collapsing_state
        .show_body_indented(&response, ui, |ui| -> Result { entry.body(ui, world) });

    if let Some(inner) = inner {
        inner.inner?;
    }

    Ok(())
}

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(AssetBrowser);
    }
}
