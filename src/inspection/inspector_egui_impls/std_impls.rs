use std::fmt::Debug;
use std::ops::{Add, Sub};
use std::{any::TypeId, borrow::Cow, path::PathBuf};

use bevy::ecs::error::Result;
use bevy::platform::time::Instant;
use bevy::reflect::{PartialReflect, Reflect, TypePath};
use bytemuck::Zeroable;
use egui::{DragValue, RichText, TextBuffer};

use super::{Inspector, InspectorUi};
use crate::inspection::inspector_options::{
    InspectorOptionsType,
    std_options::{NumberDisplay, NumberOptions, RangeOptions},
};
use std::{any::Any, time::Duration};

// just for orphan rules
trait Num: egui::emath::Numeric + Zeroable + Add<Output = Self> + Sub<Output = Self> {}

macro_rules! impl_num {
    ($($ty:ty),*) => {
        $(
            impl Num for $ty {}
        )*
    };
}

impl_num!(f32, f64, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

impl<T: Reflect + Num + Debug> Inspector for T {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        let options = options
            .downcast_ref::<NumberOptions<T>>()
            .cloned()
            .unwrap_or_default();
        Ok(display_number(value, None, &options, ui, 0.1).0)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        let options = options
            .downcast_ref::<NumberOptions<T>>()
            .cloned()
            .unwrap_or_default();
        let decimal_range = 0..=1usize;
        ui.add(
            egui::Button::new(
                RichText::new(format!(
                    "{}{}{}",
                    options.prefix,
                    egui::emath::format_with_decimals_in_range(value.to_f64(), decimal_range),
                    options.suffix
                ))
                .monospace(),
            )
            .truncate()
            .sense(egui::Sense::hover()),
        );

        Ok(())
    }

    fn ui_many(
        ui: &mut egui::Ui,
        options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        values: &mut [&mut dyn PartialReflect],
    ) -> Result<bool> {
        let values = values
            .iter_mut()
            .map(|value| value.try_downcast_mut::<Self>().unwrap())
            .collect::<Vec<_>>();
        let Some(first) = values.first() else {
            return Ok(false);
        };

        let mut target_value = **first;

        let options = options
            .downcast_ref::<NumberOptions<T>>()
            .cloned()
            .unwrap_or_default();

        let same = values.iter().all(|value| **value == target_value);

        let prev_value = target_value;
        let result = display_number(
            &mut target_value,
            if same { None } else { Some("-".into()) },
            &options,
            ui,
            0.1,
        );
        let changed = result.0;
        let draggeed = result.1;

        if changed {
            for value in values {
                if same || !draggeed {
                    *value = target_value;
                } else {
                    *value = *value + (target_value - prev_value);
                }
            }
        }

        Ok(changed)
    }
}

pub fn number_ui<T: egui::emath::Numeric>(
    value: &mut dyn Any,
    ui: &mut egui::Ui,
    options: &dyn Any,
    _: egui::Id,
    _: InspectorUi<'_, '_>,
) -> bool {
    let value = value.downcast_mut::<T>().unwrap();
    let options = options
        .downcast_ref::<NumberOptions<T>>()
        .cloned()
        .unwrap_or_default();
    display_number(value, None, &options, ui, 0.1).0
}
pub fn number_ui_readonly<T: egui::emath::Numeric>(
    value: &dyn Any,
    ui: &mut egui::Ui,
    options: &dyn Any,
    _: egui::Id,
    _: InspectorUi<'_, '_>,
) {
    let value = value.downcast_ref::<T>().unwrap();
    let options = options
        .downcast_ref::<NumberOptions<T>>()
        .cloned()
        .unwrap_or_default();
    let decimal_range = 0..=1usize;
    ui.add(
        egui::Button::new(
            RichText::new(format!(
                "{}{}{}",
                options.prefix,
                egui::emath::format_with_decimals_in_range(value.to_f64(), decimal_range),
                options.suffix
            ))
            .monospace(),
        )
        .truncate()
        .sense(egui::Sense::hover()),
    );
}

fn display_number<T: egui::emath::Numeric>(
    value: &mut T,
    label: Option<String>,
    options: &NumberOptions<T>,
    ui: &mut egui::Ui,
    default_speed: f32,
) -> (bool, bool) {
    let (mut changed, dragged) = match options.display {
        NumberDisplay::Drag => {
            let mut widget = egui::DragValue::new(value);
            if let Some(label) = label {
                widget = widget.custom_formatter(move |_, _| label.clone())
            }
            if !options.prefix.is_empty() {
                widget = widget.prefix(&options.prefix);
            }
            if !options.suffix.is_empty() {
                widget = widget.suffix(&options.suffix);
            }
            match (options.min, options.max) {
                (Some(min), Some(max)) => widget = widget.range(min.to_f64()..=max.to_f64()),
                (Some(min), None) => widget = widget.range(min.to_f64()..=f64::MAX),
                (None, Some(max)) => widget = widget.range(f64::MIN..=max.to_f64()),
                (None, None) => {}
            }
            if options.speed != 0.0 {
                widget = widget.speed(options.speed);
            } else {
                widget = widget.speed(default_speed);
            }
            let response = ui.add(widget);
            (response.changed(), response.dragged())
        }
        NumberDisplay::Slider => {
            let min = options.min.unwrap_or_else(|| T::from_f64(0.0));
            let max = options.max.unwrap_or_else(|| T::from_f64(1.0));
            let range = min..=max;
            let widget = egui::Slider::new(value, range);
            let response = ui.add(widget);
            (response.changed(), false)
        }
    };

    if let Some(min) = options.min {
        let as_f64 = value.to_f64();
        let min = min.to_f64();
        if as_f64 < min {
            *value = T::from_f64(min);
            changed = true;
        }
    }
    if let Some(max) = options.max {
        let as_f64 = value.to_f64();
        let max = max.to_f64();
        if as_f64 > max {
            *value = T::from_f64(max);
            changed = true;
        }
    }
    (changed, dragged)
}

impl Inspector for bool {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        Ok(ui.checkbox(value, "").changed())
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let mut value = value.try_downcast_ref::<Self>().unwrap().clone();
        ui.add_enabled_ui(false, |ui| -> Result {
            Self::ui(ui, options, id, env, &mut value)?;
            Ok(())
        })
        .inner?;
        Ok(())
    }
}

impl Inspector for String {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let result = if value.contains('\n') {
            ui.text_edit_multiline(value).changed()
        } else {
            ui.text_edit_singleline(value).changed()
        };

        Ok(result)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();

        if value.contains('\n') {
            ui.text_edit_multiline(&mut value.as_str());
        } else {
            ui.text_edit_singleline(&mut value.as_str());
        }

        Ok(())
    }
}

impl Inspector for Cow<'static, str> {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let mut clone = value.to_string();
        let changed = if value.contains('\n') {
            ui.text_edit_multiline(&mut clone).changed()
        } else {
            ui.text_edit_singleline(&mut clone).changed()
        };

        if changed {
            *value = Cow::Owned(clone);
        }

        Ok(changed)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        if value.contains('\n') {
            ui.text_edit_multiline(&mut value.as_str());
        } else {
            ui.text_edit_singleline(&mut value.as_str());
        }
        Ok(())
    }
}

impl Inspector for Duration {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        id: egui::Id,
        mut env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();

        let mut seconds = value.as_secs_f64();
        let options = NumberOptions {
            min: Some(0.0f64),
            suffix: "s".to_string(),
            ..Default::default()
        };

        let changed = env.ui_for_reflect_with_options(&mut seconds, ui, id, &options)?;
        if changed {
            *value = Duration::from_secs_f64(seconds);
        }
        Ok(changed)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        id: egui::Id,
        mut env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        let seconds = value.as_secs_f64();
        let options = NumberOptions {
            min: Some(0.0f64),
            suffix: "s".to_string(),
            ..Default::default()
        };
        env.ui_for_reflect_readonly_with_options(&seconds, ui, id, &options)?;
        Ok(())
    }
}

impl Inspector for Instant {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        Self::ui_readonly(ui, options, id, env, value)?;
        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        let mut secs = value.elapsed().as_secs_f32();
        ui.horizontal(|ui| {
            ui.add_enabled(false, DragValue::new(&mut secs));
            ui.label("seconds ago");
        });
        Ok(())
    }
}

impl<T: Reflect + TypePath + egui::emath::Numeric + InspectorOptionsType> Inspector
    for std::ops::Range<T>
{
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        let std::ops::Range { start, end } = value;
        Ok(display_range::<T>(
            ui,
            options,
            id,
            env,
            "..",
            Some(start),
            Some(end),
        ))
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        let std::ops::Range { start, end } = value;
        display_range_readonly::<T>(ui, options, id, env, "..", Some(start), Some(end));
        Ok(())
    }
}
fn display_range<T: egui::emath::Numeric + InspectorOptionsType>(
    ui: &mut egui::Ui,
    options: &dyn Any,
    id: egui::Id,
    mut env: InspectorUi<'_, '_>,

    // value is made to be generic but I'm currently just using it for a..b, not a..=b, ..a, a.., .., etc., because these types don't hand out mutable references
    symbol: &'static str,
    start: Option<&mut T>,
    end: Option<&mut T>,
) -> bool {
    let options = options.downcast_ref::<RangeOptions<T>>();

    let start_options = options.map(|a| &a.start as &dyn Any).unwrap_or(&());
    let end_options = options.map(|a| &a.end as &dyn Any).unwrap_or(&());

    let mut changed = false;
    ui.horizontal(|ui| {
        if let Some(start) = start {
            changed |= number_ui::<T>(start, ui, start_options, id, env.reborrow());
        }
        ui.label(symbol);
        if let Some(end) = end {
            changed |= number_ui::<T>(end, ui, end_options, id, env.reborrow());
        }
    });

    changed
}

fn display_range_readonly<T: egui::emath::Numeric + InspectorOptionsType>(
    ui: &mut egui::Ui,
    options: &dyn Any,
    id: egui::Id,
    mut env: InspectorUi<'_, '_>,

    symbol: &'static str,
    start: Option<&T>,
    end: Option<&T>,
) {
    let options = options.downcast_ref::<RangeOptions<T>>();

    let start_options = options.map(|a| &a.start as &dyn Any).unwrap_or(&());
    let end_options = options.as_ref().map(|a| &a.end as &dyn Any).unwrap_or(&());

    ui.horizontal(|ui| {
        if let Some(start) = start {
            number_ui_readonly::<T>(start, ui, start_options, id, env.reborrow());
        }
        ui.label(symbol);
        if let Some(end) = end {
            number_ui_readonly::<T>(end, ui, end_options, id, env.reborrow());
        }
    });
}

impl<T: Reflect + TypePath + egui::emath::Numeric + InspectorOptionsType> Inspector
    for std::ops::RangeInclusive<T>
{
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        let mut start = *value.start();
        let mut end = *value.end();

        let changed = display_range::<T>(
            ui,
            options,
            id,
            env,
            "..=",
            Some(&mut start),
            Some(&mut end),
        );

        if changed {
            *value = start..=end;
        }

        Ok(changed)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        display_range_readonly::<T>(
            ui,
            options,
            id,
            env,
            "..",
            Some(value.start()),
            Some(value.end()),
        );
        Ok(())
    }
}

impl Inspector for PathBuf {
    fn ui(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        let mut str = value.to_string_lossy();
        let changed = ui.text_edit_singleline(&mut str).changed();

        if changed {
            *value = PathBuf::from(str.as_str());
        }

        Ok(changed)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        ui.text_edit_singleline(&mut value.to_string_lossy());
        Ok(())
    }
}

impl Inspector for TypeId {
    fn ui(
        ui: &mut egui::Ui,
        options: &dyn Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
        value: &mut dyn PartialReflect,
    ) -> Result<bool> {
        let value = value.try_downcast_mut::<Self>().unwrap();
        Self::ui_readonly(ui, options, id, env, value)?;
        Ok(false)
    }

    fn ui_readonly(
        ui: &mut egui::Ui,
        _options: &dyn Any,
        _id: egui::Id,
        _env: InspectorUi<'_, '_>,
        value: &dyn PartialReflect,
    ) -> Result {
        let value = value.try_downcast_ref::<Self>().unwrap();
        let str = format!("{:?}", value);
        ui.label(str);
        Ok(())
    }
}
