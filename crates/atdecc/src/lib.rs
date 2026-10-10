//! IEEE 1722.1-2021 ATDECC (formerly AVDECC) with Milan 1.3 and AVB Lite
//! extensions: frame encoding and decoding, and the controller and entity
//! state machines.
//!
//! The protocol code does no I/O, keeps no clock and starts no threads.
//! The caller hands it received frames and the current time, and takes
//! back frames to send, events and the next deadline, so the same code
//! runs in a blocking loop, under an async runtime or on a microcontroller.
//!
//! Frames are the octets after the Ethernet header. Fixed-size PDUs such
//! as [`Adpdu`] and [`Acmpdu`] decode into plain values and encode into
//! the caller's buffer, without allocating. Values the standard reserves
//! are kept, so a frame decodes and encodes back unchanged.
//!
//! Features:
//! - `alloc`: the controller and entity state machines, which keep tables.
//! - `std` (default, includes `alloc`): raw Ethernet on Linux, macOS and
//!   Windows through `avb-net`, and a blocking driver for apps without a
//!   loop of their own. Without it the crate is `no_std`.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod macros;

pub mod acmp;
pub mod adp;
pub mod aecp;
pub mod aem;
pub mod avtp;
#[cfg(feature = "alloc")]
pub mod cache;
pub mod control;
pub mod descriptor;
pub mod error;
pub mod id;
pub mod lite;
#[cfg(feature = "alloc")]
pub mod maap;
#[cfg(feature = "alloc")]
pub mod media_clock;
#[cfg(feature = "alloc")]
pub mod model;
pub mod mvu;
pub mod pdu;
pub mod stream_format;
pub mod time;
pub mod wireless;

#[cfg(feature = "std")]
pub mod blocking;
#[cfg(feature = "alloc")]
pub mod controller;
#[cfg(feature = "alloc")]
pub mod entity;
#[cfg(feature = "std")]
pub mod neighbor;

pub use acmp::{AcmpFlags, AcmpIp, AcmpMessageType, AcmpStatus, Acmpdu};
pub use adp::{
    AdpMessageType, Adpdu, ControllerCapabilities, EntityCapabilities, ListenerCapabilities,
    TalkerCapabilities,
};
pub use aecp::{AecpHeader, AecpMessageType, AemCommandType, AemPdu, AemStatus, VendorUniquePdu};
pub use avb_net::MacAddress;
pub use descriptor::{DescriptorType, LocalizedStringRef};
pub use error::{DecodeError, EncodeError};
pub use id::{ClockIdentity, EntityId, EntityModelId, StreamId};
pub use pdu::{Pdu, decode};
pub use stream_format::StreamFormat;
pub use time::Instant;

#[cfg(feature = "alloc")]
pub use controller::{Controller, DiscoveredEntity, Event, OfflineReason};

/// Ethertype of AVTP, which carries ADP, AECP and ACMP.
pub const ETHERTYPE_AVTP: u16 = 0x22f0;

/// Destination of ADP and ACMP messages (IEEE 1722.1-2021, Table B.1).
pub const ADP_ACMP_MULTICAST: MacAddress = MacAddress([0x91, 0xe0, 0xf0, 0x01, 0x00, 0x00]);
