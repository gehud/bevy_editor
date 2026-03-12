mod output;

pub use output::*;

use bevy::{
    ecs::{resource::Resource, world::World},
    log::warn,
    platform::collections::HashMap,
};
use egui::{Ui, WidgetText};

use crate::dock::{DockState, NodeIndex, TabViewer};

pub trait Pane: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn ui(&mut self, world: &mut World, ui: &mut Ui);
}

#[derive(Resource)]
pub struct Panes {
    panes: HashMap<String, Box<dyn Pane>>,
}

impl Default for Panes {
    fn default() -> Self {
        let mut panes = Self {
            panes: HashMap::new(),
        };

        panes.insert(OutputPane);

        panes
    }
}

impl Panes {
    pub fn get_mut(&mut self, tab: &Tab) -> Option<&mut Box<dyn Pane>> {
        self.panes.get_mut(tab)
    }

    pub fn insert(&mut self, pane: impl Pane) -> Option<Box<dyn Pane>> {
        self.panes.insert(pane.name().into(), Box::new(pane))
    }
}

pub type Tab = String;

pub struct PaneViewer<'a> {
    pub world: &'a mut World,
}

impl<'a> TabViewer for PaneViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        self.world.resource_scope::<Panes, _>(|world, mut panes| {
            if let Some(pane) = panes.get_mut(tab) {
                pane.ui(world, ui);
            } else {
                warn!("Missing pane: {}", tab);
            }
        });
    }
}

#[derive(Resource)]
pub struct Docking(pub DockState<Tab>);

impl Default for Docking {
    fn default() -> Self {
        let mut dock_state = DockState::new(vec!["Viewport".into()]);

        let surface = dock_state.main_surface_mut();

        let [viewport_node, _hierarchy_node] =
            surface.split_left(NodeIndex::root(), 0.15, vec!["Hierarchy".into()]);

        let [viewport_node, _properties_node] =
            surface.split_right(viewport_node, 0.80, vec!["Properties".into()]);

        let [_viewport_node, _explorer_node] = surface.split_below(
            viewport_node,
            0.75,
            vec!["Output".into(), "Explorer".into()],
        );

        Self(dock_state)
    }
}
