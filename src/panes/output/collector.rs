use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::{Arc, LazyLock, Mutex},
};

use bevy::log::{
    tracing::{
        Event, Level, Metadata, Subscriber,
        field::{Field, Visit},
    },
    tracing_subscriber::{Layer, layer::Context, registry::LookupSpan},
};
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct CollectedEvent {
    pub level: Level,
    pub fields: BTreeMap<String, String>,
    pub time: DateTime<Local>,
}

impl CollectedEvent {
    pub fn new(event: &Event, meta: &Metadata) -> Self {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldVisitor(&mut fields));

        CollectedEvent {
            level: meta.level().to_owned(),
            time: Local::now(),
            fields,
        }
    }
}

struct FieldVisitor<'a>(&'a mut BTreeMap<String, String>);

impl<'a> Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .insert(field.name().to_string(), format!("{:?}", value));
    }
}

#[derive(Debug, Clone)]
pub struct EventCollector {
    level: Level,
    events: Arc<Mutex<Vec<CollectedEvent>>>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_level(self, level: Level) -> Self {
        Self { level, ..self }
    }

    pub fn events(&self) -> Vec<CollectedEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        *events = Vec::new();
    }

    fn collect(&self, event: CollectedEvent) {
        if event.level <= self.level {
            self.events.lock().unwrap().push(event);
        }
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            level: Level::TRACE, // capture everything by default.
        }
    }
}

impl<S> Layer<S> for EventCollector
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        self.collect(CollectedEvent::new(event, meta));
    }
}

pub static COLLECTOR: LazyLock<EventCollector> = LazyLock::new(Default::default);
