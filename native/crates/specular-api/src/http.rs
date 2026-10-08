//! [`Request`] and [`Response`]: an HTTP exchange with the transport taken
//! off, so the routes can be run with no socket.

use serde_json::{Value, json};

/// The HTTP methods the API answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `DELETE`.
    Delete,
}

impl Method {
    /// The method named `name`, in upper case as HTTP writes it.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }

    /// The method as HTTP writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Delete => "DELETE",
        }
    }
}

/// One request to the API.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The method.
    pub method: Method,
    /// The path, without the query string.
    pub path: String,
    /// The query string's pairs, decoded.
    pub query: Vec<(String, String)>,
    /// The JSON body. An empty body is `{}`.
    pub body: Value,
    /// The canvas the caller named with `--tab`, decoded from the
    /// `x-specular-tab` header.
    pub tab: Option<String>,
}

impl Request {
    /// A request for `target`, the path and query string as the request
    /// line carries them.
    pub fn new(method: Method, target: &str, body: Value) -> Self {
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let query = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                (percent_decode(key), percent_decode(value))
            })
            .collect();
        Self {
            method,
            path: path.to_owned(),
            query,
            body,
            tab: None,
        }
    }

    /// A `GET` of `target`.
    pub fn get(target: &str) -> Self {
        Self::new(Method::Get, target, json!({}))
    }

    /// A `POST` of `body` to `target`.
    pub fn post(target: &str, body: Value) -> Self {
        Self::new(Method::Post, target, body)
    }

    /// The first value the query string gives `key`.
    pub fn query(&self, key: &str) -> Option<&str> {
        (self.query.iter())
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// The body's string field `key`.
    pub(crate) fn text(&self, key: &str) -> Option<&str> {
        self.body.get(key).and_then(Value::as_str)
    }

    /// The body's field `key` as a list of strings. `None` when it is
    /// missing or holds anything else.
    pub(crate) fn strings(&self, key: &str) -> Option<Vec<&str>> {
        strings(self.body.get(key)?)
    }
}

/// `value` as a list of strings, or `None` when it is anything else.
pub(crate) fn strings(value: &Value) -> Option<Vec<&str>> {
    value.as_array()?.iter().map(Value::as_str).collect()
}

/// Decodes `%XX` escapes and `+` as a URL's query string writes them. A
/// malformed escape is kept as it is.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let escaped = (byte == b'%')
            .then(|| bytes.get(at + 1..at + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(decoded) = escaped {
            out.push(decoded);
            at += 3;
        } else {
            out.push(if byte == b'+' { b' ' } else { byte });
            at += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The API's answer: a status and a JSON body.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The HTTP status.
    pub status: u16,
    /// The JSON body.
    pub body: Value,
}

impl Response {
    /// A `200` with `body`, its numbers written as a `.canvas` file writes
    /// them, so 500 is not sent as 500.0.
    pub fn ok(mut body: Value) -> Self {
        specular_doc::tidy_json(&mut body);
        Self { status: 200, body }
    }

    /// An error: `{"error": message}`, the shape the CLI prints.
    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: json!({ "error": message.into() }),
        }
    }

    /// A `400`.
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::error(400, message)
    }

    /// A `404`.
    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::error(404, message)
    }

    /// Whether the status is a success.
    pub const fn is_ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_splits_into_its_path_and_decoded_query() {
        let request = Request::get("/annotations?status=all&url=http%3A%2F%2Fa.test%2Fx+y");
        assert_eq!(request.path, "/annotations");
        assert_eq!(request.query("status"), Some("all"));
        assert_eq!(request.query("url"), Some("http://a.test/x y"));
        assert_eq!(request.query("page_id"), None);
    }

    #[test]
    fn a_malformed_escape_is_kept() {
        assert_eq!(percent_decode("50%"), "50%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("caf%C3%A9"), "café");
    }
}
