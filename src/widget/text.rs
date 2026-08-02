use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{query::Changed, schedule::IntoScheduleConfigs, system::Query},
    scene::{Scene, SceneComponent, bsn},
    ui::widget::Text,
};

use crate::theme::{
    EditorThemeSystems, ThemedTextColor, ThemedTextFont,
    token::ThemeToken,
    tokens::{TEXT_BODY, TEXT_HEADING, TEXT_WEAK},
};

#[derive(Clone, Copy, Default)]
pub enum EditorTextStyle {
    Weak,
    #[default]
    Body,
    Heading,
}

impl Into<ThemeToken> for EditorTextStyle {
    fn into(self) -> ThemeToken {
        match self {
            EditorTextStyle::Weak => TEXT_WEAK,
            EditorTextStyle::Body => TEXT_BODY,
            EditorTextStyle::Heading => TEXT_HEADING,
        }
    }
}

#[derive(Default)]
pub struct EditorTextProps {
    pub text: String,
}

#[derive(Clone, SceneComponent)]
#[require(ThemedTextColor, ThemedTextFont)]
#[scene(EditorTextProps)]
pub struct EditorText {
    pub color_style: EditorTextStyle,
    pub font_style: EditorTextStyle,
}

impl EditorText {
    pub const fn styled(style: EditorTextStyle) -> Self {
        Self {
            color_style: style,
            font_style: style,
        }
    }

    pub const fn heading() -> Self {
        Self::styled(EditorTextStyle::Heading)
    }

    pub const fn body() -> Self {
        Self::styled(EditorTextStyle::Body)
    }

    pub const fn weak() -> Self {
        Self::styled(EditorTextStyle::Weak)
    }
}

impl Default for EditorText {
    fn default() -> Self {
        Self::body()
    }
}

impl EditorText {
    fn scene(props: EditorTextProps) -> impl Scene {
        bsn! {
            Text::new(props.text)
        }
    }
}

fn update_text_style(
    mut text: Query<(&EditorText, &mut ThemedTextColor, &mut ThemedTextFont), Changed<EditorText>>,
) {
    for (text, mut color, mut font) in text.iter_mut() {
        color.0 = text.color_style.into();
        font.0 = text.font_style.into();
    }
}

pub struct EditorTextPlugin;

impl Plugin for EditorTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_text_style.before(EditorThemeSystems::Update),
        );
    }
}
