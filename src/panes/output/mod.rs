mod collector;
mod widget;

use bevy::{
    ecs::world::World,
    log::{BoxedFmtLayer, BoxedLayer, tracing_subscriber::fmt::Layer},
};
use egui::Ui;

use crate::{
    pane::Pane,
    panes::output::{collector::COLLECTOR, widget::Logs},
};

pub fn custom_layer() -> BoxedLayer {
    Box::new(vec![COLLECTOR.clone()])
}

pub fn fmt_layer() -> BoxedFmtLayer {
    Box::new(Layer::default().with_target(true))
}

pub struct OutputPane;

impl Pane for OutputPane {
    fn name(&self) -> &str {
        "Output"
    }

    fn ui(&mut self, world: &mut World, ui: &mut Ui) {
        ui.add(Logs);
    }
}
