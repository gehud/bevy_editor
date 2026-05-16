use bevy::{
    app::{App, Plugin},
    ecs::{
        entity::Entity,
        hierarchy::{ChildOf, Children},
        observer::On,
        query::With,
        system::{Commands, Single},
    },
    scene::{CommandsSceneExt, Scene, bsn},
    ui::{AlignItems, FlexDirection, JustifyContent, Node, UiRect, percent, px},
    window::PrimaryWindow,
};

use crate::{
    widget::text::{EditorText, EditorTextStyle},
    window::{EditorWindowStructure, PrimaryEditorWindowConfigured},
};

fn header() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: px(34),
            padding: UiRect::horizontal(px(12)),
            align_items: AlignItems::Center
        }
        Children [
            :EditorText { @text: "Bevy", @style: EditorTextStyle::Heading }
        ]
    }
}

fn body() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
        }
    }
}

fn footer() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: px(24),
            padding: UiRect::horizontal(px(8)),
            justify_content: JustifyContent::SpaceBetween,
        }
    }
}

pub fn layout(root: Entity) -> impl Scene {
    bsn! {
        ChildOf(root)
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
        }
        Children [
            header(),
            body(),
            footer(),
        ]
    }
}

fn setup(
    _: On<PrimaryEditorWindowConfigured>,
    primary_window: Single<&EditorWindowStructure, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let root = primary_window.root();
    commands.spawn_scene(layout(root));
}

pub struct EditorLayoutPlugin;

impl Plugin for EditorLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(setup);
    }
}
