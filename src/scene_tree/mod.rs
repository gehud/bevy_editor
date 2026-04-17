use std::ops::Deref;

use bevy::{
    app::{App, Plugin, Startup},
    asset::{Assets, Handle},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        name::Name,
        query::{Or, With, Without},
        system::{Commands, ResMut},
        world::World,
    },
    gltf::Gltf,
    light::PointLight,
    log::info,
    math::{
        Quat,
        primitives::{Circle, Cuboid},
    },
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    scene::{InstanceId, Scene, SceneInstance, SceneRoot, SceneSpawner},
    transform::components::Transform,
    utils::default,
};
use egui::{
    Color32, CornerRadius, Frame, Id, InnerResponse, Label, Margin, RichText, Sense, Stroke, Ui,
    UiBuilder, Widget, collapsing_header::CollapsingState,
};
use lucide_icons::Icon;

use crate::{
    asset_browser::AssetPayload,
    assets::icons::MaterialIcon,
    pane::{Pane, RegisterPane},
    selection::{Selection, SelectionMap},
    style::ACCENT,
    utils::paint_collapsing_button,
};

pub struct SceneTreePane;

impl Pane for SceneTreePane {
    fn name(&self) -> &str {
        "Scene Tree"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let roots = world
            .query_filtered::<Entity, (With<SceneRoot>, Without<ChildOf>)>()
            .iter(world)
            .collect::<Vec<_>>();

        for root in roots {
            if !world.entity(root).contains::<SceneInstance>() {
                ui.label("Loading...");
            } else {
                self.entity_ui_recurse(ui, world, root, Id::new(root));
            }
        }

        let mut frame = Frame::new().begin(ui);
        frame.content_ui.take_available_space();
        let response = frame.allocate_space(ui);

        let scene = response
            .dnd_hover_payload::<AssetPayload>()
            .and_then(|payload| {
                payload.0.clone().try_typed::<Scene>().ok().or_else(|| {
                    payload
                        .0
                        .clone()
                        .try_typed::<Gltf>()
                        .ok()
                        .and_then(|handle| {
                            world
                                .resource::<Assets<Gltf>>()
                                .get(&handle)
                                .and_then(|gltf| gltf.default_scene.clone())
                        })
                })
            });

        if let Some(scene) = scene {
            frame.frame.fill = ui.style().visuals.widgets.active.bg_fill;
            frame.frame.stroke = ui.style().visuals.widgets.active.bg_stroke;

            if response.dnd_release_payload::<AssetPayload>().is_some() {
                let name = scene
                    .path()
                    .and_then(|path| {
                        path.path()
                            .with_extension("")
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| "Untitled".into());

                world.spawn((
                    Name::new(name),
                    Visibility::Visible,
                    Transform::IDENTITY,
                    SceneRoot(scene),
                ));
            }
        }

        frame.paint(ui);

        Ok(())
    }
}

impl SceneTreePane {
    fn entity_ui_recurse(&mut self, ui: &mut Ui, world: &mut World, entity: Entity, id: Id) {
        let name = world
            .entity(entity)
            .get::<Name>()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "Entity".into());

        let mut collapsing_state =
            CollapsingState::load_with_default_open(ui.ctx(), id.with("collapsing"), false);

        let icon_color = if world.entity(entity).contains::<SceneInstance>() {
            ACCENT
        } else {
            Color32::WHITE
        };

        let header_response = ui
            .scope_builder(
                UiBuilder::new()
                    .id_salt(id.with("frame"))
                    .sense(Sense::click()),
                |ui| {
                    let response = ui.response();

                    let mut frame = Frame::new()
                        .inner_margin(Margin {
                            top: 4,
                            right: 8,
                            bottom: 4,
                            left: 8,
                        })
                        .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                        .corner_radius(CornerRadius::same(4));

                    if ui.rect_contains_pointer(response.rect) {
                        frame.fill = ui.style().visuals.widgets.hovered.bg_fill;
                    }

                    if world.resource_mut::<SelectionMap>().is_selected(entity) {
                        frame.stroke.color = ACCENT;
                    }

                    ui.set_height(24.0);
                    frame.show(ui, |ui| {
                        ui.take_available_width();
                        ui.horizontal(|ui| {
                            if world.entity(entity).contains::<Children>() {
                                collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                            }

                            Label::new(
                                MaterialIcon::new(Icon::Box)
                                    .rich_text()
                                    .size(15.0)
                                    .color(icon_color),
                            )
                            .selectable(false)
                            .ui(ui);

                            Label::new(name).selectable(false).ui(ui);
                        });
                    });
                },
            )
            .response;

        let mut selection_map = world.resource_mut::<SelectionMap>();

        if header_response.clicked() {
            if !ui.input(|i| i.modifiers.ctrl) {
                selection_map.clear();
                selection_map.select(entity);
            } else {
                if selection_map.is_selected(entity) {
                    selection_map.deselect(entity);
                } else {
                    selection_map.select(entity);
                }
            }
        }

        collapsing_state.show_body_indented(&header_response, ui, |ui| {
            if let Some(children) = world
                .query::<&Children>()
                .get(world, entity)
                .ok()
                .map(|children| children.to_vec())
            {
                for child in children {
                    self.entity_ui_recurse(ui, world, child, Id::new(child));
                }
            }
        });
    }
}

fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut commands: Commands,
) {
    let mut scene = Scene::new(World::new());

    scene.world.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    scene.world.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    scene.world.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    for i in 0..3 {
        let a = scene.world.spawn(Name::new(format!("A{}", i))).id();
        for j in 0..3 {
            let b = scene
                .world
                .spawn((ChildOf(a), Name::new(format!("B{}", j))))
                .id();
            for k in 0..3 {
                scene
                    .world
                    .spawn((ChildOf(b), Name::new(format!("C{}", k))));
            }
        }
    }

    let scene_handle = scenes.add(scene);

    commands.spawn((
        Name::new("Sample"),
        Visibility::Visible,
        Transform::IDENTITY,
        SceneRoot(scene_handle),
    ));
}

pub struct SceneTreePlugin;

impl Plugin for SceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(SceneTreePane).add_systems(Startup, setup);
    }
}
