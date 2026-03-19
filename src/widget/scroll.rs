use std::mem::swap;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    camera::visibility::Visibility,
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        message::MessageReader,
        observer::On,
        query::{Changed, With, Without},
        reflect::ReflectComponent,
        resource::Resource,
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
        events::{Cancel, Click, Drag, DragEnd, DragStart, Pointer, Press},
        hover::HoverMap,
    },
    reflect::{Reflect, prelude::ReflectDefault},
    ui::{
        ComputedNode, ComputedUiRenderTargetInfo, Node, OverflowAxis, ScrollPosition,
        UiGlobalTransform, UiScale, UiSystems, Val, percent,
    },
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(PartialEq, Clone, Default)]
pub enum ControlOrientation {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Scrollbar {
    pub target: Entity,
    pub orientation: ControlOrientation,
    pub min_thumb_length: f32,
}

#[derive(Component, Debug)]
#[require(CoreScrollbarDragState)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct CoreScrollbarThumb;

impl Scrollbar {
    pub fn new(target: Entity, orientation: ControlOrientation, min_thumb_length: f32) -> Self {
        Self {
            target,
            orientation,
            min_thumb_length,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CoreScrollbarDragState {
    pub dragging: bool,
    drag_origin: f32,
}

fn scrollbar_on_pointer_down(
    mut ev: On<Pointer<Press>>,
    q_thumb: Query<&ChildOf, With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(
        &Scrollbar,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        &UiGlobalTransform,
    )>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<Scrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if q_thumb.contains(ev.entity) {
        ev.propagate(false);
    } else if let Ok((scrollbar, node, node_target, transform)) = q_scrollbar.get_mut(ev.entity) {
        ev.propagate(false);

        let local_pos = transform.try_inverse().unwrap().transform_point2(
            ev.event().pointer_location.position * node_target.scale_factor() / ui_scale.0,
        ) + node.size() * 0.5;

        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        let visible_size = (scroll_content.size() - scroll_content.scrollbar_size)
            * scroll_content.inverse_scale_factor;
        let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;
        let max_range = (content_size - visible_size).max(Vec2::ZERO);

        fn adjust_scroll_pos(scroll_pos: &mut f32, click_pos: f32, step: f32, range: f32) {
            *scroll_pos =
                (*scroll_pos + if click_pos > *scroll_pos { step } else { -step }).clamp(0., range);
        }

        match scrollbar.orientation {
            ControlOrientation::Horizontal => {
                if node.size().x > 0. {
                    let click_pos = local_pos.x * content_size.x / node.size().x;
                    adjust_scroll_pos(&mut scroll_pos.x, click_pos, visible_size.x, max_range.x);
                }
            }
            ControlOrientation::Vertical => {
                if node.size().y > 0. {
                    let click_pos = local_pos.y * content_size.y / node.size().y;
                    adjust_scroll_pos(&mut scroll_pos.y, click_pos, visible_size.y, max_range.y);
                }
            }
        }
    }
}

fn scrollbar_on_drag_start(
    mut ev: On<Pointer<DragStart>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    q_scrollbar: Query<&Scrollbar>,
    q_scroll_area: Query<&ScrollPosition>,
) {
    if let Ok((ChildOf(thumb_parent), mut drag)) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if let Ok(scrollbar) = q_scrollbar.get(*thumb_parent)
            && let Ok(scroll_area) = q_scroll_area.get(scrollbar.target)
        {
            drag.dragging = true;
            drag.drag_origin = match scrollbar.orientation {
                ControlOrientation::Horizontal => scroll_area.x,
                ControlOrientation::Vertical => scroll_area.y,
            };
        }
    }
}

fn scrollbar_on_drag(
    mut ev: On<Pointer<Drag>>,
    mut q_thumb: Query<(&ChildOf, &mut CoreScrollbarDragState), With<CoreScrollbarThumb>>,
    mut q_scrollbar: Query<(&ComputedNode, &Scrollbar)>,
    mut q_scroll_pos: Query<(&mut ScrollPosition, &ComputedNode), Without<Scrollbar>>,
    ui_scale: Res<UiScale>,
) {
    if let Ok((ChildOf(thumb_parent), drag)) = q_thumb.get_mut(ev.entity)
        && let Ok((node, scrollbar)) = q_scrollbar.get_mut(*thumb_parent)
    {
        ev.propagate(false);
        let Ok((mut scroll_pos, scroll_content)) = q_scroll_pos.get_mut(scrollbar.target) else {
            return;
        };

        if drag.dragging {
            let distance = ev.event().distance / ui_scale.0;

            let visible_size = (scroll_content.size() - scroll_content.scrollbar_size)
                * scroll_content.inverse_scale_factor;
            let content_size = scroll_content.content_size() * scroll_content.inverse_scale_factor;

            let scrollbar_size = (node.size() * node.inverse_scale_factor).max(Vec2::ONE);

            match scrollbar.orientation {
                ControlOrientation::Horizontal => {
                    let range = (content_size.x - visible_size.x).max(0.);
                    scroll_pos.x = (drag.drag_origin
                        + (distance.x * content_size.x) / scrollbar_size.x)
                        .clamp(0., range);
                }
                ControlOrientation::Vertical => {
                    let range = (content_size.y - visible_size.y).max(0.);
                    scroll_pos.y = (drag.drag_origin
                        + (distance.y * content_size.y) / scrollbar_size.y)
                        .clamp(0., range);
                }
            };
        }
    }
}

fn scrollbar_on_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn scrollbar_on_drag_cancel(
    mut ev: On<Pointer<Cancel>>,
    mut q_thumb: Query<&mut CoreScrollbarDragState, With<CoreScrollbarThumb>>,
) {
    if let Ok(mut drag) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if drag.dragging {
            drag.dragging = false;
        }
    }
}

fn update_scrollbar_thumb(
    q_scroll_area: Query<(&ScrollPosition, &ComputedNode)>,
    q_scrollbar: Query<(&Scrollbar, &ComputedNode, &Children)>,
    mut q_thumb: Query<&mut Node, With<CoreScrollbarThumb>>,
) {
    for (scrollbar, scrollbar_node, children) in q_scrollbar.iter() {
        let Ok(scroll_area) = q_scroll_area.get(scrollbar.target) else {
            continue;
        };

        let visible_size = (scroll_area.1.size() - scroll_area.1.scrollbar_size)
            * scroll_area.1.inverse_scale_factor;

        let content_size = scroll_area.1.content_size() * scroll_area.1.inverse_scale_factor;

        let track_length = scrollbar_node.size() * scrollbar_node.inverse_scale_factor;

        fn size_and_pos(
            content_size: f32,
            visible_size: f32,
            track_length: f32,
            min_size: f32,
            mut offset: f32,
        ) -> (f32, f32) {
            let thumb_size = if content_size > visible_size {
                (track_length * visible_size / content_size)
                    .max(min_size)
                    .min(track_length)
            } else {
                track_length
            };

            if content_size > visible_size {
                let max_offset = content_size - visible_size;

                offset = offset.clamp(0.0, max_offset);
            } else {
                offset = 0.0;
            }

            let thumb_pos = if content_size > visible_size {
                offset * (track_length - thumb_size) / (content_size - visible_size)
            } else {
                0.
            };

            (thumb_size, thumb_pos)
        }

        for child in children {
            if let Ok(mut thumb) = q_thumb.get_mut(*child) {
                match scrollbar.orientation {
                    ControlOrientation::Horizontal => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.x,
                            visible_size.x,
                            track_length.x,
                            scrollbar.min_thumb_length,
                            scroll_area.0.x,
                        );

                        thumb.top = Val::Px(0.);
                        thumb.bottom = Val::Px(0.);
                        thumb.left = Val::Px(thumb_pos);
                        thumb.width = Val::Px(thumb_size);
                    }
                    ControlOrientation::Vertical => {
                        let (thumb_size, thumb_pos) = size_and_pos(
                            content_size.y,
                            visible_size.y,
                            track_length.y,
                            scrollbar.min_thumb_length,
                            scroll_area.0.y,
                        );

                        thumb.left = Val::Px(0.);
                        thumb.right = Val::Px(0.);
                        thumb.top = Val::Px(thumb_pos);
                        thumb.height = Val::Px(thumb_size);
                    }
                };
            }
        }
    }
}

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollbar_on_pointer_down)
            .add_observer(scrollbar_on_drag_start)
            .add_observer(scrollbar_on_drag_end)
            .add_observer(scrollbar_on_drag_cancel)
            .add_observer(scrollbar_on_drag)
            .add_systems(PostUpdate, update_scrollbar_thumb);
    }
}
