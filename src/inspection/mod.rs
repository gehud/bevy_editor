#![warn(
    clippy::undocumented_unsafe_blocks,
    clippy::missing_safety_doc,
    unsafe_op_in_unsafe_fn
)]
#![allow(
    clippy::needless_lifetimes, // can be good for clarity
    clippy::needless_doctest_main, // sometimes a full copy-pasteable standalone example is better
    clippy::too_many_arguments,
    clippy::type_complexity,
)]

//! This crate contains
//! - general purpose machinery for displaying [`Reflect`](bevy::reflect::Reflect) values in [`reflect_inspector`],
//! - a way of associating arbitrary options with fields and enum variants in [`inspector_options`]
//! - utility functions for displaying bevy resource, entities and assets in [`bevy_inspector`]
//! - some drop-in plugins in [`quick`] to get you started without any code necessary.
//!
//! # Use case 1: Quick plugins
//! These plugins can be easily added to your app, but don't allow for customization of the presentation and content.
//!
//! ## WorldInspectorPlugin
//! Displays the world's entities, resources and assets.
//!
//! ![image of the world inspector](https://raw.githubusercontent.com/jakobhellermann/bevy-inspector-egui/main/docs/images/world_inspector.png)
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_inspector_egui::quick::WorldInspectorPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(WorldInspectorPlugin::new())
//!         .run();
//! }
//!
//! ```
//! ## ResourceInspectorPlugin
//! Display a single resource in a window.
//!
//! ![image of the resource inspector](https://raw.githubusercontent.com/jakobhellermann/bevy-inspector-egui/main/docs/images/resource_inspector.png)
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_inspector_egui::prelude::*;
//! use bevy_inspector_egui::quick::ResourceInspectorPlugin;
//!
//! // `InspectorOptions` are completely optional
//! #[derive(Reflect, Resource, Default, InspectorOptions)]
//! #[reflect(Resource, InspectorOptions)]
//! struct Configuration {
//!     name: String,
//!     #[inspector(min = 0.0, max = 1.0)]
//!     option: f32,
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .init_resource::<Configuration>() // `ResourceInspectorPlugin` won't initialize the resource
//!         .register_type::<Configuration>() // you need to register your type to display it
//!         .add_plugins(ResourceInspectorPlugin::<Configuration>::default())
//!         // also works with built-in resources, as long as they implement `Reflect`
//!         .add_plugins(ResourceInspectorPlugin::<Time>::default())
//!         .run();
//! }
//! ```
//!
//! <hr>
//!
//! There is also the [`StateInspectorPlugin`](quick::StateInspectorPlugin) and the [`AssetInspectorPlugin`](quick::AssetInspectorPlugin).
//!
//! # Use case 2: Manual UI
//! The [`quick`] plugins don't allow customization of the egui window or its content, but you can easily build your own UI:
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_egui::{EguiPlugin, EguiContext, EguiPrimaryContextPass};
//! use bevy_inspector_egui::prelude::*;
//! use bevy_inspector_egui::bevy_inspector;
//! use bevy::window::PrimaryWindow;
//! use std::any::TypeId;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EguiPlugin::default())
//!         .add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin) // adds default options and `InspectorEguiImpl`s
//!         .add_systems(EguiPrimaryContextPass, inspector_ui)
//!         .run();
//! }
//!
//! fn inspector_ui(world: &mut World) {
//!     let mut egui_context = world
//!         .query_filtered::<&mut EguiContext, With<bevy_egui::PrimaryEguiContext>>()
//!         .single(world)
//!         .expect("EguiContext not found")
//!         .clone();
//!
//!     egui::Window::new("UI").show(egui_context.get_mut(), |ui| {
//!         egui::ScrollArea::both().show(ui, |ui| {
//!             // equivalent to `WorldInspectorPlugin`
//!             bevy_inspector::ui_for_world(world, ui);
//!
//!             // works with any `Reflect` value, including `Handle`s
//!             let mut any_reflect_value: i32 = 5;
//!             bevy_inspector::ui_for_value(&mut any_reflect_value, ui, world);
//!
//!             egui::CollapsingHeader::new("Materials").show(ui, |ui| {
//!                 bevy_inspector::ui_for_assets::<StandardMaterial>(world, ui);
//!             });
//!
//!             ui.heading("Entities");
//!             bevy_inspector::ui_for_entities(world, ui);
//!         });
//!     });
//! }
//! ```
//!
//! Pair this with a crate like [`egui_dock`](https://docs.rs/egui_dock/latest/egui_dock/) and you have your own editor in less than 100 lines: [`examples/egui_dock.rs`](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui/examples/integrations/egui_dock.rs).
//! ![image of the egui_dock example](https://raw.githubusercontent.com/jakobhellermann/bevy-inspector-egui/main/docs/images/egui_dock.png)
//!
//! # FAQ
//!
//! **Q: How do I change the names of the entities in the world inspector?**
//!
//! **A:** You can insert the [`Name`](bevy::ecs::name::Name) component.
//!
//! **Q: What if I just want to display a single value without passing in the whole `&mut World`?**
//!
//! **A:** You can use [`ui_for_value`](crate::inspection::reflect_inspector::ui_for_value). Note that displaying things like `Handle<StandardMaterial>` won't be able to display the asset's value.
//!
//! **Q:** Can I change how exactly my type is displayed?
//!
//! **A:** Implement [`InspectorPrimitive`](crate::inspection::inspector_egui_impls::InspectorPrimitive) and call `app.register_type_data::<T, InspectorEguiImpl>`.

pub mod bevy_inspector;
pub mod inspector_egui_impls;
pub mod inspector_options;
pub mod quick;
pub mod reflect_inspector;
pub mod restricted_world_view;

pub mod dropdown;
pub mod egui_utils;
pub mod utils;

use std::any::TypeId;

/// [`bevy::app::Plugin`] used to register default [`struct@InspectorOptions`] and [`InspectorEguiImpl`](crate::inspection::inspector_egui_impls::InspectorEguiImpl)s
pub struct DefaultInspectorConfigPlugin;
impl bevy::app::Plugin for DefaultInspectorConfigPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        if app.is_plugin_added::<Self>() {
            return;
        }

        // Defensively register stuff since bevy only registers glam, color types used by other structs internally
        app.register_type::<bevy::math::IVec2>()
            .register_type::<bevy::math::IVec3>()
            .register_type::<bevy::math::IVec4>()
            .register_type::<bevy::math::UVec2>()
            .register_type::<bevy::math::UVec3>()
            .register_type::<bevy::math::UVec4>()
            .register_type::<bevy::math::DVec2>()
            .register_type::<bevy::math::DVec3>()
            .register_type::<bevy::math::DVec4>()
            .register_type::<bevy::math::BVec2>()
            .register_type::<bevy::math::BVec3>()
            .register_type::<bevy::math::BVec3A>()
            .register_type::<bevy::math::BVec4>()
            .register_type::<bevy::math::BVec4A>()
            .register_type::<bevy::math::Vec2>()
            .register_type::<bevy::math::Vec3>()
            .register_type::<bevy::math::Vec3A>()
            .register_type::<bevy::math::Vec4>()
            .register_type::<bevy::math::DAffine2>()
            .register_type::<bevy::math::DAffine3>()
            .register_type::<bevy::math::Affine2>()
            .register_type::<bevy::math::Affine3A>()
            .register_type::<bevy::math::DMat2>()
            .register_type::<bevy::math::DMat3>()
            .register_type::<bevy::math::DMat4>()
            .register_type::<bevy::math::Mat2>()
            .register_type::<bevy::math::Mat3>()
            .register_type::<bevy::math::Mat3A>()
            .register_type::<bevy::math::Mat4>()
            .register_type::<bevy::math::DQuat>()
            .register_type::<bevy::math::Quat>()
            .register_type::<bevy::math::Rect>()
            .register_type::<bevy::color::Color>()
            .register_type::<core::ops::Range<f32>>()
            .register_type::<TypeId>();

        let type_registry = app.world().resource::<bevy::ecs::prelude::AppTypeRegistry>();
        let mut type_registry = type_registry.write();

        inspector_options::default_options::register_default_options(&mut type_registry);
        inspector_egui_impls::register_std_impls(&mut type_registry);
        inspector_egui_impls::register_glam_impls(&mut type_registry);
        inspector_egui_impls::register_bevy_impls(&mut type_registry);
    }
}

#[doc(inline)]
pub use inspector_options::InspectorOptions;

#[doc(hidden)]
pub mod __macro_exports {
    pub use bevy::reflect;
}
