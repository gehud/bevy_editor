#[cfg(feature = "editor")]
use std::io::Write;
use std::{any::TypeId, fs::File, io::Read};

#[cfg(feature = "editor")]
use bevy::{
    app::AppExit,
    ecs::{message::MessageReader, system::SystemState},
    reflect::serde::ReflectSerializer,
};
use bevy::{
    app::{App, Plugin, PreStartup},
    ecs::{
        error::Result,
        reflect::{AppTypeRegistry, ReflectResource},
        resource::Resource,
        world::{CommandQueue, FromWorld, World},
    },
    platform::collections::HashSet,
    reflect::{
        PartialReflect, reflect_trait, serde::ReflectDeserializer, std_traits::ReflectDefault,
    },
};
use egui::{ScrollArea, Ui};
use serde::de::{DeserializeSeed, IntoDeserializer};
use toml::{Table, Value};

pub const SETTINGS_PATH: &'static str = "settings.toml";

#[cfg(feature = "editor")]
use crate::serde::ser::EditorSerializerProcessor;
use crate::{
    inspection::{
        reflect_inspector::{Context, InspectorUi},
        restricted_world_view::RestrictedWorldView,
    },
    panel::{Panel, PanelApp},
    serde::de::EditorDeserializerProcessor,
    style::PANEL_BG_COLOR,
};

pub struct SettingsPanel;

impl Panel for SettingsPanel {
    fn name(&self) -> &str {
        "Settings"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
        let type_registry = world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        ui.visuals_mut().panel_fill = PANEL_BG_COLOR;

        let selected_id = ui.id().with("selected_settings");

        egui::Panel::left("settings")
            .show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| -> Result {
                            let mut selected_type_id = ui.memory_mut(|memory| {
                                *memory.data.get_temp_mut_or_insert_with::<Option<TypeId>>(
                                    selected_id,
                                    || None,
                                )
                            });

                            for type_id in &world.resource::<SettingsRegistry>().0 {
                                let registration = type_registry.get(*type_id).unwrap();

                                let reflect_resouce =
                                    registration.data::<ReflectResource>().unwrap();
                                let reflect_settings =
                                    registration.data::<ReflectSettings>().unwrap();
                                let world: &World = world;
                                let resouce = reflect_resouce.reflect(world)?;
                                let setting = reflect_settings.get(resouce).unwrap();

                                let is_selected = selected_type_id
                                    .is_some_and(|selected_type_id| &selected_type_id == type_id);

                                if ui.selectable_label(is_selected, setting.title()).clicked() {
                                    selected_type_id = Some(*type_id);
                                }
                            }

                            ui.memory_mut(|memory| {
                                memory.data.insert_temp(selected_id, selected_type_id)
                            });

                            Ok(())
                        })
                        .inner
                    })
                    .inner
            })
            .inner?;

        egui::CentralPanel::default()
            .show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| -> Result {
                        let selected_type_id = ui.memory_mut(|memory| {
                            *memory
                                .data
                                .get_temp_mut_or_insert_with::<Option<TypeId>>(selected_id, || None)
                        });

                        if let Some(type_id) = selected_type_id {
                            let mut world_view = RestrictedWorldView::from(world);
                            let (resouce_world, world_view) =
                                world_view.split_off_resource(type_id.clone());

                            let registration = type_registry.get(type_id).unwrap();
                            let reflect_resouce = registration.data::<ReflectResource>().unwrap();
                            // SAFETY: ReflectResouce::reflect only get specified resouce type.
                            let world: &mut World = unsafe { resouce_world.world().world_mut() };
                            let mut resouce = reflect_resouce.reflect_mut(world)?;

                            let mut queue = CommandQueue::default();
                            let mut ctx = Context {
                                world: world_view,
                                queue: &mut queue,
                            };

                            let mut inspector = InspectorUi::new(&type_registry, &mut ctx);
                            inspector.ui_for_reflect(resouce.as_partial_reflect_mut(), ui)?;
                        }

                        Ok(())
                    })
                    .inner
            })
            .inner?;

        Ok(())
    }
}

#[derive(Default)]
pub enum SettingsGroup {
    #[default]
    Runtime,
    Editor,
}

#[reflect_trait]
pub trait Settings: Resource {
    fn title(&self) -> &str;

    fn group(&self) -> SettingsGroup {
        SettingsGroup::default()
    }
}

#[derive(Clone, Resource, Default)]
struct SettingsRegistry(HashSet<TypeId>);

fn load(world: &mut World) -> Result {
    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut settings_registry = world.resource_mut::<SettingsRegistry>();
    for registration in type_registry.iter() {
        let is_settings = registration.data::<ReflectDefault>().is_some()
            && registration.data::<ReflectSettings>().is_some()
            && registration.data::<ReflectResource>().is_some();

        if is_settings {
            settings_registry.0.insert(registration.type_id());
        }
    }

    let mut file = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(SETTINGS_PATH)?;

    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let mut table = toml::from_str::<Table>(&text)?;

    for type_id in world.resource::<SettingsRegistry>().clone().0 {
        let registration = type_registry.get(type_id).unwrap();
        let reflect_default = registration.data::<ReflectDefault>().unwrap();
        let reflect_resouce = registration.data::<ReflectResource>().unwrap();

        let value = if let Some(value) = table.remove(registration.type_info().type_path()) {
            let deserializer = value.into_deserializer();

            let mut processor = EditorDeserializerProcessor::from_world(world);

            let reflect_deserializer =
                ReflectDeserializer::with_processor(&type_registry, &mut processor);
            reflect_deserializer.deserialize(deserializer)?
        } else {
            reflect_default.default()
        };

        reflect_resouce.insert(world, value.as_ref(), &type_registry);
    }

    Ok(())
}

#[cfg(feature = "editor")]
fn save(world: &mut World, state: &mut SystemState<MessageReader<AppExit>>) -> Result {
    let mut exit = state.get_mut(world);
    let should_exit = !exit.is_empty();
    exit.clear();

    if !should_exit {
        return Ok(());
    }

    let mut table = Table::new();

    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    for type_id in world.resource::<SettingsRegistry>().0.clone() {
        let registration = type_registry.get(type_id).unwrap();
        let reflect_resouce = registration.data::<ReflectResource>().unwrap();
        let processor = EditorSerializerProcessor::from_world(world);
        let world: &World = world;
        let value = reflect_resouce.reflect(world)?;
        let reflect_serializer =
            ReflectSerializer::with_processor(value, &type_registry, &processor);
        let value = Value::try_from(reflect_serializer)?;
        table.insert(registration.type_info().type_path().to_string(), value);
    }

    let mut file = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(SETTINGS_PATH)?;

    file.write(table.to_string().as_bytes())?;

    Ok(())
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.register_panel(SettingsPanel)
            .init_resource::<SettingsRegistry>()
            .add_systems(PreStartup, load);

        #[cfg(feature = "editor")]
        {
            use bevy::app::PostUpdate;

            app.add_systems(PostUpdate, save);
        }
    }
}
