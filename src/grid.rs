//! An infinite ground grid on the y = 0 plane, with the world X and Z axes highlighted.
//!
//! The grid is a single quad whose vertex shader (`assets/shaders/infinite_grid.wesl`) recenters
//! it under whichever camera is rendering, so it never runs out in any viewport.

use bevy::{
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

use crate::editing::EditorOnly;

const SHADER: &str = "shaders/infinite_grid.wesl";

pub struct InfiniteGridPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GridMaterial>::default())
            .add_systems(Startup, spawn_grid);
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct GridMaterial {
    #[uniform(0)]
    pub settings: GridSettings,
}

#[derive(ShaderType, Clone)]
pub struct GridSettings {
    pub minor_color: LinearRgba,
    pub major_color: LinearRgba,
    pub x_axis_color: LinearRgba,
    pub z_axis_color: LinearRgba,
    /// World units between minor lines; major lines are every 10 minor lines.
    pub spacing: f32,
    /// The grid fades out over at least this many world units (more when the camera is high).
    pub fade_distance: f32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            minor_color: Color::srgba(1.0, 1.0, 1.0, 0.06).into(),
            major_color: Color::srgba(1.0, 1.0, 1.0, 0.14).into(),
            x_axis_color: Color::srgb_u8(0xC0, 0x39, 0x2B).into(),
            z_axis_color: Color::srgb_u8(0x2F, 0x6F, 0xD0).into(),
            spacing: 1.0,
            fade_distance: 80.0,
        }
    }
}

impl Material for GridMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Visible from below as well as from above.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridMaterial>>,
) {
    commands.spawn((
        Name::new("Infinite Grid"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.5)))),
        MeshMaterial3d(materials.add(GridMaterial {
            settings: GridSettings::default(),
        })),
        Transform::default(),
        // The shader moves the quad to the camera, so its bounds on the CPU mean nothing.
        NoFrustumCulling,
        NotShadowCaster,
        EditorOnly,
    ));
}
