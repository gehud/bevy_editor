use std::{env, process::Command};

use bevy::ecs::{
    error::Result,
    world::{Mut, World},
};
use egui::{
    Button, CentralPanel, Frame, Key, KeyboardShortcut, MenuBar, Modifiers, Panel, Ui, Widget,
    WidgetText,
};
use lucide_icons::Icon;

use crate::{
    PLAY_MODE_VAR,
    asset_browser::AssetPayload,
    assets::icons::MaterialIcon,
    dock::DockArea,
    panel::{PanelDocking, PanelRegistry, PanelViewer},
    scene_tree::{DraggedSceneRoot, OpenScene, SaveScene},
    style::IntoDockStyle,
};

const FILE_OPEN_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
const FILE_SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);

fn menu_bar(ui: &mut Ui, world: &mut World) -> Result {
    MenuBar::new()
        .ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if Button::new("Open")
                    .shortcut_text(ui.format_shortcut(&FILE_OPEN_SHORTCUT))
                    .ui(ui)
                    .clicked()
                {
                    world.write_message(OpenScene);
                }

                if Button::new("Save")
                    .shortcut_text(ui.format_shortcut(&FILE_SAVE_SHORTCUT))
                    .ui(ui)
                    .clicked()
                {
                    world.write_message(SaveScene);
                }
            });

            let inner = ui
                .menu_button("View", |ui| -> Result {
                    world.resource_scope(|world, pane_registry: Mut<PanelRegistry>| {
                        let mut pane_docking = world.resource_mut::<PanelDocking>();
                        for name in pane_registry.names() {
                            if ui.button(name).clicked() {
                                if let Some(tab_path) = pane_docking.0.find_tab(name) {
                                    pane_docking
                                        .0
                                        .set_focused_node_and_surface(tab_path.node_path());
                                    pane_docking.0.set_active_tab(tab_path)?;
                                } else {
                                    pane_docking.0.add_window(vec![name.clone()]);
                                }
                            }
                        }

                        Ok(())
                    })
                })
                .inner;

            if let Some(inner) = inner {
                inner?;
            }

            ui.vertical_centered(|ui| -> Result {
                if ui.small_button(MaterialIcon::new(Icon::Play)).clicked() {
                    let current_exe = env::current_exe()?;
                    let current_dir = env::current_dir()?;
                    Command::new(current_exe)
                        .current_dir(current_dir)
                        .env(PLAY_MODE_VAR, "true")
                        .spawn()?;
                }

                Ok(())
            })
            .inner?;

            Ok(())
        })
        .inner
}

pub(super) fn root(ui: &mut Ui, world: &mut World) -> Result {
    Panel::top("header")
        .show_separator_line(false)
        .exact_size(34.0)
        .show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| menu_bar(ui, world)).inner
        })
        .inner?;

    Panel::bottom("footer")
        .show_separator_line(false)
        .exact_size(24.0)
        .show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(WidgetText::from("bevy-editor").weak());
            });
        });

    CentralPanel::default()
        .frame(
            Frame::central_panel(ui.style())
                .inner_margin(0)
                .outer_margin(0),
        )
        .show_inside(ui, |ui| {
            world.resource_scope(|world, mut registry: Mut<PanelRegistry>| {
                world.resource_scope(|world, mut docking: Mut<PanelDocking>| {
                    let style = ui.style().into_dock_style();

                    let mut viewer = PanelViewer {
                        registry: &mut registry,
                        world: world,
                        result: Ok(()),
                    };

                    DockArea::new(&mut docking.0)
                        .style(style)
                        .show_close_buttons(false)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                        .show_inside(ui, &mut viewer);

                    viewer.result
                })
            })
        })
        .inner?;

    if ui
        .response()
        .dnd_release_payload::<AssetPayload>()
        .is_some()
    {
        if let Some(scene_root) = world.resource_mut::<DraggedSceneRoot>().0.take() {
            world.entity_mut(scene_root).despawn();
        }
    }

    if ui.input_mut(|input| input.consume_shortcut(&FILE_OPEN_SHORTCUT)) {
        world.write_message(OpenScene);
    }

    if ui.input_mut(|input| input.consume_shortcut(&FILE_SAVE_SHORTCUT)) {
        world.write_message(SaveScene);
    }

    Ok(())
}
