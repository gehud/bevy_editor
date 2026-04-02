use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::{SpawnRelated, SpawnableList},
    system::{Commands, Query},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::{PickingSystems, hover::Hovered};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::ui::{AlignItems, InteractionDisabled, JustifyContent, Node, Pressed, UiRect, Val};
use bevy::ui_widgets::Button;

use crate::theme::ThemedTextSize;
use crate::{
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedTextColor, ThemedTextFont, constants::size,
        tokens,
    },
    widget::EntityCursor,
};

/// Color variants for buttons. This also functions as a component used by the dynamic styling
/// system to identify which entities are buttons.
#[derive(Component, Default, Clone, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ButtonVariant {
    /// The standard button appearance
    #[default]
    Normal,
    /// A button with a more prominent color, this is used for "call to action" buttons,
    /// default buttons for dialog boxes, and so on.
    Primary,
}

/// Parameters for the button template, passed to [`button`] function.
#[derive(Default)]
pub struct ButtonProps {
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Rounded corners options
    pub corners: RoundedCorners,
}

/// Template function to spawn a button.
///
/// # Arguments
/// * `props` - construction properties for the button.
/// * `overrides` - a bundle of components that are merged in with the normal button components.
/// * `children` - a [`SpawnableList`] of child elements, such as a label or icon for the button.
///
/// # Emitted events
/// * [`bevy::ui_widgets::Activate`] when any of the following happens:
///     * the pointer is released while hovering over the button.
///     * the ENTER or SPACE key is pressed while the button has keyboard focus.
///
///  These events can be disabled by adding an [`bevy::ui::InteractionDisabled`] component to the entity
pub fn button<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
    props: ButtonProps,
    overrides: B,
    children: C,
) -> impl Bundle {
    (
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            flex_grow: 1.0,
            border_radius: props.corners.to_border_radius(4.0),
            ..Default::default()
        },
        Button,
        props.variant,
        Hovered::default(),
        EntityCursor::System(bevy::window::SystemCursorIcon::Pointer),
        TabIndex(0),
        ThemedBackgroundColor::new(tokens::BUTTON_BG),
        ThemedTextColor::new(tokens::BUTTON_TEXT),
        ThemedTextFont::new(tokens::BUTTON_TEXT),
        ThemedTextSize::new(tokens::BUTTON_TEXT),
        overrides,
        Children::spawn(children),
    )
}

fn update_button_styles(
    buttons: Query<
        (
            Entity,
            &ButtonVariant,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemedBackgroundColor,
            &ThemedTextColor,
        ),
        Or<(
            Changed<Hovered>,
            Changed<ButtonVariant>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    >,
    mut commands: Commands,
) {
    for (entity, variant, disabled, pressed, hovered, bg_color, font_color) in buttons.iter() {
        set_button_styles(
            entity,
            variant,
            disabled,
            pressed,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_button_styles_remove(
    buttons: Query<(
        Entity,
        &ButtonVariant,
        Has<InteractionDisabled>,
        Has<Pressed>,
        &Hovered,
        &ThemedBackgroundColor,
        &ThemedTextColor,
    )>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|entity| {
            if let Ok((button_ent, variant, disabled, pressed, hovered, bg_color, font_color)) =
                buttons.get(entity)
            {
                set_button_styles(
                    button_ent,
                    variant,
                    disabled,
                    pressed,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_button_styles(
    button_ent: Entity,
    variant: &ButtonVariant,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    bg_color: &ThemedBackgroundColor,
    font_color: &ThemedTextColor,
    commands: &mut Commands,
) {
    let bg_token = match (variant, disabled, pressed, hovered) {
        (ButtonVariant::Normal, true, _, _) => tokens::BUTTON_BG_DISABLED,
        (ButtonVariant::Normal, false, true, _) => tokens::BUTTON_BG_PRESSED,
        (ButtonVariant::Normal, false, false, true) => tokens::BUTTON_BG_HOVER,
        (ButtonVariant::Normal, false, false, false) => tokens::BUTTON_BG,
        (ButtonVariant::Primary, true, _, _) => tokens::BUTTON_PRIMARY_BG_DISABLED,
        (ButtonVariant::Primary, false, true, _) => tokens::BUTTON_PRIMARY_BG_PRESSED,
        (ButtonVariant::Primary, false, false, true) => tokens::BUTTON_PRIMARY_BG_HOVER,
        (ButtonVariant::Primary, false, false, false) => tokens::BUTTON_PRIMARY_BG,
    };

    let font_color_token = match (variant, disabled) {
        (ButtonVariant::Normal, true) => tokens::BUTTON_TEXT_DISABLED,
        (ButtonVariant::Normal, false) => tokens::BUTTON_TEXT,
        (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    // Change background color
    if bg_color.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemedBackgroundColor::new(bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(button_ent)
            .insert(ThemedTextColor::new(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(button_ent)
        .insert(EntityCursor::System(cursor_shape));
}

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (update_button_styles, update_button_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
