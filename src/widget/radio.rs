use bevy::app::{Plugin, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::{Spawn, SpawnRelated, SpawnableList},
    system::{Commands, Query},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::{PickingSystems, hover::Hovered};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::ui::{
    AlignItems, BorderRadius, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, UiRect, Val,
};
use bevy::ui_widgets::RadioButton;

use crate::theme::ThemedTextSize;
use crate::theme::{
    ThemedBackgroundColor, ThemedBorderColor, ThemedTextColor, ThemedTextFont, constants::size, tokens,
};
use crate::widget::EntityCursor;

/// Marker for the radio outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioOutline;

/// Marker for the radio check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioMark;

/// Template function to spawn a radio.
///
/// # Arguments
/// * `props` - construction properties for the radio.
/// * `overrides` - a bundle of components that are merged in with the normal radio components.
/// * `label` - the label of the radio.
///
/// # Emitted events
/// * [`bevy::ui_widgets::ValueChange<bool>`] with the value true when it becomes checked.
/// * [`bevy::ui_widgets::ValueChange<Entity>`] with the selected entity's id when a new radio button is selected.
///
///  These events can be disabled by adding an [`bevy::ui::InteractionDisabled`] component to the entity
pub fn radio<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
    overrides: B,
    label: C,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..Default::default()
        },
        RadioButton,
        Hovered::default(),
        EntityCursor::System(bevy::window::SystemCursorIcon::Pointer),
        TabIndex(0),
        ThemedTextColor::new(tokens::RADIO_TEXT),
        ThemedTextFont::new(tokens::RADIO_TEXT),
        ThemedTextSize::new(tokens::RADIO_TEXT),
        overrides,
        Children::spawn((
            Spawn((
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: size::RADIO_SIZE,
                    height: size::RADIO_SIZE,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..Default::default()
                },
                RadioOutline,
                ThemedBorderColor::all(tokens::RADIO_BORDER),
                children![(
                    // Cheesy checkmark: rotated node with L-shaped border.
                    Node {
                        width: Val::Px(8.),
                        height: Val::Px(8.),
                        border_radius: BorderRadius::MAX,
                        ..Default::default()
                    },
                    RadioMark,
                    ThemedBackgroundColor::new(tokens::RADIO_MARK),
                )],
            )),
            label,
        )),
    )
}

fn update_radio_styles(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &ThemedTextColor,
        ),
        (
            With<RadioButton>,
            Or<(Changed<Hovered>, Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<&ThemedBorderColor, With<RadioOutline>>,
    mut q_mark: Query<&ThemedBackgroundColor, With<RadioMark>>,
    mut commands: Commands,
) {
    for (radio_ent, disabled, checked, hovered, font_color) in q_radioes.iter() {
        let Some(outline_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_outline.contains(*en))
        else {
            continue;
        };
        let Some(mark_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_mark.contains(*en))
        else {
            continue;
        };
        let outline_border = q_outline.get_mut(outline_ent).unwrap();
        let mark_color = q_mark.get_mut(mark_ent).unwrap();
        set_radio_styles(
            radio_ent,
            outline_ent,
            mark_ent,
            disabled,
            checked,
            hovered.0,
            outline_border,
            mark_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_radio_styles_remove(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &ThemedTextColor,
        ),
        With<RadioButton>,
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<&ThemedBorderColor, With<RadioOutline>>,
    mut q_mark: Query<&ThemedBackgroundColor, With<RadioMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((radio_ent, disabled, checked, hovered, font_color)) = q_radioes.get(ent) {
                let Some(outline_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_outline.contains(*en))
                else {
                    return;
                };
                let Some(mark_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_mark.contains(*en))
                else {
                    return;
                };
                let outline_border = q_outline.get_mut(outline_ent).unwrap();
                let mark_color = q_mark.get_mut(mark_ent).unwrap();
                set_radio_styles(
                    radio_ent,
                    outline_ent,
                    mark_ent,
                    disabled,
                    checked,
                    hovered.0,
                    outline_border,
                    mark_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_radio_styles(
    radio_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    hovered: bool,
    outline_border: &ThemedBorderColor,
    mark_color: &ThemedBackgroundColor,
    font_color: &ThemedTextColor,
    commands: &mut Commands,
) {
    let outline_border_token = ThemedBorderColor::all(match (disabled, hovered) {
        (true, _) => tokens::RADIO_BORDER_DISABLED,
        (false, true) => tokens::RADIO_BORDER_HOVER,
        _ => tokens::RADIO_BORDER,
    });

    let mark_token = match disabled {
        true => tokens::RADIO_MARK_DISABLED,
        false => tokens::RADIO_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::RADIO_TEXT_DISABLED,
        false => tokens::RADIO_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    // Change outline border
    if outline_border != &outline_border_token {
        commands.entity(outline_ent).insert(outline_border_token);
    }

    // Change mark color
    if mark_color.0 != mark_token {
        commands
            .entity(mark_ent)
            .insert(ThemedBackgroundColor::new(mark_token));
    }

    // Change mark visibility
    commands.entity(mark_ent).insert(match checked {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(radio_ent)
            .insert(ThemedTextColor::new(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(radio_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the radio styles.
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_radio_styles, update_radio_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
