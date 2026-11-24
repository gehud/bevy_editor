//! Editor cameras: one per viewport panel, separate from any camera in the edited scene.
//!
//! Controls, while the pointer is over a viewport:
//!
//! | Input                       | Action                                         |
//! |-----------------------------|------------------------------------------------|
//! | Hold right mouse button     | Fly: mouse looks around, W/A/S/D move, Q/E down/up |
//! | Shift (while flying)        | Move faster                                    |
//! | Scroll (while flying)       | Change flying speed                            |
//! | Hold middle mouse button    | Pan                                            |
//! | Scroll                      | Move forward / backward                        |
//!
//! While flying or panning the cursor is hidden and locked, and keyboard focus is taken from
//! any text field so W/A/S/D don't type into it.

use bevy::{
    camera::RenderTarget,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::InputFocus,
    prelude::*,
    render::render_resource::TextureFormat,
    ui::{UiGlobalTransform, UiSystems, widget::ViewportNode},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::{editing::EditorOnly, ui::ViewportSlot};

/// Background of the 3D views.
const CLEAR_COLOR: Color = Color::srgb_u8(0x3B, 0x3B, 0x3B);
/// Starting positions of viewports 0, 1, 2…; all look at [`LOOK_AT`].
const START_POSITIONS: [Vec3; 3] = [
    Vec3::new(-5.5, 3.2, 6.5),
    Vec3::new(-6.5, 3.0, 5.0),
    Vec3::new(-6.5, 1.5, 5.0),
];
const LOOK_AT: Vec3 = Vec3::new(0.0, 0.8, 0.0);

/// Radians of rotation per pixel of mouse motion.
const LOOK_SENSITIVITY: f32 = 0.003;
/// Default flying speed, in units per second.
const DEFAULT_SPEED: f32 = 5.0;
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.05..=500.0;
/// Speed multiplier while Shift is held.
const BOOST: f32 = 3.0;
/// Flying speed changes by this factor per scroll line.
const SPEED_STEP: f32 = 1.1;
/// Distance moved per scroll line when not flying, as a fraction of the flying speed.
const DOLLY_PER_LINE: f32 = 0.1;
/// Pan distance per pixel of mouse motion, as a fraction of the flying speed.
const PAN_PER_PIXEL: f32 = 0.002;
/// Pixels per "line" for touchpads and other pixel-precise scroll devices.
const PIXELS_PER_LINE: f32 = 100.0;

pub struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .add_systems(
                Update,
                (
                    attach_viewport_cameras,
                    (begin_end_navigation, fly, pan, dolly).chain(),
                ),
            )
            // After layout, so a viewport that just became visible renders in the same frame
            // instead of showing an empty image for one frame.
            .add_systems(PostUpdate, pause_hidden_cameras.after(UiSystems::Layout));
    }
}

/// A camera owned by the editor (one per viewport), as opposed to a camera in the edited scene.
#[derive(Component)]
pub struct EditorCamera;

/// Orientation and speed of an editor camera. Yaw and pitch are kept here, rather than
/// derived from the transform each frame, so looking around never accumulates roll.
#[derive(Component)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// Flying speed in units per second.
    pub speed: f32,
}

impl FlyCamera {
    fn from_transform(transform: &Transform) -> Self {
        let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
        Self {
            yaw,
            pitch,
            speed: DEFAULT_SPEED,
        }
    }

    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }
}

/// Largest pitch magnitude: just short of straight up/down, so yaw stays well defined.
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.001;
/// Orbit distance used by [`look_along`] when the view doesn't hit the ground.
const DEFAULT_ORBIT_DISTANCE: f32 = 10.0;

/// Turns the camera to look in `direction`, orbiting around the point it was looking at, so that
/// point stays in view (as when clicking an axis on the orientation gizmo).
///
/// The orbit pivot is where the view ray meets the ground plane, if that is reasonably close,
/// and otherwise a point [`DEFAULT_ORBIT_DISTANCE`] ahead.
pub fn look_along(transform: &mut Transform, fly: &mut FlyCamera, direction: Vec3) {
    let direction = direction.normalize();
    let forward = *transform.forward();
    let ground_distance = (forward.y < -0.01)
        .then(|| -transform.translation.y / forward.y)
        .filter(|distance| (0.1..1000.0).contains(distance));
    let distance = ground_distance.unwrap_or(DEFAULT_ORBIT_DISTANCE);
    let pivot = transform.translation + forward * distance;

    // A camera's forward is -Z rotated by yaw (around Y) and then pitch (around X).
    fly.pitch = direction.y.asin().clamp(-MAX_PITCH, MAX_PITCH);
    fly.yaw = if direction.x.abs() > 1e-4 || direction.z.abs() > 1e-4 {
        (-direction.x).atan2(-direction.z)
    } else {
        // Straight up or down: align the view so +X points right, as in top/bottom views.
        0.0
    };
    transform.rotation = fly.rotation();
    transform.translation = pivot - *transform.forward() * distance;
}

#[derive(Clone, Copy, PartialEq)]
enum NavigationMode {
    Fly,
    Pan,
}

impl NavigationMode {
    fn button(self) -> MouseButton {
        match self {
            NavigationMode::Fly => MouseButton::Right,
            NavigationMode::Pan => MouseButton::Middle,
        }
    }
}

/// The camera currently being flown or panned, if any.
#[derive(Resource, Default)]
struct Navigation {
    active: Option<(NavigationMode, Entity)>,
    /// Where the cursor was when navigation started; it is put back there afterwards.
    cursor_start: Option<Vec2>,
}

/// Spawns an editor camera for each new viewport node and connects the two.
fn attach_viewport_cameras(
    mut commands: Commands,
    slots: Query<(Entity, &ViewportSlot), Without<ViewportNode>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (node, slot) in &slots {
        // `ViewportNode` resizes the render target to the node's size.
        let image = Image::new_target_texture(1, 1, TextureFormat::Bgra8UnormSrgb, None);
        let position = START_POSITIONS[slot.0 as usize % START_POSITIONS.len()];
        let transform = Transform::from_translation(position).looking_at(LOOK_AT, Vec3::Y);
        let camera = commands
            .spawn((
                Name::new(format!("Editor Camera {}", slot.0 + 1)),
                EditorCamera,
                EditorOnly,
                FlyCamera::from_transform(&transform),
                Camera3d::default(),
                Camera {
                    // Render before the UI camera that displays the result.
                    order: -1 - slot.0 as isize,
                    clear_color: ClearColorConfig::Custom(CLEAR_COLOR),
                    ..default()
                },
                RenderTarget::Image(images.add(image).into()),
                transform,
            ))
            .id();
        commands.entity(node).insert(ViewportNode::new(camera));
    }
}

/// Stops rendering viewports that are not on screen (e.g. in a background tab).
fn pause_hidden_cameras(
    viewports: Query<(&ViewportNode, &ComputedNode)>,
    mut cameras: Query<&mut Camera, With<EditorCamera>>,
) {
    for (viewport, node) in &viewports {
        if let Some(camera) = viewport.camera
            && let Ok(mut camera) = cameras.get_mut(camera)
        {
            let visible = node.size.x > 0.0 && node.size.y > 0.0;
            if camera.is_active != visible {
                camera.is_active = visible;
            }
        }
    }
}

/// The editor camera of the viewport under the cursor.
fn camera_under_cursor(
    window: &Window,
    viewports: &Query<(&ViewportNode, &ComputedNode, &UiGlobalTransform)>,
) -> Option<Entity> {
    let cursor = window.physical_cursor_position()?;
    viewports
        .iter()
        .find(|(_, node, transform)| node.contains_point(**transform, cursor))
        .and_then(|(viewport, _, _)| viewport.camera)
}

/// Starts flying / panning when a mouse button is pressed over a viewport, and stops when it
/// is released.
fn begin_end_navigation(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    viewports: Query<(&ViewportNode, &ComputedNode, &UiGlobalTransform)>,
    mut navigation: ResMut<Navigation>,
    mut focus: ResMut<InputFocus>,
) {
    let (mut window, mut cursor) = window.into_inner();
    match navigation.active {
        None => {
            let mode = if mouse.just_pressed(MouseButton::Right) {
                NavigationMode::Fly
            } else if mouse.just_pressed(MouseButton::Middle) {
                NavigationMode::Pan
            } else {
                return;
            };
            let Some(camera) = camera_under_cursor(&window, &viewports) else {
                return;
            };
            navigation.active = Some((mode, camera));
            navigation.cursor_start = window.cursor_position();
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
            focus.clear();
        }
        Some((mode, _)) => {
            if mouse.pressed(mode.button()) {
                return;
            }
            navigation.active = None;
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
            if let Some(position) = navigation.cursor_start.take() {
                window.set_cursor_position(Some(position));
            }
        }
    }
}

fn scroll_lines(scroll: &AccumulatedMouseScroll) -> f32 {
    match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
    }
}

fn fly(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    navigation: Res<Navigation>,
    mut cameras: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let Some((NavigationMode::Fly, camera)) = navigation.active else {
        return;
    };
    let Ok((mut transform, mut fly)) = cameras.get_mut(camera) else {
        return;
    };

    // Look around.
    fly.yaw -= motion.delta.x * LOOK_SENSITIVITY;
    fly.pitch = (fly.pitch - motion.delta.y * LOOK_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
    transform.rotation = fly.rotation();

    // Scrolling while flying changes speed, exponentially so it feels the same at any speed.
    let lines = scroll_lines(&scroll);
    if lines != 0.0 {
        fly.speed =
            (fly.speed * SPEED_STEP.powf(lines)).clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
    }

    // Move: W/S forward/back and A/D sideways relative to the view; Q/E down/up in the world.
    let axis = |positive: KeyCode, negative: KeyCode| {
        keys.pressed(positive) as i32 as f32 - keys.pressed(negative) as i32 as f32
    };
    let forward = axis(KeyCode::KeyW, KeyCode::KeyS);
    let right = axis(KeyCode::KeyD, KeyCode::KeyA);
    let up = axis(KeyCode::KeyE, KeyCode::KeyQ);
    let direction = (transform.forward() * forward + transform.right() * right + Vec3::Y * up)
        .normalize_or_zero();
    let boost = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        BOOST
    } else {
        1.0
    };
    transform.translation += direction * fly.speed * boost * time.delta_secs();
}

fn pan(
    motion: Res<AccumulatedMouseMotion>,
    navigation: Res<Navigation>,
    mut cameras: Query<(&mut Transform, &FlyCamera)>,
) {
    let Some((NavigationMode::Pan, camera)) = navigation.active else {
        return;
    };
    let Ok((mut transform, fly)) = cameras.get_mut(camera) else {
        return;
    };
    // Drag the scene with the mouse: moving the mouse right moves the camera left.
    let step = fly.speed * PAN_PER_PIXEL;
    let offset = transform.left() * motion.delta.x * step + transform.up() * motion.delta.y * step;
    transform.translation += offset;
}

/// Scrolling over a viewport (when not flying) moves its camera forward or backward.
fn dolly(
    scroll: Res<AccumulatedMouseScroll>,
    navigation: Res<Navigation>,
    window: Single<&Window, With<PrimaryWindow>>,
    viewports: Query<(&ViewportNode, &ComputedNode, &UiGlobalTransform)>,
    mut cameras: Query<(&mut Transform, &FlyCamera)>,
) {
    if navigation.active.is_some() {
        return;
    }
    let lines = scroll_lines(&scroll);
    if lines == 0.0 {
        return;
    }
    let Some(camera) = camera_under_cursor(&window, &viewports) else {
        return;
    };
    if let Ok((mut transform, fly)) = cameras.get_mut(camera) {
        let forward = transform.forward();
        transform.translation += forward * lines * fly.speed * DOLLY_PER_LINE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_at(position: Vec3) -> (Transform, FlyCamera) {
        let transform = Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y);
        let fly = FlyCamera::from_transform(&transform);
        (transform, fly)
    }

    #[test]
    fn look_along_faces_the_direction_and_keeps_the_ground_pivot_in_view() {
        let (mut transform, mut fly) = camera_at(Vec3::new(-5.0, 5.0, 5.0));
        look_along(&mut transform, &mut fly, Vec3::NEG_X);
        assert!(transform.forward().dot(Vec3::NEG_X) > 0.999);
        // The camera was looking at the origin; it still is, from the +X side.
        let pivot_distance = Vec3::new(5.0, 5.0, 5.0).length();
        assert!((transform.translation - Vec3::new(pivot_distance, 0.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn look_along_straight_down_is_a_top_view_with_x_to_the_right() {
        let (mut transform, mut fly) = camera_at(Vec3::new(0.0, 4.0, 4.0));
        look_along(&mut transform, &mut fly, Vec3::NEG_Y);
        assert!(transform.forward().dot(Vec3::NEG_Y) > 0.999);
        assert!(transform.right().dot(Vec3::X) > 0.999);
        assert!(transform.translation.x.abs() < 1e-3 && transform.translation.z.abs() < 1e-2);
        assert!(transform.translation.y > 0.0);
    }
}
