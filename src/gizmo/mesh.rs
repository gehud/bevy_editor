use std::f32::consts::TAU;

use bevy::{
    anti_alias::fxaa::Fxaa,
    asset::uuid_handle,
    camera::{Camera3dDepthLoadOp, visibility::RenderLayers},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    light::NotShadowCaster,
    pbr::{ExtendedMaterial, MaterialExtension, OpaqueRendererMethod},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

use super::{InteractionKind, InternalGizmoCamera, ScaleGizmo, TransformGizmo, TranslationGizmo};
use crate::{selection::NoSelect, theme::palette};

#[derive(Component)]
pub struct RotationGizmo;

#[derive(Component)]
pub struct ViewTranslateGizmo;

/// Startup system that builds the procedural mesh and materials of the gizmo.
pub fn build_gizmo(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let axis_length = 1.5;
    let plane_size = 0.3;
    let plane_offset = 0.4;

    // Define improved gizmo meshes with better proportions
    let arrow_tail_mesh = meshes.add(Capsule3d {
        radius: 0.03, // Slightly thinner for precision
        half_length: axis_length * 0.45,
    });

    let cone_mesh = meshes.add(Cone {
        height: 0.2,
        radius: 0.08, // Smaller, more precise arrow heads
        ..default()
    });

    // Plane handles for multi-axis translation
    let plane_mesh = meshes.add(Plane3d::default().mesh().size(plane_size, plane_size));

    // Center sphere for free movement
    let sphere_mesh = meshes.add(Sphere { radius: 0.15 });

    // Scale gizmo handles - small cubes at the end of axes
    let scale_tip_mesh = meshes.add(Cuboid::new(0.12, 0.12, 0.12));
    let scale_handle_mesh = meshes.add(Cuboid::new(0.06, axis_length, 0.06));

    // Rotation rings with better visibility
    let rotation_mesh = meshes.add(Torus {
        major_radius: 1.1,
        minor_radius: 0.03,
    });

    let uniform_rotation_mesh = meshes.add(Sphere { radius: 1.1 });

    /// Helper function to create a material with a specific color
    fn material(color: Color) -> StandardMaterial {
        StandardMaterial {
            base_color: color,
            unlit: true,
            cull_mode: None,
            ..default()
        }
    }

    // Editor color scheme - matching CSS specification
    let gizmo_matl_x = materials.add(material(palette::X_AXIS.lighter(0.05)));
    let gizmo_matl_y = materials.add(material(palette::Y_AXIS.lighter(0.05)));
    let gizmo_matl_z = materials.add(material(palette::Z_AXIS.lighter(0.05)));

    // Brighter versions for selected/hovered state
    let gizmo_matl_x_sel = materials.add(material(palette::X_AXIS.lighter(0.1)));
    let gizmo_matl_y_sel = materials.add(material(palette::Y_AXIS.lighter(0.1)));
    let gizmo_matl_z_sel = materials.add(material(palette::Z_AXIS.lighter(0.1)));

    // View gizmo - neutral dark/gray
    let gizmo_matl_v = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.3, 0.3, 0.3),
        unlit: true,
        alpha_mode: AlphaMode::Multiply,
        ..default()
    });

    // View gizmo - neutral white/gray
    let gizmo_matl_v_sel = materials.add(material(Color::srgba(0.9, 0.9, 0.9, 0.8)));

    // Build the gizmo using the variables above.
    commands
        .spawn(TransformGizmo::default())
        .with_children(|parent| {
            // Translation arrows
            parent.spawn((
                NoSelect,
                Mesh3d(arrow_tail_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_z(std::f32::consts::PI / 2.0),
                    Vec3::new(axis_length / 2.0, 0.0, 0.0),
                )),
                InteractionKind::TranslateAxis {
                    original: Vec3::X,
                    axis: Vec3::X,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(arrow_tail_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_y(std::f32::consts::PI / 2.0),
                    Vec3::new(0.0, axis_length / 2.0, 0.0),
                )),
                InteractionKind::TranslateAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(arrow_tail_mesh),
                MeshMaterial3d(gizmo_matl_z.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_x(std::f32::consts::PI / 2.0),
                    Vec3::new(0.0, 0.0, axis_length / 2.0),
                )),
                InteractionKind::TranslateAxis {
                    original: Vec3::Z,
                    axis: Vec3::Z,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Translation handles
            parent.spawn((
                NoSelect,
                Mesh3d(cone_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x_sel.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_z(std::f32::consts::PI / -2.0),
                    Vec3::new(axis_length, 0.0, 0.0),
                )),
                InteractionKind::TranslateAxis {
                    original: Vec3::X,
                    axis: Vec3::X,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(cone_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y_sel.clone()),
                Transform::from_translation(Vec3::new(0.0, axis_length, 0.0)),
                InteractionKind::TranslateAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(cone_mesh.clone()),
                MeshMaterial3d(gizmo_matl_z_sel.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_x(std::f32::consts::PI / 2.0),
                    Vec3::new(0.0, 0.0, axis_length),
                )),
                InteractionKind::TranslateAxis {
                    original: Vec3::Z,
                    axis: Vec3::Z,
                },
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Translation planes
            parent.spawn((
                NoSelect,
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_z(std::f32::consts::PI / -2.0),
                    Vec3::new(0., plane_offset, plane_offset),
                )),
                InteractionKind::TranslatePlane {
                    original: Vec3::X,
                    normal: Vec3::X,
                },
                TranslationGizmo,
                RayCastBackfaces,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y.clone()),
                Transform::from_translation(Vec3::new(plane_offset, 0.0, plane_offset)),
                InteractionKind::TranslatePlane {
                    original: Vec3::Y,
                    normal: Vec3::Y,
                },
                TranslationGizmo,
                RayCastBackfaces,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(plane_mesh.clone()),
                MeshMaterial3d(gizmo_matl_z.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_x(std::f32::consts::PI / 2.0),
                    Vec3::new(plane_offset, plane_offset, 0.0),
                )),
                InteractionKind::TranslatePlane {
                    original: Vec3::Z,
                    normal: Vec3::Z,
                },
                TranslationGizmo,
                RayCastBackfaces,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Free translation
            parent.spawn((
                NoSelect,
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(gizmo_matl_v_sel.clone()),
                InteractionKind::TranslatePlane {
                    original: Vec3::ZERO,
                    normal: Vec3::Z,
                },
                ViewTranslateGizmo,
                TranslationGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Rotation Arcs
            parent.spawn((
                NoSelect,
                Mesh3d(rotation_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x.clone()),
                Transform::from_rotation(Quat::from_axis_angle(Vec3::Z, f32::to_radians(90.0))),
                RotationGizmo,
                InteractionKind::RotateAxis {
                    original: Vec3::X,
                    axis: Vec3::X,
                },
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(rotation_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y.clone()),
                RotationGizmo,
                InteractionKind::RotateAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(rotation_mesh.clone()),
                MeshMaterial3d(gizmo_matl_z.clone()),
                Transform::from_rotation(
                    Quat::from_axis_angle(Vec3::Z, f32::to_radians(90.0))
                        * Quat::from_axis_angle(Vec3::X, f32::to_radians(90.0)),
                ),
                RotationGizmo,
                InteractionKind::RotateAxis {
                    original: Vec3::Z,
                    axis: Vec3::Z,
                },
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Uniform rotation
            parent.spawn((
                NoSelect,
                Mesh3d(uniform_rotation_mesh.clone()),
                MeshMaterial3d(gizmo_matl_v.clone()),
                RotationGizmo,
                InteractionKind::RotateUniform,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Scale tips
            parent.spawn((
                NoSelect,
                Mesh3d(scale_tip_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x_sel.clone()),
                Transform::from_translation(Vec3::new(axis_length, 0.0, 0.0)),
                InteractionKind::ScaleAxis {
                    original: Vec3::X,
                    axis: Vec3::X,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(scale_tip_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y_sel.clone()),
                Transform::from_translation(Vec3::new(0.0, axis_length, 0.0)),
                InteractionKind::ScaleAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(scale_tip_mesh.clone()),
                MeshMaterial3d(gizmo_matl_z_sel.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, axis_length)),
                InteractionKind::ScaleAxis {
                    original: Vec3::Z,
                    axis: Vec3::Z,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Scale handles
            parent.spawn((
                NoSelect,
                Mesh3d(scale_handle_mesh.clone()),
                MeshMaterial3d(gizmo_matl_x.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_z(std::f32::consts::PI / -2.0),
                    Vec3::new(axis_length / 2.0, 0.0, 0.0),
                )),
                InteractionKind::ScaleAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(scale_handle_mesh.clone()),
                MeshMaterial3d(gizmo_matl_y.clone()),
                Transform::from_translation(Vec3::new(0.0, axis_length / 2.0, 0.0)),
                InteractionKind::ScaleAxis {
                    original: Vec3::Y,
                    axis: Vec3::Y,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
            parent.spawn((
                NoSelect,
                Mesh3d(scale_handle_mesh.clone()),
                MeshMaterial3d(gizmo_matl_z.clone()),
                Transform::from_matrix(Mat4::from_rotation_translation(
                    Quat::from_rotation_x(std::f32::consts::PI / 2.0),
                    Vec3::new(0.0, 0.0, axis_length / 2.0),
                )),
                InteractionKind::ScaleAxis {
                    original: Vec3::Z,
                    axis: Vec3::Z,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));

            // Uniform scale handle - larger cube at center
            parent.spawn((
                NoSelect,
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 0.2))),
                MeshMaterial3d(gizmo_matl_v_sel),
                Transform::from_translation(Vec3::ZERO),
                InteractionKind::ScaleUniform {
                    original: Vec3::ONE,
                },
                ScaleGizmo,
                NotShadowCaster,
                RenderLayers::layer(12),
            ));
        });

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        InternalGizmoCamera,
        RenderLayers::layer(12),
    ));
}
