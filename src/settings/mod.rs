#[cfg(feature = "editor")]
use std::io::Write;
use std::{
    any::TypeId,
    fs::{self, File},
    io::Read,
};

#[cfg(feature = "editor")]
use bevy::{
    app::AppExit,
    ecs::{message::MessageReader, system::SystemState},
    reflect::serde::ReflectSerializer,
};
use bevy::{
    app::{App, Plugin, PreStartup, Startup},
    asset::AssetServer,
    ecs::{
        error::Result,
        reflect::{AppTypeRegistry, ReflectResource},
        resource::Resource,
        system::{Res, ResMut},
        world::World,
    },
    platform::collections::HashSet,
    reflect::{
        PartialReflect, Reflect, reflect_trait, serde::ReflectDeserializer,
        std_traits::ReflectDefault,
    },
};
use egui::Ui;
#[cfg(feature = "editor")]
use serde::Serialize;
use serde::{
    Deserialize,
    de::{DeserializeSeed, IntoDeserializer},
};
use toml::{Table, Value, de::ValueDeserializer, ser::ValueSerializer};

pub const SETTINGS_PATH: &'static str = "settings.toml";

#[cfg(feature = "editor")]
use crate::{asset::AssetDatabase, scene::serde::ser::EditorSerializerProcessor};
use crate::{
    panel::{Panel, PanelApp},
    scene::serde::de::EditorDeserializerProcessor,
};

pub struct SettingsPanel;

impl Panel for SettingsPanel {
    fn name(&self) -> &str {
        "Settings"
    }

    fn ui(&mut self, ui: &mut Ui, world: &mut World) -> Result {
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

    #[cfg(feature = "editor")]
    let asset_database = world.resource::<AssetDatabase>().clone();
    let asset_server = world.resource::<AssetServer>().clone();

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

            let mut collector = HashSet::new();
            let mut processor = EditorDeserializerProcessor::new(
                #[cfg(feature = "editor")]
                &asset_database,
                &asset_server,
                &mut collector,
            );

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

    let asset_database = world.resource::<AssetDatabase>();

    for type_id in &world.resource::<SettingsRegistry>().0 {
        let registration = type_registry.get(*type_id).unwrap();
        let reflect_resouce = registration.data::<ReflectResource>().unwrap();
        let world: &World = world;
        let value = reflect_resouce.reflect(world)?;
        let processor = EditorSerializerProcessor::new(asset_database);
        let reflect_serializer =
            ReflectSerializer::with_processor(value, &type_registry, &processor);
        let value = Value::try_from(reflect_serializer)?;
        table.insert(registration.type_info().type_path().to_string(), value);
    }

    let mut file = File::options()
        .create(true)
        .write(true)
        .open(SETTINGS_PATH)?;

    file.write(table.to_string().as_bytes())?;

    Ok(())
}

#[derive(Default, Resource, Reflect)]
#[reflect(Default, Resource, Settings)]
pub struct MySettings {
    pub value: f32,
}

impl Settings for MySettings {
    fn title(&self) -> &str {
        "My Settings"
    }
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
