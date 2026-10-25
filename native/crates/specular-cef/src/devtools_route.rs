//! Telling a devtools client's traffic from the backend's own on the one
//! channel a page has.

use serde::Deserialize;
use specular_core::DEVTOOLS_CLIENT_ID_BASE;

/// Who a message from a page's devtools agent is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// The answer to a question the backend asked.
    Backend,
    /// A client's: the answer to a message it sent, or an event, which the
    /// backend never listens for.
    Client,
}

#[derive(Deserialize)]
struct Head {
    id: Option<i64>,
}

/// Routes `message`, one devtools-protocol JSON object, by its `id`.
/// Anything that is not such an object is the backend's to ignore.
pub(crate) fn route(message: &[u8]) -> Route {
    match serde_json::from_slice::<Head>(message) {
        Ok(Head { id: Some(id) }) if id < i64::from(DEVTOOLS_CLIENT_ID_BASE) => Route::Backend,
        Ok(_) => Route::Client,
        Err(_) => Route::Backend,
    }
}
