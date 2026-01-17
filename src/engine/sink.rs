use crate::engine::events::ExecutionEvent;

pub trait EventSink: Send {
    fn emit(&mut self, event: ExecutionEvent);
}
