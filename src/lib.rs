pub mod asset;
mod cursor;
mod dock;
pub mod inspection;
pub mod pane;
pub mod prefs;
mod properties;
mod scene_tree;
pub mod selection;
mod style;
mod viewport;

pub use egui;
use serde::{Deserialize, Serialize};

use std::env;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup, Startup},
    asset::AssetPlugin,
    camera::{Camera, Camera2d, visibility::RenderLayers},
    ecs::{
        error::Result,
        observer::On,
        query::With,
        resource::Resource,
        system::{Commands, Local, Res, ResMut, Single, SystemState},
        world::{DeferredWorld, Mut, World},
    },
    picking::{
        Pickable,
        events::{Click, Pointer},
    },
    reflect::Reflect,
    utils::default,
    window::{PrimaryWindow, Window, WindowPlugin},
};
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext,
    egui::CentralPanel,
};
use egui::{
    FontData, FontFamily, Frame, Id, InnerResponse, LayerId, Memory, MenuBar, Panel, Sense,
    TopBottomPanel, Ui, UiBuilder, WidgetText,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

use crate::{
    asset::AssetDatabasePlugin,
    cursor::CursorLockPlugin,
    dock::{DockArea, DockState},
    inspection::{DefaultInspectorConfigPlugin, quick::WorldInspectorPlugin},
    pane::{PaneDocking, PanePlugin, PaneRegistry, PaneViewer},
    prefs::{Load, PrefsPlugin, RegisterPref, Save},
    properties::PropertiesPlugin,
    scene_tree::SceneTreePlugin,
    selection::{SelectionMap, SelectionPlugin},
    style::{IntoDockStyle, set_dark_style},
    viewport::ViewportPlugin,
};

pub const PLAY_MODE_VAR: &'static str = "BEVY_EDITOR_PLAY";

pub fn is_play_mode() -> bool {
    let Ok(var) = env::var(PLAY_MODE_VAR) else {
        return false;
    };

    let Ok(value) = var.parse::<bool>() else {
        return false;
    };

    value
}

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetDatabasePlugin)
            .add_plugins(
                DefaultPlugins
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Bevy Editor".into(),
                            ..default()
                        }),
                        ..default()
                    })
                    .disable::<AssetPlugin>(),
            )
            .add_plugins(PrefsPlugin)
            .add_plugins(CursorLockPlugin)
            .add_plugins(EguiPlugin::default())
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_plugins(PanePlugin)
            .add_plugins(SelectionPlugin)
            .add_plugins(ViewportPlugin)
            .add_plugins(SceneTreePlugin)
            .add_plugins(PropertiesPlugin)
            .register_pref::<EguiMemory>()
            .insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_systems(Startup, setup)
            .add_systems(EguiPrimaryContextPass, ui)
            .add_observer(on_save);
    }
}

#[derive(Resource, Default, Serialize, Deserialize)]
struct EguiMemory(Memory);

fn on_save(_: On<Save>, mut contexts: EguiContexts, mut egui_memory: ResMut<EguiMemory>) -> Result {
    let ctx = contexts.ctx_mut()?;
    ctx.memory(|memory| {
        egui_memory.0 = memory.clone();
    });

    Ok(())
}

fn setup(mut commands: Commands, mut primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    commands.spawn((
        Pickable::IGNORE,
        Camera {
            order: 1,
            ..default()
        },
        Camera2d,
        PrimaryEguiContext,
    ));
    primary_window.set_maximized(true);
}

fn ui(
    world: &mut World,
    state: &mut SystemState<(
        EguiContexts,
        Res<EguiMemory>,
        Res<PaneRegistry>,
        ResMut<PaneDocking>,
        Local<bool>,
    )>,
) -> Result {
    let (mut contexts, loaded_memory, pane_registry, mut pane_docking, mut is_intialized) =
        state.get_mut(world);
    let ctx = contexts.ctx_mut()?.clone();

    if !*is_intialized {
        ctx.memory_mut(|memory| *memory = loaded_memory.0.clone());
        ctx.all_styles_mut(|style| set_dark_style(style));
        ctx.add_font(FontInsert::new(
            "fira",
            FontData::from_static(include_bytes!(
                "assets/fonts/fira_sans/FiraSans-Regular.ttf"
            )),
            vec![InsertFontFamily {
                family: FontFamily::Proportional,
                priority: FontPriority::Highest,
            }],
        ));

        *is_intialized = true;
    }

    let mut ui = Ui::new(
        ctx.clone(),
        ctx.viewport_id().into(),
        UiBuilder::new().layer_id(LayerId::background()),
    );

    Panel::top("header")
        .show_separator_line(false)
        .exact_size(34.0)
        .show_inside(&mut ui, |ui| {
            ui.horizontal_centered(|ui| {
                MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| if ui.button("Open").clicked() {});

                    ui.menu_button("View", |ui| {
                        for name in pane_registry.names() {
                            if ui.button(name).clicked() {
                                if let Some(tab_path) = pane_docking.0.find_tab(name) {
                                    pane_docking
                                        .0
                                        .set_focused_node_and_surface(tab_path.node_path());
                                    pane_docking.0.set_active_tab(tab_path);
                                } else {
                                    pane_docking.0.add_window(vec![name.clone()]);
                                }
                            }
                        }
                    })
                });
            });
        });

    Panel::bottom("footer")
        .show_separator_line(false)
        .exact_size(24.0)
        .show_inside(&mut ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::from("bevy-editor").weak());
            });
        });

    let InnerResponse { inner, .. } = CentralPanel::default()
        .frame(
            Frame::central_panel(&ctx.style())
                .inner_margin(0)
                .outer_margin(0),
        )
        .show_inside(&mut ui, |ui| {
            world.resource_scope(|world, mut registry: Mut<PaneRegistry>| {
                world.resource_scope(|world, mut docking: Mut<PaneDocking>| {
                    let style = ui.style().into_dock_style();

                    let mut viewer = PaneViewer {
                        registry: &mut registry,
                        world: world,
                        result: Ok(()),
                    };

                    DockArea::new(&mut docking.0)
                        .style(style)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                        .show_inside(ui, &mut viewer);

                    viewer.result
                })
            })
        });

    inner?;

    if ui.response().interact(Sense::click()).clicked() {
        world.resource_mut::<SelectionMap>().clear();
    }

    Ok(())
}
