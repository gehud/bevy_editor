use std::{f32::consts::TAU, fs::read_dir, path::PathBuf};

use bevy::{
    app::{App, Plugin},
    ecs::{
        error::{BevyError, Result},
        resource::Resource,
        world::World,
    },
};
use egui::{
    Frame, Id, InnerResponse, Margin, ScrollArea, Shape, Stroke, TextureOptions, Ui, UiBuilder,
    collapsing_header::{CollapsingState, paint_default_icon},
    epaint::{PathShape, PathStroke},
    load::SizedTexture,
};
use serde::{Deserialize, Serialize};

use crate::{
    assets::{
        Icons,
        icons::{self, CHEVRON_DOWN},
    },
    pane::{Pane, RegisterPane},
    prefs::RegisterPref,
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
                    ui.horizontal(|ui| -> Result {
                        let icon = world.resource::<Icons>().get(CHEVRON_DOWN)?;

                        collapsing_state.show_toggle_button(ui, move |ui, openness, response| {
                            let rotation = egui::remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0);

                            let rect = response.rect;

                            egui::Image::new((icon, rect.size()))
                                .rotate(rotation, egui::Vec2::splat(0.5))
                                .paint_at(ui, rect);
                        });
                        ui.label(file_name);
                        Ok(())
                    })
                    .inner
                })
                .inner
        });

    inner?;

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

            let icon = world.resource::<Icons>().get(icons::BOX)?;

            ui.set_width(ui.available_width());
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Image::new((icon, (15.0, 15.0).into())));
                    ui.label(file_name);
                });
            });

            Ok(())
        });

    inner?;

    Ok(())
}

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(AssetBrowser);
    }
}
