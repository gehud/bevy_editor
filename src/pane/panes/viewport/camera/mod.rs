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

pub struct ViewportCameraPlugin;

impl Plugin for ViewportCameraPlugin {
    fn build(&self, app: &mut App) {

    }
}
