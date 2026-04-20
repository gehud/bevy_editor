use std::{any::TypeId, f32, ops::DerefMut, path::Path};

use bevy::{
    app::{App, Plugin, Startup},
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
        reflect::{AppTypeRegistry, ReflectComponent},
        resource::Resource,
        system::{Commands, Res},
        world::{CommandQueue, World},
    },
    picking::{events::Scroll, hover::PickingInteraction},
    platform::collections::{HashMap, HashSet},
    reflect::{TypePathTable, TypeRegistry, prelude::ReflectDefault},
    render::sync_world::{RenderEntity, SyncToRenderWorld},
    transform::components::{GlobalTransform, Transform, TransformTreeChanged},
};
use egui::{
    Align, Button, Color32, Frame, Grid, Id, InnerResponse, Label, Layout, Margin, Popup, Response,
    ScrollArea, Sense, SetOpenCommand, TextEdit, TextWrapMode, TextureId, Ui, Vec2, Widget, Window,
    collapsing_header::{CollapsingState, paint_default_icon},
    output::OutputEvent,
    vec2,
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
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

    add_component_ui(world, &mut queue, entity, ui, &type_registry);
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

fn add_component_ui(
    world: &mut World,
    queue: &mut CommandQueue,
    entity: Entity,
    ui: &mut Ui,
    type_registry: &TypeRegistry,
) {
    let response = ui
        .allocate_ui(vec2(ui.available_width(), 24.0), |ui| {
            Frame::new()
                .fill(ui.style().visuals.widgets.inactive.bg_fill)
                .inner_margin(Margin::symmetric(10, 5))
                .corner_radius(5)
                .show(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.columns(3, |ui| {
                            ui[1].horizontal_centered(|ui| {
                                ui.add(MaterialIcon::new(Icon::PackagePlus));
                                ui.add(Label::new("Add Component").selectable(false));
                            });
                        });
                    });
                });
        })
        .response
        .interact(Sense::click());

    let mut menu = world.resource_mut::<AddComponentMenu>();

    let clicked = response.clicked();
    Popup::menu(&response)
        .open_memory(if clicked {
            Some(SetOpenCommand::Bool(true))
        } else {
            None
        })
        .at_pointer_fixed()
        .show(|ui| {
            let search_response = TextEdit::singleline(&mut menu.search)
                .prefix(Icon::Search.unicode().to_string())
                .ui(ui);

            if clicked {
                menu.search = String::new();
                search_response.request_focus();
            }

            ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                let matcher = SkimMatcherV2::default();

                let mut items = menu.items.clone();

                items.sort_by(|a, b| {
                    let a = matcher.fuzzy_match(a, &menu.search).unwrap_or_default();
                    let b = matcher.fuzzy_match(b, &menu.search).unwrap_or_default();
                    b.cmp(&a)
                });

                for path in items {
                    if ui.button(&path).clicked() {
                        let entity = entity;
                        let data = &menu.map[&path];
                        let component = data.component.clone();
                        let default = data.default.clone();
                        queue.push(move |world: &mut World| {
                            let type_registry = world.resource::<AppTypeRegistry>().clone();
                            let type_registry = type_registry.read();
                            let mut entity = world.entity_mut(entity);
                            component.insert(
                                &mut entity,
                                default.default().as_partial_reflect(),
                                &type_registry,
                            );
                        });
                    }
                }
            });
        });
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

    ui.allocate_ui(vec2(ui.available_width(), 24.0), |ui| {
        Frame::new()
            .fill(ui.style().visuals.widgets.inactive.bg_fill)
            .inner_margin(Margin::symmetric(10, 5))
            .corner_radius(5)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    tags_collapsing_state.show_toggle_button(ui, paint_collapsing_button);
                    ui.vertical_centered(|ui| {
                        ui.label("Tags");
                    });
                });
            });
    });

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
                                let entity = entity;
                                let component_id = data.component_id;
                                queue.push(move |world: &mut World| {
                                    world.entity_mut(entity).remove_by_id(component_id);
                                });
                            }
                        });
                }
            });
        });
    });

    for data in components {
        let id = Id::new(data.component_id);

        #[cfg(feature = "documentation")]
        let type_docs = type_registry
            .get_type_info(data.type_id)
            .and_then(|info| info.docs());

        let mut collapsing_state =
            CollapsingState::load_with_default_open(ui.ctx(), id.with("collapsing"), true);

        let (mut component_view, world) = world.split_off_component((entity, data.type_id));

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

        let header_response = Frame::new()
            .stroke(ui.style().visuals.widgets.open.bg_stroke)
            .corner_radius(5)
            .show(ui, |ui| -> Result<Response> {
                let header_response = ui
                    .allocate_ui(vec2(ui.available_width(), 24.0), |ui| {
                        Frame::new()
                            .fill(ui.style().visuals.widgets.inactive.bg_fill)
                            .inner_margin(Margin::symmetric(10, 5))
                            .show(ui, |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.columns(3, |ui| {
                                        ui[0].with_layout(Layout::top_down(Align::Min), |ui| {
                                            collapsing_state
                                                .show_toggle_button(ui, paint_collapsing_button);
                                        });

                                        ui[1].with_layout(Layout::top_down(Align::Center), |ui| {
                                            ui.label(&data.name)
                                        });

                                        ui[2].with_layout(Layout::top_down(Align::Max), |ui| {
                                            if ui
                                                .add(MaterialIcon::new(Icon::X))
                                                .interact(Sense::click())
                                                .clicked()
                                            {
                                                let entity = entity;
                                                let component_id = data.component_id;
                                                queue.push(move |world: &mut World| {
                                                    world
                                                        .entity_mut(entity)
                                                        .remove_by_id(component_id);
                                                });
                                            }
                                        });
                                    });
                                });
                            })
                    })
                    .response;

                let inner = collapsing_state.show_body_unindented(ui, |ui| -> Result {
                    Frame::new()
                        .inner_margin(7)
                        .show(ui, |ui| {
                            ScrollArea::horizontal()
                                .show(ui, |ui| -> Result {
                                    let mut ctx = Context { world, queue };
                                    let mut env = InspectorUi::new(type_registry, &mut ctx);
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
                        })
                        .inner
                });

                if let Some(inner) = inner {
                    inner.inner?;
                }

                Ok(header_response)
            })
            .inner?;

        #[cfg(feature = "documentation")]
        crate::inspection::egui_utils::show_docs(header_response, type_docs);
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

    if let Some(transform_index) = components
        .iter()
        .position(|item| item.type_id == TypeId::of::<Transform>())
    {
        components.swap(transform_index, 0);
    }

    Ok(components)
}

#[derive(Clone)]
struct AddComponentItem {
    default: ReflectDefault,
    component: ReflectComponent,
}

#[derive(Resource)]
struct AddComponentMenu {
    search: String,
    items: Vec<String>,
    map: HashMap<String, AddComponentItem>,
}

fn collect_add_component_tree(
    type_registry: Res<AppTypeRegistry>,
    component_ignore: Res<ComponentIgnore>,
    mut commands: Commands,
) {
    let type_registry = type_registry.read();

    let map = type_registry
        .iter()
        .filter_map(|registration| {
            let component = registration.data::<ReflectComponent>()?;

            if component_ignore.ids.contains(&registration.type_id()) {
                return None;
            }

            let default = registration.data::<ReflectDefault>()?;

            Some((
                registration.type_info().type_path().to_string(),
                AddComponentItem {
                    default: default.clone(),
                    component: component.clone(),
                },
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut items = map.keys().cloned().collect::<Vec<_>>();
    items.sort_by(|a, b| a.cmp(b));

    commands.insert_resource(AddComponentMenu {
        search: String::new(),
        map,
        items,
    });
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
            .register_pane(PropertiesPane)
            .add_systems(Startup, collect_add_component_tree);
    }
}
