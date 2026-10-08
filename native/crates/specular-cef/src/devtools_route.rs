//! Telling a devtools client's traffic from the backend's own on the one
//! channel a page has.

use serde::Deserialize;
use specular_core::DEVTOOLS_CLIENT_ID_BASE;

/// Who a message from a page's devtools agent is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
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
pub fn route(message: &[u8]) -> Route {
    match serde_json::from_slice::<Head>(message) {
        Ok(Head { id: Some(id) }) if id < i64::from(DEVTOOLS_CLIENT_ID_BASE) => Route::Backend,
        Ok(_) => Route::Client,
        Err(_) => Route::Backend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_goes_to_whoever_owns_its_id() {
        assert_eq!(route(br#"{"id":3,"result":{}}"#), Route::Backend);
        let client = format!(r#"{{"id":{DEVTOOLS_CLIENT_ID_BASE},"result":{{"data":"x"}}}}"#);
        assert_eq!(route(client.as_bytes()), Route::Client);
    }

    #[test]
    fn an_event_is_the_clients_and_noise_is_nobodys() {
        let event = br#"{"method":"Page.loadEventFired","params":{"timestamp":1}}"#;
        assert_eq!(route(event), Route::Client);
        assert_eq!(route(b"not json"), Route::Backend);
        assert_eq!(route(b""), Route::Backend);
    }
}
