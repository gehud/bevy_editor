//! One entity whose components cover every kind of field the inspector handles, plus the cases
//! it shows read-only. "Every Field" starts out selected.
//!
//! `cargo run --example inspector_fields`

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_editor::{EditorPlugin, editing::Selection};

fn main() {
    App::new()
        .add_plugins(EditorPlugin {
            demo_scene: false,
            ..default()
        })
        .register_type::<Numbers>()
        .register_type::<Vectors>()
        .register_type::<Structured>()
        .register_type::<ReadOnlyValues>()
        .register_type::<Frozen>()
        .add_systems(Startup, spawn)
        .run();
}

/// Every number type (editable).
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Numbers {
    f32_value: f32,
    f64_value: f64,
    i8_value: i8,
    i16_value: i16,
    i32_value: i32,
    i64_value: i64,
    isize_value: isize,
    u8_value: u8,
    u16_value: u16,
    u32_value: u32,
    u64_value: u64,
    usize_value: usize,
}

/// Vectors and rotations (editable; the `Quat` as Euler degrees).
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Vectors {
    position_2d: Vec2,
    position: Vec3,
    aligned: Vec3A,
    color_like: Vec4,
    orientation: Quat,
}

#[derive(Reflect, Default)]
struct Inner {
    enabled: bool,
    weight: f32,
    deeper: Deeper,
}

#[derive(Reflect, Default)]
struct Deeper {
    label: String,
    count: u32,
}

#[derive(Reflect, Default)]
struct Meters(f32);

/// Booleans, strings, nesting, newtypes and tuples (editable).
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Structured {
    flag: bool,
    title: String,
    inner: Inner,
    distance: Meters,
    pair: (f32, bool),
}

#[derive(Reflect, Default, Debug)]
enum Mode {
    #[default]
    Idle,
    Moving {
        speed: f32,
    },
}

/// Values the inspector shows as text only.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReadOnlyValues {
    mode: Mode,
    moving: Mode,
    maybe: Option<f32>,
    items: Vec<u32>,
    table: HashMap<String, i32>,
    target: Option<Entity>,
    tint: Color,
}

/// An immutable component: shown disabled.
#[derive(Component, Reflect, Default)]
#[component(immutable)]
#[reflect(Component)]
struct Frozen {
    value: f32,
    on: bool,
}

/// A component without reflection: shown as "Not reflected".
#[derive(Component)]
struct Opaque;

fn spawn(mut commands: Commands, mut selection: ResMut<Selection>) {
    let target = commands
        .spawn((Name::new("Target"), Transform::default()))
        .id();
    let every_field = commands.spawn((
        Name::new("Every Field"),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Numbers {
            f32_value: 1.5,
            f64_value: -2.25,
            i8_value: -8,
            u64_value: 64,
            ..default()
        },
        Vectors {
            position_2d: Vec2::new(1.0, 2.0),
            position: Vec3::new(1.0, 2.0, 3.0),
            aligned: Vec3A::new(4.0, 5.0, 6.0),
            color_like: Vec4::new(0.1, 0.2, 0.3, 1.0),
            orientation: Quat::from_euler(EulerRot::XYZ, 0.0, 45f32.to_radians(), 0.0),
        },
        Structured {
            flag: true,
            title: "Hello".into(),
            inner: Inner {
                enabled: true,
                weight: 0.5,
                deeper: Deeper {
                    label: "Nested".into(),
                    count: 3,
                },
            },
            distance: Meters(12.0),
            pair: (0.25, false),
        },
        ReadOnlyValues {
            moving: Mode::Moving { speed: 2.0 },
            maybe: Some(4.0),
            items: vec![1, 2, 3],
            table: HashMap::from_iter([("a".to_owned(), 1)]),
            target: Some(target),
            tint: Color::srgb(1.0, 0.5, 0.0),
            ..default()
        },
        Frozen {
            value: 7.0,
            on: true,
        },
        Opaque,
    ));
    selection.select(every_field.id());
}
