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
        system::{Commands, Query, ResMut},
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
    Color32, CornerRadius, DragAndDrop, Frame, Id, InnerResponse, Label, LayerId, Margin, Modal,
    Order, RichText, Sense, Stroke, Ui, UiBuilder, Widget, collapsing_header::CollapsingState,
    emath,
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

pub struct EntityPayload {
    pub entity: Entity,
}

pub struct SceneTreePane;

impl Pane for SceneTreePane {
    fn name(&self) -> &str {
        "Scene Tree"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let tree = world
            .query_filtered::<Entity, With<SceneTree>>()
            .single(world)?;

        let roots = world
            .entity(tree)
            .get::<Children>()
            .map(|children| children.to_vec())
            .unwrap_or_default();

        for root in roots {
            self.entity_ui_recurse(ui, world, root, ui.id());
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
                    ChildOf(tree),
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

        let global_id = id;

        let id = id.with(entity);

        let mut collapsing_state =
            CollapsingState::load_with_default_open(ui.ctx(), id.with("collapsing"), false);

        let is_scene = world.entity(entity).contains::<SceneTree>();
        let is_loaded_scene = world.entity(entity).contains::<SceneInstance>();
        let is_loading_scene = is_scene && !is_loaded_scene;

        let icon_color = if is_loaded_scene {
            ACCENT
        } else {
            Color32::WHITE
        };

        let header_response = ui
            .scope_builder(
                UiBuilder::new()
                    .id_salt(id.with("header"))
                    .sense(Sense::click_and_drag()),
                |ui| {
                    let response = ui.response();
                    let is_dragged = response.dragged();

                    response.dnd_set_drag_payload(EntityPayload { entity });

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

                            let id = id.with("dnd");
                            let mut ui_builder = UiBuilder::new().id(id);
                            let layer_id = LayerId::new(Order::Tooltip, id);

                            if is_dragged {
                                ui_builder.layer_id = Some(layer_id);
                            }

                            let response = ui
                                .scope_builder(ui_builder, |ui| {
                                    Label::new(
                                        MaterialIcon::new(Icon::Box)
                                            .rich_text()
                                            .size(15.0)
                                            .color(icon_color),
                                    )
                                    .selectable(false)
                                    .ui(ui);

                                    if is_loading_scene {
                                        Label::new("Loading...").selectable(false).ui(ui);
                                    } else {
                                        Label::new(name.clone()).selectable(false).ui(ui);
                                    }
                                })
                                .response;

                            if is_dragged {
                                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                    let delta = pointer_pos - response.rect.center();
                                    ui.ctx().transform_layer_shapes(
                                        layer_id,
                                        emath::TSTransform::from_translation(delta),
                                    );
                                }
                            }
                        });
                    });
                },
            )
            .response;

        collapsing_state.show_body_indented(&header_response, ui, |ui| {
            if let Some(children) = world
                .query::<&Children>()
                .get(world, entity)
                .ok()
                .map(|children| children.to_vec())
            {
                for child in children {
                    self.entity_ui_recurse(ui, world, child, global_id);
                }
            }
        });

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

        if let (Some(pointer), Some(payload)) = (
            ui.input(|i| i.pointer.interact_pos()),
            header_response.dnd_hover_payload::<EntityPayload>(),
        ) {
            let rect = header_response.rect;
            let stroke = Stroke::new(1.0, ACCENT);

            let parent = world.entity(entity).get::<ChildOf>().unwrap().parent();
            let is_within_parent = parent
                == world
                    .entity(payload.entity)
                    .get::<ChildOf>()
                    .unwrap()
                    .parent();
            let siblings = world.entity(parent).get::<Children>().unwrap();

            let mut drop_index = siblings
                .iter()
                .position(|sibling| *sibling == entity)
                .unwrap();

            let mut drop_target = parent;

            if pointer.y < rect.center().y - 6.0 {
                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                if is_within_parent {
                    drop_index = drop_index.saturating_sub(1);
                }
            } else if pointer.y > rect.center().y + 6.0 {
                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                if !is_within_parent {
                    drop_index += 1;
                }
            } else {
                drop_target = entity;
            }

            if let Some(payload) = header_response.dnd_release_payload::<EntityPayload>() {
                let is_recurse = drop_target == payload.entity
                    || world
                        .query::<&ChildOf>()
                        .query(world)
                        .iter_ancestors(drop_target)
                        .any(|ancestor| ancestor == payload.entity);

                if !is_recurse {
                    if drop_target == entity {
                        world.entity_mut(entity).add_child(payload.entity);
                    } else {
                        world
                            .entity_mut(drop_target)
                            .insert_child(drop_index, payload.entity);
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct SceneTree;

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

    let tree = commands
        .spawn((SceneTree, Visibility::Visible, Transform::IDENTITY))
        .id();

    let scene_handle = scenes.add(scene);

    commands.spawn((
        ChildOf(tree),
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
