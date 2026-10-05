//! IEEE 802.1Q-2022 Multiple Registration Protocol (clause 10) and the
//! applications AVB uses on it: MSRP for stream reservation (clause 35)
//! and MVRP for VLAN membership (clause 11).
//!
//! Like `atdecc`, the protocol code does no I/O, keeps no clock and starts
//! no threads: the caller hands it received MRPDUs and the current time,
//! and takes back MRPDUs to send, registration events and the next
//! deadline. MSRP attribute lists can also be carried by other means than
//! MRPDUs, as AVB Lite's CVU SRP carries them inside ATDECC messages.
//!
//! Features:
//! - `std` (default): raw Ethernet on Linux, macOS and Windows through
//!   `avb-net`, and a blocking driver. Without it the crate is `no_std`.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

pub use avb_net::MacAddress;

/// Ethertype of MSRP.
pub const ETHERTYPE_MSRP: u16 = 0x22ea;

/// Ethertype of MVRP.
pub const ETHERTYPE_MVRP: u16 = 0x88f5;

/// Destination of MSRP: the nearest bridge group address.
pub const MSRP_DESTINATION: MacAddress = MacAddress([0x01, 0x80, 0xc2, 0x00, 0x00, 0x0e]);

/// Destination of MVRP: the customer bridge MVRP address.
pub const MVRP_DESTINATION: MacAddress = MacAddress([0x01, 0x80, 0xc2, 0x00, 0x00, 0x21]);
