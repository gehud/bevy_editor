use std::mem::swap;

use bevy::{
    app::{App, Plugin, PostUpdate, PreUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        message::MessageReader,
        observer::On,
        query::{Or, With, Without},
        reflect::ReflectComponent,
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
        PickingSystems,
        events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press},
        hover::HoverMap,
    },
    reflect::{Reflect, prelude::ReflectDefault},
    ui::{
        ComputedNode, ComputedUiRenderTargetInfo, Node, OverflowAxis, ScrollPosition,
        UiGlobalTransform, UiScale, UiSystems, Val,
    },
};

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ScrollArea {
    pub target: Entity,
    pub horizontal: bool,
    pub vertical: bool,
    pub main_axis: ScrollAxis,
    pub horizontal_line: f32,
    pub vertical_line: f32,
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            horizontal: true,
            vertical: true,
            main_axis: Default::default(),
            horizontal_line: 20.0,
            vertical_line: 20.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(PartialEq, Clone, Default)]
pub enum ScrollAxis {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct Scrollbar {
    pub target: Entity,
    pub axis: ScrollAxis,
    pub min_thumb_length: f32,
}

impl Default for Scrollbar {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            axis: ScrollAxis::Vertical,
            min_thumb_length: 0.0,
        }
    }
}

#[derive(Component, Debug)]
#[require(ScrollbarDragState)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct ScrollbarThumb;

#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct ScrollbarDragState {
    pub dragging: bool,
    drag_origin: f32,
}

fn scrollbar_on_press(
    mut ev: On<Pointer<Press>>,
    q_thumb: Query<&ChildOf, With<ScrollbarThumb>>,
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

        match scrollbar.axis {
            ScrollAxis::Horizontal => {
                if node.size().x > 0. {
                    let click_pos = local_pos.x * content_size.x / node.size().x;
                    adjust_scroll_pos(&mut scroll_pos.x, click_pos, visible_size.x, max_range.x);
                }
            }
            ScrollAxis::Vertical => {
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
    mut q_thumb: Query<(&ChildOf, &mut ScrollbarDragState), With<ScrollbarThumb>>,
    q_scrollbar: Query<&Scrollbar>,
    q_scroll_area: Query<&ScrollPosition>,
) {
    if let Ok((ChildOf(thumb_parent), mut drag)) = q_thumb.get_mut(ev.entity) {
        ev.propagate(false);
        if let Ok(scrollbar) = q_scrollbar.get(*thumb_parent)
            && let Ok(scroll_area) = q_scroll_area.get(scrollbar.target)
        {
            drag.dragging = true;
            drag.drag_origin = match scrollbar.axis {
                ScrollAxis::Horizontal => scroll_area.x,
                ScrollAxis::Vertical => scroll_area.y,
            };
        }
    }
}

fn scrollbar_on_drag(
    mut ev: On<Pointer<Drag>>,
    mut q_thumb: Query<(&ChildOf, &mut ScrollbarDragState), With<ScrollbarThumb>>,
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

            match scrollbar.axis {
                ScrollAxis::Horizontal => {
                    let range = (content_size.x - visible_size.x).max(0.);
                    scroll_pos.x = (drag.drag_origin
                        + (distance.x * content_size.x) / scrollbar_size.x)
                        .clamp(0., range);
                }
                ScrollAxis::Vertical => {
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
    mut q_thumb: Query<&mut ScrollbarDragState, With<ScrollbarThumb>>,
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
    mut q_thumb: Query<&mut ScrollbarDragState, With<ScrollbarThumb>>,
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
    mut q_thumb: Query<&mut Node, With<ScrollbarThumb>>,
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
            min_length: f32,
            mut offset: f32,
        ) -> (f32, f32) {
            let thumb_size = if content_size > visible_size {
                (track_length * visible_size / content_size)
                    .max(min_length)
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
                match scrollbar.axis {
                    ScrollAxis::Horizontal => {
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
                    ScrollAxis::Vertical => {
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

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ScrollDelta {
    entity: Entity,
    delta: Vec2,
    unit: MouseScrollUnit,
}

fn send_mouse_scroll_delta(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) -> Result {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(ScrollDelta {
                    entity,
                    delta,
                    unit: mouse_wheel.unit,
                });
            }
        }
    }

    Ok(())
}

fn scrollarea_on_scroll_delta(
    mut trigger: On<ScrollDelta>,
    scrollareas: Query<&ScrollArea>,
    mut contents: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) -> Result {
    let Ok(scrollarea) = scrollareas.get(trigger.entity) else {
        return Ok(());
    };

    trigger.propagate(false);

    let mut delta = trigger.delta;

    if trigger.unit == MouseScrollUnit::Line {
        delta.x *= scrollarea.horizontal_line;
        delta.y *= scrollarea.vertical_line;
    }

    if scrollarea.main_axis != ScrollAxis::default() {
        swap(&mut delta.x, &mut delta.y);
    }

    let (mut scroll_position, node, computed) = contents.get_mut(scrollarea.target)?;

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        scroll_position.x = (scroll_position.x + delta.x).clamp(0.0, max_offset.x.max(0.0));
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        scroll_position.y = (scroll_position.y + delta.y).clamp(0.0, max_offset.y.max(0.0));
    }

    Ok(())
}

pub struct ScrollPlugin;

impl Plugin for ScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scrollbar_on_press)
            .add_observer(scrollbar_on_drag_start)
            .add_observer(scrollbar_on_drag_end)
            .add_observer(scrollbar_on_drag_cancel)
            .add_observer(scrollbar_on_drag)
            .add_observer(scrollarea_on_scroll_delta)
            .add_systems(
                PreUpdate,
                send_mouse_scroll_delta.in_set(PickingSystems::Last),
            )
            .add_systems(PostUpdate, update_scrollbar_thumb.before(UiSystems::Layout));
    }
}
