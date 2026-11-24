//! Window behavior that the OS normally provides for decorated windows: resizing from the
//! edges, and tracking whether the window is maximized.
//!
//! The editor window has `decorations: false` (it draws its own title bar), which also removes
//! the OS resize border, so these handles stand in for it.

use bevy::{
    ecs::system::NonSendMarker,
    math::CompassOctant,
    picking::{cursor::EntityCursor, events::PointerPress},
    prelude::*,
    window::{PrimaryWindow, SystemCursorIcon},
    winit::WINIT_WINDOWS,
};

/// Thickness of the edge resize zones.
const EDGE: f32 = 5.0;
/// Size of the corner resize zones (larger, so corners are easy to grab).
const CORNER: f32 = 10.0;

/// Whether the primary window is currently maximized, as reported by the OS.
#[derive(Resource, Default, PartialEq)]
pub struct WindowMaximized(pub bool);

/// An invisible zone that resizes the window when dragged.
#[derive(Component, Clone, Copy)]
struct ResizeHandle(CompassOctant);

impl Default for ResizeHandle {
    fn default() -> Self {
        Self(CompassOctant::North)
    }
}

/// Resize zones along all edges and corners, drawn above the rest of the UI.
pub fn resize_handles() -> impl Scene {
    use CompassOctant::*;
    let handles = [
        North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest,
    ]
    .into_iter()
    .map(|direction| bsn! { @resize_handle(direction) })
    .collect::<Vec<_>>();
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
        }
        Pickable::IGNORE
        GlobalZIndex(i32::MAX)
        ResizeHandles
        Children [ {handles} ]
    }
}

/// Container of the resize zones; hidden while the window is maximized.
#[derive(Component, Clone, Default)]
pub(super) struct ResizeHandles;

fn resize_handle(direction: CompassOctant) -> impl Scene {
    use CompassOctant::*;
    let auto = Val::Auto;
    // (left, right, top, bottom, width, height)
    let (left, right, top, bottom, width, height) = match direction {
        North => (px(CORNER), px(CORNER), px(0), auto, auto, px(EDGE)),
        South => (px(CORNER), px(CORNER), auto, px(0), auto, px(EDGE)),
        West => (px(0), auto, px(CORNER), px(CORNER), px(EDGE), auto),
        East => (auto, px(0), px(CORNER), px(CORNER), px(EDGE), auto),
        NorthWest => (px(0), auto, px(0), auto, px(CORNER), px(CORNER)),
        NorthEast => (auto, px(0), px(0), auto, px(CORNER), px(CORNER)),
        SouthWest => (px(0), auto, auto, px(0), px(CORNER), px(CORNER)),
        SouthEast => (auto, px(0), auto, px(0), px(CORNER), px(CORNER)),
    };
    let cursor = match direction {
        North | South => SystemCursorIcon::NsResize,
        East | West => SystemCursorIcon::EwResize,
        NorthWest | SouthEast => SystemCursorIcon::NwseResize,
        NorthEast | SouthWest => SystemCursorIcon::NeswResize,
    };
    let handle = ResizeHandle(direction);
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: left,
            right: right,
            top: top,
            bottom: bottom,
            width: width,
            height: height,
        }
        handle
        EntityCursor::System(cursor)
        on(start_resize)
    }
}

fn start_resize(
    mut press: On<PointerPress>,
    handles: Query<&ResizeHandle>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(handle) = handles.get(press.entity) else {
        return;
    };
    press.propagate(false);
    if press.button == PointerButton::Primary {
        window.start_drag_resize(handle.0);
    }
}

/// Reads the maximized state from the OS window, so it stays correct however the window was
/// maximized (caption button, double-click, Win+Up, snapping to the screen edge).
pub(super) fn sync_window_maximized(
    window: Single<Entity, With<PrimaryWindow>>,
    mut maximized: ResMut<WindowMaximized>,
    // `WINIT_WINDOWS` is thread-local to the main thread.
    _main_thread: NonSendMarker,
) {
    let is_maximized = WINIT_WINDOWS.with_borrow(|windows| {
        windows
            .get_window(*window)
            .is_some_and(|window| window.is_maximized())
    });
    maximized.set_if_neq(WindowMaximized(is_maximized));
}

/// A maximized window has no border to drag, so the resize zones are turned off.
pub(super) fn toggle_resize_handles(
    maximized: Res<WindowMaximized>,
    mut handles: Query<&mut Node, With<ResizeHandles>>,
) {
    let display = if maximized.0 {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut handles {
        if node.display != display {
            node.display = display;
        }
    }
}
