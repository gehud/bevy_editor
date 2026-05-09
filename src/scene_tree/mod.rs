use std::{env, fs, ops::Deref, path::PathBuf};

use bevy::{
    app::{App, Plugin, PostUpdate, Startup, Update},
    asset::{AssetPath, AssetServer, Assets, Handle},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::Event,
        hierarchy::{ChildOf, Children},
        message::{Message, MessageReader},
        name::Name,
        observer::On,
        query::{Or, QueryState, With, Without},
        reflect::{AppTypeRegistry, ReflectComponent},
        resource::Resource,
        system::{Commands, Query, Res, ResMut, Single, SystemState},
        world::World,
    },
    gltf::Gltf,
    light::PointLight,
    log::{error, info},
    math::{
        Quat,
        primitives::{Circle, Cuboid},
    },
    mesh::{Mesh, Mesh3d},
    pbr::{MeshMaterial3d, StandardMaterial},
    platform::collections::{HashMap, HashSet},
    scene::{
        DynamicScene, DynamicSceneBuilder, DynamicSceneRoot, InstanceId, Scene, SceneFilter,
        SceneInstance, SceneRoot, SceneSpawner,
    },
    tasks::block_on,
    transform::components::Transform,
    utils::default,
};
use bevy_mod_outline::InheritOutline;
use egui::{
    Button, Color32, CornerRadius, DragAndDrop, Frame, Id, InnerResponse, Key, KeyboardShortcut,
    Label, LayerId, Margin, Modal, Modifiers, Order, RichText, Sense, Stroke, Ui, UiBuilder,
    Widget, collapsing_header::CollapsingState, emath,
};
use lucide_icons::Icon;
use rfd::FileDialog;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{
    asset::AssetDatabase,
    asset_browser::AssetPayload,
    assets::icons::MaterialIcon,
    panel::{Panel, PanelApp},
    prefs::{RegisterPref, Save},
    properties::ComponentIgnore,
    scene::{EditorScene, serde::ser::SceneSerializer},
    selection::{EntitySelection, SelectionMap},
    style::ACCENT,
    utils::paint_collapsing_button,
};

pub const DESPAWN_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);

#[derive(Message)]
pub struct MarkSceneDirty;

pub struct EntityPayload {
    pub entity: Entity,
}

pub struct SceneTreePane;

impl Panel for SceneTreePane {
    fn name(&self) -> &str {
        "Scene Tree"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let (root, state) = world.query::<(Entity, &InspectedScene)>().single(world)?;

        let is_dirty = state.dirty;

        let name = world
            .entity(root)
            .get::<Name>()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "untiteled".into());

        ui.heading(format!("{}{}", name, if is_dirty { "*" } else { "" }));

        ui.separator();

        let children = world
            .entity(root)
            .get::<Children>()
            .map(|children| children.to_vec())
            .unwrap_or_default();

        for child in children {
            entity_ui_recurse(ui, world, child, ui.id());
        }

        let mut frame = Frame::new().begin(ui);
        frame.content_ui.take_available_space();
        let response = frame.allocate_space(ui).interact(Sense::click());

        if response.clicked() {
            world.resource_mut::<SelectionMap>().clear();
        }

        response.context_menu(|ui| {
            if ui.button("Spawn Entity").clicked() {
                world.spawn(ChildOf(root));
            }
        });

        let is_dragging_scene = response.dnd_hover_payload::<AssetPayload>().is_some()
            && world.resource::<DraggedSceneRoot>().0.is_some();

        if is_dragging_scene {
            frame.frame.fill = ui.style().visuals.widgets.active.bg_fill;
            frame.frame.stroke = ui.style().visuals.widgets.active.bg_stroke;

            if response.dnd_release_payload::<AssetPayload>().is_some() {
                let dragged = world.resource_mut::<DraggedSceneRoot>().0.take().unwrap();
                world
                    .entity_mut(dragged)
                    .insert(Visibility::Visible)
                    .insert(Transform::IDENTITY);

                world.entity_mut(root).add_child(dragged);
                world.write_message(MarkSceneDirty);
            }
        }

        frame.paint(ui);

        if ui.input_mut(|input| input.consume_shortcut(&DESPAWN_SHORTCUT)) {
            despawn_selected(world);
        }

        Ok(())
    }
}

fn entity_ui_recurse(ui: &mut Ui, world: &mut World, entity: Entity, id: Id) {
    let name = world
        .entity(entity)
        .get::<Name>()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "Entity".into());

    let global_id = id;

    let id = id.with(entity);

    let mut collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), id.with("collapsing"), false);

    let is_scene = world.entity(entity).contains::<SceneRoot>();
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
                    .inner_margin(Margin::symmetric(8, 4))
                    .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                    .corner_radius(CornerRadius::same(4));

                if ui.rect_contains_pointer(response.rect) {
                    frame.fill = ui.style().visuals.widgets.hovered.bg_fill;
                }

                if world
                    .resource_mut::<SelectionMap>()
                    .is_selected(&EntitySelection::new(entity))
                {
                    frame.stroke.color = ACCENT;
                }

                ui.set_height(24.0);
                frame.show(ui, |ui| {
                    ui.take_available_width();
                    ui.horizontal(|ui| {
                        if world.entity(entity).contains::<Children>() {
                            collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                        } else {
                            ui.add_space(ui.spacing().indent + ui.spacing().item_spacing.x);
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

    header_response.context_menu(|ui| {
        if Button::new("Despawn")
            .shortcut_text(ui.ctx().format_shortcut(&DESPAWN_SHORTCUT))
            .ui(ui)
            .clicked()
        {
            despawn_selected(world);
        }
    });

    if world.get_entity(entity).is_err() {
        return;
    }

    collapsing_state.show_body_unindented(ui, |ui| {
        if let Some(children) = world
            .query::<&Children>()
            .get(world, entity)
            .ok()
            .map(|children| children.to_vec())
        {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().item_spacing.x);
                ui.vertical(|ui| {
                    ui.indent(id.with("children"), |ui| {
                        for child in children {
                            entity_ui_recurse(ui, world, child, global_id);
                        }
                    });
                });
            });
        }
    });

    let mut selection_map = world.resource_mut::<SelectionMap>();

    if header_response.clicked() {
        if !ui.input(|i| i.modifiers.ctrl) {
            selection_map.clear();
            selection_map.select(EntitySelection::new(entity));
        } else {
            if selection_map.is_selected(&EntitySelection::new(entity)) {
                selection_map.deselect(&EntitySelection::new(entity));
            } else {
                selection_map.select(EntitySelection::new(entity));
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

                world.write_message(MarkSceneDirty);
            }
        }
    }
}

fn despawn_selected(world: &mut World) {
    if let Some(selected) = world
        .resource::<SelectionMap>()
        .of_type::<EntitySelection>()
        .map(|selected| {
            selected
                .map(|selection| selection.entity)
                .collect::<Vec<_>>()
        })
    {
        for entity in selected {
            if let Ok(entity) = world.get_entity_mut(entity) {
                entity.despawn();
                world.write_message(MarkSceneDirty);
            }
        }
    }
}

#[derive(Message)]
pub struct OpenScene;

#[derive(Message)]
pub struct SaveScene;

#[derive(Default, Resource)]
struct OpenedScene(Option<PathBuf>);

#[derive(Default, Component)]
pub(crate) struct InspectedScene {
    pub dirty: bool,
}

#[derive(Default, Resource)]
pub(crate) struct DraggedSceneRoot(pub Option<Entity>);

pub(crate) fn scene_name<'a>(asset_path: Option<&AssetPath<'a>>) -> String {
    asset_path
        .and_then(|path| {
            path.path()
                .with_extension("")
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "untitled".into())
}

fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scenes: ResMut<Assets<Scene>>,
    mut commands: Commands,
) {
    let mut world = World::new();

    world.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    world.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    world.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    let handle = scenes.add(Scene::new(world));

    commands.spawn((
        InspectedScene { dirty: false },
        Name::new(scene_name(handle.path())),
        SceneRoot(handle),
    ));
}

#[derive(Default, Resource)]
struct WaitingScene(Option<Handle<EditorScene>>);

fn open_scene(world: &mut World, state: &mut SystemState<MessageReader<OpenScene>>) -> Result {
    let mut requests = state.get_mut(world);

    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    let current_dir = env::current_dir()?;

    let Some(path) = FileDialog::new()
        .add_filter("Scene", &["asn"])
        .set_directory(current_dir.join("assets"))
        .pick_file()
    else {
        return Ok(());
    };

    let asset_path = path.strip_prefix(current_dir.join("assets"))?.to_path_buf();

    let asset_scene = world
        .resource::<AssetServer>()
        .load::<EditorScene>(asset_path);

    world.resource_mut::<WaitingScene>().0.replace(asset_scene);

    state.apply(world);

    Ok(())
}

fn wait_scene(world: &mut World) -> Result {
    let Some(waiting_scene) = world.resource::<WaitingScene>().0.clone() else {
        return Ok(());
    };

    let Some(asset_scene) = world
        .resource_mut::<Assets<EditorScene>>()
        .remove(&waiting_scene)
    else {
        return Ok(());
    };

    world.resource_mut::<WaitingScene>().0 = None;

    let name = scene_name(waiting_scene.path());
    let scene = asset_scene.scene;

    let handle = world.resource_mut::<Assets<DynamicScene>>().add(scene);

    let inspected_scene = world
        .query_filtered::<Entity, With<InspectedScene>>()
        .single(world)?;
    world.entity_mut(inspected_scene).despawn();
    world.spawn((
        InspectedScene { dirty: false },
        Name::new(name),
        DynamicSceneRoot(handle),
    ));

    let path = waiting_scene.path().unwrap().path().to_path_buf();

    world.resource_mut::<OpenedScene>().0 = Some(path);

    Ok(())
}

fn collect_scene_entities(
    entities: &mut Vec<Entity>,
    restore_children: &mut HashMap<Entity, Children>,
    root: Entity,
    world: &mut World,
) {
    for child in world
        .query::<&Children>()
        .get(world, root)
        .ok()
        .map(|children| children.to_vec())
        .unwrap_or_default()
    {
        entities.push(child);

        let child_ref = world.entity(child);
        if child_ref.contains::<SceneRoot>() || child_ref.contains::<DynamicSceneRoot>() {
            if let Some(children) = world.entity_mut(child).take::<Children>() {
                restore_children.insert(child, children);
            }

            continue;
        }

        collect_scene_entities(entities, restore_children, child, world);
    }
}

fn save_scene(world: &mut World, state: &mut SystemState<MessageReader<SaveScene>>) -> Result {
    let mut requests = state.get_mut(world);

    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    if !world.query::<&InspectedScene>().single(world)?.dirty {
        return Ok(());
    }

    let current_dir = env::current_dir()?;

    let Some(path) = world
        .resource::<OpenedScene>()
        .0
        .clone()
        .map(|path| current_dir.join("assets").join(path))
        .or_else(|| {
            FileDialog::new()
                .add_filter("Scene", &["asn"])
                .set_file_name("my_scene.asn")
                .set_directory(current_dir.join("assets"))
                .save_file()
        })
    else {
        return Ok(());
    };

    let root = world
        .query_filtered::<Entity, With<InspectedScene>>()
        .single(world)?;

    let roots = world
        .query::<&Children>()
        .get(world, root)
        .ok()
        .map(|children| children.to_vec())
        .unwrap_or_default();

    let mut entities = Vec::new();
    let mut restore_children = HashMap::new();

    collect_scene_entities(&mut entities, &mut restore_children, root, world);

    world.entity_mut(root).detach_all_children();

    let mut builder = DynamicSceneBuilder::from_world(world);

    let component_ignore = world.resource::<ComponentIgnore>();

    let allowed_components = world
        .resource::<AppTypeRegistry>()
        .read()
        .iter()
        .filter_map(|registration| {
            if registration.data::<ReflectComponent>().is_some() {
                if component_ignore
                    .serialization_ignore()
                    .contains(&registration.type_id())
                {
                    None
                } else {
                    Some(registration.type_id())
                }
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();

    builder.component_filter = SceneFilter::Allowlist(allowed_components);

    let scene = builder.extract_entities(entities.iter().cloned()).build();

    world.entity_mut(root).add_children(&roots);
    for (entity, children) in restore_children {
        world.entity_mut(entity).add_children(&children);
    }

    let output = {
        let type_registry = world.resource::<AppTypeRegistry>().read();
        let asset_database = world.resource::<AssetDatabase>();
        let serializer = SceneSerializer::new(&scene, &type_registry, &asset_database);
        ron::ser::to_string_pretty(&serializer, PrettyConfig::default())?
    };

    fs::write(&path, output)?;

    let asset_path = path.strip_prefix(current_dir.join("assets"))?.to_path_buf();

    world
        .query::<&mut InspectedScene>()
        .single_mut(world)?
        .dirty = false;
    world.resource_mut::<OpenedScene>().0 = Some(asset_path);

    state.apply(world);

    Ok(())
}

fn mark_scene_dirty(
    mut requests: MessageReader<MarkSceneDirty>,
    mut inspected_scene: Single<&mut InspectedScene>,
) {
    if requests.is_empty() {
        return;
    }

    requests.clear();

    inspected_scene.dirty = true;
}

pub struct SceneTreePlugin;

impl Plugin for SceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DraggedSceneRoot>()
            .init_resource::<WaitingScene>()
            .init_resource::<OpenedScene>()
            .register_panel(SceneTreePane)
            .add_message::<OpenScene>()
            .add_message::<SaveScene>()
            .add_message::<MarkSceneDirty>()
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                (open_scene, save_scene, wait_scene, mark_scene_dirty),
            );
    }
}
