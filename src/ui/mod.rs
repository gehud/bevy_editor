use bevy::ecs::{
    error::Result,
    world::{Mut, World},
};
use egui::{CentralPanel, Frame, MenuBar, Panel, Ui, WidgetText};

use crate::{
    asset_browser::AssetPayload,
    dock::DockArea,
    pane::{PaneDocking, PaneRegistry, PaneViewer},
    scene_tree::{DraggedSceneRoot, OpenScene, SaveScene},
    style::IntoDockStyle,
};

fn menu_bar(ui: &mut Ui, world: &mut World) -> Result {
    MenuBar::new()
        .ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    world.write_message(OpenScene);
                }

                if ui.button("Save").clicked() {
                    world.write_message(SaveScene);
                }
            });

            let inner = ui
                .menu_button("View", |ui| -> Result {
                    world.resource_scope(|world, pane_registry: Mut<PaneRegistry>| {
                        let mut pane_docking = world.resource_mut::<PaneDocking>();
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

    Ok(())
}
