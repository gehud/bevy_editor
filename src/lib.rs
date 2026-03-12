use bevy::{
    color::palettes,
    feathers::{
        FeathersPlugins,
        controls::{
            ButtonProps, ButtonVariant, ColorChannel, ColorPlane, ColorPlaneValue, ColorSlider,
            ColorSliderProps, ColorSwatch, ColorSwatchValue, SliderBaseColor, SliderProps, button,
            checkbox, color_plane, color_slider, color_swatch, radio, slider, toggle_switch,
        },
        cursor::{EntityCursor, OverrideCursor},
        dark_theme::create_dark_theme,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemeBorderColor, ThemedText, UiTheme},
        tokens::*,
    },
    input_focus::tab_navigation::TabGroup,
    math::CompassOctant,
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{
        Activate, RadioButton, RadioGroup, SliderPrecision, SliderStep, SliderValue, ValueChange,
        checkbox_self_update, observe, slider_self_update,
    },
    window::{
        CompositeAlphaMode, CursorIcon, PresentMode, PrimaryWindow, RequestRedraw, SystemCursorIcon,
    },
    winit::WinitSettings,
};

pub const WINDOW_RESIZE_GRIP_SIZE: Val = Val::Px(5.0);

#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy".into(),
                decorations: false,
                transparent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FeathersPlugins)
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(ClearColor(Color::BLACK.with_alpha(0.0)))
        .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Root
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                border: UiRect::all(px(1)),
                border_radius: RoundedCorners::All.to_border_radius(8.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ThemeBorderColor(RADIO_BORDER),
            ThemeBackgroundColor(WINDOW_BG),
        ))
        .with_children(|commands| {
            // Header
            commands
                .spawn(Node {
                    width: percent(100),
                    height: px(34),
                    ..default()
                })
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_move();
                    },
                );

            // Body
            commands.spawn(Node {
                width: percent(100),
                height: percent(100),
                ..default()
            });

            // Footer
            commands.spawn(Node {
                width: percent(100),
                height: px(24),
                ..default()
            });

            // North resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    top: px(0),
                    width: percent(100),
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::RowResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::North);
                    },
                );

            // South resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    bottom: px(0),
                    width: percent(100),
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::RowResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::South);
                    },
                );

            // West resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: percent(100),
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::ColResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::West);
                    },
                );

            // East resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: percent(100),
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::ColResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::East);
                    },
                );

            // Northwest resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::NwResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::NorthWest);
                    },
                );

            // Northeast resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::NeResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::NorthEast);
                    },
                );

            // Southwest resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    bottom: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::SwResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::SouthWest);
                    },
                );

            // Southeast resize
            commands
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    bottom: px(0),
                    width: WINDOW_RESIZE_GRIP_SIZE,
                    height: WINDOW_RESIZE_GRIP_SIZE,
                    ..default()
                })
                .insert(EntityCursor::System(SystemCursorIcon::SeResize))
                .observe(
                    |_: On<Pointer<Press>>,
                     mut window: Single<&mut Window, With<PrimaryWindow>>| {
                        window.start_drag_resize(CompassOctant::SouthEast);
                    },
                );
        });
}
