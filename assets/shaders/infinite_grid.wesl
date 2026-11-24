// Infinite ground grid on the y = 0 plane.
//
// The mesh is a unit quad. The vertex shader stretches it around the camera that is rendering,
// so every viewport sees a grid that never ends. Lines are computed per pixel from world
// coordinates and anti-aliased with screen-space derivatives.

import bevy_pbr::render::mesh_view_bindings::view;

struct GridSettings {
    minor_color: vec4<f32>,
    major_color: vec4<f32>,
    x_axis_color: vec4<f32>,
    z_axis_color: vec4<f32>,
    /// World units between minor lines; major lines are every 10 minor lines.
    spacing: f32,
    /// Grid fades out over this many world units at minimum (grows with camera height).
    fade_distance: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> grid: GridSettings;

struct Vertex {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

/// Distance at which the grid has fully faded, for the current camera.
fn fade_end() -> f32 {
    return max(grid.fade_distance, abs(view.world_position.y) * 40.0);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // The quad spans [-0.5, 0.5] on x and z; make it cover the whole visible grid.
    let extent = fade_end() * 2.0;
    let world = vec3<f32>(
        vertex.position.x * extent + view.world_position.x,
        0.0,
        vertex.position.z * extent + view.world_position.z,
    );
    var out: VertexOutput;
    out.clip_position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_position = world;
    return out;
}

/// Coverage (0..1) of grid lines spaced `spacing` apart, about one pixel wide.
fn lines(coord: vec2<f32>, spacing: f32) -> f32 {
    let cell = coord / spacing;
    let width = fwidth(cell);
    // Distance to the nearest line, in pixels.
    let distance = abs(fract(cell - 0.5) - 0.5) / width;
    let coverage = 1.0 - min(min(distance.x, distance.y), 1.0);
    // Fade lines out where they get closer together than a few pixels (avoids moiré).
    let density = max(width.x, width.y);
    return coverage * (1.0 - smoothstep(0.1, 0.4, density));
}

/// Coverage (0..1) of a single line where `coord` is 0, slightly wider than grid lines.
fn axis_line(coord: f32) -> f32 {
    let distance = abs(coord) / fwidth(coord);
    return 1.0 - min(distance / 1.5, 1.0);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let coord = in.world_position.xz;

    var color = grid.minor_color * lines(coord, grid.spacing);
    let major = lines(coord, grid.spacing * 10.0);
    color = mix(color, grid.major_color, major * grid.major_color.a);
    color.a = max(color.a, grid.major_color.a * major);

    // World axes: the X axis runs where z = 0, the Z axis where x = 0.
    let x_axis = axis_line(coord.y);
    let z_axis = axis_line(coord.x);
    color = mix(color, grid.x_axis_color, x_axis);
    color = mix(color, grid.z_axis_color, z_axis);

    // Fade with distance from the camera.
    let distance = length(coord - view.world_position.xz);
    let end = fade_end();
    color.a *= 1.0 - smoothstep(end * 0.25, end, distance);

    if color.a <= 0.001 {
        discard;
    }
    return color;
}
