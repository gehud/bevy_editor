use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::{
        change_detection::DetectChanges,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Local, Query, Res},
    },
    input::InputSystems,
    math::Vec2,
    prelude::{Deref, DerefMut},
    window::{CursorGrabMode, CursorOptions, Window},
};
use bevy_egui::{EguiInputSet, EguiPreUpdateSet};

#[derive(Default, Deref, DerefMut, Resource)]
pub struct CursorLock(pub bool);

fn lock(
    lock: Res<CursorLock>,
    mut locked: Local<bool>,
    mut cursor_position: Local<Vec2>,
    mut windows: Query<(&mut Window, &mut CursorOptions)>,
) {
    if lock.is_changed() {
        if **lock && !*locked {
            for (window, mut cursor_options) in &mut windows {
                if !window.focused {
                    continue;
                }

                *cursor_position = window.cursor_position().unwrap_or_default();
                cursor_options.visible = false;
                cursor_options.grab_mode = CursorGrabMode::Locked;
            }

            *locked = true;
        } else if !**lock && *locked {
            for (_, mut cursor_options) in &mut windows {
                cursor_options.grab_mode = CursorGrabMode::None;
                cursor_options.visible = true;
            }

            *locked = false;
        }
    }

    if **lock {
        for (mut window, _) in &mut windows {
            if !window.focused {
                continue;
            }

            window.set_cursor_position(Some(*cursor_position));
        }
    }
}

pub struct CursorLockPlugin;

impl Plugin for CursorLockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorLock>().add_systems(
            PreUpdate,
            lock.after(EguiInputSet::WriteEguiEvents)
                .after(InputSystems)
                .before(EguiPreUpdateSet::BeginPass),
        );
    }
}
