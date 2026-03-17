use bevy::{
    app::{App, AppExit, First, Plugin, Update},
    asset::{AssetServer, Handle, embedded_asset},
    camera::{Camera2d, ClearColor, RenderTarget},
    color::Color,
    ecs::{
        change_detection::DetectChangesMut,
        component::Component,
        entity::Entity,
        error::Result,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        message::MessageWriter,
        observer::On,
        query::{Added, Changed, With},
        system::{Commands, EntityCommands, In, Query, Res, Single, SystemState},
        world::World,
    },
    image::Image,
    math::CompassOctant,
    picking::{
        Pickable,
        events::{Click, Out, Over, Pointer, Press},
    },
    ui::{
        AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, PositionType, UiRect,
        UiTargetCamera, Val, percent, px, widget::ImageNode,
    },
    utils::default,
    window::{PrimaryWindow, SystemCursorIcon, Window, WindowRef},
    winit::WINIT_WINDOWS,
};

use crate::{
    theme::{
        RoundedCorners, ThemeBackgroundColor, ThemeBorderColor, palette,
        tokens::{BORDER, WINDOW_BG},
    },
    widget::EntityCursor,
};

pub const WINDOW_RESIZE_GRIP_SIZE: f32 = 5.0;
pub const WINDOW_BORDER_RADIUS: f32 = 8.0;
pub const WINDOW_BUTTON_SIZE: f32 = 26.0;
pub const WINDOW_BUTTON_RADIUS: f32 = WINDOW_BUTTON_SIZE / 2.0;

pub struct EditorWindowPlugin;

impl Plugin for EditorWindowPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::NONE))
            .add_systems(First, check_actually_maximized)
            .add_systems(Update, configure_windows)
            .add_systems(Update, maximize_windows);

        embedded_asset!(app, "src/window", "assets/window/icons/close.png");
        embedded_asset!(app, "src/window", "assets/window/icons/maximize.png");
        embedded_asset!(app, "src/window", "assets/window/icons/minimize.png");
        embedded_asset!(app, "src/window", "assets/window/icons/restore.png");
    }
}

#[derive(Component)]
pub struct IsWindowMaximized(pub bool);

#[derive(Component)]
pub struct EditorWindow {
    root: Entity,
    titlebar: Entity,
    content: Entity,
    maximize: Entity,
}

#[derive(EntityEvent)]
pub struct PrimaryWindowConfigured {
    pub entity: Entity,
}

#[derive(EntityEvent)]
pub struct WindowConfigured {
    pub entity: Entity,
}

impl EditorWindow {
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

impl Default for IsWindowMaximized {
    fn default() -> Self {
        Self(false)
    }
}

#[derive(Component)]
struct EditorWindowRef(Entity);

fn configure_windows(
    windows: Query<Entity, Added<Window>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for window in windows {
        configure_window(window, &mut commands, &asset_server);
        commands.trigger(WindowConfigured { entity: window });
        if window == *primary_window {
            commands.trigger(PrimaryWindowConfigured { entity: window });
        }
    }
}

fn configure_window(window: Entity, commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .entity(window)
        .insert(IsWindowMaximized::default())
        .entry::<Window>()
        .and_modify(|mut window| {
            window.decorations = false;
        });

    let camera = commands
        .spawn((Camera2d, RenderTarget::Window(WindowRef::Entity(window))))
        .id();

    let mut titlebar = Entity::PLACEHOLDER;
    let mut content = Entity::PLACEHOLDER;
    let mut maximize = Entity::PLACEHOLDER;

    let root = commands
        .spawn((
            UiTargetCamera(camera),
            Node {
                width: percent(100),
                height: percent(100),
                border: UiRect::all(px(1)),
                border_radius: RoundedCorners::All.to_border_radius(WINDOW_BORDER_RADIUS),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ThemeBorderColor(BORDER),
            ThemeBackgroundColor(WINDOW_BG),
        ))
        .id();

    commands.entity(root).with_children(|commands| {
        commands
            .spawn((Node {
                width: percent(100),
                height: px(34),
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
                            let mut window = windows.get_mut(window_refs.get(trigger.entity)?.0)?;
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
                            asset_server
                                .load("embedded://bevy_editor/assets/window/icons/minimize.png"),
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
                            asset_server
                                .load("embedded://bevy_editor/assets/window/icons/maximize.png"),
                        )
                        .observe(
                            |trigger: On<Pointer<Click>>,
                             window_refs: Query<&EditorWindowRef>,
                             mut windows: Query<&mut IsWindowMaximized>|
                             -> Result {
                                let mut is_window_maximized =
                                    windows.get_mut(window_refs.get(trigger.entity)?.0)?;
                                is_window_maximized.0 = !is_window_maximized.0;
                                Ok(())
                            },
                        )
                        .id();

                        spawn_titlebar_button(
                            commands.target_entity(),
                            &mut commands.commands(),
                            window,
                            asset_server
                                .load("embedded://bevy_editor/assets/window/icons/close.png"),
                        )
                        .observe(
                            |_: On<Pointer<Click>>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            },
                        );
                    });
            });

        // User content
        content = commands
            .spawn(Node {
                width: percent(100),
                height: percent(100),
                ..default()
            })
            .id();

        // North resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                top: px(0),
                width: percent(100),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::RowResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::North);
                },
            );

        // South resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                width: percent(100),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::RowResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::South);
                },
            );

        // West resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: percent(100),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::ColResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::West);
                },
            );

        // East resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                right: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: percent(100),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::ColResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::East);
                },
            );

        // Northwest resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::NwResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::NorthWest);
                },
            );

        // Northeast resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::NeResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::NorthEast);
                },
            );

        // Southwest resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(0),
                bottom: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::SwResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::SouthWest);
                },
            );

        // Southeast resize
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                right: px(0),
                bottom: px(0),
                width: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                height: Val::Px(WINDOW_RESIZE_GRIP_SIZE),
                ..default()
            })
            .insert(EntityCursor::System(SystemCursorIcon::SeResize))
            .observe(
                |_: On<Pointer<Press>>, mut window: Single<&mut Window, With<PrimaryWindow>>| {
                    window.start_drag_resize(CompassOctant::SouthEast);
                },
            );
    });

    commands.entity(window).insert(EditorWindow {
        root,
        titlebar,
        content,
        maximize,
    });
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
    editor_windows: Query<&EditorWindow>,
    mut image_nodes: Query<&mut ImageNode>,
    mut nodes: Query<&mut Node>,
    asset_server: Res<AssetServer>,
) -> Result {
    let editor_window = editor_windows.get(window)?;
    let mut image = image_nodes.get_mut(children.get(editor_window.maximize)?[0])?;

    image.image = if is_maximized {
        asset_server.load("embedded://bevy_editor/assets/window/icons/restore.png")
    } else {
        asset_server.load("embedded://bevy_editor/assets/window/icons/maximize.png")
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
    state: &mut SystemState<(Commands, Query<(Entity, &mut IsWindowMaximized)>)>,
) -> Result {
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let (mut commands, windows) = state.get_mut(world);

        for (window, mut is_maximized) in windows {
            let Some(winit_window) = winit_windows.get_window(window) else {
                continue;
            };

            let is_actually_maximized = winit_window.is_maximized();
            if is_actually_maximized != is_maximized.0 {
                is_maximized.bypass_change_detection().0 = is_actually_maximized;
                commands
                    .run_system_cached_with(set_maximize_style, (window, is_actually_maximized));
            }
        }

        state.apply(world);
    });

    Ok(())
}

fn maximize_windows(
    windows: Query<(Entity, &mut Window, &IsWindowMaximized), Changed<IsWindowMaximized>>,
    mut commands: Commands,
) -> Result {
    for (entity, mut window, is_maximized) in windows {
        window.set_maximized(is_maximized.0);
        commands.run_system_cached_with(set_maximize_style, (entity, is_maximized.0));
    }

    Ok(())
}
