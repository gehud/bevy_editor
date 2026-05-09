pub mod asset;
mod asset_browser;
pub mod assets;
pub mod cursor;
mod dock;
pub mod inspection;
pub mod panel;
pub mod prefs;
mod properties;
pub mod scene;
mod scene_tree;
pub mod selection;
pub mod serde;
pub mod settings;
mod style;
mod ui;
pub mod utils;
mod viewport;

use ::serde::{Deserialize, Serialize};
pub use egui;
use lucide_icons::LUCIDE_FONT_BYTES;

use std::{env, fs::File};

use bevy::{
    DefaultPlugins,
    app::{App, AppExit, Plugin, PluginGroup, PluginGroupBuilder, Startup},
    asset::{AssetMetaCheck, AssetMode, AssetPlugin},
    camera::{Camera, Camera2d},
    ecs::{
        error::Result,
        observer::On,
        query::With,
        resource::Resource,
        system::{ResMut, Single, SystemState},
        world::World,
    },
    log::{
        Level, LogPlugin,
        tracing_subscriber::{self, Layer, fmt::writer::MakeWriterExt},
    },
    picking::Pickable,
    utils::default,
    window::{PrimaryWindow, Window, WindowPlugin},
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext,
};
use egui::{
    FontData, FontFamily, LayerId, Memory, Ui, UiBuilder,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

use crate::{
    asset::EditorAssetPlugin,
    asset_browser::AssetBrowserPlugin,
    assets::{AssetsPlugin, LUCIDE_FONT_FAMILY},
    cursor::CursorLockPlugin,
    inspection::DefaultInspectorConfigPlugin,
    panel::PanelPlugin,
    prefs::{PrefsPlugin, RegisterPref, Save},
    properties::PropertiesPlugin,
    scene::EditorScenePlugin,
    scene_tree::SceneTreePlugin,
    selection::SelectionPlugin,
    serde::EditorSerdePlugin,
    settings::SettingsPlugin,
    style::set_dark_style,
    ui::root,
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

pub struct EditorSharedPlugins;

impl PluginGroup for EditorSharedPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<EditorSharedPlugins>()
            .add(EditorSerdePlugin)
            .add(EditorScenePlugin)
            .add(SettingsPlugin)
    }
}

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorAssetPlugin)
            .add_plugins(
                DefaultPlugins
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "Bevy Editor".into(),
                            ..default()
                        }),
                        ..default()
                    })
                    .set(LogPlugin {
                        level: Level::TRACE,
                        fmt_layer: |_| {
                            let layer = tracing_subscriber::fmt::layer()
                                .with_writer(std::io::stdout.with_max_level(Level::INFO));
                            Some(layer.boxed())
                        },
                        custom_layer: |_| {
                            let file =
                                File::create("trace.log").expect("failed to create log file");
                            let layer = tracing_subscriber::fmt::layer()
                                .with_writer(file.with_max_level(Level::TRACE))
                                .with_ansi(false);
                            Some(layer.boxed())
                        },
                        ..default()
                    })
                    .disable::<AssetPlugin>(),
            )
            .add_plugins(PrefsPlugin)
            .add_plugins(EguiPlugin::default())
            .insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            })
            .add_plugins(AssetsPlugin)
            .add_plugins(CursorLockPlugin)
            .add_plugins(SelectionPlugin)
            .add_plugins(PanelPlugin)
            .add_plugins(EditorSharedPlugins)
            .add_plugins(PropertiesPlugin)
            .add_plugins(ViewportPlugin)
            .add_plugins(SceneTreePlugin)
            .add_plugins(AssetBrowserPlugin)
            .add_plugins(DefaultInspectorConfigPlugin)
            .register_pref::<EguiMemory>()
            .add_systems(Startup, (load_context, maximize_window))
            .add_systems(EguiPrimaryContextPass, ui)
            .add_observer(on_save);
    }
}

#[derive(Clone, Default, Deserialize, Resource, Serialize)]
struct EguiMemory(Memory);

fn on_save(_: On<Save>, mut contexts: EguiContexts, mut egui_memory: ResMut<EguiMemory>) -> Result {
    let ctx = contexts.ctx_mut()?;
    ctx.memory(|memory| {
        egui_memory.0 = memory.clone();
    });

    Ok(())
}

fn load_context(world: &mut World) {
    let loaded_memory = world.resource::<EguiMemory>().clone().0;

    let mut ctx = world.spawn((
        Pickable::IGNORE,
        Camera {
            order: 1,
            ..default()
        },
        Camera2d,
        PrimaryEguiContext,
    ));

    let mut ctx = ctx.get_mut::<EguiContext>().unwrap();
    let ctx = ctx.get_mut();

    ctx.memory_mut(|memory| *memory = loaded_memory);
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
}

fn maximize_window(mut primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    primary_window.set_maximized(true);
}

fn ui(world: &mut World, state: &mut SystemState<EguiContexts>) -> Result {
    let mut contexts = state.get_mut(world);

    let ctx = contexts.ctx_mut()?.clone();

    let mut ui = Ui::new(
        ctx.clone(),
        ctx.viewport_id().into(),
        UiBuilder::new().layer_id(LayerId::background()),
    );

    root(&mut ui, world)?;

    state.apply(world);

    Ok(())
}

pub struct EditorApp {
    #[cfg(feature = "editor")]
    editor_plugin: Box<dyn FnOnce(&mut App)>,
    shared_plugin: Box<dyn FnOnce(&mut App)>,
    runtime_plugin: Box<dyn FnOnce(&mut App)>,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            editor_plugin: Box::new(|_| {}),
            shared_plugin: Box::new(|_| {}),
            runtime_plugin: Box::new(|_| {}),
        }
    }
}

impl EditorApp {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "editor")]
    pub fn editor_plugin(mut self, plugin: impl Plugin) -> Self {
        self.editor_plugin = Box::new(move |app| {
            app.add_plugins(plugin);
        });
        self
    }

    pub fn shared_plugin(mut self, plugin: impl Plugin) -> Self {
        self.shared_plugin = Box::new(move |app| {
            app.add_plugins(plugin);
        });
        self
    }

    pub fn runtime_plugin(mut self, plugin: impl Plugin) -> Self {
        self.runtime_plugin = Box::new(move |app| {
            app.add_plugins(plugin);
        });
        self
    }

    pub fn run(self) -> AppExit {
        let mut app = App::new();

        if cfg!(feature = "editor") {
            if is_play_mode() {
                app.add_plugins(DefaultPlugins.set(AssetPlugin {
                    watch_for_changes_override: Some(false),
                    use_asset_processor_override: Some(false),
                    mode: AssetMode::Processed,
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }))
                .add_plugins(EditorSharedPlugins);
                (self.shared_plugin)(&mut app);
                (self.runtime_plugin)(&mut app);
            } else {
                app.add_plugins(EditorPlugin::default());
                (self.shared_plugin)(&mut app);
                (self.editor_plugin)(&mut app);
            }
        } else {
            app.add_plugins(DefaultPlugins.set(AssetPlugin {
                watch_for_changes_override: Some(false),
                use_asset_processor_override: Some(false),
                mode: AssetMode::Processed,
                meta_check: AssetMetaCheck::Never,
                ..default()
            }))
            .add_plugins(EditorSharedPlugins);
            (self.shared_plugin)(&mut app);
            (self.runtime_plugin)(&mut app);
        }

        app.run()
    }
}
