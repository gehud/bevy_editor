use std::mem;

use bevy::{
    app::{App, Plugin, PostUpdate, PreUpdate},
    camera::visibility::Visibility,
    ecs::{
        component::Component,
        entity::Entity,
        entity_disabling::Disabled,
        error::Result,
        event::EntityEvent,
        hierarchy::Children,
        message::MessageReader,
        observer::On,
        query::Changed,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
        template::FromTemplate,
    },
    input::{
        ButtonInput,
        keyboard::KeyCode,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    math::Vec2,
    picking::{PickingSystems, hover::HoverMap},
    scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on},
    ui::{
        BorderRadius, ComputedNode, Node, Overflow, PositionType, ScrollPosition, UiSystems,
        percent, px,
    },
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
    utils::default,
};
use bitflags::bitflags;

use crate::theme::{ThemedBackgroundColor, tokens::SCROLLBAR_THUMB};

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum ScrollbarVisibility {
    #[default]
    Auto,
    Visible,
    Hidden,
}

bitflags! {
    #[derive(Clone, Copy, Eq, Hash, PartialEq)]
    pub struct ScrollAxes: u8 {
        const HORIZONTAL = 1;
        const VERTICAL = 2;
    }
}

impl Default for ScrollAxes {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Clone, Copy, SceneComponent)]
#[scene(EditorScrollAreaProps)]
pub struct EditorScrollArea {
    pub axes: ScrollAxes,
    pub horizontal_scrollbar_visibility: ScrollbarVisibility,
    pub vertical_scrollbar_visibility: ScrollbarVisibility,
    pub orientation: ControlOrientation,
    pub horizontal_line: f32,
    pub vertical_line: f32,
}

impl Default for EditorScrollArea {
    fn default() -> Self {
        Self {
            axes: default(),
            horizontal_scrollbar_visibility: default(),
            vertical_scrollbar_visibility: default(),
            orientation: default(),
            horizontal_line: 20.0,
            vertical_line: 20.0,
        }
    }
}

pub struct EditorScrollAreaProps {
    pub content: Box<dyn SceneList>,
}

impl Default for EditorScrollAreaProps {
    fn default() -> Self {
        Self {
            content: Box::new(bsn_list!()),
        }
    }
}

impl EditorScrollArea {
    fn scene(props: EditorScrollAreaProps) -> impl Scene {
        bsn! {
            Node
            EditorScrollAreaStructure {
                area: #Area,
                horizontal: #Horizontal,
                vertical: #Vertical
            }
            Children [
                #Area
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    overflow: Overflow::scroll()
                }
                Children [
                    {props.content}
                ],
                #Horizontal
                Disabled
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
                Disabled
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
            on(on_scroll)
        }
    }
}

#[derive(Clone, Component, Copy, FromTemplate)]
struct EditorScrollAreaStructure {
    area: Entity,
    horizontal: Entity,
    vertical: Entity,
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    unit: MouseScrollUnit,
    delta: Vec2,
}

fn send_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            mem::swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll {
                    entity,
                    unit: mouse_wheel.unit,
                    delta,
                });
            }
        }
    }
}

fn on_scroll(
    mut trigger: On<Scroll>,
    scroll_areas: Query<(&EditorScrollArea, &EditorScrollAreaStructure)>,
    mut nodes: Query<(&mut ScrollPosition, &ComputedNode)>,
) -> Result {
    let (scroll_area, structure) = scroll_areas.get(trigger.event_target())?;

    trigger.propagate(false);

    let mut delta = trigger.delta;

    if trigger.unit == MouseScrollUnit::Line {
        delta.x *= scroll_area.horizontal_line;
        delta.y *= scroll_area.vertical_line;
    }

    if scroll_area.orientation != ControlOrientation::default() {
        mem::swap(&mut delta.x, &mut delta.y);
    }

    let (mut scroll_position, computed) = nodes.get_mut(structure.area)?;

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    if scroll_area.axes.contains(ScrollAxes::HORIZONTAL) && delta.x != 0.0 {
        scroll_position.x = (scroll_position.x + delta.x).clamp(0.0, max_offset.x.max(0.0));
    }

    if scroll_area.axes.contains(ScrollAxes::VERTICAL) && delta.y != 0.0 {
        scroll_position.y = (scroll_position.y + delta.y).clamp(0.0, max_offset.y.max(0.0));
    }

    Ok(())
}

fn update_scroll_areas(
    scroll_areas: Query<(&EditorScrollArea, &EditorScrollAreaStructure), Changed<EditorScrollArea>>,
    mut commands: Commands,
) {
    for (scroll_area, structure) in scroll_areas.iter() {
        if scroll_area.axes.contains(ScrollAxes::HORIZONTAL) {
            commands.entity(structure.horizontal).remove::<Disabled>();
        } else {
            commands.entity(structure.horizontal).insert(Disabled);
        }

        if scroll_area.axes.contains(ScrollAxes::VERTICAL) {
            commands.entity(structure.vertical).remove::<Disabled>();
        } else {
            commands.entity(structure.vertical).insert(Disabled);
        }
    }
}

fn update_scrollbars_visibility(
    mut scroll_areas: Query<(&EditorScrollArea, &EditorScrollAreaStructure)>,
    mut visibilities: Query<&mut Visibility>,
    nodes: Query<&ComputedNode>,
) -> Result {
    for (scroll_area, structure) in scroll_areas.iter_mut() {
        let area = nodes.get(structure.area)?;

        if let Ok(mut horizontal_visibility) = visibilities.get_mut(structure.horizontal) {
            let should_hide_horizontal = area.content_size().x <= area.size().x;

            match scroll_area.horizontal_scrollbar_visibility {
                ScrollbarVisibility::Auto => {
                    if should_hide_horizontal && *horizontal_visibility != Visibility::Hidden {
                        *horizontal_visibility = Visibility::Hidden;
                    } else if !should_hide_horizontal
                        && *horizontal_visibility != Visibility::Inherited
                    {
                        *horizontal_visibility = Visibility::Inherited;
                    }
                }
                ScrollbarVisibility::Visible => {
                    if *horizontal_visibility != Visibility::Inherited {
                        *horizontal_visibility = Visibility::Inherited;
                    }
                }
                ScrollbarVisibility::Hidden => {
                    if *horizontal_visibility != Visibility::Hidden {
                        *horizontal_visibility = Visibility::Hidden;
                    }
                }
            }
        }

        if let Ok(mut vertical_visibility) = visibilities.get_mut(structure.vertical) {
            let should_hide_vertical = area.content_size().y <= area.size().y;

            match scroll_area.vertical_scrollbar_visibility {
                ScrollbarVisibility::Auto => {
                    if should_hide_vertical && *vertical_visibility != Visibility::Hidden {
                        *vertical_visibility = Visibility::Hidden;
                    } else if !should_hide_vertical && *vertical_visibility != Visibility::Inherited
                    {
                        *vertical_visibility = Visibility::Inherited;
                    }
                }
                ScrollbarVisibility::Visible => {
                    if *vertical_visibility != Visibility::Inherited {
                        *vertical_visibility = Visibility::Inherited;
                    }
                }
                ScrollbarVisibility::Hidden => {
                    if *vertical_visibility != Visibility::Hidden {
                        *vertical_visibility = Visibility::Hidden;
                    }
                }
            }
        }
    }

    Ok(())
}

pub struct EditorScrollPlugin;

impl Plugin for EditorScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, send_scroll_events.in_set(PickingSystems::Last))
            .add_systems(
                PostUpdate,
                (
                    update_scroll_areas.before(UiSystems::Layout),
                    update_scrollbars_visibility.after(UiSystems::Layout),
                ),
            );
    }
}
