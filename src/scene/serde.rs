//! `serde` serialization and deserialization implementation for Bevy scenes.

use bevy::{
    asset::{AssetServer, UntypedAssetId},
    ecs::entity::Entity,
    platform::collections::HashSet,
    reflect::{
        PartialReflect, ReflectFromReflect, TypeRegistry,
        serde::{
            ReflectDeserializer, TypeRegistrationDeserializer, TypedReflectDeserializer,
            TypedReflectSerializer,
        },
    },
    scene::{DynamicEntity, DynamicScene},
};
use core::fmt::Formatter;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeStruct},
};

use crate::{
    asset::serde::{AssetHandleDeserializerProcessor, AssetHandleSerializerProcessor},
    scene::AssetScene,
};

/// Name of the serialized scene struct type.
pub const SCENE_STRUCT: &str = "Scene";
/// Name of the serialized resources field in a scene struct.
pub const SCENE_RESOURCES: &str = "resources";
/// Name of the serialized entities field in a scene struct.
pub const SCENE_ENTITIES: &str = "entities";

/// Name of the serialized entity struct type.
pub const ENTITY_STRUCT: &str = "Entity";
/// Name of the serialized component field in an entity struct.
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

/// Serializer for a [`DynamicScene`].
///
/// Helper object defining Bevy's serialize format for a [`DynamicScene`] and implementing
/// the [`Serialize`] trait for use with Serde.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_scene::{DynamicScene, serde::SceneSerializer};
/// # let mut world = World::default();
/// # world.insert_resource(AppTypeRegistry::default());
/// // Get the type registry
/// let registry = world.resource::<AppTypeRegistry>();
/// let registry = registry.read();
///
/// // Get a DynamicScene to serialize, for example from the World itself
/// let scene = DynamicScene::from_world(&world);
///
/// // Create a serializer for that DynamicScene, using the associated TypeRegistry
/// let scene_serializer = SceneSerializer::new(&scene, &registry);
///
/// // Serialize through any serde-compatible Serializer
/// let ron_string = ron::ser::to_string(&scene_serializer);
/// ```
pub struct AssetSceneSerializer<'a> {
    /// The scene to serialize.
    pub scene: &'a DynamicScene,
    /// The type registry containing the types present in the scene.
    pub registry: &'a TypeRegistry,
}

impl<'a> AssetSceneSerializer<'a> {
    /// Create a new serializer from a [`DynamicScene`] and an associated [`TypeRegistry`].
    ///
    /// The type registry must contain all types present in the scene. This is generally the case
    /// if you obtain both the scene and the registry from the same [`World`].
    ///
    /// [`World`]: bevy_ecs::world::World
    pub fn new(scene: &'a DynamicScene, registry: &'a TypeRegistry) -> Self {
        AssetSceneSerializer { scene, registry }
    }
}

impl<'a> Serialize for AssetSceneSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct(SCENE_STRUCT, 2)?;
        state.serialize_field(
            SCENE_RESOURCES,
            &SceneMapSerializer {
                entries: &self.scene.resources,
                registry: self.registry,
            },
        )?;
        state.serialize_field(
            SCENE_ENTITIES,
            &EntitiesSerializer {
                entities: &self.scene.entities,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

/// Handles serialization of multiple entities as a map of entity id to serialized entity.
pub struct EntitiesSerializer<'a> {
    /// The entities to serialize.
    pub entities: &'a [DynamicEntity],
    /// Type registry in which the component types used by the entities are registered.
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for EntitiesSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.entities.len()))?;
        for entity in self.entities {
            state.serialize_entry(
                &entity.entity,
                &EntitySerializer {
                    entity,
                    registry: self.registry,
                },
            )?;
        }
        state.end()
    }
}

/// Handles entity serialization as a map of component type to component value.
pub struct EntitySerializer<'a> {
    /// The entity to serialize.
    pub entity: &'a DynamicEntity,
    /// Type registry in which the component types used by the entity are registered.
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for EntitySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct(ENTITY_STRUCT, 1)?;
        state.serialize_field(
            ENTITY_FIELD_COMPONENTS,
            &SceneMapSerializer {
                entries: &self.entity.components,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

/// Handles serializing a list of values with a unique type as a map of type to value.
///
/// Used to serialize scene resources in [`SceneSerializer`] and entity components in [`EntitySerializer`].
/// Note that having several entries of the same type in `entries` will lead to an error when using the RON format and
/// deserializing through [`SceneMapDeserializer`].
///
/// Note: The entries are sorted by type path before they're serialized.
pub struct SceneMapSerializer<'a> {
    /// List of boxed values of unique type to serialize.
    pub entries: &'a [Box<dyn PartialReflect>],
    /// Type registry in which the types used in `entries` are registered.
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for SceneMapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.entries.len()))?;
        let sorted_entries = {
            let mut entries = self
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.get_represented_type_info().unwrap().type_path(),
                        entry.as_partial_reflect(),
                    )
                })
                .collect::<Vec<_>>();
            entries.sort_by_key(|(type_path, _)| *type_path);
            entries
        };

        let mut processor = AssetHandleSerializerProcessor::new();

        for (type_path, partial_reflect) in sorted_entries {
            state.serialize_entry(
                type_path,
                &TypedReflectSerializer::with_processor(
                    partial_reflect,
                    self.registry,
                    &mut processor,
                ),
            )?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum SceneField {
    Resources,
    Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Components,
}

/// Handles scene deserialization.
pub struct AssetSceneDeserializer<'a> {
    /// Type registry in which the components and resources types used in the scene to deserialize are registered.
    pub type_registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
}

impl<'a> AssetSceneDeserializer<'a> {
    pub fn new(type_registry: &'a TypeRegistry, asset_server: &'a AssetServer) -> Self {
        Self {
            type_registry,
            asset_server,
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for AssetSceneDeserializer<'a> {
    type Value = AssetScene;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut dependencies = HashSet::new();
        let scene = deserializer.deserialize_struct(
            SCENE_STRUCT,
            &[SCENE_RESOURCES, SCENE_ENTITIES],
            SceneVisitor {
                type_registry: self.type_registry,
                asset_server: self.asset_server,
                dependencies: &mut dependencies,
            },
        )?;

        Ok(AssetScene {
            scene,
            dependencies,
        })
    }
}

struct SceneVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> Visitor<'de> for SceneVisitor<'a> {
    type Value = DynamicScene;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_str("scene struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let resources = seq
            .next_element_seed(SceneMapDeserializer {
                registry: self.type_registry,
                asset_server: self.asset_server,
                dependencies: self.dependencies,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_RESOURCES))?;

        let entities = seq
            .next_element_seed(SceneEntitiesDeserializer {
                type_registry: self.type_registry,
                asset_server: self.asset_server,
                dependencies: self.dependencies,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene {
            resources,
            entities,
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut resources = None;
        let mut entities = None;
        while let Some(key) = map.next_key()? {
            match key {
                SceneField::Resources => {
                    if resources.is_some() {
                        return Err(Error::duplicate_field(SCENE_RESOURCES));
                    }
                    resources = Some(map.next_value_seed(SceneMapDeserializer {
                        registry: self.type_registry,
                        asset_server: self.asset_server,
                        dependencies: self.dependencies,
                    })?);
                }
                SceneField::Entities => {
                    if entities.is_some() {
                        return Err(Error::duplicate_field(SCENE_ENTITIES));
                    }
                    entities = Some(map.next_value_seed(SceneEntitiesDeserializer {
                        type_registry: self.type_registry,
                        asset_server: self.asset_server,
                        dependencies: self.dependencies,
                    })?);
                }
            }
        }

        let resources = resources.ok_or_else(|| Error::missing_field(SCENE_RESOURCES))?;
        let entities = entities.ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene {
            resources,
            entities,
        })
    }
}

/// Handles deserialization for a collection of entities.
pub struct SceneEntitiesDeserializer<'a> {
    /// Type registry in which the component types used by the entities to deserialize are registered.
    pub type_registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntitiesDeserializer<'a> {
    type Value = Vec<DynamicEntity>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SceneEntitiesVisitor {
            type_registry: self.type_registry,
            asset_server: self.asset_server,
            dependencies: self.dependencies,
        })
    }
}

struct SceneEntitiesVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> Visitor<'de> for SceneEntitiesVisitor<'a> {
    type Value = Vec<DynamicEntity>;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_str("map of entities")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entities = Vec::new();
        while let Some(entity) = map.next_key::<Entity>()? {
            let entity = map.next_value_seed(SceneEntityDeserializer {
                entity,
                type_registry: self.type_registry,
                asset_server: self.asset_server,
                dependencies: self.dependencies,
            })?;
            entities.push(entity);
        }

        Ok(entities)
    }
}

/// Handle deserialization of an entity and its components.
pub struct SceneEntityDeserializer<'a> {
    /// Id of the deserialized entity.
    pub entity: Entity,
    /// Type registry in which the component types used by the entity to deserialize are registered.
    pub type_registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntityDeserializer<'a> {
    type Value = DynamicEntity;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            ENTITY_STRUCT,
            &[ENTITY_FIELD_COMPONENTS],
            SceneEntityVisitor {
                entity: self.entity,
                registry: self.type_registry,
                asset_server: self.asset_server,
                dependencies: self.dependencies,
            },
        )
    }
}

struct SceneEntityVisitor<'a> {
    pub entity: Entity,
    pub registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> Visitor<'de> for SceneEntityVisitor<'a> {
    type Value = DynamicEntity;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_str("entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let components = seq
            .next_element_seed(SceneMapDeserializer {
                registry: self.registry,
                asset_server: self.asset_server,
                dependencies: self.dependencies,
            })?
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;

        Ok(DynamicEntity {
            entity: self.entity,
            components,
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
                EntityField::Components => {
                    if components.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    components = Some(map.next_value_seed(SceneMapDeserializer {
                        registry: self.registry,
                        asset_server: self.asset_server,
                        dependencies: self.dependencies,
                    })?);
                }
            }
        }

        let components = components
            .take()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(DynamicEntity {
            entity: self.entity,
            components,
        })
    }
}

/// Handles deserialization of a sequence of values with unique types.
pub struct SceneMapDeserializer<'a> {
    /// Type registry in which the types of the values to deserialize are registered.
    pub registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneMapDeserializer<'a> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SceneMapVisitor {
            registry: self.registry,
            asset_server: self.asset_server,
            dependencies: self.dependencies,
        })
    }
}

struct SceneMapVisitor<'a> {
    pub registry: &'a TypeRegistry,
    pub asset_server: &'a AssetServer,
    pub dependencies: &'a mut HashSet<UntypedAssetId>,
}

impl<'a, 'de> Visitor<'de> for SceneMapVisitor<'a> {
    type Value = Vec<Box<dyn PartialReflect>>;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        formatter.write_str("map of reflect types")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut dynamic_properties = Vec::new();
        let mut processor =
            AssetHandleDeserializerProcessor::new(self.asset_server, self.dependencies);
        while let Some(entity) = seq.next_element_seed(ReflectDeserializer::with_processor(
            self.registry,
            &mut processor,
        ))? {
            dynamic_properties.push(entity);
        }

        Ok(dynamic_properties)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut added = <HashSet<_>>::default();
        let mut entries = Vec::new();
        while let Some(registration) =
            map.next_key_seed(TypeRegistrationDeserializer::new(self.registry))?
        {
            if !added.insert(registration.type_id()) {
                return Err(Error::custom(format_args!(
                    "duplicate reflect type: `{}`",
                    registration.type_info().type_path(),
                )));
            }

            let mut processor =
                AssetHandleDeserializerProcessor::new(self.asset_server, self.dependencies);

            let value = map.next_value_seed(TypedReflectDeserializer::with_processor(
                registration,
                self.registry,
                &mut processor,
            ))?;

            // Attempt to convert using FromReflect.
            let value = self
                .registry
                .get(registration.type_id())
                .and_then(|tr| tr.data::<ReflectFromReflect>())
                .and_then(|fr| fr.from_reflect(value.as_partial_reflect()))
                .map(PartialReflect::into_partial_reflect)
                .unwrap_or(value);

            entries.push(value);
        }

        Ok(entries)
    }
}
