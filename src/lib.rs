pub mod pane;
pub mod theme;
pub mod widget;
pub mod window;

use std::fmt::Debug;

use bevy::{
    DefaultPlugins,
    app::{App, Plugin, PluginGroup},
    camera::NormalizedRenderTarget,
    color::Color,
    ecs::{
        entity::ContainsEntity,
        error::Result,
        observer::On,
        system::{Commands, Query, ResMut, SystemParam},
    },
    input_focus::{InputFocus, tab_navigation::TabIndex},
    picking::{events::Pointer, hover::Hovered},
    reflect::Reflect,
    render::RenderPlugin,
    text::TextColor,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, OverrideClip,
        PositionType, UiRect, UiScale, percent, px,
        widget::{Text, TextShadow},
    },
    ui_widgets::{
        MenuItem, MenuLayout, MenuPopup,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    },
    utils::default,
    window::{Window, WindowPlugin},
};

use crate::{
    pane::{
        EditorPanePlugin, PaneLayoutRoot,
        panes::{SceneTreePanePlugin, Viewport3dPanePlugin},
    },
    theme::{
        EditorThemePlugin, Theme, ThemeBackgroundColor, ThemeBorderColor,
        constants::fonts::REGULAR,
        tokens::{BORDER, WINDOW_BG},
    },
    widget::EditorWidgetPlugins,
    window::{EditorWindow, EditorWindowPlugin, IsWindowMaximized, PrimaryWindowConfigured},
};

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                transparent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EditorThemePlugin)
        .add_plugins(EditorWindowPlugin)
        .add_plugins(EditorWidgetPlugins)
        .add_plugins(EditorPanePlugin)
        .add_plugins(SceneTreePanePlugin)
        .add_plugins(Viewport3dPanePlugin)
        .add_observer(setup);
    }
}

#[derive(SystemParam)]
pub struct EditorWindowTargetHelper<'w, 's> {
    editor_windows: Query<'w, 's, &'static EditorWindow>,
}

impl EditorWindowTargetHelper<'_, '_> {
    pub fn is_editor_window_target(&self, target: &NormalizedRenderTarget) -> bool {
        let NormalizedRenderTarget::Window(window_ref) = target else {
            return false;
        };

        self.editor_windows.contains(window_ref.entity())
    }

    pub fn is_editor_window_pointer_event<E: Debug + Clone + Reflect>(
        &self,
        event: &Pointer<E>,
    ) -> bool {
        self.is_editor_window_target(&event.pointer_location.target)
    }
}

fn setup(
    trigger: On<PrimaryWindowConfigured>,
    editor_windows: Query<&EditorWindow>,
    mut windows: Query<&mut IsWindowMaximized>,
    mut commands: Commands,
    mut ui_scale: ResMut<UiScale>,
) -> Result {
    // ui_scale.0 = 1.5;
    windows.get_mut(trigger.entity)?.0 = true;

    let editor_window = editor_windows.get(trigger.entity)?;

    commands
        .entity(editor_window.content())
        .with_children(|commands| {
            commands
                .spawn(Node {
                    width: percent(100),
                    height: percent(100),
                    padding: UiRect::horizontal(px(4)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|commands| {
                    // Panes
                    commands.spawn((
                        PaneLayoutRoot,
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                    ));

                    // Footer
                    commands.spawn(Node {
                        width: percent(100),
                        height: px(24),
                        padding: UiRect::horizontal(px(8)),
                        ..default()
                    });
                });
        });

    Ok(())
}
