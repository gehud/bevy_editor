//! The editor window's user interface, built with `bevy_feathers`.
//!
//! Panels live in a docking layout ([`dock`]) that starts out as the "Bevy Editor" Figma
//! design:
//!
//! ```text
//! ┌ title bar ──────────────────────────────────────────────────────────────┐
//! │ Scene Tree    │ main viewport              │ viewport │ Components      │
//! │               │                            ├──────────┤ (inspector)     │
//! │               │                            │ viewport │                 │
//! │               ├────────────────────────────┴──────────┤                 │
//! │               │ Assets                                │                 │
//! └ status bar ─────────────────────────────────────────────────────────────┘
//! ```

mod asset_browser;
mod axis_gizmo;
mod dock;
mod hierarchy;
pub mod icons;
mod inspector;
mod panels;
mod status_bar;
mod style;
mod title_bar;
mod viewport;
mod widgets;
mod window_frame;

use bevy::{
    feathers::{
        FeathersPlugins,
        dark_theme::create_dark_theme,
        theme::{SemanticToken, ThemeProps, UiTheme},
    },
    input_focus::tab_navigation::TabGroup,
    prelude::*,
};

use style::colors;

pub use viewport::{ViewportSlot, spawn_demo_scene};

/// Theme tokens the editor adds on top of the feathers dark theme.
pub mod tokens {
    use bevy::feathers::theme::ThemeToken;

    /// Placeholder text in search and filter fields.
    pub const PLACEHOLDER: ThemeToken = ThemeToken::new_static("editor.text.placeholder");
}

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        let panels = panels::Panels::default();
        let layout = panels::default_layout(&panels);
        app.add_plugins((FeathersPlugins, bevy_ui_svg::SvgPlugin, dock::DockPlugin))
            .add_plugins(asset_browser::AssetBrowserPlugin)
            .insert_resource(UiTheme(editor_theme()))
            .insert_resource(panels)
            .insert_resource(layout)
            .init_resource::<window_frame::WindowMaximized>()
            .init_resource::<hierarchy::SceneTreeState>()
            .add_observer(hierarchy::record_spawn_order)
            .add_observer(hierarchy::forget_spawn_order)
            .init_resource::<inspector::InspectorState>()
            .add_systems(
                Startup,
                (
                    setup_ui_camera,
                    hierarchy::setup_drop_indicator,
                    editor_root.spawn(),
                ),
            )
            .add_systems(
                PostUpdate,
                axis_gizmo::update_axis_gizmos.before(bevy::ui::UiSystems::Layout),
            )
            .add_systems(
                Update,
                (
                    style::update_hover_backgrounds,
                    status_bar::update_project_label,
                    (
                        hierarchy::scene_tree_shortcuts,
                        hierarchy::cancel_lost_drag,
                        hierarchy::read_scene_tree_filter,
                        hierarchy::sync_scene_tree,
                        hierarchy::init_rename_fields,
                    )
                        .chain(),
                    (
                        inspector::read_inspector_filter,
                        inspector::sync_inspector,
                        inspector::refresh_inspector_fields,
                    )
                        .chain(),
                    (
                        window_frame::sync_window_maximized,
                        (
                            title_bar::update_maximize_icon,
                            window_frame::toggle_resize_handles,
                            window_frame::toggle_window_outline,
                        ),
                    )
                        .chain(),
                ),
            );
    }
}

/// The feathers dark theme plus the editor's own tokens.
fn editor_theme() -> ThemeProps {
    let mut theme = create_dark_theme();
    let semantic = SemanticToken::new_static("editor.text.placeholder");
    theme
        .token_assignments
        .insert(tokens::PLACEHOLDER, semantic.clone());
    theme.semantic_base.insert(semantic, style::ink::DADA);
    theme
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("UI Camera"),
        Camera2d,
        IsDefaultUiCamera,
        crate::editing::EditorOnly,
    ));
}

fn editor_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            // Without these the root grows to fit tall panel contents instead of the window.
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor::all(colors::WINDOW_BORDER)
        window_frame::WindowOutline
        TabGroup
        Children [
            @title_bar::title_bar()
            --
            // Editor area: the dock above a status bar.
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::horizontal(px(4)),
            }
            Children [
                Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    width: percent(100),
                }
                dock::DockRoot
                --
                @status_bar::status_bar()
            ]
            --
            @window_frame::resize_handles()
        ]
    }
}
