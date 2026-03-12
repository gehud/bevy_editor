use std::sync::{Arc, Mutex};

use bevy::log::Level;
use chrono::{DateTime, Local};
use egui::{Color32, Grid, Label, Response, TextWrapMode, Ui, Widget};
use serde::{Deserialize, Serialize};

use crate::pane::output::collector::COLLECTOR;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LogsState {
    pub level_filter: LevelFilter,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelFilter {
    pub trace: bool,
    pub debug: bool,
    pub info: bool,
    pub warn: bool,
    pub error: bool,
}

impl Default for LevelFilter {
    fn default() -> Self {
        Self {
            trace: true,
            debug: true,
            info: true,
            warn: true,
            error: true,
        }
    }
}

impl LevelFilter {
    pub fn get(&self, level: Level) -> bool {
        match level {
            Level::TRACE => self.trace,
            Level::DEBUG => self.debug,
            Level::INFO => self.info,
            Level::WARN => self.warn,
            Level::ERROR => self.error,
        }
    }
}

pub trait DateTimeFormatExt {
    fn format_short(&self) -> String;
    fn format_detailed(&self) -> String;
}

impl DateTimeFormatExt for DateTime<Local> {
    fn format_short(&self) -> String {
        self.format("%H:%M:%S%.3f").to_string()
    }
    fn format_detailed(&self) -> String {
        self.format("%Y-%m-%dT%H:%M:%S%.f%:z").to_string()
    }
}

pub const TRACE_COLOR: Color32 = Color32::from_rgb(117, 80, 123);
pub const DEBUG_COLOR: Color32 = Color32::from_rgb(114, 159, 207);
pub const INFO_COLOR: Color32 = Color32::from_rgb(78, 154, 6);
pub const WARN_COLOR: Color32 = Color32::from_rgb(196, 160, 0);
pub const ERROR_COLOR: Color32 = Color32::from_rgb(204, 0, 0);

pub trait ToColor32 {
    fn to_color32(self) -> Color32;
}

impl ToColor32 for Level {
    fn to_color32(self) -> Color32 {
        match self {
            Self::TRACE => TRACE_COLOR,
            Self::DEBUG => DEBUG_COLOR,
            Self::INFO => INFO_COLOR,
            Self::WARN => WARN_COLOR,
            Self::ERROR => ERROR_COLOR,
        }
    }
}

pub struct Logs;

impl Widget for Logs {
    fn ui(self, ui: &mut Ui) -> Response {
        let state = ui.memory_mut(|mem| {
            let state_mem_id = ui.id();
            mem.data
                .get_temp_mut_or_insert_with(state_mem_id, || {
                    Arc::new(Mutex::new(LogsState::default()))
                })
                .clone()
        });

        let mut state = state.lock().unwrap();

        let events = COLLECTOR.events();

        let filtered_events = events
            .iter()
            .filter(|event| state.level_filter.get(event.level))
            .collect::<Vec<_>>();

        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                COLLECTOR.clear();
            }
        });

        Grid::new("log_grid")
            .striped(true)
            .min_col_width(ui.available_width())
            .show(ui, |ui| {
                for event in filtered_events {
                    ui.horizontal(|ui| {
                        ui.label(event.time.format_short())
                            .on_hover_text(event.time.format_detailed());

                        ui.colored_label(event.level.to_color32(), event.level.as_str());

                        let mut short_message = String::new();
                        let mut complete_message = String::new();
                        let mut log_message = String::new();

                        if let Some(msg) = event.fields.get("message") {
                            let msg = msg.trim();
                            short_message.push_str(msg);
                            complete_message.push_str(msg);
                        }

                        for (key, value) in &event.fields {
                            if key == "message" {
                                continue;
                            }
                            if key.starts_with("log.") {
                                log_message.push_str(&format!("\n {}: {}", key, value));
                            } else {
                                short_message.push_str(&format!(", {}: {}", key, value));
                                complete_message.push_str(&format!("\n {}: {}", key, value));
                            }
                        }

                        complete_message.push_str("\n\n");
                        complete_message.push_str(&log_message);

                        ui.add(Label::new(short_message).wrap_mode(TextWrapMode::Extend))
                            .on_hover_text(complete_message);
                    });

                    ui.end_row();
                }
            })
            .response
    }
}
