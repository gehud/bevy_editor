use egui::{Atoms, FontFamily, Label, Response, RichText, TextStyle, Ui, Widget};
pub use lucide_icons::Icon;

use crate::assets::{ICON_TEXT_STYLE, LUCIDE_FONT_FAMILY};

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

    pub fn text_style(&self) -> TextStyle {
        TextStyle::Name(ICON_TEXT_STYLE.into())
    }

    pub fn rich_text(self) -> RichText {
        RichText::new(self.icon.unicode())
            .family(self.font_family())
            .text_style(self.text_style())
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

impl Widget for MaterialIcon {
    fn ui(self, ui: &mut Ui) -> Response {
        Label::new(self).selectable(false).ui(ui)
    }
}
