//! Why a frame could not be decoded or encoded.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The frame ends before the fields it must have.
    Truncated { needed: usize, available: usize },
    /// The subtype is not the one the decoder handles.
    WrongSubtype(u8),
    /// The version field is not one this crate knows.
    UnsupportedVersion(u8),
    /// The message type is not one the decoder handles.
    WrongMessageType(u8),
    /// A vendor unique message of another protocol.
    WrongProtocol,
    /// The control_data_length is shorter than the message's fields.
    ShortControlData(u16),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated { needed, available } => {
                write!(
                    formatter,
                    "frame too short: {available} octets, {needed} needed"
                )
            }
            DecodeError::WrongSubtype(subtype) => {
                write!(formatter, "unexpected subtype {subtype:#04x}")
            }
            DecodeError::UnsupportedVersion(version) => {
                write!(formatter, "unsupported version {version}")
            }
            DecodeError::ShortControlData(length) => {
                write!(formatter, "control_data_length {length} is too short")
            }
            DecodeError::WrongMessageType(message_type) => {
                write!(formatter, "unexpected message type {message_type}")
            }
            DecodeError::WrongProtocol => formatter.write_str("another vendor unique protocol"),
        }
    }
}

impl core::error::Error for DecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The buffer is smaller than the encoded message.
    BufferTooSmall { needed: usize, available: usize },
    /// A field holds a value wider than its place in the frame.
    OutOfRange(&'static str),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::BufferTooSmall { needed, available } => {
                write!(
                    formatter,
                    "buffer too small: {available} octets, {needed} needed"
                )
            }
            EncodeError::OutOfRange(field) => write!(formatter, "{field} does not fit its field"),
        }
    }
}

impl core::error::Error for EncodeError {}
