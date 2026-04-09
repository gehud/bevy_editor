use std::ops::Range;

use bevy::{
    asset::{load_internal_asset, uuid_handle},
    camera::{MainPassResolutionOverride, Viewport, visibility::RenderLayers},
    core_pipeline::core_3d::{
        CORE_3D_DEPTH_FORMAT,
        graph::{Core3d, Node3d},
    },
    ecs::{
        query::QueryItem,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    light::NotShadowCaster,
    math::FloatOrd,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        DrawMesh, MeshInputUniform, MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey,
        MeshUniform, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
        SetMeshViewEmptyBindGroup,
    },
    platform::collections::HashSet,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderDebugFlags, RenderStartup, RenderSystems,
        batching::{
            GetBatchData, GetFullBatchData,
            gpu_preprocessing::{
                IndirectParametersCpuMetadata, UntypedPhaseIndirectParametersBuffers,
                batch_and_prepare_sorted_render_phase,
            },
        },
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{RenderMesh, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_phase::{
            AddRenderCommand, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions,
            PhaseItem, PhaseItemExtraIndex, SetItemPipeline, SortedPhaseItem,
            SortedRenderPhasePlugin, ViewSortedRenderPhases, sort_phase_system,
        },
        render_resource::{
            BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, Face, FragmentState, MultisampleState, PipelineCache,
            PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            StoreOp, TextureFormat, VertexState,
        },
        renderer::RenderContext,
        sync_world::MainEntity,
        view::{
            ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewDepthTexture, ViewTarget,
        },
    },
};
use nonmax::NonMaxU32;

use super::{InteractionKind, InternalGizmoCamera, ScaleGizmo, TransformGizmo, TranslationGizmo};
use crate::{selection::NoSelect, theme::palette};

const SHADER_HANDLE: Handle<Shader> = uuid_handle!("4fbe65bc-2bf2-4281-9b77-6b5003acae10");

#[derive(Component, ExtractComponent, Clone, Copy, Default)]
struct DrawStencil;

#[derive(Resource)]
struct StencilPipeline {
    /// The base mesh pipeline defined by bevy
    ///
    /// Since we want to draw a stencil of an existing bevy mesh we want to reuse the default
    /// pipeline as much as possible
    mesh_pipeline: MeshPipeline,
    /// Stores the shader used for this pipeline directly on the pipeline.
    /// This isn't required, it's only done like this for simplicity.
    shader_handle: Handle<Shader>,
}

fn init_stencil_pipeline(mut commands: Commands, mesh_pipeline: Res<MeshPipeline>) {
    commands.insert_resource(StencilPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        shader_handle: SHADER_HANDLE,
    });
}

// For more information on how SpecializedMeshPipeline work, please look at the
// specialized_mesh_pipeline example
impl SpecializedMeshPipeline for StencilPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // We will only use the position of the mesh in our shader so we only need to specify that
        let mut vertex_attributes = Vec::new();
        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            // Make sure this matches the shader location
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }
        // This will automatically generate the correct `VertexBufferLayout` based on the vertex attributes
        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;
        let view_layout = self
            .mesh_pipeline
            .get_view_layout(MeshPipelineViewLayoutKey::from(key));
        Ok(RenderPipelineDescriptor {
            label: Some("stencil_pipeline".into()),
            // We want to reuse the data from bevy so we use the same bind groups as the default
            // mesh pipeline
            layout: vec![
                // Bind group 0 is the view uniform
                view_layout.main_layout.clone(),
                // Bind group 1 is empty
                view_layout.empty_layout.clone(),
                // Bind group 2 is the mesh uniform
                self.mesh_pipeline.mesh_layouts.model_only.clone(),
            ],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                buffers: vec![vertex_buffer_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                bias: default(),
                depth_compare: CompareFunction::Greater,
                depth_write_enabled: false,
                format: CORE_3D_DEPTH_FORMAT,
                stencil: default(),
            }),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                cull_mode: Some(Face::Back),
                ..default()
            },
            multisample: MultisampleState {
                count: key.msaa_samples(),
                ..default()
            },
            ..default()
        })
    }
}

// We will reuse render commands already defined by bevy to draw a 3d mesh
type DrawMesh3dStencil = (
    SetItemPipeline,
    // This will set the view bindings in group 0
    SetMeshViewBindGroup<0>,
    // This will set an empty bind group in group 1
    SetMeshViewEmptyBindGroup<1>,
    // This will set the mesh bindings in group 2
    SetMeshBindGroup<2>,
    // This will draw the mesh
    DrawMesh,
);

// This is the data required per entity drawn in a custom phase in bevy. More specifically this is the
// data required when using a ViewSortedRenderPhase. This would look differently if we wanted a
// batched render phase. Sorted phases are a bit easier to implement, but a batched phase would
// look similar.
//
// If you want to see how a batched phase implementation looks, you should look at the Opaque2d
// phase.
struct Stencil3d {
    pub sort_key: FloatOrd,
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    pub indexed: bool,
}

// For more information about writing a phase item, please look at the custom_phase_item example
impl PhaseItem for Stencil3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for Stencil3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // bevy normally uses radsort instead of the std slice::sort_by_key
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        // Since it is not re-exported by bevy, we just use the std sort for the purpose of the example
        items.sort_by_key(SortedPhaseItem::sort_key);
    }

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for Stencil3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

impl GetBatchData for StencilPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type CompareData = AssetId<Mesh>;
    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, _render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let mesh_uniform = {
            let mesh_transforms = &mesh_instance.transforms;
            let (local_from_world_transpose_a, local_from_world_transpose_b) =
                mesh_transforms.world_from_local.inverse_transpose_3x3();
            MeshUniform {
                world_from_local: mesh_transforms.world_from_local.to_transpose(),
                previous_world_from_local: mesh_transforms.previous_world_from_local.to_transpose(),
                lightmap_uv_rect: UVec2::ZERO,
                local_from_world_transpose_a,
                local_from_world_transpose_b,
                flags: mesh_transforms.flags,
                first_vertex_index,
                current_skin_index: u32::MAX,
                material_and_lightmap_bind_group_slot: 0,
                tag: 0,
                pad: 0,
            }
        };
        Some((mesh_uniform, None))
    }
}

impl GetFullBatchData for StencilPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        Some((
            mesh_instance.current_uniform_index,
            mesh_instance
                .should_batch()
                .then_some(mesh_instance.mesh_asset_id),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, _render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            None,
            None,
            None,
        ))
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        // Note that `IndirectParameters` covers both of these structures, even
        // though they actually have distinct layouts. See the comment above that
        // type for more information.
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                None => !0,
                Some(batch_set_index) => u32::from(batch_set_index),
            },
        };

        if indexed {
            indirect_parameters_buffers
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            indirect_parameters_buffers
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }

    fn get_binned_index(
        _param: &SystemParamItem<Self::Param>,
        _query_item: MainEntity,
    ) -> Option<NonMaxU32> {
        None
    }
}

// When defining a phase, we need to extract it from the main world and add it to a resource
// that will be used by the render world. We need to give that resource all views that will use
// that phase
fn extract_camera_phases(
    mut stencil_phases: ResMut<ViewSortedRenderPhases<Stencil3d>>,
    cameras: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();
    for (main_entity, camera) in &cameras {
        if !camera.is_active {
            continue;
        }
        // This is the main camera, so we use the first subview index (0)
        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);

        stencil_phases.insert_or_clear(retained_view_entity);
        live_entities.insert(retained_view_entity);
    }

    // Clear out all dead views.
    stencil_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

// This is a very important step when writing a custom phase.
//
// This system determines which meshes will be added to the phase.
fn queue_custom_meshes(
    custom_draw_functions: Res<DrawFunctions<Stencil3d>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<StencilPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    custom_draw_pipeline: Res<StencilPipeline>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut custom_render_phases: ResMut<ViewSortedRenderPhases<Stencil3d>>,
    mut views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa)>,
    has_marker: Query<(), With<DrawStencil>>,
) {
    for (view, visible_entities, msaa) in &mut views {
        let Some(custom_phase) = custom_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let draw_custom = custom_draw_functions.read().id::<DrawMesh3dStencil>();

        // Create the key based on the view.
        // In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        let rangefinder = view.rangefinder3d();
        // Since our phase can work on any 3d mesh we can reuse the default mesh 3d filter
        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            // We only want meshes with the marker component to be queued to our phase.
            if has_marker.get(*render_entity).is_err() {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            // Specialize the key for the current mesh entity
            // For this example we only specialize based on the mesh topology
            // but you could have more complex keys and that's where you'd need to create those keys
            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &custom_draw_pipeline,
                mesh_key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };
            let distance = rangefinder.distance(&mesh_instance.center);
            // At this point we have all the data we need to create a phase item and add it to our
            // phase
            custom_phase.add(Stencil3d {
                // Sort the data based on the distance to the view
                sort_key: FloatOrd(distance),
                entity: (*render_entity, *visible_entity),
                pipeline: pipeline_id,
                draw_function: draw_custom,
                // Sorted phase items aren't batched
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: mesh.indexed(),
            });
        }
    }
}

// Render label used to order our render graph node that will render our phase
#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
struct CustomDrawPassLabel;

#[derive(Default)]
struct CustomDrawNode;
impl ViewNode for CustomDrawNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        Option<&'static MainPassResolutionOverride>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, target, depth_target, resolution_override): QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // First, we need to get our phases resource
        let Some(stencil_phases) = world.get_resource::<ViewSortedRenderPhases<Stencil3d>>() else {
            return Ok(());
        };

        // Get the view entity from the graph
        let view_entity = graph.view_entity();

        // Get the phase for the current view running our node
        let Some(stencil_phase) = stencil_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        // Render pass setup
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("stencil_pass"),
            // For the purpose of the example, we will write directly to the view target. A real
            // stencil pass would write to a custom texture and that texture would be used in later
            // passes to render custom effects using it.
            color_attachments: &[Some(target.get_color_attachment())],
            // We don't bind any depth buffer for this pass
            depth_stencil_attachment: Some(depth_target.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        // Render the phase
        // This will execute each draw functions of each phase items queued in this phase
        stencil_phase.render(&mut render_pass, world, view_entity)?;

        Ok(())
    }
}

#[derive(Component)]
pub struct RotationGizmo;

#[derive(Component)]
pub struct ViewTranslateGizmo;

pub const GIZMO_LAYER: RenderLayers = RenderLayers::layer(12);

/// Startup system that builds the procedural mesh and materials of the gizmo.
fn build_gizmo(
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
            alpha_mode: AlphaMode::AlphaToCoverage,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
            ));

            // Uniform rotation
            parent.spawn((
                NoSelect,
                Mesh3d(uniform_rotation_mesh.clone()),
                DrawStencil,
                RotationGizmo,
                InteractionKind::RotateUniform,
                NotShadowCaster,
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
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
                GIZMO_LAYER,
            ));
        });

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            order: -1,
            ..default()
        },
        InternalGizmoCamera,
        GIZMO_LAYER,
    ));
}

pub(super) struct GizmoMeshPlugin;

impl Plugin for GizmoMeshPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_HANDLE, "stencil.wgsl", Shader::from_wgsl);

        app.add_systems(Startup, build_gizmo);

        app.add_plugins((
            ExtractComponentPlugin::<DrawStencil>::default(),
            SortedRenderPhasePlugin::<Stencil3d, MeshPipeline>::new(RenderDebugFlags::default()),
        ));

        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedMeshPipelines<StencilPipeline>>()
            .init_resource::<DrawFunctions<Stencil3d>>()
            .add_render_command::<Stencil3d, DrawMesh3dStencil>()
            .init_resource::<ViewSortedRenderPhases<Stencil3d>>()
            .add_systems(RenderStartup, init_stencil_pipeline)
            .add_systems(ExtractSchedule, extract_camera_phases)
            .add_systems(
                Render,
                (
                    queue_custom_meshes.in_set(RenderSystems::QueueMeshes),
                    sort_phase_system::<Stencil3d>.in_set(RenderSystems::PhaseSort),
                    batch_and_prepare_sorted_render_phase::<Stencil3d, StencilPipeline>
                        .in_set(RenderSystems::PrepareResources),
                ),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<CustomDrawNode>>(Core3d, CustomDrawPassLabel)
            // Tell the node to run after the main pass
            .add_render_graph_edges(Core3d, (Node3d::EndMainPass, CustomDrawPassLabel));
    }
}
