mod archive;
mod layer;

pub use crate::{archive::*, layer::*};

use crossbeam_queue::SegQueue;
use parking_lot::Mutex;
use std::sync::Arc;

static EVENT_LOG: Mutex<Vec<Arc<Event>>> = parking_lot::const_mutex(Vec::new());
static EVENT_QUEUE: SegQueue<Arc<Event>> = SegQueue::new();

/// Run some callback with the recorded events.
///
/// This is not reentrancy safe, and reentrant use will deadlock.
///
/// Will _not_ block the recording of new events.
pub fn with_events<R>(cb: impl FnOnce(&mut Vec<Arc<Event>>) -> R) -> R {
    let mut events = EVENT_LOG.lock();
    events.reserve(EVENT_QUEUE.len());
    events.extend(std::iter::from_fn(|| EVENT_QUEUE.pop()));
    cb(&mut events)
}

/// A new [recording layer](Layer) that can be [composed](mod@tracing_subscriber::layer) with other layers.
///
/// Shorthand for the equivalent [`Layer::default`].
pub fn layer<S>() -> Layer<S> {
    Layer::default()
}
