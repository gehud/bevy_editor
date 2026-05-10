use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        error::Result,
        event::Event,
        message::{Message, MessageReader},
    },
};

#[derive(Message)]
pub struct Build;

fn build(mut requests: MessageReader<Build>) -> Result {
    if requests.is_empty() {
        return Ok(());
    }

    requests.clear();

    Ok(())
}

pub struct EditorBuildPlugin;

impl Plugin for EditorBuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Build>().add_systems(Update, build);
    }
}
