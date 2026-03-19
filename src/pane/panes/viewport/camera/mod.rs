use bevy::{
    app::{App, Plugin, Update},
    camera::{Camera, RenderTarget},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        lifecycle::{Insert, Remove},
        observer::On,
        query::With,
        reflect::ReflectComponent,
        system::{Commands, Query, Res, Single},
    },
    input::{
        ButtonInput,
        mouse::{MouseButton, MouseButtonInput},
    },
    log::info,
    picking::{
        events::{DragStart, Pointer},
        pointer::{PointerId, PointerLocation},
    },
    reflect::{Reflect, prelude::ReflectDefault},
    ui::{Node, UiTargetCamera, percent},
    utils::default,
    window::PrimaryWindow,
};

#[derive(Component, Debug, Default, Reflect)]
#[reflect(Debug, Component, Default)]
#[require(Camera)]
pub struct ViewportCamera {
    drag: bool,
}

#[derive(Component)]
struct ViewportCameraUi(Entity);

pub struct ViewportCameraPlugin;

impl Plugin for ViewportCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_insert_viewport_camera)
            .add_observer(on_remove_viewport_camera);
    }
}

fn on_insert_viewport_camera(trigger: On<Insert, ViewportCamera>, mut commands: Commands) {
    let root = commands
        .spawn((
            UiTargetCamera(trigger.entity),
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .observe(|_: On<Pointer<DragStart>>| {
            info!("Drag");
        })
        .id();

    commands
        .entity(trigger.entity)
        .insert(ViewportCameraUi(root));
}

fn on_remove_viewport_camera(
    trigger: On<Remove, ViewportCamera>,
    viewport_camera_ui: Query<&ViewportCameraUi>,
    mut commands: Commands,
) -> Result {
    let root = viewport_camera_ui.get(trigger.entity)?.0;
    commands.entity(root).despawn();
    Ok(())
}
