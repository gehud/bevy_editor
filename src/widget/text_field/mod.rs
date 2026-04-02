use bevy::{
    app::{App, Plugin},
    ecs::{
        bundle::Bundle, children, component::Component, entity::Entity, event::EntityEvent,
        observer::On, system::BoxedSystem,
    },
    picking::{
        events::{Click, Pointer},
        pointer::PointerButton,
    },
    text::TextLayout,
    ui::{AlignItems, JustifyContent, Node, Overflow, UiRect, percent, px, widget::Text},
    ui_widgets::observe,
    utils::default,
};

use crate::{
    theme::{RoundedCorners, ThemedBackgroundColor, tokens::BUTTON_BG},
    widget::{EditorText, ValueChange},
};

#[derive(Component)]
pub struct TextFieldSettings {
    pub text: String,
    pub caret_blink_rate: f32,
}

impl Default for TextFieldSettings {
    fn default() -> Self {
        Self {
            text: default(),
            caret_blink_rate: 1.7,
        }
    }
}

#[derive(Default)]
pub struct TextField {
    settings: TextFieldSettings,
}

impl TextField {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> impl Bundle {
        (
            Node {
                width: percent(100),
                border_radius: RoundedCorners::All.to_border_radius(4.0),
                padding: UiRect::horizontal(px(8)).with_top(px(2)).with_bottom(px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Start,
                overflow: Overflow::hidden(),
                ..default()
            },
            ThemedBackgroundColor(BUTTON_BG),
            self.settings,
            children![(
                TextLayout::new_with_no_wrap(),
                Text::default(),
                EditorText::body()
            )],
        )
    }
}

fn on_click(trigger: On<Pointer<Click>>) {
    if trigger.button != PointerButton::Primary {
        return;
    }
}

pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click);
    }
}
