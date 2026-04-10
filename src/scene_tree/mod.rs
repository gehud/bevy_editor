use bevy::{
    app::{App, Plugin, Startup},
    asset::Assets,
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        name::Name,
        system::{Commands, ResMut},
        world::World,
    },
    light::PointLight,
    math::{
        Quat,
        primitives::{Circle, Cuboid},
    },
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    scene::{InstanceId, Scene, SceneSpawner},
    transform::components::Transform,
    utils::default,
};
use egui::{Id, RichText, Sense, Ui, collapsing_header::CollapsingState};

use crate::{
    pane::{Pane, RegisterPane},
    selection::{Selection, SelectionMap},
};

pub struct SceneTreePane;

impl Pane for SceneTreePane {
    fn name(&self) -> &str {
        "Scene Tree"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let (root, instance_id) = world
            .query::<(Entity, &InspectedScene)>()
            .single(world)
            .map(|(entity, scene)| (entity, scene.instance))?;

        let scene_name = world
            .query::<&Name>()
            .get(world, root)
            .map(|name| name.to_string())
            .ok()
            .unwrap_or_else(|| "Unnamed".into());

        ui.label(RichText::from(scene_name).heading());
        ui.separator();

        if !world
            .resource::<SceneSpawner>()
            .instance_is_ready(instance_id)
        {
            ui.label("Loading...");
            return Ok(());
        }

        let id = ui.make_persistent_id("scene_tree");

        if let Some(children) = world
            .query::<&Children>()
            .get(world, root)
            .ok()
            .map(|children| children.to_vec())
        {
            for (i, entity) in children.iter().enumerate() {
                self.entity_ui_recurse(ui, world, *entity, id.with(i));
            }
        }

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
            CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    self.entity_ui_header(ui, world, entity, &name);
                })
                .body(|ui| {
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
struct InspectedScene {
    instance: InstanceId,
}

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

    let scene_handle = scenes.add(scene);

    let root = commands.spawn(Name::new("Unnamed")).id();

    let instance = scene_spawner.spawn_as_child(scene_handle, root);

    commands.entity(root).insert((
        Visibility::Visible,
        Transform::IDENTITY,
        InspectedScene { instance },
    ));
}

pub struct SceneTreePlugin;

impl Plugin for SceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.register_pane(SceneTreePane).add_systems(Startup, setup);
    }
}
