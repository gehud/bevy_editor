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
    camera::visibility::Visibility,
    ecs::{
        error::{BevyError, Result},
        name::Name,
        resource::Resource,
        world::World,
    },
    gltf::Gltf,
    log::{info, info_once},
    platform::collections::{HashMap, HashSet},
    scene::{Scene, SceneRoot},
    tasks::block_on,
    transform::components::Transform,
    utils::default,
};
use egui::{
    Align2, Color32, CornerRadius, FontId, FontSelection, Frame, Id, InnerResponse, Label, LayerId,
    Margin, Order, RichText, ScrollArea, Sense, Shape, Stroke, TextureOptions, Ui, UiBuilder,
    collapsing_header::{CollapsingState, paint_default_icon},
    emath::{Rot2, TSTransform},
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
    scene_tree::{DraggedSceneRoot, scene_name},
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

        let id = ui.id();

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
                            ui_for_asset(ui, id, world, asset_path, &mut inspected_assets)?;
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

pub(crate) struct FilePayload(pub PathBuf);

enum AssetBrowserEntry {
    Directory,
    File,
    Asset { labels: Option<HashSet<Box<str>>> },
    LabeledAsset { label: String },
}

impl AssetBrowserEntry {
    fn new<'a>(asset_path: &AssetPath<'a>, path: &PathBuf, world: &mut World) -> Result<Self> {
        if let Some(label) = asset_path.label() {
            Ok(Self::LabeledAsset {
                label: label.to_string(),
            })
        } else {
            if path.is_dir() {
                Ok(Self::Directory)
            } else {
                let extension = path
                    .extension()
                    .map(|extension| extension.to_string_lossy())
                    .unwrap_or_else(|| "".into());

                let asset_server = world.resource::<AssetServer>();

                if block_on(asset_server.get_asset_loader_with_extension(&extension)).is_ok() {
                    let labels = asset_server.get_living_labeled_assets(asset_path);

                    Ok(Self::Asset { labels })
                } else {
                    Ok(Self::File)
                }
            }
        }
    }
}

fn ui_for_asset<'a>(
    ui: &mut Ui,
    id: Id,
    world: &mut World,
    asset_path: impl Into<AssetPath<'a>>,
    inspected_assets: &mut HashSet<String>,
) -> Result {
    let asset_path = asset_path.into();
    let path = Path::new("assets").join(asset_path.path().to_path_buf());
    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .ok_or_else(|| BevyError::from("Wrong path"))?;

    let mut collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), Id::new(&asset_path), false);

    let entry = AssetBrowserEntry::new(&asset_path, &path, world)?;

    let asset_path = asset_path.to_string();
    let global_id = id;
    let id = global_id.with(&asset_path);

    ui.scope_builder(
        UiBuilder::new()
            .id_salt("frame")
            .sense(Sense::click_and_drag()),
        |ui| -> Result {
            let header_response = ui.response();

            let mut frame = Frame::new()
                .inner_margin(Margin::symmetric(8, 4))
                .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                .corner_radius(CornerRadius::same(4));

            if ui.rect_contains_pointer(header_response.rect) {
                frame = frame.fill(ui.style().visuals.widgets.hovered.bg_fill);
            }

            ui.set_height(24.0);
            frame
                .show(ui, |ui| -> Result {
                    ui.take_available_width();
                    ui.horizontal(|ui| -> Result {
                        let has_children = match &entry {
                            AssetBrowserEntry::Directory => true,
                            AssetBrowserEntry::Asset { labels } => {
                                labels.as_ref().is_some_and(|assets| !assets.is_empty())
                            }
                            _ => false,
                        };

                        if has_children {
                            collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                        } else {
                            ui.add_space(ui.spacing().indent + ui.spacing().item_spacing.x);
                        }

                        let icon = match &entry {
                            AssetBrowserEntry::Directory => {
                                if collapsing_state.is_open() {
                                    Icon::FolderOpen
                                } else {
                                    Icon::Folder
                                }
                            }
                            AssetBrowserEntry::File => Icon::File,
                            _ => Icon::Package,
                        };

                        let state = world.resource::<AssetTree>().0.get(&asset_path).cloned();
                        let is_ready = state
                            .as_ref()
                            .is_some_and(|state| matches!(state, AssetState::Ready(_)));

                        let label = match &entry {
                            AssetBrowserEntry::Directory | AssetBrowserEntry::File => &file_name,
                            AssetBrowserEntry::Asset { labels: _ } => {
                                if is_ready {
                                    &file_name
                                } else {
                                    "Loading..."
                                }
                            }
                            AssetBrowserEntry::LabeledAsset { label } => {
                                if is_ready {
                                    label
                                } else {
                                    "Loading..."
                                }
                            }
                        };

                        let id = id.with("dnd");
                        let mut ui_builder = UiBuilder::new().id(id);
                        let layer_id = LayerId::new(Order::Tooltip, id);

                        let is_dragged = header_response.dragged();

                        if is_dragged {
                            ui_builder.layer_id = Some(layer_id);
                        }

                        let dnd_response = ui
                            .scope_builder(ui_builder, |ui| {
                                ui.add(MaterialIcon::new(icon));
                                ui.label(label);
                            })
                            .response;

                        if is_dragged {
                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - dnd_response.rect.center();
                                ui.ctx().transform_layer_shapes(
                                    layer_id,
                                    TSTransform::from_translation(delta),
                                );
                            }
                        }

                        if matches!(
                            entry,
                            AssetBrowserEntry::Asset { labels: _ }
                                | AssetBrowserEntry::LabeledAsset { label: _ }
                        ) {
                            inspected_assets.insert(asset_path.clone());
                            if let Some(state) = &state {
                                match state {
                                    AssetState::Loading(handle) => {
                                        if let Some(loaded) = world
                                            .resource::<Assets<LoadedUntypedAsset>>()
                                            .get(handle)
                                            .map(|handle| handle.handle.clone())
                                        {
                                            world.resource_mut::<AssetTree>().0.insert(
                                                asset_path.clone(),
                                                AssetState::Ready(loaded),
                                            );
                                        }
                                    }
                                    AssetState::Ready(handle) => {
                                        header_response
                                            .dnd_set_drag_payload(AssetPayload(handle.clone()));

                                        if is_dragged {
                                            let scene =
                                                handle.clone().try_typed::<Scene>().ok().or_else(
                                                    || {
                                                        handle
                                                            .clone()
                                                            .try_typed::<Gltf>()
                                                            .ok()
                                                            .and_then(|handle| {
                                                                world
                                                                    .resource::<Assets<Gltf>>()
                                                                    .get(&handle)
                                                                    .and_then(|gltf| {
                                                                        gltf.default_scene.clone()
                                                                    })
                                                            })
                                                    },
                                                );

                                            if let Some(scene) = scene {
                                                if world.resource::<DraggedSceneRoot>().0.is_none()
                                                {
                                                    let root = world
                                                        .spawn((
                                                            Visibility::Hidden,
                                                            Name::new(scene_name(scene.path())),
                                                            SceneRoot(scene),
                                                        ))
                                                        .id();

                                                    world.resource_mut::<DraggedSceneRoot>().0 =
                                                        Some(root);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                let handle = world
                                    .resource::<AssetServer>()
                                    .load_untyped(asset_path.clone());

                                world
                                    .resource_mut::<AssetTree>()
                                    .0
                                    .insert(asset_path.clone(), AssetState::Loading(handle));
                            }
                        }

                        Ok(())
                    })
                    .inner
                })
                .inner
        },
    )
    .inner?;

    let inner = collapsing_state.show_body_unindented(ui, |ui| -> Result {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().item_spacing.x);
            ui.vertical(|ui| {
                ui.indent(id.with("body"), |ui| -> Result {
                    match &entry {
                        AssetBrowserEntry::Directory => {
                            for entry in read_dir(path)? {
                                let asset_path = entry?.path().strip_prefix("assets")?.to_owned();
                                ui_for_asset(ui, global_id, world, asset_path, inspected_assets)?;
                            }
                        }
                        AssetBrowserEntry::Asset { labels } => {
                            if let Some(labels) = labels {
                                for label in labels {
                                    ui_for_asset(
                                        ui,
                                        global_id,
                                        world,
                                        AssetPath::from(asset_path.clone())
                                            .with_label(label.to_string()),
                                        inspected_assets,
                                    )?;
                                }
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                })
                .inner
            })
            .inner
        })
        .inner
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
