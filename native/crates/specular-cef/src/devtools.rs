//! Questions put to a page over CEF's in-process devtools channel
//! (`SendDevToolsMessage` and a message observer), and their answers turned
//! into [`PageEvent`]s.
//!
//! The channel needs no remote-debugging port and no socket: CEF routes the
//! message to the page's devtools agent and calls the observer back on the
//! UI thread. The messages themselves are built and read in
//! [`dom_query`](crate::dom_query).
//!
//! A devtools client's messages share the channel
//! ([`Devtools::send_raw`]). Its answers and every event the page raises go
//! to the [`DevtoolsSink`], told apart from the answers to the questions
//! above by [`devtools_route`](crate::devtools_route).
#![expect(
    clippy::transmute_ptr_to_ptr,
    reason = "cef's wrap_* macros transmute the ref-count base in their expansion"
)]

use std::collections::HashMap;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cef::rc::Rc as _;
use cef::{
    Browser, BrowserHost, DevToolsMessageObserver, ImplBrowserHost, ImplDevToolsMessageObserver,
    Registration, WrapDevToolsMessageObserver, wrap_dev_tools_message_observer,
};
use specular_core::{DevtoolsSink, PageEvent};

use crate::devtools_route::{Route, route};
use crate::dom_query;
use crate::page::PageContext;

/// What a message sent to a page asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asked {
    /// The element under a point, for this request.
    Element(u64),
    /// The elements in a rect, for this request.
    ElementsInRect(u64),
    /// The page's own devtools target id.
    Target,
}

#[derive(Debug, Default)]
struct Sent {
    next_id: c_int,
    asked: HashMap<c_int, Asked>,
}

/// The messages a page has been sent and not answered, by message id.
/// Shared with the observer, which CEF may drop on another thread.
#[derive(Debug, Clone, Default)]
struct Pending(Arc<Mutex<Sent>>);

impl Pending {
    /// A panic while the lock was held cannot leave the map half-written
    /// (every write is one insert or remove), so poisoning is ignored.
    fn lock(&self) -> MutexGuard<'_, Sent> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn ask(&self, asked: Asked) -> c_int {
        let mut sent = self.lock();
        sent.next_id = sent.next_id.wrapping_add(1).max(1);
        let id = sent.next_id;
        sent.asked.insert(id, asked);
        id
    }

    fn take(&self, id: c_int) -> Option<Asked> {
        self.lock().asked.remove(&id)
    }
}

/// Where client answers and page events go, shared by every page's
/// observer and set by the source.
#[derive(Clone, Default)]
pub(crate) struct SinkSlot(Arc<Mutex<Option<DevtoolsSink>>>);

impl SinkSlot {
    pub(crate) fn set(&self, sink: Option<DevtoolsSink>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = sink;
    }

    /// The sink, cloned out so it is not called with the lock held.
    fn get(&self) -> Option<DevtoolsSink> {
        (self.0.lock().unwrap_or_else(PoisonError::into_inner)).clone()
    }
}

wrap_dev_tools_message_observer! {
    struct PageObserver {
        ctx: PageContext,
        pending: Pending,
        sink: SinkSlot,
    }

    impl DevToolsMessageObserver {
        fn on_dev_tools_message(
            &self,
            _browser: Option<&mut Browser>,
            message: Option<&[u8]>,
        ) -> c_int {
            let message = message.unwrap_or_default();
            if route(message) == Route::Backend {
                // Not handled: CEF goes on to `on_dev_tools_method_result`.
                return 0;
            }
            if let Some(sink) = self.sink.get() {
                sink(self.ctx.id, &String::from_utf8_lossy(message));
            }
            1
        }

        fn on_dev_tools_method_result(
            &self,
            _browser: Option<&mut Browser>,
            message_id: c_int,
            success: c_int,
            result: Option<&[u8]>,
        ) {
            let Some(asked) = self.pending.take(message_id) else {
                return;
            };
            // A failed method still answers: no element, nothing grabbed.
            let result = result.filter(|_| success != 0).unwrap_or_default();
            let page = self.ctx.id;
            let event = match asked {
                Asked::Element(request) => PageEvent::ElementAt {
                    page,
                    request,
                    element: dom_query::parse_element(result),
                },
                Asked::ElementsInRect(request) => PageEvent::ElementsInRect {
                    page,
                    request,
                    count: dom_query::parse_count(result),
                },
                Asked::Target => {
                    let Some(id) = dom_query::parse_target_id(result) else {
                        tracing::warn!(%page, "page did not name its devtools target");
                        return;
                    };
                    PageEvent::DevtoolsTarget { page, id }
                }
            };
            self.ctx.push(event);
        }
    }
}

/// One page's devtools channel. Dropping it stops the observer.
pub(crate) struct Devtools {
    pending: Pending,
    /// Keeps the observer registered for as long as the page is hosted.
    _registration: Option<Registration>,
}

impl Devtools {
    /// Starts observing `host`'s devtools answers for the page `ctx` names.
    pub(crate) fn attach(host: &BrowserHost, ctx: &PageContext, sink: &SinkSlot) -> Self {
        let pending = Pending::default();
        let mut observer: DevToolsMessageObserver =
            PageObserver::new(ctx.clone(), pending.clone(), sink.clone());
        let registration = host.add_dev_tools_message_observer(Some(&mut observer));
        if registration.is_none() {
            tracing::warn!(page = %ctx.id, "no DevTools observer; the page will not answer questions");
        }
        Self {
            pending,
            _registration: registration,
        }
    }

    /// Sends the message `build` makes for a fresh message id. Returns
    /// whether CEF took it; a refused message will never be answered.
    pub(crate) fn send(
        &self,
        host: &BrowserHost,
        asked: Asked,
        build: impl FnOnce(c_int) -> Vec<u8>,
    ) -> bool {
        let id = self.pending.ask(asked);
        let sent = host.send_dev_tools_message(Some(&build(id))) != 0;
        if !sent {
            self.pending.take(id);
        }
        sent
    }

    /// Sends a client's message as it is. Its id is the client's to keep
    /// out of the range the questions above use. Returns whether CEF took
    /// it.
    pub(crate) fn send_raw(host: &BrowserHost, message: &str) -> bool {
        host.send_dev_tools_message(Some(message.as_bytes())) != 0
    }
}
