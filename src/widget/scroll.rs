use bevy::{
    app::{App, Plugin, Update},
    color::Color,
    ecs::{component::Component, entity::Entity, hierarchy::Children, template::FromTemplate},
    scene::{Scene, SceneComponent, bsn},
    ui::{BackgroundColor, BorderRadius, Node, PositionType, percent, px},
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarPlugin, ScrollbarThumb},
};

#[derive(Clone, Copy, FromTemplate, SceneComponent)]
pub struct EditorScrollArea {
    pub target: Entity,
    pub horizontal: bool,
    pub vertical: bool,
    pub main_axis: ControlOrientation,
    pub horizontal_line: f32,
    pub vertical_line: f32,
}

impl Default for EditorScrollArea {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            horizontal: false,
            vertical: false,
            main_axis: ControlOrientation::default(),
            horizontal_line: 20.0,
            vertical_line: 20.0,
        }
    }
}

impl EditorScrollArea {
    fn scene() -> impl Scene {
        bsn! {
            EditorScrollAreaScrollbars {
                horizontal: #Horizontal,
                vertical: #Vertical
            }
            Children [
                #Horizontal
                Scrollbar
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
                ],
                #Vertical
                Scrollbar
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
                ]
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
