//! Putting the page bridge in a hosted page and taking it out, and running
//! the app's answers in it, over the page's own devtools channel. The
//! messages are [`bridge_query`]'s; the calls come back through the
//! devtools observer as `PageEvent::BridgeCall`.

use specular_core::{PageId, PageSourceError};

use crate::bridge_query;
use crate::devtools::Asked;
use crate::dom_query::method_message;
use crate::source::{CefPageSource, refused};

/// What of the bridge a page has been given.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BridgeState {
    /// Whether the binding is in, so its calls are reported.
    bound: bool,
    /// Whether the install script runs in each new document. It stays for
    /// the page's life, and does nothing where the binding is out.
    for_new_documents: bool,
}

impl CefPageSource {
    pub(crate) fn put_bridge(
        &mut self,
        page: PageId,
        script: Option<&str>,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry_mut(page)?;
        let state = entry.bridge;
        let methods = match script {
            Some(script) => {
                bridge_query::attach_methods(script, !state.bound, !state.for_new_documents)
            }
            None if state.bound => vec![bridge_query::detach_method()],
            None => return Ok(()),
        };
        for (method, params) in methods {
            let sent = (entry.devtools).send(&entry.host, Asked::Done, |id| {
                method_message(id, method, &params)
            });
            if !sent {
                return Err(refused("page bridge"));
            }
        }
        entry.bridge = BridgeState {
            bound: script.is_some(),
            for_new_documents: state.for_new_documents || script.is_some(),
        };
        Ok(())
    }

    pub(crate) fn run_in_page(&self, page: PageId, script: &str) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        let params = bridge_query::evaluate_params(script);
        (entry.devtools)
            .send(&entry.host, Asked::Done, |id| {
                method_message(id, "Runtime.evaluate", &params)
            })
            .then_some(())
            .ok_or_else(|| refused("bridge answer"))
    }
}
