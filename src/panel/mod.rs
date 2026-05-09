use bevy::{
    app::{App, Plugin},
    ecs::{error::Result, resource::Resource, world::World},
    log::{warn, warn_once},
    platform::collections::HashMap,
    utils::default,
};
use egui::{LayerId, Margin, RichText, Sense, Ui, UiBuilder, WidgetText};
use serde::{Deserialize, Serialize};

use crate::{
    dock::{DockState, NodeIndex, TabStyle, TabViewer},
    prefs::RegisterPref,
    selection::SelectionMap,
};

pub trait Panel: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn padding(&self) -> Option<Margin> {
        None
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result;
}

#[derive(Default, Resource)]
pub(crate) struct PanelRegistry {
    panes: HashMap<String, Box<dyn Panel>>,
}

impl PanelRegistry {
    pub fn get_panel_mut(&mut self, name: &str) -> Option<&mut dyn Panel> {
        self.panes.get_mut(name).map(|pane| pane.as_mut())
    }

    pub fn get_panel(&self, name: &str) -> Option<&dyn Panel> {
        self.panes.get(name).map(|pane| pane.as_ref())
    }

    pub fn names(&self) -> impl ExactSizeIterator<Item = &String> {
        self.panes.keys()
    }
}

pub trait PanelApp {
    fn register_panel<P: Panel>(&mut self, pane: P) -> &mut Self;
}

impl PanelApp for App {
    fn register_panel<P: Panel>(&mut self, pane: P) -> &mut Self {
        let pane = Box::new(pane);

        if let Some(old) = self
            .world_mut()
            .resource_mut::<PanelRegistry>()
            .panes
            .insert(pane.name().into(), pane)
        {
            warn!("Pane with name: '{}' has be replaced", old.name());
        }

        self
    }
}

pub(crate) type Tab = String;

pub(crate) struct PanelViewer<'a> {
    pub registry: &'a mut PanelRegistry,
    pub world: &'a mut World,
    pub result: Result,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        RichText::from(tab).into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        if self.result.is_err() {
            return;
        }

        if let Some(panel) = self.registry.get_panel_mut(tab) {
            self.result = panel.ui(ui, self.world);
        } else {
            ui.label("Missing");
        }
    }

    fn tab_style_override(&self, tab: &Self::Tab, global_style: &TabStyle) -> Option<TabStyle> {
        let pane = self.registry.get_panel(tab)?;
        let custom_padding = pane.padding()?;
        let mut style_override = global_style.clone();
        style_override.tab_body.inner_margin = custom_padding;
        Some(style_override)
    }
}

#[derive(Resource, Serialize, Deserialize)]
pub(crate) struct PanelDocking(pub DockState<Tab>);

impl Default for PanelDocking {
    fn default() -> Self {
        let mut dock_state = DockState::new(vec!["Viewport".into()]);

        let surface = dock_state.main_surface_mut();

        let [viewport_node, _hierarchy_node] =
            surface.split_left(NodeIndex::root(), 0.15, vec!["Scene Tree".into()]);

        let [viewport_node, _properties_node] =
            surface.split_right(viewport_node, 0.80, vec!["Properties".into()]);

        let [_viewport_node, _explorer_node] = surface.split_below(
            viewport_node,
            0.75,
            vec!["Asset Browser".into()],
        );

        Self(dock_state)
    }
}

pub struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelRegistry>()
            .register_pref::<PanelDocking>();
    }
}
