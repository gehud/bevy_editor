use std::{any::TypeId, f32, ops::DerefMut, path::Path};

use bevy::{
    app::{App, Plugin},
    camera::{
        primitives::{Aabb, CubemapFrusta},
        visibility::{
            CubemapVisibleEntities, InheritedVisibility, ViewVisibility, Visibility,
            VisibilityClass,
        },
    },
    ecs::{
        change_detection::{DetectChanges, DetectChangesMut},
        component::{Component, ComponentId},
        entity::Entity,
        error::Result,
        hierarchy::{ChildOf, Children},
        name::Name,
        reflect::AppTypeRegistry,
        resource::Resource,
        world::{CommandQueue, World},
    },
    picking::{events::Scroll, hover::PickingInteraction},
    platform::collections::{HashMap, HashSet},
    reflect::TypeRegistry,
    render::sync_world::{RenderEntity, SyncToRenderWorld},
    transform::components::{GlobalTransform, TransformTreeChanged},
};
use egui::{
    Button, Color32, Frame, Id, InnerResponse, Label, Margin, Response, ScrollArea, TextEdit,
    TextureId, Ui, Vec2, Widget, Window,
    collapsing_header::{CollapsingState, paint_default_icon},
};
use lucide_icons::Icon;

use crate::{
    assets::icons::MaterialIcon,
    inspection::{
        self, error,
        reflect_inspector::{Context, InspectorUi},
        restricted_world_view::{ReflectBorrow, RestrictedWorldView},
        utils::{self, pretty_type_name, pretty_type_name_str},
    },
    pane::{Pane, RegisterPane},
    selection::{EntitySelection, SelectionMap},
    utils::paint_collapsing_button,
};

#[derive(Default, Resource)]
pub struct ComponentIgnore {
    ids: HashSet<TypeId>,
}

impl ComponentIgnore {
    fn insert<C: Component>(&mut self) -> &mut Self {
        self.ids.insert(TypeId::of::<C>());
        self
    }
}

pub struct PropertiesPane;

impl Pane for PropertiesPane {
    fn name(&self) -> &str {
        "Properties"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let selection_map = world.resource::<SelectionMap>();

        if selection_map.type_ids().len() > 1 {
            ui.heading("Selected");
            ui.separator();

            for (_, items) in selection_map.iter() {
                ui.label(format!("{} x{}", items.label(), items.len()));
            }
        } else if selection_map.type_ids().len() == 1 {
            if let Some(selected) = selection_map
                .of_type::<EntitySelection>()
                .map(|selected| selected.map(|item| item.entity).collect::<Vec<_>>())
            {
                let id = ui.id();

                match selected.as_slice() {
                    &[entity] => ui_for_entity(ui, world, entity, id)?,
                    entities => ui_for_entities(ui, world, entities, id)?,
                }
            }
        }

        Ok(())
    }
}

fn ui_for_entity_name(ui: &mut Ui, world: &mut World, entity: Entity) -> Result {
    ui.horizontal(|ui| {
        Label::new(MaterialIcon::new(Icon::Box).rich_text().size(26.0))
            .selectable(false)
            .ui(ui);

        let mut entity_mut = world.entity_mut(entity);

        if entity_mut.contains::<Name>() {
            if Button::new(MaterialIcon::new(Icon::Minus).rich_text().size(12.0))
                .small()
                .min_size(Vec2::new(18.0, 18.0))
                .ui(ui)
                .clicked()
            {
                entity_mut.remove::<Name>();
            }
        } else {
            if Button::new(MaterialIcon::new(Icon::Plus).rich_text().size(12.0))
                .small()
                .min_size(Vec2::new(18.0, 18.0))
                .ui(ui)
                .clicked()
            {
                entity_mut.insert(Name::new("Entity"));
            }
        }

        if let Some(mut name) = entity_mut.get_mut::<Name>() {
            name.mutate(|name| {
                TextEdit::singleline(name)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(8, 4))
                    .show(ui);
            });
        } else {
            ui.add_enabled_ui(false, |ui| {
                let mut name = String::from("Entity");
                TextEdit::singleline(&mut name)
                    .desired_width(f32::INFINITY)
                    .margin(Margin::symmetric(8, 4))
                    .show(ui);
            });
        }
    });

    Ok(())
}

fn ui_for_entity(ui: &mut Ui, world: &mut World, entity: Entity, id: Id) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let id = id.with(entity);

    ui_for_entity_name(ui, world, entity)?;
    ui.separator();

    let mut queue = CommandQueue::default();
    ui_for_entity_components(
        &mut world.into(),
        &mut queue,
        entity,
        ui,
        id,
        &type_registry,
    )?;

    queue.apply(world);

    Ok(())
}

fn ui_for_entities(ui: &mut Ui, world: &mut World, entities: &[Entity], id: Id) -> Result {
    // let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    // let type_registry = type_registry.read();

    // let Some(&first) = entities.first() else {
    //     return Ok(());
    // };

    // let Ok(mut components) = get_entity_component_data(&mut world.into(), first, &type_registry)
    // else {
    //     error::nonexistent_entity(ui, first);
    //     return Ok(());
    // };

    // for &entity in entities.iter().skip(1) {
    //     components.retain(|data| {
    //         world
    //             .get_entity(entity)
    //             .map_or(true, |entity| entity.contains_id(data.component_id))
    //     })
    // }

    // let (resources_view, components_view) = RestrictedWorldView::resources_components(world);
    // let mut queue = CommandQueue::default();
    // let mut cx = Context {
    //     world: resources_view,
    //     queue: &mut queue,
    // };
    // let mut env = InspectorUi::new(&type_registry, &mut cx);

    // let id = egui::Id::NULL;
    // for data in components {
    //     let id = id.with(data.component_id);
    //     let inner = egui::CollapsingHeader::new(&data.name)
    //         .id_salt(id)
    //         .show(ui, |ui| -> Result {
    //             if data.size == 0 {
    //                 return Ok(());
    //             }

    //             let mut values = Vec::with_capacity(entities.len());

    //             for (i, &entity) in entities.iter().enumerate() {
    //                 // skip duplicate entities
    //                 if entities[0..i].contains(&entity) {
    //                     continue;
    //                 };

    //                 // SAFETY: entities are distinct, env has a context with just resources
    //                 match unsafe {
    //                     components_view.get_entity_component_reflect_unchecked(
    //                         entity,
    //                         data.type_id,
    //                         &type_registry,
    //                     )
    //                 } {
    //                     Ok(value) => {
    //                         values.push(value);
    //                     }
    //                     Err(_) => {
    //                         continue;
    //                     }
    //                 }
    //             }

    //             let mut values_reflect: Vec<_> = values
    //                 .iter_mut()
    //                 .map(|value| value.bypass_change_detection().as_partial_reflect_mut())
    //                 .collect();
    //             let changed = env.ui_for_reflect_many_with_options(
    //                 data.type_id,
    //                 &data.name,
    //                 ui,
    //                 id.with(data.component_id),
    //                 &(),
    //                 values_reflect.as_mut_slice(),
    //             )?;
    //             if changed {
    //                 for value in values.iter_mut() {
    //                     value.set_changed();
    //                 }
    //             }

    //             Ok(())
    //         })
    //         .body_returned;

    //     if let Some(inner) = inner {
    //         inner?;
    //     }
    // }

    // queue.apply(world);

    Ok(())
}

fn ui_for_entity_components(
    world: &mut RestrictedWorldView<'_>,
    queue: &mut CommandQueue,
    entity: Entity,
    ui: &mut egui::Ui,
    id: Id,
    type_registry: &TypeRegistry,
) -> Result {
    let Ok(components) = get_entity_component_data(world, entity, type_registry) else {
        error::nonexistent_entity(ui, entity);
        return Ok(());
    };

    let tags = components.iter().filter(|data| data.size == 0);
    let components = components.iter().filter(|data| data.size != 0);

    let mut tags_collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), id.with("tags_collapsing"), true);

    Frame::new()
        .fill(ui.style().visuals.widgets.inactive.bg_fill)
        .inner_margin(Margin::symmetric(10, 5))
        .corner_radius(5)
        .show(ui, |ui| {
            ui.take_available_width();
            ui.horizontal(|ui| {
                tags_collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                ui.heading("Tags");
            });
        });

    let mut component_to_remove = None;

    tags_collapsing_state.show_body_unindented(ui, |ui| {
        ui.take_available_width();
        ui.set_max_height(50.0);
        ScrollArea::new([false, true]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for data in tags {
                    Frame::new()
                        .fill(ui.style().visuals.widgets.inactive.bg_fill)
                        .inner_margin(Margin::same(5))
                        .corner_radius(8)
                        .show(ui, |ui| {
                            #[cfg(feature = "documentation")]
                            let type_docs = type_registry
                                .get_type_info(data.type_id)
                                .and_then(|info| info.docs());

                            let _response = ui.label(&data.name);
                            #[cfg(feature = "documentation")]
                            crate::inspection::egui_utils::show_docs(_response, type_docs);

                            if Button::new(MaterialIcon::new(Icon::X))
                                .small()
                                .ui(ui)
                                .clicked()
                            {
                                component_to_remove = Some(data.type_id);
                            }
                        });
                }
            });
        });
    });

    let mut components_collapsing_state =
        CollapsingState::load_with_default_open(ui.ctx(), id.with("components_collapsing"), true);

    Frame::new()
        .fill(ui.style().visuals.widgets.inactive.bg_fill)
        .inner_margin(Margin::symmetric(10, 5))
        .corner_radius(5)
        .show(ui, |ui| {
            ui.take_available_width();
            ui.horizontal(|ui| {
                components_collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                ui.heading("Components");
            });
        });

    let inner = components_collapsing_state.show_body_unindented(ui, |ui| -> Result {
        for data in components {
            let id = Id::new(data.component_id);

            #[cfg(feature = "documentation")]
            let type_docs = type_registry
                .get_type_info(data.type_id)
                .and_then(|info| info.docs());

            let mut collapsing_state = CollapsingState::load_with_default_open(ui.ctx(), id, true);

            // create a context with access to the world except for the currently viewed component
            let (mut component_view, world) = world.split_off_component((entity, data.type_id));
            let mut cx = Context {
                world: world,
                queue: queue,
            };

            let value = match component_view.get_entity_component_reflect(
                entity,
                data.type_id,
                type_registry,
            ) {
                Ok(value) => value,
                Err(_) => {
                    continue;
                }
            };

            let changed_by = match &value {
                ReflectBorrow::Mutable(val) => val.changed_by().into_option(),
                ReflectBorrow::Immutable(_) => None,
            };

            let InnerResponse {
                inner: header_response,
                ..
            } = Frame::new()
                .stroke(ui.style().visuals.widgets.open.bg_stroke)
                .corner_radius(ui.style().visuals.widgets.active.corner_radius)
                .show(ui, |ui| -> Result<Response> {
                    let InnerResponse {
                        response: header_response,
                        ..
                    } = Frame::new()
                        .fill(ui.style().visuals.widgets.active.bg_fill)
                        .inner_margin(ui.style().spacing.button_padding)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                collapsing_state.show_toggle_button(ui, paint_default_icon);
                                ui.vertical_centered(|ui| {
                                    ui.label(&data.name);
                                });
                            });
                        });

                    let inner = collapsing_state.show_body_unindented(ui, |ui| -> Result {
                        Frame::new()
                            .inner_margin(ui.style().spacing.button_padding)
                            .show(ui, |ui| -> Result {
                                let mut env = InspectorUi::new(type_registry, &mut cx);
                                let id = id.with(data.component_id);
                                let options = &();

                                match value {
                                    ReflectBorrow::Mutable(mut value) => {
                                        let changed = env.ui_for_reflect_with_options(
                                            value
                                                .bypass_change_detection()
                                                .as_partial_reflect_mut(),
                                            ui,
                                            id,
                                            options,
                                        )?;

                                        if changed {
                                            value.set_changed();
                                        }
                                    }
                                    ReflectBorrow::Immutable(value) => env
                                        .ui_for_reflect_readonly_with_options(
                                            value.as_partial_reflect(),
                                            ui,
                                            id,
                                            options,
                                        )?,
                                };

                                Ok(())
                            })
                            .inner
                    });

                    if let Some(inner) = inner {
                        inner.inner?;
                    }

                    Ok(header_response)
                });

            let response = header_response?;

            if let Some(location) = changed_by {
                response.context_menu(|ui| {
                    ui.label("Last change:");
                    let path = Path::new(location.file());
                    let pretty = utils::trim_cargo_registry_path(path);

                    if ui
                        .button(format!(
                            "{}:{}:{}",
                            pretty.as_deref().unwrap_or(path).display(),
                            location.line(),
                            location.column()
                        ))
                        .clicked()
                    {
                        if let Err(e) = utils::open_file_at(location) {
                            bevy::log::error!("Failed to open last change location: {}", e);
                        } else {
                            bevy::log::info!("Successfully opened {location}");
                        }
                    }
                });
            }

            #[cfg(feature = "documentation")]
            crate::inspection::egui_utils::show_docs(response, type_docs);
        }

        Ok(())
    });

    if let Some(inner) = inner {
        inner.inner?;
    }

    Ok(())
}

struct ComponentData {
    name: String,
    component_id: ComponentId,
    type_id: TypeId,
    size: usize,
}

fn get_entity_component_data(
    world: &mut RestrictedWorldView<'_>,
    entity: Entity,
    type_registry: &TypeRegistry,
) -> Result<Vec<ComponentData>> {
    let mut split = world.split_off_resource(TypeId::of::<ComponentIgnore>());

    let component_ignore = split.0.get_resource_mut::<ComponentIgnore>()?;
    let world = split.1;

    let entity_ref = world.world().get_entity(entity)?;

    let archetype = entity_ref.archetype();
    let mut components: Vec<_> = archetype
        .components()
        .iter()
        .filter_map(|component_id| {
            let info = world.world().components().get_info(*component_id)?;

            let type_id = info.type_id()?;

            if !type_registry.contains(type_id.clone()) {
                return None;
            }

            if component_ignore.ids.contains(&type_id.clone()) {
                return None;
            }

            let name = pretty_type_name_str(&info.name().to_string());

            Some(ComponentData {
                name,
                component_id: *component_id,
                type_id: type_id,
                size: info.layout().size(),
            })
        })
        .collect();

    components.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(components)
}

pub trait PropertiesApp {
    fn ignore_component<C: Component>(&mut self) -> &mut Self;
}

impl PropertiesApp for App {
    fn ignore_component<C: Component>(&mut self) -> &mut Self {
        self.world_mut()
            .resource_mut::<ComponentIgnore>()
            .insert::<C>();
        self
    }
}

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComponentIgnore>()
            .ignore_component::<Name>()
            .ignore_component::<ChildOf>()
            .ignore_component::<Children>()
            .ignore_component::<Aabb>()
            .ignore_component::<GlobalTransform>()
            .ignore_component::<Visibility>()
            .ignore_component::<InheritedVisibility>()
            .ignore_component::<PickingInteraction>()
            .ignore_component::<RenderEntity>()
            .ignore_component::<ViewVisibility>()
            .ignore_component::<VisibilityClass>()
            .ignore_component::<CubemapFrusta>()
            .ignore_component::<CubemapVisibleEntities>()
            .ignore_component::<SyncToRenderWorld>()
            .ignore_component::<TransformTreeChanged>()
            .register_pane(PropertiesPane);
    }
}
