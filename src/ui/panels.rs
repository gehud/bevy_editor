//! The editor's panels and the default dock arrangement.

use bevy::prelude::*;

use super::{
    asset_browser,
    dock::{Axis, DockLayout, PanelId},
    hierarchy, inspector,
    style::{colors, ink, label},
    viewport,
};

/// Every kind of panel the editor can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    SceneTree,
    Import,
    Viewport(u8),
    Console,
    Assets,
    Components,
    Resources,
    Systems,
}

impl PanelKind {
    pub fn title(self) -> String {
        match self {
            PanelKind::SceneTree => "Scene Tree".into(),
            PanelKind::Import => "Import".into(),
            PanelKind::Viewport(index) => format!("Viewport {}", index + 1),
            PanelKind::Console => "Console".into(),
            PanelKind::Assets => "Assets".into(),
            PanelKind::Components => "Components".into(),
            PanelKind::Resources => "Resources".into(),
            PanelKind::Systems => "Systems".into(),
        }
    }

    /// The panel's contents. Spawned once and kept alive while the layout changes.
    pub fn content(self) -> Box<dyn Scene> {
        match self {
            PanelKind::SceneTree => Box::new(hierarchy::scene_tree_content()),
            PanelKind::Viewport(index) => Box::new(viewport::viewport_content(index, index == 0)),
            PanelKind::Assets => Box::new(asset_browser::asset_browser_content()),
            PanelKind::Components => Box::new(inspector::components_content()),
            PanelKind::Import | PanelKind::Console | PanelKind::Resources | PanelKind::Systems => {
                Box::new(empty_panel())
            }
        }
    }
}

/// All panels; a [`PanelId`] indexes into this list.
#[derive(Resource)]
pub struct Panels(pub Vec<PanelKind>);

impl Panels {
    pub fn get(&self, id: PanelId) -> PanelKind {
        self.0[id.0 as usize]
    }

    pub fn ids(&self) -> impl Iterator<Item = PanelId> + '_ {
        (0..self.0.len() as u32).map(PanelId)
    }

    fn id_of(&self, kind: PanelKind) -> PanelId {
        PanelId(self.0.iter().position(|k| *k == kind).unwrap() as u32)
    }
}

impl Default for Panels {
    fn default() -> Self {
        Self(vec![
            PanelKind::SceneTree,
            PanelKind::Import,
            PanelKind::Viewport(0),
            PanelKind::Viewport(1),
            PanelKind::Viewport(2),
            PanelKind::Console,
            PanelKind::Assets,
            PanelKind::Components,
            PanelKind::Resources,
            PanelKind::Systems,
        ])
    }
}

/// The arrangement from the design. Sizes are the design's pixel sizes, used as weights.
pub fn default_layout(panels: &Panels) -> DockLayout {
    use PanelKind::*;
    let id = |kind| panels.id_of(kind);
    let mut layout = DockLayout::default();

    let scene_tree = layout.add_leaf(vec![id(SceneTree), id(Import)]);

    let main_view = layout.add_leaf(vec![id(Viewport(0))]);
    let side_view_top = layout.add_leaf(vec![id(Viewport(1))]);
    let side_view_bottom = layout.add_leaf(vec![id(Viewport(2))]);
    let side_views = layout.add_split(
        Axis::Vertical,
        vec![(side_view_top, 284.5), (side_view_bottom, 284.5)],
    );
    let views = layout.add_split(
        Axis::Horizontal,
        vec![(main_view, 713.0), (side_views, 351.0)],
    );
    // The bottom area opens on the asset browser.
    let assets = layout.add_leaf(vec![id(Console), id(Assets)]);
    layout.activate(assets, 1);
    let middle = layout.add_split(Axis::Vertical, vec![(views, 573.0), (assets, 245.0)]);

    let inspector = layout.add_leaf(vec![id(Components), id(Resources), id(Systems)]);

    let root = layout.add_split(
        Axis::Horizontal,
        vec![(scene_tree, 266.0), (middle, 1068.0), (inspector, 310.0)],
    );
    layout.set_root(root);
    layout
}

/// Placeholder for panels that are not implemented yet.
fn empty_panel() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BackgroundColor(colors::PANE_BG)
        Children [ @label("Nothing here yet", ink::DIM) ]
    }
}
