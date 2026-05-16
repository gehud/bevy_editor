use bevy::{scene::{Scene, SceneComponent, bsn}, ui::widget::Text};

use crate::theme::{
    ThemedTextColor, ThemedTextFont,
    token::ThemeToken,
    tokens::{TEXT_WEAK, TEXT_HEADING, TEXT_BODY},
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
    pub style: EditorTextStyle,
}

#[derive(Clone, Default, SceneComponent)]
#[scene(EditorTextProps)]
pub struct EditorText;

impl EditorText {
    fn scene(props: EditorTextProps) -> impl Scene {
        bsn! {
            Text::new(props.text)
            ThemedTextColor({props.style})
            ThemedTextFont({props.style})
        }
    }
}
