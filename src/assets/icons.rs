use egui::{FontFamily, RichText};
pub use lucide_icons::Icon;

use crate::assets::LUCIDE_FONT_FAMILY;

#[derive(Clone, Copy, Debug)]
pub struct MaterialIcon {
    pub icon: Icon,
}

impl MaterialIcon {
    pub fn new(icon: Icon) -> Self {
        Self { icon }
    }

    pub fn font_family(&self) -> FontFamily {
        FontFamily::Name(LUCIDE_FONT_FAMILY.into())
    }

    pub fn rich_text(self) -> RichText {
        RichText::new(self.icon.unicode()).family(self.font_family())
    }
}

impl From<MaterialIcon> for RichText {
    fn from(icon: MaterialIcon) -> Self {
        icon.rich_text()
    }
}

impl From<MaterialIcon> for egui::WidgetText {
    fn from(icon: MaterialIcon) -> Self {
        icon.rich_text().into()
    }
}

impl From<MaterialIcon> for char {
    fn from(icon: MaterialIcon) -> Self {
        icon.icon.unicode()
    }
}

impl From<MaterialIcon> for String {
    fn from(icon: MaterialIcon) -> Self {
        icon.icon.unicode().to_string()
    }
}
