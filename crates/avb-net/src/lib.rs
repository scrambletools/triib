//! Raw Ethernet for AVB protocols: MAC addresses, the computer's
//! interfaces and, as it grows, sending and receiving frames with hardware
//! timestamps on Linux, macOS and Windows. This is the only platform code
//! the protocol crates depend on.
//!
//! Without the `std` feature only the address types remain, so protocol
//! crates built on them also run on microcontrollers.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

mod mac;

pub use mac::{MacAddress, ParseError};

#[cfg(feature = "std")]
pub mod interfaces;

#[cfg(feature = "std")]
pub mod socket;

#[cfg(feature = "std")]
pub use interfaces::{Interface, interfaces};
#[cfg(feature = "std")]
pub use socket::{Received, Socket};
