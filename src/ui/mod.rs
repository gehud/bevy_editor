//! The editor window's user interface, built with `bevy_feathers`.
//!
//! Layout follows the "Bevy Editor" Figma design:
//!
//! ```text
//! ┌ title bar ──────────────────────────────────────────────────────────────┐
//! │ Scene Tree    │ main viewport              │ viewport │ Components      │
//! │               │                            ├──────────┤ (inspector)     │
//! ├───────────────┤                            │ viewport │                 │
//! │ Project Files ├────────────────────────────┴──────────┤                 │
//! │               │ asset browser                         │                 │
//! └ status bar ─────────────────────────────────────────────────────────────┘
//! ```

mod asset_browser;
mod inspector;
mod scene_tree;
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

/// Theme tokens the editor adds on top of the feathers dark theme.
pub mod tokens {
    use bevy::feathers::theme::ThemeToken;

    /// Placeholder text in search and filter fields.
    pub const PLACEHOLDER: ThemeToken = ThemeToken::new_static("editor.text.placeholder");
}

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FeathersPlugins)
            .insert_resource(UiTheme(editor_theme()))
            .init_resource::<window_frame::WindowMaximized>()
            .add_systems(
                Startup,
                (
                    setup_ui_camera,
                    viewport::spawn_demo_scene,
                    editor_root.spawn(),
                ),
            )
            .add_systems(
                Update,
                (
                    style::update_hover_backgrounds,
                    viewport::attach_viewport_cameras,
                    viewport::draw_grid,
                    (
                        window_frame::sync_window_maximized,
                        (
                            title_bar::update_maximize_glyph,
                            window_frame::toggle_resize_handles,
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
    commands.spawn((Name::new("UI Camera"), Camera2d, IsDefaultUiCamera));
}

fn editor_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
        }
        BackgroundColor(colors::WINDOW_BG)
        BorderColor::all(colors::WINDOW_BORDER)
        TabGroup
        Children [
            @title_bar::title_bar()
            --
            // Editor area: three columns above a status bar.
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
                    column_gap: px(4),
                }
                Children [
                    @scene_tree::left_column()
                    --
                    // Middle column: viewports above the bottom panel.
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                    }
                    Children [
                        @viewport::viewport_grid()
                        --
                        @asset_browser::bottom_panel()
                    ]
                    --
                    @inspector::inspector_panel()
                ]
                --
                @status_bar::status_bar()
            ]
            --
            @window_frame::resize_handles()
        ]
    }
}
