mod dock;
mod pane;
mod panes;
mod selection;
mod style;

pub use pane::*;
pub use selection::*;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup, PostStartup, Startup},
    asset::Assets,
    camera::Camera2d,
    ecs::{
        error::Result,
        query::With,
        system::{Commands, ResMut, Single, SystemState},
        world::World,
    },
    log::LogPlugin,
    scene::{DynamicScene, Scene, SceneRoot, SceneSpawner},
    utils::default,
    window::{Window, WindowPlugin},
};
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
    PrimaryEguiContext,
};
use egui::{
    CentralPanel, FontData, FontFamily, Frame, MenuBar, TopBottomPanel,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    panel::TopBottomSide,
};

use crate::{
    dock::{DockArea, Style},
    pane::{Docking, PaneViewer},
    panes::{HierarchyPane, InspectedScene, OutputPane, PropertiesPane, custom_layer, fmt_layer},
    style::set_dark_style,
};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    custom_layer: |app| Some(custom_layer()),
                    fmt_layer: |app| Some(fmt_layer()),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .init_resource::<Panes>()
        .init_resource::<Docking>()
        .init_resource::<Selection>()
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_panes)
        .add_systems(Startup, setup_default_scene)
        .add_systems(EguiPrimaryContextPass, ui);
    }
}

fn setup(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    egui_global_settings.auto_create_primary_context = false;
    commands
        .spawn((Camera2d::default(), PrimaryEguiContext))
        .entry::<EguiContext>()
        .and_modify(|mut ctx| {
            ctx.get_mut().style_mut(|style| {
                set_dark_style(style);
            });

            ctx.get_mut().all_styles_mut(|style| set_dark_style(style));
            ctx.get_mut().add_font(FontInsert::new(
                "fira",
                FontData::from_static(include_bytes!(
                    "./assets/fonts/fira_sans/FiraSans-Regular.ttf"
                )),
                vec![InsertFontFamily {
                    family: FontFamily::Proportional,
                    priority: FontPriority::Highest,
                }],
            ));
        });
}

fn setup_panes(mut panes: ResMut<Panes>) {
    panes.insert(OutputPane);
    panes.insert(HierarchyPane);
    panes.insert(PropertiesPane);
}

fn setup_default_scene(mut commands: Commands, mut scenes: ResMut<Assets<Scene>>) {
    let root = commands
        .spawn(SceneRoot(scenes.add(Scene::new(World::new()))))
        .id();
    commands.insert_resource(InspectedScene { root });
}

fn ui(
    world: &mut World,
    ctx: &mut SystemState<Single<&mut EguiContext, With<PrimaryEguiContext>>>,
) -> Result {
    let ctx = ctx.get_mut(world).get_mut().clone();

    TopBottomPanel::new(TopBottomSide::Top, "header")
        .show_separator_line(false)
        .exact_height(34.0)
        .show(&ctx, |ui| {
            ui.horizontal_centered(|ui| {
                MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| if ui.button("Open").clicked() {});

                    ui.menu_button("View", |_ui| {});
                });
            });
        });

    TopBottomPanel::new(TopBottomSide::Bottom, "footer")
        .show_separator_line(false)
        .exact_height(24.0)
        .show(&ctx, |_ui| {
            // TODO: Make footer.
        });

    CentralPanel::default()
        .frame(
            Frame::central_panel(&ctx.style())
                .inner_margin(0)
                .outer_margin(0),
        )
        .show(&ctx, |ui| {
            let style = Style::from_egui(ui.style());

            world.resource_scope::<Docking, _>(|world, mut docking| {
                let mut pane_viewer = PaneViewer { world };

                DockArea::new(&mut docking.0)
                    .style(style)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_inside(ui, &mut pane_viewer);
            });
        });

    Ok(())
}
