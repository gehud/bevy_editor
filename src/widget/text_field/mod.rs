use bevy::{
    app::{App, Plugin, Update},
    camera::visibility::Visibility,
    ecs::{
        bundle::Bundle,
        change_detection::DetectChanges,
        children,
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::Children,
        lifecycle::Add,
        observer::On,
        query::With,
        system::{BoxedSystem, Commands, Query, Res, ResMut},
    },
    input_focus::{InputFocus, IsFocused, IsFocusedHelper},
    picking::{
        events::{Click, Pointer},
        pointer::PointerButton,
    },
    reflect::PartialReflect,
    text::TextLayout,
    time::{Time, Timer, TimerMode},
    ui::{
        AlignItems, AlignSelf, JustifyContent, Node, Overflow, PositionType, UiRect, percent, px,
        widget::Text,
    },
    ui_widgets::observe,
    utils::default,
};

use crate::{
    theme::{RoundedCorners, ThemedBackgroundColor, tokens::BUTTON_BG},
    widget::{EditorText, ValueChange},
};

fn default_text_field_node() -> Node {
    Node {
        width: percent(100),
        border_radius: RoundedCorners::All.to_border_radius(4.0),
        padding: UiRect::horizontal(px(8)).with_top(px(2)).with_bottom(px(2)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Start,
        overflow: Overflow::hidden(),
        ..default()
    }
}

#[derive(Component)]
#[require(Node = default_text_field_node())]
pub struct TextField {
    pub text: String,
    pub caret_blink_rate: f32,
}

impl Default for TextField {
    fn default() -> Self {
        Self {
            text: default(),
            caret_blink_rate: 0.5,
        }
    }
}

impl TextField {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Component)]
struct Cursor {
    timer: Timer,
    visible: bool,
}

fn on_click(
    mut trigger: On<Pointer<Click>>,
    text_fields: Query<&TextField>,
    mut focus: ResMut<InputFocus>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }

    let target = trigger.event_target();

    let Ok(_) = text_fields.get(target) else {
        return;
    };

    trigger.propagate(false);

    focus.set(target);
}

fn on_focus_changed(
    focus: Res<InputFocus>,
    focus_helper: IsFocusedHelper,
    text_fields: Query<(Entity, &TextField, &TextFieldStructure)>,
    mut commands: Commands,
) -> Result {
    if !focus.is_changed() {
        return Ok(());
    }

    for (entity, text_field, structure) in text_fields {
        if focus_helper.is_focus_within(entity) {
            commands.entity(structure.cursor).insert(Cursor {
                timer: Timer::from_seconds(text_field.caret_blink_rate, TimerMode::Repeating),
                visible: false,
            });
        } else {
            commands.entity(structure.cursor).remove::<Cursor>();
        }
    }

    Ok(())
}

fn update_cursor(time: Res<Time>, mut q_cursors: Query<(&mut Cursor, &mut Visibility)>) {
    for (mut cursor, mut visibility) in q_cursors.iter_mut() {
        if cursor.timer.tick(time.delta()).just_finished() {
            cursor.visible = !cursor.visible;
            if cursor.visible {
                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

#[derive(Component)]
struct TextFieldStructure {
    cursor: Entity,
}

fn build(
    trigger: On<Add, TextField>,
    text_fields: Query<&TextField>,
    mut commands: Commands,
) -> Result {
    let target = trigger.event_target();

    let text_field = text_fields.get(target)?;

    commands
        .entity(target)
        .insert(ThemedBackgroundColor(BUTTON_BG))
        .with_children(|commands| {
            commands.spawn((
                TextLayout::new_with_no_wrap(),
                Text::new(text_field.text.clone()),
                EditorText::body(),
            ));

            let cursor = commands
                .spawn((
                    Visibility::Hidden,
                    Text::new("|"),
                    EditorText::body(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        ..default()
                    },
                ))
                .id();

            let text_field = commands.target_entity();

            commands
                .commands_mut()
                .entity(text_field)
                .insert(TextFieldStructure { cursor });
        });

    Ok(())
}

pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, on_focus_changed)
            .add_systems(Update, update_cursor)
            .add_observer(build)
            .add_observer(on_click);
    }
}
