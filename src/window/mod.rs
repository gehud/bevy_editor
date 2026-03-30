use std::isize;

use bevy::{
    app::{App, First, Plugin, PreUpdate, Update},
    asset::{AssetServer, Handle},
    camera::{Camera, Camera2d, ClearColor, NormalizedRenderTarget, RenderTarget},
    color::Color,
    ecs::{
        change_detection::DetectChangesMut,
        component::Component,
        entity::{ContainsEntity, Entity},
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::Remove,
        message::MessageReader,
        observer::On,
        query::{Added, Changed, With},
        reflect::ReflectComponent,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, EntityCommands, In, Query, Res, Single, SystemState},
        world::World,
    },
    image::Image,
    math::{CompassOctant, Vec2},
    picking::{
        Pickable, PickingSystems,
        events::{Click, Out, Over, Pointer, Press},
        pointer::{Location, PointerLocation},
    },
    reflect::Reflect,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, PositionType, UiRect,
        UiTargetCamera, Val, percent, px, widget::ImageNode,
    },
    utils::default,
    window::{
        ExitCondition, PrimaryWindow, SystemCursorIcon, Window, WindowEvent, WindowPlugin,
        WindowRef,
    },
    winit::WINIT_WINDOWS,
};

use crate::{
    selection::DeselectionLayer,
    theme::{
        RoundedCorners, ThemedBackgroundColor, ThemedBorderColor, palette,
        tokens::{BORDER, WINDOW_BG},
    },
    widget::EntityCursor,
};

pub const WINDOW_RESIZE_GRIP_SIZE: f32 = 5.0;
pub const WINDOW_BORDER_RADIUS: f32 = 8.0;
pub const WINDOW_BUTTON_SIZE: f32 = 26.0;
pub const WINDOW_BUTTON_RADIUS: f32 = WINDOW_BUTTON_SIZE / 2.0;

#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Clone, Component, Debug)]
pub struct EditorWindowStructure {
    camera: Entity,
    root: Entity,
    titlebar: Entity,
    content: Entity,
    maximize: Entity,
}

impl EditorWindowStructure {
    pub fn root(&self) -> Entity {
        self.root
    }

    pub fn titlebar(&self) -> Entity {
        self.titlebar
    }

    pub fn content(&self) -> Entity {
        self.content
    }
}

#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Clone, Component, Debug)]
pub struct EditorWindow {
    pub is_maximized: bool,
}

#[derive(Clone, Debug, EntityEvent, Reflect)]
#[reflect(Clone, Debug)]
pub struct PrimaryEditorWindowConfigured {
    pub entity: Entity,
}

#[derive(Clone, Debug, EntityEvent, Reflect)]
#[reflect(Clone, Debug)]
pub struct EditorWindowConfigured {
    pub entity: Entity,
}

#[derive(Component)]
struct EditorWindowRef(Entity);

fn configure_windows(
    windows: Query<Entity, Added<Window>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) {
    for window in windows {
        commands
            .entity(window)
            .insert(EditorWindow {
                is_maximized: false,
            })
            .entry::<Window>()
            .and_modify(|mut window| {
                window.decorations = false;
                window.transparent = true;
            });

        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    order: isize::MAX,
                    ..default()
                },
                RenderTarget::Window(WindowRef::Entity(window)),
            ))
            .id();

        let mut titlebar = Entity::PLACEHOLDER;
        let mut content = Entity::PLACEHOLDER;
        let mut maximize = Entity::PLACEHOLDER;

        let root = commands
            .spawn((
                UiTargetCamera(camera),
                DeselectionLayer,
                Node {
                    width: percent(100),
                    height: percent(100),
                    border: UiRect::all(px(1)),
                    border_radius: RoundedCorners::All.to_border_radius(WINDOW_BORDER_RADIUS),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ThemedBorderColor::all(BORDER),
                ThemedBackgroundColor::new(WINDOW_BG),
            ))
            .id();

        commands.entity(root).with_children(|commands| {
            commands
                .spawn((Node {
                    width: percent(100),
                    height: px(34),
                    flex_shrink: 0.0,
                    ..default()
                },))
                .with_children(|commands| {
                    // Move
                    commands
                        .spawn((
                            EditorWindowRef(window),
                            Node {
                                position_type: PositionType::Absolute,
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                        ))
                        .observe(
                            |trigger: On<Pointer<Press>>,
                             window_refs: Query<&EditorWindowRef>,
                             mut windows: Query<&mut Window>|
                             -> Result {
                                let mut window =
                                    windows.get_mut(window_refs.get(trigger.entity)?.0)?;
                                window.start_drag_move();
                                Ok(())
                            },
                        );

                    // User titlebar
                    titlebar = commands
                        .spawn((
                            Node {
                                width: percent(100),
                                height: percent(100),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .id();

                    // Window controls
                    commands
                        .spawn((
                            Node {
                                width: px(136),
                                height: percent(100),
                                right: px(0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|commands| {
                            spawn_titlebar_button(
                                commands.target_entity(),
                                &mut commands.commands(),
                                window,
                                assets.load("embedded://bevy_editor//icons/minimize.png"),
                            )
                            .observe(
                                |trigger: On<Pointer<Click>>,
                                 window_refs: Query<&EditorWindowRef>,
                                 mut windows: Query<&mut Window>|
                                 -> Result {
                                    let mut window =
                                        windows.get_mut(window_refs.get(trigger.entity)?.0)?;
                                    window.set_minimized(true);
                                    Ok(())
                                },
                            );

                            maximize = spawn_titlebar_button(
                                commands.target_entity(),
                                &mut commands.commands(),
                                window,
                                assets.load("embedded://bevy_editor//icons/maximize.png"),
                            )
                            .observe(
                                |trigger: On<Pointer<Click>>,
                                 editor_window_refs: Query<&EditorWindowRef>,
                                 mut editor_windows: Query<&mut EditorWindow>|
                                 -> Result {
                                    let mut editor_window = editor_windows
                                        .get_mut(editor_window_refs.get(trigger.entity)?.0)?;
                                    editor_window.is_maximized = !editor_window.is_maximized;
                                    Ok(())
                                },
                            )
                            .id();

                            spawn_titlebar_button(
                                commands.target_entity(),
                                &mut commands.commands(),
                                window,
                                assets.load("embedded://bevy_editor//icons/close.png"),
                            )
                            .observe(
                                |trigger: On<Pointer<Click>>,
                                 editor_window_refs: Query<&EditorWindowRef>,
                                 mut commands: Commands|
                                 -> Result {
                                    commands
                                        .entity(editor_window_refs.get(trigger.entity)?.0)
                                        .despawn();
                                    Ok(())
                                },
                            );
                        });
                });

            // User content
            content = commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();

            // North resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        width: percent(100),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::RowResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::North);
                        Ok(())
                    },
                );

            // South resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(0),
                        width: percent(100),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::RowResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::South);
                        Ok(())
                    },
                );

            // West resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: percent(100),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::ColResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::West);
                        Ok(())
                    },
                );

            // East resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: percent(100),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::ColResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::East);
                        Ok(())
                    },
                );

            // Northwest resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::NwResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::NorthWest);
                        Ok(())
                    },
                );

            // Northeast resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        top: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::NeResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::NorthEast);
                        Ok(())
                    },
                );

            // Southwest resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        bottom: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::SwResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::SouthWest);
                        Ok(())
                    },
                );

            // Southeast resize
            commands
                .spawn((
                    EditorWindowRef(window),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        bottom: px(0),
                        width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                        ..default()
                    },
                ))
                .insert(EntityCursor::System(SystemCursorIcon::SeResize))
                .observe(
                    |trigger: On<Pointer<Press>>,
                     editor_window_refs: Query<&EditorWindowRef>,
                     mut windows: Query<&mut Window>|
                     -> Result {
                        windows
                            .get_mut(editor_window_refs.get(trigger.entity)?.0)?
                            .start_drag_resize(CompassOctant::SouthEast);
                        Ok(())
                    },
                );
        });

        commands
            .entity(window)
            .insert(EditorWindowStructure {
                camera,
                root,
                titlebar,
                content,
                maximize,
            })
            .observe(
                move |_: On<Remove, EditorWindowStructure>, mut commands: Commands| {
                    commands.entity(root).despawn();
                    commands.entity(camera).despawn();
                },
            );

        commands.trigger(EditorWindowConfigured { entity: window });

        if window == *primary_window {
            commands.trigger(PrimaryEditorWindowConfigured { entity: window });
        }
    }
}

fn spawn_titlebar_button<'a>(
    container: Entity,
    commands: &'a mut Commands,
    window: Entity,
    icon: Handle<Image>,
) -> EntityCommands<'a> {
    let root = commands
        .spawn((
            ChildOf(container),
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();

    let button = commands
        .spawn((
            EditorWindowRef(window),
            ChildOf(root),
            Node {
                width: px(WINDOW_BUTTON_SIZE),
                height: px(WINDOW_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: RoundedCorners::All.to_border_radius(WINDOW_BUTTON_RADIUS),
                ..default()
            },
        ))
        .observe(|trigger: On<Pointer<Over>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(BackgroundColor(palette::GRAY_1));
        })
        .observe(|trigger: On<Pointer<Out>>, mut commands: Commands| {
            commands
                .entity(trigger.entity)
                .insert(BackgroundColor::default());
        })
        .with_children(|commands| {
            commands.spawn((
                Node {
                    width: px(16),
                    height: px(16),
                    ..default()
                },
                ImageNode::new(icon),
            ));
        })
        .id();

    commands.entity(button)
}

fn set_maximize_style(
    In((window, is_maximized)): In<(Entity, bool)>,
    children: Query<&Children>,
    editor_windows: Query<&EditorWindowStructure>,
    mut image_nodes: Query<&mut ImageNode>,
    mut nodes: Query<&mut Node>,
    asset_server: Res<AssetServer>,
) -> Result {
    let editor_window = editor_windows.get(window)?;
    let mut image = image_nodes.get_mut(children.get(editor_window.maximize)?[0])?;

    image.image = if is_maximized {
        asset_server.load("embedded://bevy_editor//icons/restore.png")
    } else {
        asset_server.load("embedded://bevy_editor//icons/maximize.png")
    };

    let border_radius = if is_maximized {
        0.0
    } else {
        WINDOW_BORDER_RADIUS
    };

    let border = if is_maximized { 0.0 } else { 1.0 };

    let mut root = nodes.get_mut(editor_window.root)?;

    root.border_radius = RoundedCorners::All.to_border_radius(border_radius);
    root.border = UiRect::all(px(border));

    Ok(())
}

/// We need exclusive system to access [winit windows](bevy::winit::WINIT_WINDOWS)
fn check_actually_maximized(
    world: &mut World,
    state: &mut SystemState<(Commands, Query<(Entity, &mut EditorWindow)>)>,
) -> Result {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let (mut commands, windows) = state.get_mut(world);

        for (window, mut editor_window) in windows {
            let Some(winit_window) = winit_windows.get_window(window) else {
                continue;
            };

            let is_actually_maximized = winit_window.is_maximized();

            if is_actually_maximized != editor_window.is_maximized {
                editor_window.bypass_change_detection().is_maximized = is_actually_maximized;

                commands
                    .run_system_cached_with(set_maximize_style, (window, is_actually_maximized));
            }
        }

        state.apply(world);
    });

    Ok(())
}

fn maximize_windows(
    editor_windows: Query<(Entity, &mut Window, &EditorWindow), Changed<EditorWindow>>,
    mut commands: Commands,
) -> Result {
    for (entity, mut window, editor_window) in editor_windows {
        window.set_maximized(editor_window.is_maximized);
        commands.run_system_cached_with(set_maximize_style, (entity, editor_window.is_maximized));
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Default, Reflect, Resource)]
pub struct EditorWindowAutoFocus(pub bool);

fn auto_focus(
    auto_focus: Res<EditorWindowAutoFocus>,
    mut window_events: MessageReader<WindowEvent>,
    mut windows: Query<&mut Window, With<EditorWindow>>,
) {
    if !auto_focus.0 {
        window_events.clear();
        return;
    }

    for window_event in window_events.read() {
        match window_event {
            WindowEvent::CursorEntered(entered) => {
                if let Ok(mut window) = windows.get_mut(entered.window) {
                    window.focused = true;
                };
            }
            _ => {}
        }
    }
}

// Dragging from one window to another results in a strange pointer target result.
// We need to redefine the location depending on where the window pointer is dragged.
fn override_pointer_drag_location(
    world: &mut World,
    state: &mut SystemState<(
        Query<&mut PointerLocation>,
        Query<(Entity, &Window), With<EditorWindow>>,
        Single<Entity, With<PrimaryWindow>>,
    )>,
) {
    let (pointers, editor_windows, primary_window) = state.get_mut(world);

    for mut pointer_location in pointers {
        let Some(location) = &pointer_location.location.clone() else {
            continue;
        };

        let NormalizedRenderTarget::Window(window_ref) = location.target else {
            continue;
        };

        let Ok((source, _)) = editor_windows.get(window_ref.entity()) else {
            continue;
        };

        let Some((destination, _)) = editor_windows.iter().find(|(_, window)| window.focused)
        else {
            continue;
        };

        if source == destination {
            continue;
        }

        WINIT_WINDOWS.with_borrow(|winit_windows| {
            let source_window = winit_windows.get_window(source).unwrap();
            let source_window_position = source_window.inner_position().unwrap_or_default();
            let source_absolute_position = Vec2 {
                x: source_window_position.x as f32 + location.position.x,
                y: source_window_position.y as f32 + location.position.y,
            };

            let destination_window = winit_windows.get_window(destination).unwrap();
            let destination_window_position =
                destination_window.inner_position().unwrap_or_default();
            let destination_absolute_position = Vec2 {
                x: source_absolute_position.x - destination_window_position.x as f32,
                y: source_absolute_position.y - destination_window_position.y as f32,
            };

            pointer_location.location = Some(Location {
                position: destination_absolute_position,
                target: RenderTarget::Window(WindowRef::Entity(destination))
                    .normalize(Some(*primary_window))
                    .unwrap(),
            });
        });
    }
}

pub struct EditorWindowPlugin;

impl Plugin for EditorWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                decorations: false,
                transparent: true,
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        })
        .insert_resource(ClearColor(Color::NONE))
        .init_resource::<EditorWindowAutoFocus>()
        .add_systems(First, (configure_windows, check_actually_maximized).chain())
        .add_systems(
            PreUpdate,
            override_pointer_drag_location
                .after(PickingSystems::ProcessInput)
                .before(PickingSystems::Backend),
        )
        .add_systems(Update, (maximize_windows, auto_focus));
    }
}
