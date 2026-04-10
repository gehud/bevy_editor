use std::{any::type_name, borrow::Cow, fs, path::PathBuf};

use bevy::{
    app::{App, AppExit, Plugin, PostUpdate, PreStartup, PreUpdate, Update},
    ecs::{
        event::Event,
        message::MessageReader,
        reflect::AppTypeRegistry,
        resource::Resource,
        system::{Res, SystemState},
        world::{Mut, World},
    }, utils::default,
};
use platform_dirs::AppDirs;
use ron::{Deserializer, ser::PrettyConfig};
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, DeserializeSeed},
};

fn data_path<R: Resource>() -> PathBuf {
    AppDirs::new(Some(env!("CARGO_PKG_NAME")), false)
        .unwrap()
        .state_dir
        .join(type_name::<R>())
        .with_extension("ron")
}

pub trait RegisterPref {
    fn register_pref<R: Resource + Default + Serialize + DeserializeOwned>(&mut self) -> &mut Self;
}

impl RegisterPref for App {
    fn register_pref<R: Resource + Default + Serialize + DeserializeOwned>(&mut self) -> &mut Self {
        let mut prefs_registry = self.world_mut().resource_mut::<PrefsRegistry>();

        prefs_registry.on_load.push(Box::new(|world| {
            let resource = if let Ok(text) = fs::read_to_string(data_path::<R>()) {
                ron::de::from_str::<R>(&text).unwrap_or_default()
            } else {
                default()
            };

            world.insert_resource(resource);
        }));

        prefs_registry.on_save.push(Box::new(|world| {
            let text =
                ron::ser::to_string_pretty(world.resource::<R>(), PrettyConfig::default()).unwrap();
            let path = data_path::<R>();
            fs::create_dir_all(path.parent().unwrap());
            fs::write(path, text).unwrap();
        }));

        self
    }
}

#[derive(Event)]
pub struct Save;

#[derive(Event)]
pub struct Load;

#[derive(Default, Resource)]
struct PrefsRegistry {
    on_load: Vec<Box<dyn Fn(&mut World) + Send + Sync>>,
    on_save: Vec<Box<dyn Fn(&mut World) + Send + Sync>>,
}

fn load(world: &mut World) {
    world.resource_scope(|world, registry: Mut<PrefsRegistry>| {
        for load in &registry.on_load {
            load(world);
        }
    });

    world.trigger(Load);
}

fn save(world: &mut World, state: &mut SystemState<MessageReader<AppExit>>) {
    let mut exit = state.get_mut(world);
    let should_exit = !exit.is_empty();
    exit.clear();

    if !should_exit {
        return;
    }

    world.trigger(Save);

    world.resource_scope(|world, registry: Mut<PrefsRegistry>| {
        for save in &registry.on_save {
            save(world);
        }
    });
}

pub struct PrefsPlugin;

impl Plugin for PrefsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrefsRegistry>()
            .add_systems(PreStartup, load)
            .add_systems(PostUpdate, save);
    }
}
