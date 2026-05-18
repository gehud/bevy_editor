use bevy::{
    app::{App, Plugin}, color::Color, ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        lifecycle::Add,
        observer::On,
        resource::Resource,
        system::{Commands, In, IntoSystem, SystemId},
    }, log::warn, platform::collections::HashMap, ui::{BackgroundColor, Node, PositionType, percent}, utils::default
};

pub type PanelSystem = SystemId<In<PanelStructure>, Result>;

pub struct PanelStructure {
    root: Entity,
}

impl PanelStructure {
    pub fn root(&self) -> Entity {
        self.root
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

#[derive(Clone, Component, Copy, Default)]
pub struct PanelArea;

#[derive(Clone, Component, Copy, Default)]
pub(crate) struct PanelAreaRoot;

pub fn setup_area(trigger: On<Add, PanelArea>, mut commands: Commands) {
    let area = trigger.event_target();

    commands.entity(area).with_children(|commands| {
        commands.spawn((
            PanelAreaRoot,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ));
    });
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
        app.init_resource::<PanelRegistry>()
            .add_observer(setup_area);
    }
}
