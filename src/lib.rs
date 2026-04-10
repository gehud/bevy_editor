mod cursor;
mod dock;
pub mod inspection;
pub mod pane;
mod properties;
mod scene_tree;
pub mod selection;
mod style;
mod viewport;

pub use egui;

use std::env;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup, Startup},
    camera::Camera2d,
    ecs::{
        error::Result,
        query::With,
        system::{Commands, Local, ResMut, Single, SystemState},
        world::{DeferredWorld, Mut, World},
    },
    utils::default,
    window::{PrimaryWindow, Window, WindowPlugin},
};
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext,
    egui::CentralPanel,
};
use egui::{
    FontData, FontFamily, Frame, InnerResponse, MenuBar, TopBottomPanel, WidgetText,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

use crate::{
    cursor::CursorLockPlugin,
    dock::{DockArea, DockState},
    inspection::{DefaultInspectorConfigPlugin, quick::WorldInspectorPlugin},
    pane::{PaneDocking, PanePlugin, PaneRegistry, PaneViewer},
    properties::PropertiesPlugin,
    scene_tree::SceneTreePlugin,
    selection::SelectionPlugin,
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
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CursorLockPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(PanePlugin)
        .add_plugins(SelectionPlugin)
        .add_plugins(ViewportPlugin)
        .add_plugins(SceneTreePlugin)
        .add_plugins(PropertiesPlugin)
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, ui);
    }
}

fn setup(mut commands: Commands, mut primary_window: Single<&mut Window, With<PrimaryWindow>>) {
    commands.spawn((Camera2d, PrimaryEguiContext));
    primary_window.set_maximized(true);
}

fn ui(world: &mut World, state: &mut SystemState<(EguiContexts, Local<bool>)>) -> Result {
    let (mut contexts, mut is_intialized) = state.get_mut(world);
    let mut ctx = contexts.ctx_mut()?.clone();

    if !*is_intialized {
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

    TopBottomPanel::top("header")
        .show_separator_line(false)
        .exact_height(34.0)
        .show(&mut ctx, |ui| {
            ui.horizontal_centered(|ui| {
                MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| if ui.button("Open").clicked() {});
                });
            });
        });

    TopBottomPanel::bottom("footer")
        .show_separator_line(false)
        .exact_height(24.0)
        .show(&mut ctx, |ui| {
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
        .show(&mut ctx, |ui| {
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

    Ok(())
}
