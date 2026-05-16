use std::fmt::{Display, Formatter, Result as FmtResult};

use bevy::reflect::Reflect;
use smol_str::SmolStr;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Reflect)]
pub struct ThemeToken(SmolStr);

impl ThemeToken {
    pub const fn new(token: SmolStr) -> Self {
        Self(token)
    }

    pub const fn new_static(token: &'static str) -> Self {
        Self(SmolStr::new_static(token))
    }
}

impl Display for ThemeToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}
