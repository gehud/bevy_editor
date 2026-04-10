use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::{
        change_detection::DetectChanges,
        resource::Resource,
        system::{Local, Query, ResMut},
    },
    math::Vec2,
    window::{CursorGrabMode, CursorOptions, Window},
};

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CursorLock(pub bool);

fn lock_cursor(
    cursor_lock: ResMut<CursorLock>,
    editor_windows: Query<(&mut Window, &mut CursorOptions)>,
    mut lock_position: Local<Vec2>,
) {
    for (mut window, mut cursor) in editor_windows {
        if cursor_lock.is_changed() {
            if cursor_lock.0 && window.focused {
                *lock_position = window.cursor_position().unwrap_or_default();
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            } else {
                cursor.grab_mode = CursorGrabMode::None;
                cursor.visible = true;
            }
        }

        if cursor_lock.0 {
            window.set_cursor_position(Some(*lock_position));
        }
    }
}

pub struct CursorLockPlugin;

impl Plugin for CursorLockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorLock>()
            .add_systems(PreUpdate, lock_cursor);
    }
}
