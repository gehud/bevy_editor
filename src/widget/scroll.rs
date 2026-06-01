use bevy::{
    app::{App, Plugin, Update},
    color::Color,
    ecs::{component::Component, entity::Entity, hierarchy::Children, template::FromTemplate},
    scene::{Scene, SceneComponent, SceneList, bsn, bsn_list},
    ui::{BackgroundColor, BorderRadius, Node, Overflow, PositionType, percent, px},
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarPlugin, ScrollbarThumb},
};

use crate::theme::{ThemedBackgroundColor, tokens::SCROLLBAR_THUMB};

#[derive(Clone, Copy, FromTemplate, SceneComponent)]
#[scene(EditorScrollAreaProps)]
pub struct EditorScrollArea {
    pub horizontal: bool,
    pub vertical: bool,
    pub main_axis: ControlOrientation,
    pub horizontal_line: f32,
    pub vertical_line: f32,
}

impl Default for EditorScrollArea {
    fn default() -> Self {
        Self {
            horizontal: false,
            vertical: false,
            main_axis: ControlOrientation::default(),
            horizontal_line: 20.0,
            vertical_line: 20.0,
        }
    }
}

pub struct EditorScrollAreaProps {
    pub content: Box<dyn SceneList>
}

impl Default for EditorScrollAreaProps {
    fn default() -> Self {
        Self { content: Box::new(bsn_list!()) }
    }
}

impl EditorScrollArea {
    fn scene(props: EditorScrollAreaProps) -> impl Scene {
        bsn! {
            EditorScrollAreaScrollbars {
                horizontal: #Horizontal,
                vertical: #Vertical
            }
            Node
            Children [
                #Area
                Node {
                    flex_grow: 1.0,
                    overflow: Overflow::scroll()
                }
                Children [
                    {props.content}
                ],
                #Horizontal
                Scrollbar {
                    target: #Area,
                    orientation: ControlOrientation::Horizontal
                }
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: px(4),
                    bottom: px(0)
                }
                Children [
                    ScrollbarThumb {
                        border_radius: BorderRadius::all(px(4))
                    }
                    ThemedBackgroundColor(SCROLLBAR_THUMB)
                ],
                #Vertical
                Scrollbar {
                    target: #Area,
                    orientation: ControlOrientation::Vertical
                }
                Node {
                    position_type: PositionType::Absolute,
                    height: percent(100),
                    width: px(4),
                    right: px(0)
                }
                Children [
                    ScrollbarThumb {
                        border_radius: BorderRadius::all(px(4))
                    }
                    ThemedBackgroundColor(SCROLLBAR_THUMB)
                ],
            ]
        }
    }
}

#[derive(Clone, Component, Copy, FromTemplate)]
struct EditorScrollAreaScrollbars {
    horizontal: Entity,
    vertical: Entity,
}

fn update_scroll_areas() {

}

pub struct EditorScrollPlugin;

impl Plugin for EditorScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_scroll_areas);
    }
}
