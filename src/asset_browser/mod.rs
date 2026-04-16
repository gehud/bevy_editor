use std::{
    any::TypeId,
    borrow::Cow,
    f32::consts::TAU,
    fs::read_dir,
    path::{Path, PathBuf},
};

use bevy::{
    app::{App, Plugin},
    asset::{AssetPath, AssetServer, Assets, Handle, LoadedUntypedAsset, UntypedHandle},
    ecs::{
        error::{BevyError, Result},
        resource::Resource,
        world::World,
    },
    log::{info, info_once},
    platform::collections::{HashMap, HashSet},
    tasks::block_on,
    utils::default,
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

use crate::{
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

        let mut inspected_assets = HashSet::new();

        Frame::new()
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
                            ui_for_asset(ui, world, asset_path, &mut inspected_assets)?;
                        }

                        Ok(())
                    })
                    .inner
            })
            .inner?;

        let mut tree = world.resource_mut::<AssetTree>();
        tree.0.retain(|path, _| inspected_assets.contains(path));

        Ok(())
    }
}

#[derive(Clone)]
enum AssetState {
    Loading(Handle<LoadedUntypedAsset>),
    Ready(UntypedHandle),
}

#[derive(Default, Resource)]
struct AssetTree(HashMap<String, AssetState>);

pub(crate) struct AssetPayload(pub UntypedHandle);

enum AssetBrowserEntry {
    Directory {
        file_name: String,
        path: PathBuf,
    },
    File {
        file_name: String,
    },
    Asset {
        asset_path: String,
        file_name: String,
        labels: Option<HashSet<Box<str>>>,
    },
    LabeledAsset {
        asset_path: String,
        label: String,
    },
}

impl AssetBrowserEntry {
    fn new<'a>(path: impl Into<AssetPath<'a>>, world: &mut World) -> Result<Self> {
        let asset_path = path.into();
        let path = Path::new("assets").join(asset_path.path().to_path_buf());

        if let Some(label) = asset_path.label() {
            Ok(Self::LabeledAsset {
                asset_path: asset_path.to_string(),
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
                let extension = path
                    .extension()
                    .map(|extension| extension.to_string_lossy())
                    .unwrap_or_else(|| "".into());

                let asset_server = world.resource::<AssetServer>();

                if block_on(asset_server.get_asset_loader_with_extension(&extension)).is_ok() {
                    let labels = asset_server.get_living_labeled_assets(&asset_path);

                    Ok(Self::Asset {
                        asset_path: asset_path.to_string(),
                        file_name,
                        labels,
                    })
                } else {
                    Ok(Self::File { file_name })
                }
            }
        }
    }

    fn header(
        &self,
        ui: &mut Ui,
        state: &mut CollapsingState,
        world: &mut World,
        inspected_assets: &mut HashSet<String>,
    ) -> Result {
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
                asset_path,
            } => {
                inspected_assets.insert(asset_path.clone());

                if labels.as_ref().is_some_and(|labels| !labels.is_empty()) {
                    state.show_toggle_button(ui, paint_collapsing_button);
                }

                let state = world.resource::<AssetTree>().0.get(asset_path).cloned();

                ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));

                if let Some(state) = state {
                    match state {
                        AssetState::Loading(handle) => {
                            ui.label("Loading...");

                            if let Some(loaded) = world
                                .resource::<Assets<LoadedUntypedAsset>>()
                                .get(&handle)
                                .map(|handle| handle.handle.clone())
                            {
                                world
                                    .resource_mut::<AssetTree>()
                                    .0
                                    .insert(asset_path.clone(), AssetState::Ready(loaded));
                            }
                        }
                        AssetState::Ready(untyped_handle) => {
                            ui.dnd_drag_source(
                                Id::new(asset_path).with("dnd_drag_source"),
                                AssetPayload(untyped_handle),
                                |ui| ui.label(file_name),
                            );
                        }
                    }
                } else {
                    let handle = world.resource::<AssetServer>().load_untyped(asset_path);
                    world
                        .resource_mut::<AssetTree>()
                        .0
                        .insert(asset_path.clone(), AssetState::Loading(handle));
                }
            }
            AssetBrowserEntry::LabeledAsset { label, asset_path } => {
                inspected_assets.insert(asset_path.clone());
                ui.label(MaterialIcon::new(Icon::Box).rich_text().size(15.0));

                let state = world.resource::<AssetTree>().0.get(asset_path).cloned();

                if let Some(state) = state {
                    match state {
                        AssetState::Loading(handle) => {
                            ui.label("Loading...");

                            if let Some(loaded) = world
                                .resource::<Assets<LoadedUntypedAsset>>()
                                .get(&handle)
                                .map(|handle| handle.handle.clone())
                            {
                                world
                                    .resource_mut::<AssetTree>()
                                    .0
                                    .insert(asset_path.clone(), AssetState::Ready(loaded));
                            }
                        }
                        AssetState::Ready(untyped_handle) => {
                            ui.dnd_drag_source(
                                Id::new(asset_path).with("dnd_drag_source"),
                                AssetPayload(untyped_handle),
                                |ui| ui.label(label),
                            );
                        }
                    }
                } else {
                    let handle = world.resource::<AssetServer>().load_untyped(asset_path);
                    world
                        .resource_mut::<AssetTree>()
                        .0
                        .insert(asset_path.clone(), AssetState::Loading(handle));
                }
            }
        }

        Ok(())
    }

    fn body(
        &self,
        ui: &mut Ui,
        world: &mut World,
        inspected_assets: &mut HashSet<String>,
    ) -> Result {
        match self {
            AssetBrowserEntry::Directory { path, .. } => {
                for entry in read_dir(path)? {
                    let asset_path = entry?.path().strip_prefix("assets")?.to_owned();
                    ui_for_asset(ui, world, asset_path, inspected_assets)?;
                }
            }
            AssetBrowserEntry::Asset {
                labels, asset_path, ..
            } => {
                if let Some(labels) = labels {
                    for label in labels {
                        ui_for_asset(
                            ui,
                            world,
                            AssetPath::from(asset_path.clone()).with_label(label.to_string()),
                            inspected_assets,
                        )?;
                    }
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
    inspected_assets: &mut HashSet<String>,
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
                    ui.horizontal(|ui| -> Result {
                        entry.header(ui, &mut collapsing_state, world, inspected_assets)
                    })
                    .inner
                })
                .inner
        });

    inner?;

    let inner = collapsing_state.show_body_indented(&response, ui, |ui| -> Result {
        entry.body(ui, world, inspected_assets)
    });

    if let Some(inner) = inner {
        inner.inner?;
    }

    Ok(())
}

pub struct AssetBrowserPlugin;

impl Plugin for AssetBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetTree>().register_pane(AssetBrowser);
    }
}
