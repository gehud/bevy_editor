//! Methods for displaying `bevy` resources, assets and entities
//!
//! # Example
//!
//! ```rust
//! use bevy_inspector_egui::bevy_inspector;
//! # use bevy::ecs::prelude::*;
//! # use bevy::state::prelude::States;
//! # use bevy::reflect::Reflect;
//! # use bevy::time::Time;
//! # use bevy::math::Vec3;
//!
//! #[derive(States, Debug, Clone, Eq, PartialEq, Hash, Reflect, Default)]
//! enum AppState { #[default] A, B, C }
//!
//! fn show_ui(world: &mut World, ui: &mut egui::Ui) {
//!     let mut any_reflect_value = Vec3::new(1.0, 2.0, 3.0);
//!     bevy_inspector::ui_for_value(&mut any_reflect_value, ui, world);
//!
//!     ui.heading("Time resource");
//!     bevy_inspector::ui_for_resource::<Time>(world, ui);
//!
//!     ui.heading("App State");
//!     bevy_inspector::ui_for_state::<AppState>(world, ui);
//!
//!     egui::CollapsingHeader::new("Entities")
//!         .default_open(true)
//!         .show(ui, |ui| {
//!             bevy_inspector::ui_for_entities(world, ui);
//!         });
//!     egui::CollapsingHeader::new("Resources").show(ui, |ui| {
//!         bevy_inspector::ui_for_resources(world, ui);
//!     });
//!     egui::CollapsingHeader::new("Assets").show(ui, |ui| {
//!         bevy_inspector::ui_for_all_assets(world, ui);
//!     });
//! }
//! ```

use std::any::TypeId;
use std::marker::PhantomData;

use crate::inspection::utils::{pretty_type_name, pretty_type_name_str};
use bevy::asset::{Asset, AssetServer, Assets, ReflectAsset, UntypedAssetId};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::world::CommandQueue;
use bevy::ecs::{component::ComponentId, prelude::*};
use bevy::reflect::Reflect;
use bevy::state::state::{FreelyMutableState, NextState, State};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Helper functions for a consistent set of error messages.
pub mod errors;

use crate::inspection::reflect_inspector::{Context, InspectorUi};
use crate::inspection::restricted_world_view::RestrictedWorldView;

/// Display a single [`&mut dyn Reflect`](bevy::reflect::Reflect).
///
/// If you are wondering why this function takes in a [`&mut World`](bevy::ecs::world::World), it's so that if the value contains e.g. a
/// `Handle<StandardMaterial>` it can look up the corresponding asset resource and display the asset value inline.
///
/// If all you're displaying is a simple value without any references into the bevy world, consider just using
/// [`reflect_inspector::ui_for_value`](crate::inspection::reflect_inspector::ui_for_value).
pub fn ui_for_value(value: &mut dyn Reflect, ui: &mut egui::Ui, world: &mut World) -> Result<bool> {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(RestrictedWorldView::new(world)),
        queue: Some(&mut queue),
    };
    let mut env = InspectorUi::new(&type_registry, &mut cx);
    let changed = env.ui_for_reflect(value.as_partial_reflect_mut(), ui)?;
    queue.apply(world);
    Ok(changed)
}

/// Display all reflectable resources in the world
pub fn ui_for_resources(world: &mut World, ui: &mut egui::Ui) {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| {
            (
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
            )
        })
        .collect();
    resources.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    for (name, type_id) in resources {
        ui.collapsing(name, |ui| {
            by_type_id::ui_for_resource(world, type_id, ui, name, &type_registry);
        });
    }
}

/// Display the resource `R`
pub fn ui_for_resource<R: Resource + Reflect>(world: &mut World, ui: &mut egui::Ui) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    // create a context with access to the world except for the `R` resource
    let Some((mut resource, world_view)) =
        RestrictedWorldView::new(world).split_off_resource_typed::<R>()
    else {
        errors::nonexistent_resource(ui, &pretty_type_name::<R>());
        return Ok(());
    };
    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(world_view),
        queue: Some(&mut queue),
    };
    let mut env = InspectorUi::new(&type_registry, &mut cx);

    if env.ui_for_reflect(resource.bypass_change_detection(), ui)? {
        resource.set_changed();
    }

    queue.apply(world);

    Ok(())
}

/// Display all reflectable assets
pub fn ui_for_all_assets(world: &mut World, ui: &mut egui::Ui) {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut assets: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectAsset>().is_some())
        .map(|registration| {
            (
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
            )
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    for (name, type_id) in assets {
        ui.collapsing(name, |ui| {
            by_type_id::ui_for_assets(world, type_id, ui, &type_registry);
        });
    }
}

/// Display all assets of the specified asset type `A`
pub fn ui_for_assets<A: Asset + Reflect>(world: &mut World, ui: &mut egui::Ui) -> Result {
    let asset_server = world.get_resource::<AssetServer>().cloned();

    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    // create a context with access to the world except for the `R` resource
    let Some((mut assets, world_view)) =
        RestrictedWorldView::new(world).split_off_resource_typed::<Assets<A>>()
    else {
        errors::nonexistent_resource(ui, &pretty_type_name::<Assets<A>>());
        return Ok(());
    };

    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(world_view),
        queue: Some(&mut queue),
    };

    let mut assets: Vec<_> = assets.iter_mut().collect();
    assets.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (handle_id, asset) in assets {
        let id = egui::Id::new(handle_id);

        let inner =
            egui::CollapsingHeader::new(handle_name(handle_id.untyped(), asset_server.as_ref()))
                .id_salt(id)
                .show(ui, |ui| -> Result {
                    let mut env = InspectorUi::new(&type_registry, &mut cx);
                    env.ui_for_reflect_with_options(asset, ui, id, &())?;
                    Ok(())
                })
                .body_returned;

        if let Some(inner) = inner {
            inner?;
        }
    }

    queue.apply(world);

    Ok(())
}

/// Display state `T` and change state on edit
pub fn ui_for_state<T: FreelyMutableState + Reflect>(
    world: &mut World,
    ui: &mut egui::Ui,
) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    // create a context with access to the world except for the `State<T>` resource
    let Some((state, world_view)) =
        RestrictedWorldView::new(world).split_off_resource_typed::<State<T>>()
    else {
        errors::nonexistent_state(ui, &pretty_type_name::<T>());
        return Ok(());
    };
    let Some((mut next_state, world_view)) = world_view.split_off_resource_typed::<NextState<T>>()
    else {
        errors::nonexistent_state(ui, &pretty_type_name::<T>());
        return Ok(());
    };
    let mut queue = CommandQueue::default();
    let mut cx = Context {
        world: Some(world_view),
        queue: Some(&mut queue),
    };
    let mut env = InspectorUi::new(&type_registry, &mut cx);

    let mut current = state.get().clone();
    let changed = env.ui_for_reflect(&mut current, ui)?;

    if changed {
        *next_state = NextState::Pending(current);
    }
    queue.apply(world);

    Ok(())
}

pub trait EntityFilter {
    type StaticFilter: QueryFilter;

    /// Returns true if the filter term is currently active
    ///
    /// Used in the default impl of [`EntityFilter::filter_entities`] to skip filtering if false
    ///
    /// default impl is true
    fn is_active(&self) -> bool {
        true
    }

    /// Filters entities in place
    ///
    /// default impl:
    /// - uses [`EntityFilter::filter_entity`] to mark what entities to retain
    /// - skips filtering if [`EntityFilter::is_active`] returns false
    fn filter_entities(&self, world: &mut World, entities: &mut Vec<Entity>) {
        if !self.is_active() {
            return;
        }
        entities.retain(|&entity| self.filter_entity(world, entity));
    }

    /// Returns true if entity matches the filter term
    fn filter_entity(&self, world: &mut World, entity: Entity) -> bool;
}

#[derive(Debug)]
pub struct Filter<F: QueryFilter = Without<ChildOf>> {
    pub word: String,
    pub is_fuzzy: bool,
    pub marker: PhantomData<F>,
}

impl<F: QueryFilter + Clone> Clone for Filter<F> {
    fn clone(&self) -> Self {
        Self {
            word: self.word.clone(),
            is_fuzzy: self.is_fuzzy,
            marker: PhantomData,
        }
    }
}

impl<F: QueryFilter> Filter<F> {
    pub fn from_ui_fuzzy(ui: &mut egui::Ui, id: egui::Id) -> Self {
        ui.horizontal(|ui| {
            let word = {
                let id = id.with("word");
                // filter, using eguis memory and provided id
                let mut filter_string = ui.memory_mut(|mem| {
                    let filter: &mut String = mem.data.get_persisted_mut_or_default(id);
                    filter.clone()
                });
                ui.add(egui::TextEdit::singleline(&mut filter_string).desired_width(180.));
                ui.memory_mut(|mem| {
                    *mem.data.get_persisted_mut_or_default(id) = filter_string.clone();
                });

                // improves overall matching
                filter_string.to_lowercase()
            };

            Filter {
                word,
                is_fuzzy: true,
                marker: PhantomData,
            }
        })
        .inner
    }

    pub fn from_ui(ui: &mut egui::Ui, id: egui::Id) -> Self {
        ui.horizontal(|ui| {
            // filter kind
            let is_fuzzy = {
                let id = id.with("is_fuzzy");
                let mut is_fuzzy = ui.memory_mut(|mem| {
                    let fuzzy: &mut bool = mem.data.get_persisted_mut_or_default(id);
                    *fuzzy
                });
                ui.checkbox(&mut is_fuzzy, "Fuzzy");
                ui.memory_mut(|mem| {
                    *mem.data.get_persisted_mut_or_default(id) = is_fuzzy;
                });
                is_fuzzy
            };
            let word = {
                let id = id.with("word");
                // filter, using eguis memory and provided id
                let mut filter_string = ui.memory_mut(|mem| {
                    let filter: &mut String = mem.data.get_persisted_mut_or_default(id);
                    filter.clone()
                });
                ui.text_edit_singleline(&mut filter_string);
                ui.memory_mut(|mem| {
                    *mem.data.get_persisted_mut_or_default(id) = filter_string.clone();
                });

                // improves overall matching
                filter_string.to_lowercase()
            };

            Filter {
                word,
                is_fuzzy,
                marker: PhantomData,
            }
        })
        .inner
    }

    /// empty filter which does nothing
    pub fn all() -> Self {
        Self {
            word: String::from(""),
            is_fuzzy: false,
            marker: PhantomData,
        }
    }
}

impl<F: QueryFilter> EntityFilter for Filter<F> {
    type StaticFilter = F;

    fn is_active(&self) -> bool {
        !self.word.is_empty()
    }

    fn filter_entity(&self, world: &mut World, entity: Entity) -> bool {
        self_or_children_satisfy_filter(world, entity, self.word.as_str(), self.is_fuzzy)
    }
}

fn self_or_children_satisfy_filter(
    world: &mut World,
    entity: Entity,
    filter: &str,
    is_fuzzy: bool,
) -> bool {
    let name = guess_entity_name(world, entity);

    let self_matches = if is_fuzzy {
        let matcher = SkimMatcherV2::default();
        matcher.fuzzy_match(name.as_str(), filter).is_some()
    } else {
        name.to_lowercase().contains(filter)
    };
    self_matches || {
        let Ok(children) = world
            .query::<&Children>()
            .get(world, entity)
            .map(|children| children.to_vec())
        else {
            return false;
        };

        children
            .iter()
            .any(|child| self_or_children_satisfy_filter(world, *child, filter, is_fuzzy))
    }
}

fn components_of_entity(
    world: &mut RestrictedWorldView<'_>,
    entity: Entity,
) -> Result<Vec<(String, ComponentId, Option<TypeId>, usize)>> {
    let entity_ref = world.world().get_entity(entity)?;

    let archetype = entity_ref.archetype();
    let mut components: Vec<_> = archetype
        .components()
        .iter()
        .map(|component_id| {
            let info = world.world().components().get_info(*component_id).unwrap();
            let name = pretty_type_name_str(&info.name().to_string());

            (name, *component_id, info.type_id(), info.layout().size())
        })
        .collect();
    components.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    Ok(components)
}

pub mod by_type_id {
    use std::any::TypeId;

    use bevy::asset::{AssetServer, ReflectAsset, ReflectHandle, UntypedAssetId, UntypedHandle};
    use bevy::ecs::{prelude::*, world::CommandQueue};
    use bevy::reflect::TypeRegistry;

    use crate::inspection::{
        reflect_inspector::{Context, InspectorUi},
        restricted_world_view::RestrictedWorldView,
    };

    use super::{
        errors::{self, typeid_name},
        handle_name,
    };

    /// Display the resource with the given [`TypeId`]
    pub fn ui_for_resource(
        world: &mut World,
        resource_type_id: TypeId,
        ui: &mut egui::Ui,
        name_of_type: &str,
        type_registry: &TypeRegistry,
    ) -> Result {
        let mut queue = CommandQueue::default();

        {
            // create a context with access to the world except for the current resource
            let mut world_view = RestrictedWorldView::new(world);
            let (mut resource_view, world_view) = world_view.split_off_resource(resource_type_id);
            let mut cx = Context {
                world: Some(world_view),
                queue: Some(&mut queue),
            };
            let mut env = InspectorUi::new(type_registry, &mut cx);

            let mut resource = match resource_view
                .get_resource_reflect_mut_by_id(resource_type_id, type_registry)
            {
                Ok(resource) => resource,
                Err(err) => {
                    errors::no_access(err, ui, name_of_type);
                    return Ok(());
                }
            };

            let changed = env.ui_for_reflect(
                resource.bypass_change_detection().as_partial_reflect_mut(),
                ui,
            )?;
            if changed {
                resource.set_changed();
            }
        }

        queue.apply(world);
        Ok(())
    }

    /// Display all assets of the given asset [`TypeId`]
    pub fn ui_for_assets(
        world: &mut World,
        asset_type_id: TypeId,
        ui: &mut egui::Ui,
        type_registry: &TypeRegistry,
    ) -> Result {
        let asset_server = world.get_resource::<AssetServer>().cloned();

        let Some(registration) = type_registry.get(asset_type_id) else {
            crate::inspection::reflect_inspector::errors::not_in_type_registry(
                ui,
                &typeid_name(asset_type_id, type_registry),
            );

            return Ok(());
        };
        let Some(reflect_asset) = registration.data::<ReflectAsset>() else {
            errors::missing_typedata(
                ui,
                &typeid_name(asset_type_id, type_registry),
                "ReflectAsset",
            );

            return Ok(());
        };
        let Some(reflect_handle) =
            type_registry.get_type_data::<ReflectHandle>(reflect_asset.handle_type_id())
        else {
            errors::missing_typedata(
                ui,
                &typeid_name(reflect_asset.handle_type_id(), type_registry),
                "ReflectHandle",
            );

            return Ok(());
        };

        let ids: Vec<_> = reflect_asset.ids(world).collect();

        for handle_id in ids {
            let id = egui::Id::new(handle_id);

            // If we have a `Uuid` asset handle, we can obtain a `Handle<T>`,
            // which can be useful e.g. for `Handle<Image>` which has its own explicit egui UI.
            if let UntypedAssetId::Uuid { uuid, type_id } = handle_id {
                let mut handle = reflect_handle
                    .typed(UntypedHandle::Uuid { uuid, type_id })
                    .into_partial_reflect();

                // Create a context with access to the entire world. Displaying the `Handle<T>` will short circuit into
                // displaying the T with a world view excluding Assets<T>.
                let mut queue = CommandQueue::default();
                let mut cx = Context {
                    world: Some(RestrictedWorldView::new(world)),
                    queue: Some(&mut queue),
                };

                egui::CollapsingHeader::new(handle_name(handle_id, asset_server.as_ref()))
                    .id_salt(id)
                    .show(ui, |ui| {
                        let mut env = InspectorUi::new(type_registry, &mut cx);
                        env.ui_for_reflect_with_options(&mut *handle, ui, id, &());
                    });

                queue.apply(world);
            }
            // in the general case, we currently cannot cast get a `Handle<T>` through
            // `ReflectAsset`/`ReflectHandle`. But we can still get the `&mut T`
            // and inspect that (which works for e.g. `StandardMaterial`).
            else {
                let mut world_view = RestrictedWorldView::new(world);
                let (asset_world, rest_world) =
                    world_view.split_off_resource(reflect_asset.assets_resource_type_id());

                // # SAFETY:
                // - The world cell has unique access to `Assets<T>`
                // - We only call this once, so no overlapping references possible
                let data = unsafe {
                    reflect_asset
                        .get_unchecked_mut(asset_world.world(), handle_id)
                        .unwrap()
                };

                let mut queue = CommandQueue::default();
                let mut cx = Context {
                    world: Some(rest_world),
                    queue: Some(&mut queue),
                };

                egui::CollapsingHeader::new(handle_name(handle_id, asset_server.as_ref()))
                    .id_salt(id)
                    .show(ui, |ui| {
                        let mut env = InspectorUi::new(type_registry, &mut cx);
                        env.ui_for_reflect_with_options(data, ui, id, &());
                    });
            }
        }

        Ok(())
    }

    /// Display a given asset by handle and asset [`TypeId`]
    pub fn ui_for_asset(
        world: &mut World,
        asset_type_id: TypeId,
        handle: UntypedAssetId,
        ui: &mut egui::Ui,
        type_registry: &TypeRegistry,
    ) -> Result<bool> {
        let Some(registration) = type_registry.get(asset_type_id) else {
            crate::inspection::reflect_inspector::errors::not_in_type_registry(
                ui,
                &typeid_name(asset_type_id, type_registry),
            );
            return Ok(false);
        };
        let Some(reflect_asset) = registration.data::<ReflectAsset>() else {
            errors::missing_typedata(
                ui,
                &typeid_name(asset_type_id, type_registry),
                "ReflectAsset",
            );
            return Ok(false);
        };
        let Some(reflect_handle) =
            type_registry.get_type_data::<ReflectHandle>(reflect_asset.handle_type_id())
        else {
            errors::missing_typedata(
                ui,
                &typeid_name(reflect_asset.handle_type_id(), type_registry),
                "ReflectHandle",
            );
            return Ok(false);
        };

        let _: Vec<_> = reflect_asset.ids(world).collect();

        // Create a context with access to the entire world. Displaying the `Handle<T>` will short circuit into
        // displaying the T with a world view excluding Assets<T>.
        let world_view = RestrictedWorldView::new(world);
        let mut queue = CommandQueue::default();
        let mut cx = Context {
            world: Some(world_view),
            queue: Some(&mut queue),
        };

        let id = egui::Id::new(handle);

        if let UntypedAssetId::Uuid { uuid, type_id } = handle {
            let mut handle = reflect_handle
                .typed(UntypedHandle::Uuid { uuid, type_id })
                .into_partial_reflect();

            let mut env = InspectorUi::new(type_registry, &mut cx);
            let changed = env.ui_for_reflect_with_options(&mut *handle, ui, id, &())?;

            queue.apply(world);

            Ok(changed)
        } else {
            Ok(false)
        }
    }
}

fn handle_name(handle: UntypedAssetId, asset_server: Option<&AssetServer>) -> String {
    if let Some(path) = asset_server
        .as_ref()
        .and_then(|server| server.get_path(handle))
    {
        return path.to_string();
    }

    match handle {
        UntypedAssetId::Index { index, .. } => {
            format!("{:?}", egui::Id::new(index))
        }
        UntypedAssetId::Uuid { uuid, .. } => {
            format!("{uuid}")
        }
    }
}

pub use crate::inspection::utils::guess_entity_name::guess_entity_name;
