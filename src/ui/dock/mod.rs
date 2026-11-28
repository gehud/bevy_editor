//! A docking layout: panels arranged in resizable splits and tab stacks, rearranged by
//! dragging tabs onto another area.
//!
//! - [`layout`]: the data model, independent of the UI.
//! - [`view`]: builds the UI from the model, keeping panel contents alive across rebuilds.
//! - [`interact`]: tab clicks, splitter drags and panel drag-and-drop.

mod interact;
mod layout;
mod view;

use bevy::{prelude::*, ui::UiSystems};

pub use layout::{Axis, DockLayout, PanelId};
pub use view::DockRoot;

pub struct DockPlugin;

impl Plugin for DockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<interact::DockDrag>()
            .add_systems(Startup, (view::setup_dock, interact::setup_drop_preview))
            .add_systems(Update, view::rebuild_dock)
            .add_systems(
                PostUpdate,
                view::place_panel_contents.before(UiSystems::Prepare),
            );
    }
}
