//! A minimal blocking WebSocket client, for tests of the server.

use std::io::{self, BufReader, Write as _};
use std::net::TcpStream;
use std::time::Duration;

use super::frame::{Decoder, Incoming, Opcode, Role, encode};
use super::handshake::{self, accept_key};

const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const MASK: [u8; 4] = [0x37, 0xFA, 0x21, 0x3D];
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// One connection to a [`WsServer`](super::socket::WsServer).
#[derive(Debug)]
pub(crate) struct WsClient {
    from: BufReader<TcpStream>,
    to: TcpStream,
    decoder: Decoder,
}

impl WsClient {
    /// Connects and upgrades at `path`; an error for any status but 101.
    pub(crate) fn connect(port: u16, path: &str) -> io::Result<Self> {
        let mut to = TcpStream::connect(("127.0.0.1", port))?;
        to.set_nodelay(true)?;
        to.set_read_timeout(Some(READ_TIMEOUT))?;
        write!(
            to,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {KEY}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )?;
        let mut from = BufReader::new(to.try_clone()?);
        let head = handshake::read_head(&mut from)?;
        let status = head.lines().next().unwrap_or_default();
        if !status.contains(" 101 ") {
            return Err(io::Error::other(format!("upgrade refused: {status}")));
        }
        let expected = format!("Sec-WebSocket-Accept: {}", accept_key(KEY));
        if !head
            .lines()
            .any(|line| line.eq_ignore_ascii_case(&expected))
        {
            return Err(io::Error::other("wrong Sec-WebSocket-Accept"));
        }
        Ok(Self {
            from,
            to,
            decoder: Decoder::new(Role::Client),
        })
    }

    /// Sends one text message.
    pub(crate) fn send(&mut self, text: &str) -> io::Result<()> {
        self.to
            .write_all(&encode(Opcode::Text, text.as_bytes(), Some(MASK)))
    }

    /// The next text message, answering pings on the way.
    pub(crate) fn recv(&mut self) -> io::Result<String> {
        loop {
            match self.decoder.read(&mut self.from)? {
                Incoming::Data(bytes) => return Ok(String::from_utf8_lossy(&bytes).into_owned()),
                Incoming::Ping(payload) => {
                    self.to
                        .write_all(&encode(Opcode::Pong, &payload, Some(MASK)))?;
                }
                Incoming::Pong(_) => {}
                Incoming::Close => return Err(io::ErrorKind::ConnectionAborted.into()),
            }
        }
    }

    /// Sends a close frame and keeps the connection open for the reply.
    pub(crate) fn send_close(&mut self) -> io::Result<()> {
        self.to.write_all(&encode(Opcode::Close, &[], Some(MASK)))
    }
}
