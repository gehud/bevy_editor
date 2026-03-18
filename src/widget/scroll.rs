use std::mem::swap;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    camera::visibility::Visibility,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        message::MessageReader,
        observer::On,
        query::Changed,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
    },
    input::{
        ButtonInput,
        keyboard::KeyCode,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    math::Vec2,
    picking::{
        events::{Click, Pointer},
        hover::HoverMap,
    },
    ui::{ComputedNode, Node, OverflowAxis, ScrollPosition, UiSystems, percent},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Component)]
pub struct ScrollRect {
    pub content: Option<Entity>,
    pub horizontal: bool,
    pub vertical: bool,
    pub horizontal_scrollbar: Option<Entity>,
    pub auto_hide_horizontal: bool,
    pub vertical_scrollbar: Option<Entity>,
    pub auto_hide_vertical: bool,
    pub step: f32,
    pub main_axis: ScrollAxis,
}

impl Default for ScrollRect {
    fn default() -> Self {
        Self {
            content: Default::default(),
            horizontal: Default::default(),
            vertical: Default::default(),
            horizontal_scrollbar: Default::default(),
            auto_hide_horizontal: true,
            vertical_scrollbar: Default::default(),
            auto_hide_vertical: true,
            step: 20.0,
            main_axis: Default::default(),
        }
    }
}

#[derive(Default, Component)]
pub struct Scrollbar {
    pub handle: Option<Entity>,
}

#[derive(EntityEvent, Debug)]
struct ScrollDelta {
    entity: Entity,
    /// Scroll delta in logical coordinates.
    delta: Vec2,
}

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_scrollrect, send_scroll_delta))
            .add_systems(PostUpdate, update_scrollbars.after(UiSystems::Layout))
            .add_observer(on_scroll_delta)
            .add_observer(on_scrollbar_click);
    }
}

fn update_scrollrect(
    scrollrects: Query<&ScrollRect, Changed<ScrollRect>>,
    mut nodes: Query<&mut Node>,
) {
    for scrollrect in scrollrects {
        let overflow_x = scrollrect.horizontal.then_some(OverflowAxis::Scroll);
        let overflow_y = scrollrect.vertical.then_some(OverflowAxis::Scroll);

        if let Some(content) = scrollrect.content {
            if let Ok(mut content_node) = nodes.get_mut(content) {
                if let Some(overflow_x) = overflow_x {
                    if overflow_x != content_node.overflow.x {
                        content_node.overflow.x = overflow_x;
                    }
                }

                if let Some(overflow_y) = overflow_y {
                    if overflow_y != content_node.overflow.y {
                        content_node.overflow.y = overflow_y;
                    }
                }
            }
        }
    }
}

fn update_scrollbars(
    scrollrects: Query<&ScrollRect>,
    scroll_positions: Query<&ScrollPosition>,
    computed_nodes: Query<&ComputedNode>,
    scrollbars: Query<&Scrollbar>,
    mut visibilities: Query<&mut Visibility>,
    mut nodes: Query<&mut Node>,
) -> Result {
    for scrollrect in scrollrects {
        let Some(content) = scrollrect.content else {
            continue;
        };

        let content_node = computed_nodes.get(content)?;
        let scroll_position = scroll_positions.get(content)?;

        if let Some(horizontal_scrollbar) = scrollrect.horizontal_scrollbar {
            if let Ok(scrollbar) = scrollbars.get(horizontal_scrollbar) {
                if let Some(handle) = scrollbar.handle {
                    let mut handle_node = nodes.get_mut(handle)?;

                    let length = (content_node.size().x / content_node.content_size().x).min(1.0);

                    if scrollrect.auto_hide_horizontal && length == 1.0 {
                        *visibilities.get_mut(horizontal_scrollbar)? = Visibility::Hidden;
                    } else {
                        *visibilities.get_mut(horizontal_scrollbar)? = Visibility::Inherited;

                        handle_node.width = percent(length * 100.0);

                        let offset = (scroll_position.x
                            / (content_node.content_size().x
                                * content_node.inverse_scale_factor()))
                        .min(1.0);

                        handle_node.left = percent(offset * 100.0);
                    }
                }
            }
        }

        if let Some(vertical_scrollbar) = scrollrect.vertical_scrollbar {
            if let Ok(scrollbar) = scrollbars.get(vertical_scrollbar) {
                if let Some(handle) = scrollbar.handle {
                    let mut handle_node = nodes.get_mut(handle)?;

                    let length = (content_node.size().y / content_node.content_size().y).min(1.0);

                    if scrollrect.auto_hide_vertical && length == 1.0 {
                        *visibilities.get_mut(vertical_scrollbar)? = Visibility::Hidden;
                    } else {
                        *visibilities.get_mut(vertical_scrollbar)? = Visibility::Inherited;

                        handle_node.height = percent(length * 100.0);

                        let offset = (scroll_position.y
                            / (content_node.content_size().y
                                * content_node.inverse_scale_factor()))
                        .min(1.0);

                        handle_node.top = percent(offset * 100.0);
                    }
                }
            }
        }
    }

    Ok(())
}

fn send_scroll_delta(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    scrollrects: Query<&ScrollRect>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                let Ok(scrollrect) = scrollrects.get(entity) else {
                    continue;
                };

                let mut delta = delta
                    * if mouse_wheel.unit == MouseScrollUnit::Line {
                        scrollrect.step
                    } else {
                        1.0
                    };

                if scrollrect.main_axis == ScrollAxis::Horizontal {
                    swap(&mut delta.x, &mut delta.y);
                }

                commands.trigger(ScrollDelta { entity, delta });
            }
        }
    }
}

fn on_scroll_delta(
    mut trigger: On<ScrollDelta>,
    scrollrects: Query<&ScrollRect>,
    mut contents: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) -> Result {
    let scrollrect = scrollrects.get(trigger.entity)?;

    let Some(content) = scrollrect.content else {
        return Ok(());
    };

    let (mut scroll_position, node, computed) = contents.get_mut(content)?;

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    let delta = &mut trigger.delta;

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        scroll_position.x = (scroll_position.x + delta.x).clamp(0.0, max_offset.x.max(0.0));
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        scroll_position.y = (scroll_position.y + delta.y).clamp(0.0, max_offset.y.max(0.0));
    }

    Ok(())
}

fn on_scrollbar_click(trigger: On<Pointer<Click>>) {}
