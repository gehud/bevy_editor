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
        query::{With, Without},
        system::{Commands, ResMut},
        world::World,
    },
    gltf::Gltf,
    light::PointLight,
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
use egui::{Frame, Id, RichText, Sense, Ui, collapsing_header::CollapsingState};

use crate::{
    asset_browser::AssetPayload,
    pane::{Pane, RegisterPane},
    selection::{Selection, SelectionMap},
    utils::paint_collapsing_button,
};

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
            if world.entity(root).contains::<SceneRoot>() {
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
                world.resource_mut::<SceneSpawner>().spawn_as_child(scene, tree);
            }
        }

        frame.paint(ui);

        Ok(())
    }
}

impl SceneTreePane {
    fn entity_ui_recurse(&mut self, ui: &mut Ui, world: &mut World, entity: Entity, id: Id) {
        let name = world
            .query::<&Name>()
            .get(world, entity)
            .map(|name| name.to_string())
            .ok()
            .unwrap_or_else(|| "Entity".into());

        if let Some(children) = world
            .query::<&Children>()
            .get(world, entity)
            .ok()
            .map(|children| children.to_vec())
        {
            let mut collapsing_state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
            let header_response = ui
                .horizontal(|ui| {
                    collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                    self.entity_ui_header(ui, world, entity, &name);
                })
                .response;

            collapsing_state.show_body_indented(&header_response, ui, |ui| {
                for (i, child) in children.iter().enumerate() {
                    self.entity_ui_recurse(ui, world, *child, id.with(i));
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                self.entity_ui_header(ui, world, entity, &name);
            });
        }
    }

    fn entity_ui_header(&mut self, ui: &mut Ui, world: &mut World, entity: Entity, name: &str) {
        let mut selection_map = world.resource_mut::<SelectionMap>();

        let is_selected = selection_map.is_selected(entity);

        if ui.selectable_label(is_selected, name).clicked() {
            if !ui.input(|i| i.modifiers.ctrl) {
                selection_map.clear();
                selection_map.select(entity);
            } else {
                if is_selected {
                    selection_map.deselect(entity);
                } else {
                    selection_map.select(entity);
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
    mut scene_spawner: ResMut<SceneSpawner>,
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

    let scene_tree = commands
        .spawn((SceneTree, Visibility::Visible, Transform::IDENTITY))
        .id();

    let scene_handle = scenes.add(scene);

    let scene_root = commands
        .spawn((
            ChildOf(scene_tree),
            Name::new("Untiteled"),
            Visibility::Visible,
            Transform::IDENTITY,
        ))
        .id();

    scene_spawner.spawn_as_child(scene_handle, scene_root);
}

pub struct SceneTreePlugin;

impl Plugin for SceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(SceneTreePane).add_systems(Startup, setup);
    }
}
