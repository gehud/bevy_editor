use bevy::{
    app::{App, Last, Plugin},
    camera::{Camera, Camera2d, RenderTarget},
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EntityEvent, Event},
        query::{Added, With},
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Commands, Query},
    },
    ui::{BackgroundColor, Node, UiTargetCamera, percent},
    utils::default,
    window::{ExitCondition, PrimaryWindow, Window, WindowPlugin, WindowRef},
};

#[derive(EntityEvent)]
pub struct EditorWindowConfigured {
    #[event_target]
    pub window: Entity,
}

#[derive(Event)]
pub struct PrimaryEditorWindowConfigured;

#[derive(Component)]
pub struct EditorWindowStructure {
    root: Entity,
}

impl EditorWindowStructure {
    pub fn root(&self) -> Entity {
        self.root
    }
}

fn configure_windows(
    windows: Query<Entity, Added<Window>>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let primary_window = primary_windows.single().ok();

    for window in windows {
        let window_ref = primary_window
            .and_then(|primary_window| (primary_window == window).then_some(WindowRef::Primary))
            .unwrap_or(WindowRef::Entity(window));

        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    order: isize::MAX,
                    ..default()
                },
                RenderTarget::Window(window_ref),
            ))
            .id();

        let root = commands
            .spawn((
                UiTargetCamera(camera),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ))
            .id();

        commands
            .entity(window)
            .insert(EditorWindowStructure { root });

        commands.trigger(EditorWindowConfigured { window });

        if matches!(window_ref, WindowRef::Primary) {
            commands.trigger(PrimaryEditorWindowConfigured);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum EditorWindowSystems {
    Configure,
}

pub struct EditorWindowPlugin;

impl Plugin for EditorWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".into(),
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        })
        .configure_sets(Last, EditorWindowSystems::Configure)
        .add_systems(
            Last,
            configure_windows.in_set(EditorWindowSystems::Configure),
        );
    }
}
