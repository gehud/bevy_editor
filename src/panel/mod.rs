pub(crate) mod layout;

use bevy::{
    app::{App, Plugin},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::Children,
        lifecycle::Add,
        observer::On,
        resource::Resource,
        system::{Commands, In, IntoSystem, SystemId},
        template::FromTemplate,
    },
    log::warn,
    picking::events::{DragStart, Pointer},
    platform::collections::HashMap,
    scene::{Scene, bsn, on},
    ui::{FlexDirection, Node, percent, px},
    utils::default,
    window::SystemCursorIcon,
};

use crate::{cursor::EntityCursor, panel::layout::EditorPanelLayoutPlugin};

pub type PanelSystem = SystemId<In<PanelStructure>, Result>;

#[derive(Clone, Component, Copy, FromTemplate)]
pub struct PanelStructure {
    content: Entity,
}

impl Default for PanelStructure {
    fn default() -> Self {
        Self {
            content: Entity::PLACEHOLDER,
        }
    }
}

impl PanelStructure {
    pub fn content(&self) -> Entity {
        self.content
    }
}

#[derive(Default, Resource)]
pub struct PanelRegistry {
    panels: HashMap<String, PanelSystem>,
}

impl PanelRegistry {
    pub fn register_panel(&mut self, name: impl Into<String>, system: PanelSystem) -> &mut Self {
        let name = name.into();
        if self.panels.insert(name.clone(), system).is_some() {
            warn!("Panel is name '{}' was replaced", name);
        }
        self
    }
}

pub trait EditorPanelApp {
    fn register_panel<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PanelStructure>, Result, M> + 'static,
    ) -> &mut Self;
}

impl EditorPanelApp for App {
    fn register_panel<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PanelStructure>, Result, M> + 'static,
    ) -> &mut Self {
        let id = self.world_mut().register_system(system);
        self.world_mut()
            .resource_mut::<PanelRegistry>()
            .register_panel(name, id);
        self
    }
}

pub struct EditorPanelPlugin;

impl Plugin for EditorPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorPanelLayoutPlugin)
            .init_resource::<PanelRegistry>();
    }
}
