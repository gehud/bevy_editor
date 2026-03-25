use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        resource::Resource,
        schedule::{IntoScheduleConfigs, SystemSet, common_conditions::resource_changed},
        system::{BoxedSystem, In, IntoSystem, SystemId},
        world::{Mut, World},
    },
    log::warn,
    platform::collections::HashMap,
};

use super::PaneStructure;

enum PaneSystem {
    Unregistered(BoxedSystem<In<PaneStructure>>),
    Registered(SystemId<In<PaneStructure>>),
}

struct PaneState {
    name: String,
    system: Option<PaneSystem>,
}

#[derive(Resource, Default)]
pub struct PaneRegistry {
    panes: HashMap<String, PaneState>,
}

impl PaneRegistry {
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.panes.iter().filter_map(|(name, state)| {
            matches!(state.system.as_ref()?, PaneSystem::Registered(_)).then(|| name)
        })
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<SystemId<In<PaneStructure>>> {
        let state = self.panes.get(name.as_ref())?;
        let system = state.system.as_ref()?;

        match system {
            PaneSystem::Registered(system_id) => Some(*system_id),
            _ => None,
        }
    }

    pub fn register<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) {
        let name = name.into();
        if let Some(old) = self.panes.insert(
            name.clone(),
            PaneState {
                name: name.clone(),
                system: Some(PaneSystem::Unregistered(Box::new(IntoSystem::into_system(
                    system,
                )))),
            },
        ) {
            warn!("'{}' pane replaced with {} pane.", old.name, name);
        }
    }
}

pub trait PaneApp {
    fn register_pane<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) -> &mut Self;
}

impl PaneApp for App {
    fn register_pane<M>(
        &mut self,
        name: impl Into<String>,
        system: impl IntoSystem<In<PaneStructure>, (), M>,
    ) -> &mut Self {
        self.world_mut()
            .resource_mut::<PaneRegistry>()
            .register(name, system);
        self
    }
}

fn register_pane_systems(world: &mut World) {
    world.resource_scope(|world, mut pane_registry: Mut<PaneRegistry>| {
        for (_, state) in &mut pane_registry.panes {
            if let Some(system) = state.system.take() {
                let system_id = match system {
                    PaneSystem::Unregistered(system) => world.register_boxed_system(system),
                    PaneSystem::Registered(system_id) => system_id,
                };

                state.system = Some(PaneSystem::Registered(system_id));
            }
        }
    });
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PaneRegistrySystems {
    Registration,
}

pub(super) struct PaneRegistryPlugin;

impl Plugin for PaneRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaneRegistry>()
            .configure_sets(Update, PaneRegistrySystems::Registration)
            .add_systems(
                Update,
                register_pane_systems
                    .in_set(PaneRegistrySystems::Registration)
                    .run_if(resource_changed::<PaneRegistry>),
            );
    }
}
