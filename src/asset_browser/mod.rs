use std::{f32::consts::TAU, fs::read_dir, path::PathBuf};

use bevy::{
    app::{App, Plugin},
    asset::AssetServer,
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
                            let path = entry?.path();
                            ui_for_dir_entry(ui, world, path)?;
                        }

                        Ok(())
                    })
                    .inner
            });

        inner?;

        Ok(())
    }
}

fn ui_for_dir_entry(ui: &mut Ui, world: &mut World, path: impl Into<PathBuf>) -> Result {
    let path = path.into();

    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .ok_or_else(|| BevyError::from("Wrong path"))?;

    if path.is_dir() {
        ui_for_dir(ui, world, file_name, path)?;
    } else {
        ui_for_file(ui, world, file_name, path)?;
    }

    Ok(())
}

fn ui_for_dir(ui: &mut Ui, world: &mut World, file_name: String, path: PathBuf) -> Result {
    let mut collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), Id::new(&path), false);

    let response = ui
        .scope_builder(UiBuilder::new().id_salt("frame"), |ui| {
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

            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                    ui.label(file_name);
                });
            });
        })
        .response;

    let inner = collapsing_state.show_body_indented(&response, ui, |ui| -> Result {
        for entry in read_dir(path)? {
            let path = entry?.path();
            ui_for_dir_entry(ui, world, path)?;
        }

        Ok(())
    });

    if let Some(inner) = inner {
        inner.inner?;
    }

    Ok(())
}

fn ui_for_file(ui: &mut Ui, world: &mut World, file_name: String, path: PathBuf) -> Result {
    let mut collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), Id::new(&path), false);

    let asset_path = path.strip_prefix("assets")?.to_path_buf();
    let labeled_uuids = world
        .resource::<AssetDatabase>()
        .get_asset_labeled_uuids(asset_path.clone())?;
    let has_labeled_assets = !labeled_uuids.is_empty();

    let response = ui
        .scope_builder(UiBuilder::new().id_salt("frame"), |ui| {
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

            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if has_labeled_assets {
                        collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                    }
                    ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));
                    ui.label(file_name);
                });
            });
        })
        .response;

    if has_labeled_assets {
        let inner = collapsing_state.show_body_indented(&response, ui, |ui| -> Result {
            for uuid in labeled_uuids {
                ui_for_labeled_asset(ui, world, uuid)?
            }

            Ok(())
        });

        if let Some(inner) = inner {
            inner.inner?;
        }
    }

    Ok(())
}

fn ui_for_labeled_asset(ui: &mut Ui, world: &mut World, uuid: Uuid) -> Result {
    let asset_path = world.resource::<AssetDatabase>().get_path_by_uuid(&uuid)?;
    let label = asset_path.label().unwrap();

    let response = ui
        .scope_builder(UiBuilder::new().id_salt("frame"), |ui| {
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

            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));
                    ui.label(label);
                });
            });
        })
        .response;

    Ok(())
}

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(AssetBrowser);
    }
}
