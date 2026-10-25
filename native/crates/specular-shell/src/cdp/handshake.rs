//! The HTTP/1.1 upgrade that opens a WebSocket: parsing the request,
//! answering it, and refusing what is not one.

use std::io::{self, Read};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// The most a request head may be before the server gives up on it.
pub(super) const MAX_HEAD: usize = 16 * 1024;

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// What an upgrade request asked for.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Upgrade {
    /// The request target, query string included.
    pub(super) path: String,
    /// The `Sec-WebSocket-Key` header.
    pub(super) key: String,
}

/// Reads up to and including the blank line that ends an HTTP head. It reads
/// a byte at a time so nothing after the head is consumed.
pub(super) fn read_head(from: &mut impl Read) -> io::Result<String> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= MAX_HEAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "head too large"));
        }
        from.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    String::from_utf8(head)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "head is not UTF-8"))
}

/// Parses a request head. `Err` is the status to refuse with.
pub(super) fn parse_request(head: &str) -> Result<Upgrade, u16> {
    let mut lines = head.split("\r\n");
    let mut request = lines.next().unwrap_or_default().split(' ');
    let (Some("GET"), Some(path), Some(version)) = (request.next(), request.next(), request.next())
    else {
        return Err(400);
    };
    if !version.starts_with("HTTP/1.") {
        return Err(400);
    }
    let key = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("sec-websocket-key"))
        .map(|(_, value)| value.trim())
        .filter(|key| !key.is_empty())
        .ok_or(400u16)?;
    Ok(Upgrade {
        path: path.to_owned(),
        key: key.to_owned(),
    })
}

/// The `Sec-WebSocket-Accept` value for a client's key.
pub(super) fn accept_key(key: &str) -> String {
    let mut sha = sha1_smol::Sha1::new();
    sha.update(key.as_bytes());
    sha.update(GUID.as_bytes());
    STANDARD.encode(sha.digest().bytes())
}

/// The `101 Switching Protocols` answer.
pub(super) fn switching_protocols(key: &str) -> String {
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key(key)
    )
}

/// A bodiless refusal. Only 400 and 404 are used.
pub(super) fn refusal(status: u16) -> String {
    let reason = if status == 404 {
        "Not Found"
    } else {
        "Bad Request"
    };
    format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rfc_example_key() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }
}
