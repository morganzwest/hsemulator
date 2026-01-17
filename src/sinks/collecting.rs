use crate::engine::events::ExecutionEvent;
use crate::engine::sink::EventSink;

/// An in-memory event sink used to collect execution events
/// during a single run.
///
/// This sink is intentionally simple and synchronous.
/// It is `Send` as long as `ExecutionEvent` is `Send`.
#[derive(Debug, Default)]
pub struct CollectingEventSink {
    events: Vec<ExecutionEvent>,
}

impl CollectingEventSink {
    /// Create a new, empty collecting sink.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    /// Borrow all collected events.
    #[allow(dead_code)]
    pub fn events(&self) -> &[ExecutionEvent] {
        &self.events
    }

    /// Consume the sink and return the collected events.
    #[allow(dead_code)]
    pub fn into_events(self) -> Vec<ExecutionEvent> {
        self.events
    }
}

impl EventSink for CollectingEventSink {
    fn emit(&mut self, event: ExecutionEvent) {
        self.events.push(event);
    }
}
