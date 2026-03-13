use bevy::app::Plugin;
use bevy::asset::{Asset, Assets, Handle};
use bevy::ecs::{
    component::Component,
    lifecycle::Add,
    observer::On,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Query, Res},
    world::FromWorld,
};
use bevy::reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};

#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
pub(crate) struct AlphaPatternMaterial {}

impl UiMaterial for AlphaPatternMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_editor/assets/widget/shaders/alpha_pattern.wgsl".into()
    }
}

#[derive(Resource)]
pub(crate) struct AlphaPatternResource(pub(crate) Handle<AlphaPatternMaterial>);

impl FromWorld for AlphaPatternResource {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let mut ui_materials = world
            .get_resource_mut::<Assets<AlphaPatternMaterial>>()
            .unwrap();
        Self(ui_materials.add(AlphaPatternMaterial::default()))
    }
}

/// Marker that tells us we want to fill in the [`MaterialNode`] with the alpha material.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct AlphaPattern;

/// Observer to fill in the material handle (since we don't have access to the materials asset
/// in the template)
fn on_add_alpha_pattern(
    add: On<Add, AlphaPattern>,
    mut q_material_node: Query<&mut MaterialNode<AlphaPatternMaterial>>,
    r_material: Res<AlphaPatternResource>,
) {
    if let Ok(mut material) = q_material_node.get_mut(add.entity) {
        material.0 = r_material.0.clone();
    }
}

/// Plugin which registers the systems for updating the button styles.
pub struct AlphaPatternPlugin;

impl Plugin for AlphaPatternPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_add_alpha_pattern);
    }
}
