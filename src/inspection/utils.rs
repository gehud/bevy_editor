use std::{
    panic::Location,
    path::{Path, PathBuf},
};

use bevy::ecs::error::Result;

pub fn pretty_type_name<T>() -> String {
    format!("{:?}", disqualified::ShortName::of::<T>())
}
pub fn pretty_type_name_str(val: &str) -> String {
    format!("{:?}", disqualified::ShortName(val))
}

pub mod guess_entity_name {
    use bevy::ecs::prelude::Name;
    use bevy::ecs::{archetype::Archetype, prelude::*, world::unsafe_world_cell::UnsafeWorldCell};

    use crate::inspection::restricted_world_view::RestrictedWorldView;

    /// Guesses an appropriate entity name like `Light (6)` or falls back to `Entity (8)`
    pub fn guess_entity_name(world: &World, entity: Entity) -> String {
        match world.get_entity(entity) {
            Ok(entity_ref) => {
                if let Some(name) = entity_ref.get::<Name>() {
                    return format!("{} ({})", name.as_str(), entity);
                }

                guess_entity_name_inner(
                    world.as_unsafe_world_cell_readonly(),
                    entity,
                    entity_ref.archetype(),
                )
            }
            Err(_) => format!("Entity {} (inexistent)", entity.index()),
        }
    }

    pub(crate) fn guess_entity_name_restricted(
        world: &mut RestrictedWorldView<'_>,
        entity: Entity,
    ) -> String {
        match world.world().get_entity(entity) {
            Ok(cell) => {
                if world.allows_access_to_component((entity, std::any::TypeId::of::<Name>())) {
                    // SAFETY: we have access and don't keep reference
                    if let Some(name) = unsafe { cell.get::<Name>() } {
                        return format!("{} ({})", name.as_str(), entity);
                    }
                }
                guess_entity_name_inner(world.world(), entity, cell.archetype())
            }
            Err(_) => format!("Entity {} (inexistent)", entity.index()),
        }
    }

    fn guess_entity_name_inner(
        world: UnsafeWorldCell<'_>,
        entity: Entity,
        archetype: &Archetype,
    ) -> String {
        #[rustfmt::skip]
        let associations = &[
            ("bevy::window::window::PrimaryWindow", "Primary Window"),
            ("bevy::camera::components::Camera3d", "Camera3d"),
            ("bevy::camera::components::Camera2d", "Camera2d"),
            ("bevy::light::point_light::PointLight", "PointLight"),
            ("bevy::light::directional_light::DirectionalLight", "DirectionalLight"),
            ("bevy::text::text::Text", "Text"),
            ("bevy::ui::ui_node::Node", "Node"),
            ("bevy::pbr::mesh_material::MeshMaterial3d<bevy::pbr::pbr_material::StandardMaterial>", "Pbr Mesh"),
            ("bevy::window::window::Window", "Window"),
            ("bevy::ecs::observer::distributed_storage::Observer", "Observer"),
            ("bevy::window::monitor::Monitor", "Monitor"),
            ("bevy::picking::pointer::PointerId", "Pointer"),
        ];

        let type_names = archetype.components().iter().filter_map(|id| {
            let name = world.components().get_info(*id)?.name();
            Some(name)
        });

        for component_type in type_names {
            if let Some(name) = associations.iter().find_map(|&(name, matches)| {
                (component_type.to_string() == name).then_some(matches)
            }) {
                return format!("{name} ({entity})");
            }
        }

        format!("Entity ({entity})")
    }
}

pub fn trim_cargo_registry_path(path: &Path) -> Option<PathBuf> {
    let mut components = path.components().peekable();
    while let Some(c) = components.next() {
        if c.as_os_str() == ".cargo" {
            if components.next()?.as_os_str() != "registry" {
                return None;
            }
            if components.next()?.as_os_str() != "src" {
                return None;
            }
            components.next()?;
            return Some(components.collect());
        }
    }

    None
}

pub fn open_file_at(location: &Location<'_>) -> Result<()> {
    let path = Path::new(location.file());

    // try editors supporting opening file:col first (in order of most likely to be explicitly installed)
    if std::process::Command::new("zeditor")
        .arg(location.to_string())
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    if std::process::Command::new("code")
        .arg("--goto")
        .arg(location.to_string())
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    opener::open(path)?;

    Ok(())
}
