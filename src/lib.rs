pub mod asset;
mod asset_browser;
pub mod assets;
mod dock;
pub mod inspection;
pub mod pane;
pub mod prefs;
mod properties;
mod scene_tree;
pub mod selection;
mod style;
pub mod utils;
mod viewport;

pub use egui;
use lucide_icons::LUCIDE_FONT_BYTES;
use serde::{Deserialize, Serialize};

use std::env;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup, Startup},
    asset::AssetPlugin,
    camera::{Camera, Camera2d, visibility::RenderLayers},
    ecs::{
        entity::Entity,
        error::Result,
        event::EntityEvent,
        observer::On,
        query::With,
        resource::Resource,
        schedule::LogLevel,
        system::{Commands, Local, Query, Res, ResMut, Single, SystemState},
        world::{DeferredWorld, Mut, World},
    },
    log::{Level, LogPlugin},
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
    FontData, FontFamily, Frame, Id, InnerResponse, LayerId, Memory, MenuBar, Panel, Sense, Ui,
    UiBuilder, WidgetText,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

use crate::{
    asset::AssetDatabasePlugin,
    asset_browser::AssetBrowserPlugin,
    assets::{AssetsPlugin, LUCIDE_FONT_FAMILY},
    dock::{DockArea, DockState},
    inspection::DefaultInspectorConfigPlugin,
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
            .add_plugins(EguiPlugin::default())
            .add_plugins(AssetsPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .add_plugins(PanePlugin)
            .add_plugins(SelectionPlugin)
            .add_plugins(ViewportPlugin)
            .add_plugins(SceneTreePlugin)
            .add_plugins(PropertiesPlugin)
            .add_plugins(AssetBrowserPlugin)
            .register_pref::<EguiMemory>()
            .insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_systems(Startup, setup)
            .add_observer(setup_context)
            .add_systems(EguiPrimaryContextPass, ui)
            .add_observer(on_save);
    }
}

#[derive(EntityEvent)]
struct PrimaryEguiContextConfigured {
    pub entity: Entity,
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

fn setup(world: &mut World) -> Result {
    let entity = world
        .spawn((
            Pickable::IGNORE,
            Camera {
                order: 1,
                ..default()
            },
            Camera2d,
            PrimaryEguiContext,
        ))
        .id();

    world.trigger(PrimaryEguiContextConfigured { entity });

    let mut primary_window = world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(world)?;
    primary_window.set_maximized(true);

    Ok(())
}

fn setup_context(
    trigger: On<PrimaryEguiContextConfigured>,
    loaded_memory: Res<EguiMemory>,
    mut contexts: EguiContexts,
) -> Result {
    let ctx = contexts.ctx_for_entity_mut(trigger.event_target())?;

    ctx.memory_mut(|memory| *memory = loaded_memory.0.clone());
    ctx.all_styles_mut(|style| set_dark_style(style));
    ctx.add_font(FontInsert::new(
        "fira_regular",
        FontData::from_static(include_bytes!(
            "assets/fonts/fira_sans/FiraSans-Regular.ttf"
        )),
        vec![InsertFontFamily {
            family: FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));

    ctx.add_font(FontInsert::new(
        LUCIDE_FONT_FAMILY,
        FontData::from_static(&LUCIDE_FONT_BYTES),
        vec![
            InsertFontFamily {
                family: FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: FontFamily::Name(LUCIDE_FONT_FAMILY.into()),
                priority: FontPriority::Highest,
            },
        ],
    ));

    Ok(())
}

fn ui(
    world: &mut World,
    state: &mut SystemState<(EguiContexts, Res<PaneRegistry>, ResMut<PaneDocking>)>,
) -> Result {
    let (mut contexts, pane_registry, mut pane_docking) = state.get_mut(world);

    let ctx = contexts.ctx_mut()?.clone();

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
