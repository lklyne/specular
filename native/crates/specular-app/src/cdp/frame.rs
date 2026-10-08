//! RFC 6455 framing: encoding a frame and decoding messages from a stream.

use std::io::{self, Read};

/// The largest message, after reassembly, a peer may send.
pub(super) const MAX_MESSAGE: usize = 256 * 1024 * 1024;

/// A frame's opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Opcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

impl Opcode {
    fn bits(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xA,
        }
    }

    fn from_bits(bits: u8) -> Option<Self> {
        Some(match bits {
            0x0 => Self::Continuation,
            0x1 => Self::Text,
            0x2 => Self::Binary,
            0x8 => Self::Close,
            0x9 => Self::Ping,
            0xA => Self::Pong,
            _ => return None,
        })
    }

    fn is_control(self) -> bool {
        self.bits() >= 0x8
    }
}

/// Which end of the connection is decoding. A server only accepts masked
/// frames; a client only receives unmasked ones from a conforming server but
/// is lenient about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Server,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "only the test client reads as one")
    )]
    Client,
}

/// What the peer sent, with fragments already joined.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Incoming {
    /// A whole text or binary message.
    Data(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

/// One whole frame. `mask` is set for frames a client sends.
pub(super) fn encode(opcode: Opcode, payload: &[u8], mask: Option<[u8; 4]>) -> Vec<u8> {
    encode_fragment(true, opcode, payload, mask)
}

/// One frame of a possibly fragmented message; `fin` marks the last.
pub(super) fn encode_fragment(
    fin: bool,
    opcode: Opcode,
    payload: &[u8],
    mask: Option<[u8; 4]>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 14);
    out.push(opcode.bits() | if fin { 0x80 } else { 0 });
    let mask_bit = if mask.is_some() { 0x80 } else { 0 };
    let len = payload.len();
    if len < 126 {
        out.push(mask_bit | len as u8);
    } else if let Ok(len) = u16::try_from(len) {
        out.push(mask_bit | 0x7E);
        out.extend_from_slice(&len.to_be_bytes());
    } else {
        out.push(mask_bit | 0x7F);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
    match mask {
        Some(key) => {
            out.extend_from_slice(&key);
            out.extend(payload.iter().enumerate().map(|(i, b)| b ^ key[i % 4]));
        }
        None => out.extend_from_slice(payload),
    }
    out
}

fn invalid(why: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// Reads messages off one connection. It keeps the fragments of a message in
/// progress, so control frames may arrive between them.
#[derive(Debug)]
pub(super) struct Decoder {
    role: Role,
    partial: Vec<u8>,
    in_message: bool,
}

impl Decoder {
    pub(super) fn new(role: Role) -> Self {
        Self {
            role,
            partial: Vec::new(),
            in_message: false,
        }
    }

    /// Blocks until the next message or control frame is complete.
    pub(super) fn read(&mut self, from: &mut impl Read) -> io::Result<Incoming> {
        loop {
            let mut head = [0u8; 2];
            from.read_exact(&mut head)?;
            let fin = head[0] & 0x80 != 0;
            if head[0] & 0x70 != 0 {
                return Err(invalid("reserved bits set"));
            }
            let opcode = Opcode::from_bits(head[0] & 0x0F).ok_or_else(|| invalid("bad opcode"))?;
            let masked = head[1] & 0x80 != 0;
            if self.role == Role::Server && !masked {
                return Err(invalid("unmasked client frame"));
            }
            let len = match head[1] & 0x7F {
                0x7E => {
                    let mut b = [0u8; 2];
                    from.read_exact(&mut b)?;
                    u64::from(u16::from_be_bytes(b))
                }
                0x7F => {
                    let mut b = [0u8; 8];
                    from.read_exact(&mut b)?;
                    u64::from_be_bytes(b)
                }
                n => u64::from(n),
            };
            if opcode.is_control() && (!fin || len > 125) {
                return Err(invalid("bad control frame"));
            }
            let held = self.partial.len() as u64;
            if len > MAX_MESSAGE as u64 || held + len > MAX_MESSAGE as u64 {
                return Err(invalid("message too large"));
            }
            let mask = if masked {
                let mut key = [0u8; 4];
                from.read_exact(&mut key)?;
                Some(key)
            } else {
                None
            };
            let mut payload = Vec::new();
            let got = from.take(len).read_to_end(&mut payload)?;
            if got as u64 != len {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            if let Some(key) = mask {
                for (i, b) in payload.iter_mut().enumerate() {
                    *b ^= key[i % 4];
                }
            }
            match opcode {
                Opcode::Ping => return Ok(Incoming::Ping(payload)),
                Opcode::Pong => return Ok(Incoming::Pong(payload)),
                Opcode::Close => return Ok(Incoming::Close),
                Opcode::Text | Opcode::Binary | Opcode::Continuation => {
                    let first = opcode != Opcode::Continuation;
                    if first == self.in_message {
                        return Err(invalid("fragment out of order"));
                    }
                    self.partial.extend_from_slice(&payload);
                    self.in_message = !fin;
                    if fin {
                        return Ok(Incoming::Data(std::mem::take(&mut self.partial)));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const KEY: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

    fn decode_one(role: Role, bytes: Vec<u8>) -> io::Result<Incoming> {
        Decoder::new(role).read(&mut Cursor::new(bytes))
    }

    #[test]
    fn round_trips_at_every_length_class() {
        for len in [0usize, 125, 126, 65_535, 65_536] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let plain = encode(Opcode::Binary, &payload, None);
            assert_eq!(
                decode_one(Role::Client, plain).unwrap(),
                Incoming::Data(payload.clone()),
                "unmasked {len}"
            );
            let masked = encode(Opcode::Binary, &payload, Some(KEY));
            assert_eq!(
                decode_one(Role::Server, masked).unwrap(),
                Incoming::Data(payload),
                "masked {len}"
            );
        }
    }

    #[test]
    fn length_prefix_widths() {
        // The marker byte, then the bytes the length takes before the payload.
        for (len, marker, prefix) in [
            (125, 125, 0),
            (126, 126, 2),
            (65_535, 126, 2),
            (65_536, 127, 8),
        ] {
            let wire = encode(Opcode::Text, &vec![0; len], None);
            assert_eq!(wire[1], marker, "{len}");
            assert_eq!(wire.len(), 2 + prefix + len, "{len}");
        }
    }

    #[test]
    fn masking_scrambles_the_wire_bytes() {
        let wire = encode(Opcode::Text, b"hello", Some(KEY));
        assert_eq!(&wire[2..6], &KEY);
        let expected: Vec<u8> = b"hello"
            .iter()
            .zip(KEY.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect();
        assert_eq!(&wire[6..], expected.as_slice());
    }

    #[test]
    fn fragments_are_reassembled_around_a_ping() {
        let mut wire = encode_fragment(false, Opcode::Text, b"hel", Some(KEY));
        wire.extend(encode(Opcode::Ping, b"p", Some(KEY)));
        wire.extend(encode_fragment(
            false,
            Opcode::Continuation,
            b"l",
            Some(KEY),
        ));
        wire.extend(encode_fragment(true, Opcode::Continuation, b"o", Some(KEY)));
        let mut from = Cursor::new(wire);
        let mut decoder = Decoder::new(Role::Server);
        assert_eq!(
            decoder.read(&mut from).unwrap(),
            Incoming::Ping(b"p".to_vec())
        );
        assert_eq!(
            decoder.read(&mut from).unwrap(),
            Incoming::Data(b"hello".to_vec())
        );
    }

    #[test]
    fn control_frames_surface() {
        let ping = encode(Opcode::Ping, b"x", Some(KEY));
        assert_eq!(
            decode_one(Role::Server, ping).unwrap(),
            Incoming::Ping(b"x".to_vec())
        );
        let pong = encode(Opcode::Pong, b"x", None);
        assert_eq!(
            decode_one(Role::Client, pong).unwrap(),
            Incoming::Pong(b"x".to_vec())
        );
        let close = encode(Opcode::Close, &[], Some(KEY));
        assert_eq!(decode_one(Role::Server, close).unwrap(), Incoming::Close);
    }

    #[test]
    fn a_server_refuses_unmasked_frames() {
        let wire = encode(Opcode::Text, b"hi", None);
        let err = decode_one(Role::Server, wire).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn an_oversize_message_is_refused_before_it_is_read() {
        let mut wire = vec![0x82, 0xFF];
        wire.extend_from_slice(&(MAX_MESSAGE as u64 + 1).to_be_bytes());
        wire.extend_from_slice(&KEY);
        let err = decode_one(Role::Server, wire).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_continuation_with_nothing_to_continue_is_refused() {
        let wire = encode(Opcode::Continuation, b"x", Some(KEY));
        assert!(decode_one(Role::Server, wire).is_err());
    }
}
