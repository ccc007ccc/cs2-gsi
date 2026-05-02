//! Type-erased event dispatcher.
//!
//! Internally backed by a `HashMap<TypeId, Vec<Box<dyn Fn>>>` so we can
//! support an unbounded number of event types with a single registration
//! API. Handlers fire **synchronously** on the HTTP listener task — mirror
//! the upstream C# library's behaviour. If your handler does heavy work,
//! `tokio::spawn` from inside it.

use crate::events::GameEvent;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

type Handler = Box<dyn Fn(&dyn Any) + Send + Sync + 'static>;
type AnyHandler = Box<dyn Fn(&GameEvent) + Send + Sync + 'static>;

#[derive(Default)]
struct DispatcherInner {
    by_type: HashMap<TypeId, Vec<Handler>>,
    on_any: Vec<AnyHandler>,
}

/// Cheap-to-clone handle to a shared dispatcher.
#[derive(Clone, Default)]
pub(crate) struct Dispatcher {
    inner: Arc<RwLock<DispatcherInner>>,
}

impl Dispatcher {
    /// Register a handler for a specific event type `E`.
    ///
    /// Multiple handlers per type are supported and fire in registration
    /// order.
    pub fn register<E, F>(&self, f: F)
    where
        E: Any + Send + Sync + 'static,
        F: Fn(&E) + Send + Sync + 'static,
    {
        let boxed: Handler = Box::new(move |any: &dyn Any| {
            if let Some(e) = any.downcast_ref::<E>() {
                f(e);
            }
        });
        self.inner
            .write()
            .by_type
            .entry(TypeId::of::<E>())
            .or_default()
            .push(boxed);
    }

    /// Register a handler that receives **every** event as a [`GameEvent`].
    pub fn register_any<F>(&self, f: F)
    where
        F: Fn(&GameEvent) + Send + Sync + 'static,
    {
        self.inner.write().on_any.push(Box::new(f));
    }

    /// Fire an event. Calls every registered handler matching the type, then
    /// every catch-all handler.
    ///
    /// Handlers are invoked under a read lock — they MUST NOT register new
    /// handlers from inside a callback (would deadlock).
    pub fn fire<E>(&self, event: &E, as_game_event: GameEvent)
    where
        E: Any + Send + Sync + 'static,
    {
        let guard = self.inner.read();
        if let Some(list) = guard.by_type.get(&TypeId::of::<E>()) {
            for h in list {
                h(event);
            }
        }
        for h in &guard.on_any {
            h(&as_game_event);
        }
    }

    /// Returns the number of typed handlers registered for type `E`.
    #[allow(dead_code)] // used by tests; kept public-ish for potential future API
    pub fn count<E: Any + 'static>(&self) -> usize {
        self.inner
            .read()
            .by_type
            .get(&TypeId::of::<E>())
            .map(Vec::len)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{GameOver, MatchStarted};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn fires_only_matching_handlers() {
        let d = Dispatcher::default();
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();
        d.register::<MatchStarted, _>(move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });
        d.fire(&MatchStarted, GameEvent::MatchStarted(MatchStarted));
        d.fire(&GameOver, GameEvent::GameOver(GameOver));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn on_any_receives_everything() {
        let d = Dispatcher::default();
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();
        d.register_any(move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });
        d.fire(&MatchStarted, GameEvent::MatchStarted(MatchStarted));
        d.fire(&GameOver, GameEvent::GameOver(GameOver));
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
