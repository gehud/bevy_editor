use std::f32::consts::TAU;

use egui::{Color32, FontId, Response, Ui, emath::Rot2, epaint::TextShape};
use lucide_icons::Icon;

use crate::assets::icons::MaterialIcon;

pub fn paint_collapsing_button(ui: &mut Ui, openness: f32, response: &Response) {
    let rotation = egui::remap(openness, 0.0..=1.0, -TAU / 4.0..=0.0);

    let icon = MaterialIcon::new(Icon::ChevronDown);

    let galley = {
        let color = ui.style().visuals.text_color();
        let font_id = FontId::new(15.0, icon.font_family());
        ui.painter().layout_no_wrap(icon.into(), font_id, color)
    };

    let rect = response.rect;

    let pos = rect.center() - (Rot2::from_angle(rotation) * (galley.size() / 2.0));

    ui.painter()
        .add(TextShape::new(pos, galley, Color32::WHITE).with_angle(rotation));
}
