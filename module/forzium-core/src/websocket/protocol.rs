//! # WEBSOCKET PROTOCOL IMPLEMENTATION
//!
//! **RFC 6455 COMPLIANT PROTOCOL WITH ZERO-COPY OPTIMIZATIONS**
//!
//! This module implements the WebSocket protocol with focus on performance
//! and memory efficiency through zero-copy message passing.

use bytes::{Bytes, BytesMut};
use std::fmt;

/// **WEBSOCKET OPCODE**
///
/// As defined in RFC 6455 Section 5.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// Continuation frame
    Continue = 0x0,
    /// Text frame (UTF-8)
    Text = 0x1,
    /// Binary frame
    Binary = 0x2,
    /// Connection close
    Close = 0x8,
    /// Ping
    Ping = 0x9,
    /// Pong
    Pong = 0xA,
}

impl OpCode {
    /// Check if this is a control frame
    pub fn is_control(self) -> bool {
        matches!(self, OpCode::Close | OpCode::Ping | OpCode::Pong)
    }

    /// Check if this is a data frame
    pub fn is_data(self) -> bool {
        matches!(self, OpCode::Continue | OpCode::Text | OpCode::Binary)
    }
}

/// **WEBSOCKET CLOSE CODE**
///
/// As defined in RFC 6455 Section 7.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CloseCode {
    /// Normal closure
    Normal = 1000,
    /// Going away
    Away = 1001,
    /// Protocol error
    Protocol = 1002,
    /// Unsupported data
    Unsupported = 1003,
    /// Invalid frame payload data
    Invalid = 1007,
    /// Policy violation
    Policy = 1008,
    /// Message too big
    Size = 1009,
    /// Missing extension
    Extension = 1010,
    /// Internal error
    Error = 1011,
    /// Service restart
    Restart = 1012,
    /// Try again later
    Again = 1013,
}

impl CloseCode {
    /// Convert to u16
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Create from u16
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            1000 => Some(CloseCode::Normal),
            1001 => Some(CloseCode::Away),
            1002 => Some(CloseCode::Protocol),
            1003 => Some(CloseCode::Unsupported),
            1007 => Some(CloseCode::Invalid),
            1008 => Some(CloseCode::Policy),
            1009 => Some(CloseCode::Size),
            1010 => Some(CloseCode::Extension),
            1011 => Some(CloseCode::Error),
            1012 => Some(CloseCode::Restart),
            1013 => Some(CloseCode::Again),
            _ => None,
        }
    }
}

/// **WEBSOCKET MESSAGE**
///
/// **ZERO-COPY DESIGN**: Uses Bytes for efficient memory sharing
#[derive(Debug, Clone)]
pub enum Message {
    /// Text message (UTF-8)
    Text(String),
    /// Binary message
    Binary(Bytes),
    /// Ping message
    Ping(Bytes),
    /// Pong message
    Pong(Bytes),
    /// Close message
    Close(Option<(CloseCode, String)>),
}

impl Message {
    /// Create a text message
    pub fn text(text: impl Into<String>) -> Self {
        Message::Text(text.into())
    }

    /// Create a binary message
    pub fn binary(data: impl Into<Bytes>) -> Self {
        Message::Binary(data.into())
    }

    /// Create a ping message
    pub fn ping(data: impl Into<Bytes>) -> Self {
        Message::Ping(data.into())
    }

    /// Create a pong message
    pub fn pong(data: impl Into<Bytes>) -> Self {
        Message::Pong(data.into())
    }

    /// Create a close message
    pub fn close(code: CloseCode, reason: impl Into<String>) -> Self {
        Message::Close(Some((code, reason.into())))
    }

    /// Get the OpCode for this message
    pub fn opcode(&self) -> OpCode {
        match self {
            Message::Text(_) => OpCode::Text,
            Message::Binary(_) => OpCode::Binary,
            Message::Ping(_) => OpCode::Ping,
            Message::Pong(_) => OpCode::Pong,
            Message::Close(_) => OpCode::Close,
        }
    }

    /// Get the payload bytes
    pub fn payload(&self) -> Bytes {
        match self {
            Message::Text(text) => Bytes::from(text.as_bytes().to_vec()),
            Message::Binary(data) => data.clone(),
            Message::Ping(data) => data.clone(),
            Message::Pong(data) => data.clone(),
            Message::Close(None) => Bytes::new(),
            Message::Close(Some((code, reason))) => {
                let mut bytes = BytesMut::with_capacity(2 + reason.len());
                bytes.extend_from_slice(&code.as_u16().to_be_bytes());
                bytes.extend_from_slice(reason.as_bytes());
                bytes.freeze()
            }
        }
    }

    /// Get the payload length
    pub fn len(&self) -> usize {
        match self {
            Message::Text(text) => text.len(),
            Message::Binary(data) => data.len(),
            Message::Ping(data) => data.len(),
            Message::Pong(data) => data.len(),
            Message::Close(None) => 0,
            Message::Close(Some((_, reason))) => 2 + reason.len(),
        }
    }

    /// Check if message is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if this is a control message
    pub fn is_control(&self) -> bool {
        self.opcode().is_control()
    }

    /// Check if this is a data message
    pub fn is_data(&self) -> bool {
        self.opcode().is_data()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Text(text) => write!(f, "Text({})", text),
            Message::Binary(data) => write!(f, "Binary({} bytes)", data.len()),
            Message::Ping(data) => write!(f, "Ping({} bytes)", data.len()),
            Message::Pong(data) => write!(f, "Pong({} bytes)", data.len()),
            Message::Close(None) => write!(f, "Close"),
            Message::Close(Some((code, reason))) => write!(f, "Close({:?}, {})", code, reason),
        }
    }
}

/// **FRAME HEADER**
///
/// WebSocket frame header structure
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Final frame flag
    pub fin: bool,
    /// RSV1 flag (compression)
    pub rsv1: bool,
    /// RSV2 flag (reserved)
    pub rsv2: bool,
    /// RSV3 flag (reserved)
    pub rsv3: bool,
    /// Operation code
    pub opcode: OpCode,
    /// Mask flag
    pub masked: bool,
    /// Payload length
    pub payload_len: u64,
    /// Masking key
    pub mask_key: Option<[u8; 4]>,
}

impl FrameHeader {
    /// Create a new frame header
    pub fn new(opcode: OpCode, payload_len: usize, masked: bool) -> Self {
        Self {
            fin: true,
            rsv1: false,
            rsv2: false,
            rsv3: false,
            opcode,
            masked,
            payload_len: payload_len as u64,
            mask_key: if masked { Some(rand::random()) } else { None },
        }
    }

    /// Get header size in bytes
    pub fn size(&self) -> usize {
        let mut size = 2; // Base header

        if self.payload_len > 65535 {
            size += 8;
        } else if self.payload_len > 125 {
            size += 2;
        }

        if self.masked {
            size += 4;
        }

        size
    }

    /// Encode header to bytes
    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(self.size());

        // First byte: FIN, RSV, OpCode
        let mut byte1 = self.opcode as u8;
        if self.fin {
            byte1 |= 0x80;
        }
        if self.rsv1 {
            byte1 |= 0x40;
        }
        if self.rsv2 {
            byte1 |= 0x20;
        }
        if self.rsv3 {
            byte1 |= 0x10;
        }
        buf.extend_from_slice(&[byte1]);

        // Second byte: MASK, Payload length
        let mut byte2 = 0u8;
        if self.masked {
            byte2 |= 0x80;
        }

        if self.payload_len > 65535 {
            byte2 |= 127;
            buf.extend_from_slice(&[byte2]);
            buf.extend_from_slice(&self.payload_len.to_be_bytes());
        } else if self.payload_len > 125 {
            byte2 |= 126;
            buf.extend_from_slice(&[byte2]);
            buf.extend_from_slice(&(self.payload_len as u16).to_be_bytes());
        } else {
            byte2 |= self.payload_len as u8;
            buf.extend_from_slice(&[byte2]);
        }

        // Masking key
        if let Some(mask_key) = self.mask_key {
            buf.extend_from_slice(&mask_key);
        }

        buf
    }
}

/// **FRAME**
///
/// Complete WebSocket frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame header
    pub header: FrameHeader,
    /// Frame payload
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame
    pub fn new(header: FrameHeader, payload: Bytes) -> Self {
        Self { header, payload }
    }

    /// Create from a message
    pub fn from_message(message: &Message) -> Self {
        let payload = message.payload();
        let header = FrameHeader::new(message.opcode(), payload.len(), false);
        Self::new(header, payload)
    }

    /// Apply masking to payload
    pub fn apply_mask(&mut self) {
        if let Some(mask_key) = self.header.mask_key {
            let payload = self.payload.to_vec();
            let mut masked = Vec::with_capacity(payload.len());

            for (i, byte) in payload.iter().enumerate() {
                masked.push(byte ^ mask_key[i % 4]);
            }

            self.payload = Bytes::from(masked);
        }
    }

    /// Remove masking from payload
    pub fn remove_mask(&mut self) {
        self.apply_mask(); // XOR is its own inverse
    }
}

/// **PROTOCOL EXTENSIONS**
#[derive(Debug, Clone)]
pub struct Extensions {
    /// Per-message compression
    pub permessage_deflate: bool,
    /// Client max window bits
    pub client_max_window_bits: Option<u8>,
    /// Server max window bits
    pub server_max_window_bits: Option<u8>,
}

impl Default for Extensions {
    fn default() -> Self {
        Self {
            permessage_deflate: false,
            client_max_window_bits: None,
            server_max_window_bits: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_properties() {
        assert!(OpCode::Close.is_control());
        assert!(OpCode::Ping.is_control());
        assert!(OpCode::Pong.is_control());

        assert!(OpCode::Text.is_data());
        assert!(OpCode::Binary.is_data());
        assert!(OpCode::Continue.is_data());
    }

    #[test]
    fn test_close_code_conversion() {
        assert_eq!(CloseCode::Normal.as_u16(), 1000);
        assert_eq!(CloseCode::from_u16(1000), Some(CloseCode::Normal));
        assert_eq!(CloseCode::from_u16(9999), None);
    }

    #[test]
    fn test_message_creation() {
        let text = Message::text("Hello");
        assert_eq!(text.opcode(), OpCode::Text);
        assert_eq!(text.len(), 5);

        let binary = Message::binary(vec![1, 2, 3]);
        assert_eq!(binary.opcode(), OpCode::Binary);
        assert_eq!(binary.len(), 3);

        let close = Message::close(CloseCode::Normal, "Goodbye");
        assert_eq!(close.opcode(), OpCode::Close);
        assert_eq!(close.len(), 9); // 2 bytes for code + 7 for "Goodbye"
    }

    #[test]
    fn test_frame_header_encoding() {
        let header = FrameHeader::new(OpCode::Text, 10, false);
        let encoded = header.encode();

        assert_eq!(encoded[0] & 0x80, 0x80); // FIN bit set
        assert_eq!(encoded[0] & 0x0F, OpCode::Text as u8);
        assert_eq!(encoded[1] & 0x7F, 10); // Payload length

        // Test extended payload length
        let header = FrameHeader::new(OpCode::Binary, 1000, false);
        let encoded = header.encode();
        assert_eq!(encoded[1] & 0x7F, 126);
        assert_eq!(u16::from_be_bytes([encoded[2], encoded[3]]), 1000);
    }

    #[test]
    fn test_frame_masking() {
        let payload = Bytes::from("Hello");
        let mut header = FrameHeader::new(OpCode::Text, payload.len(), true);
        header.mask_key = Some([0x12, 0x34, 0x56, 0x78]);

        let mut frame = Frame::new(header, payload);
        let original_payload = frame.payload.clone();

        frame.apply_mask();
        assert_ne!(frame.payload, original_payload);

        frame.remove_mask();
        assert_eq!(frame.payload, original_payload);
    }
}
