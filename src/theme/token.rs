use std::fmt::{Display, Formatter, Result as FmtResult};

use bevy::reflect::Reflect;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
#[reflect(Clone, Debug, Hash, PartialEq)]
pub struct EditorThemeToken(&'static str);

impl Into<EditorThemeToken> for &'static str {
    fn into(self) -> EditorThemeToken {
        EditorThemeToken::new(self)
    }
}

impl EditorThemeToken {
    pub const fn new(token: &'static str) -> Self {
        Self(token)
    }
}

impl Display for EditorThemeToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}
