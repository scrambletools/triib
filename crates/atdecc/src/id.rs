//! The EUI-64 identifiers of ATDECC: entity IDs, entity model IDs, stream
//! IDs and gPTP clock identities.

use core::fmt;
use core::str::FromStr;

use avb_net::MacAddress;

/// An EUI-64 newtype shown as `0x001b92fffe01abcd`, as Hive and most
/// controllers show them, and parsed from that, plain hex, or octets
/// separated by `:` or `-`.
macro_rules! eui64 {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn to_bytes(self) -> [u8; 8] {
                self.0.to_be_bytes()
            }

            pub const fn from_bytes(bytes: [u8; 8]) -> Self {
                Self(u64::from_be_bytes(bytes))
            }

            /// Neither all zeros nor all ones, which stand for "none".
            pub const fn is_valid(self) -> bool {
                self.0 != 0 && self.0 != u64::MAX
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "0x{:016x}", self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}(0x{:016x})", stringify!($name), self.0)
            }
        }

        impl FromStr for $name {
            type Err = ParseError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                parse_eui64(text).map(Self)
            }
        }
    };
}

eui64!(
    /// An ATDECC entity ID.
    EntityId
);

eui64!(
    /// Identifies an entity model; entities with the same ID have the same
    /// descriptors, apart from their current values.
    EntityModelId
);

eui64!(
    /// An IEEE 1722 stream ID: the talker's MAC address and a 16-bit
    /// unique ID.
    StreamId
);

eui64!(
    /// An IEEE 1588 clock identity, as in gptp_grandmaster_id.
    ClockIdentity
);

impl EntityId {
    /// The EUI-64 made from a MAC address by putting `ff:fe` between its
    /// OUI and the rest, as entities commonly derive their ID.
    pub const fn from_mac(mac: MacAddress) -> Self {
        let [a, b, c, d, e, f] = mac.0;
        Self(u64::from_be_bytes([a, b, c, 0xff, 0xfe, d, e, f]))
    }
}

impl StreamId {
    pub const fn new(mac: MacAddress, unique_id: u16) -> Self {
        let [a, b, c, d, e, f] = mac.0;
        let [g, h] = unique_id.to_be_bytes();
        Self(u64::from_be_bytes([a, b, c, d, e, f, g, h]))
    }

    pub const fn mac(self) -> MacAddress {
        let [a, b, c, d, e, f, _, _] = self.0.to_be_bytes();
        MacAddress([a, b, c, d, e, f])
    }

    pub const fn unique_id(self) -> u16 {
        self.0 as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a valid EUI-64")
    }
}

impl core::error::Error for ParseError {}

fn parse_eui64(text: &str) -> Result<u64, ParseError> {
    let text = text.trim();
    if text.contains([':', '-']) {
        let mut octets = [0; 8];
        let mut parts = text.split([':', '-']);
        for octet in &mut octets {
            let part = parts.next().ok_or(ParseError)?;
            if part.len() != 2 {
                return Err(ParseError);
            }
            *octet = u8::from_str_radix(part, 16).map_err(|_| ParseError)?;
        }
        if parts.next().is_some() {
            return Err(ParseError);
        }
        return Ok(u64::from_be_bytes(octets));
    }
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    if digits.is_empty() || digits.len() > 16 {
        return Err(ParseError);
    }
    u64::from_str_radix(digits, 16).map_err(|_| ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_mac_inserts_fffe() {
        let mac: MacAddress = "80:f1:b2:d1:da:10".parse().unwrap();
        assert_eq!(EntityId::from_mac(mac).to_string(), "0x80f1b2fffed1da10");
    }

    #[test]
    fn parses_every_form() {
        let id = EntityId(0x001b_92ff_fe01_abcd);
        for text in [
            "0x001b92fffe01abcd",
            "001B92FFFE01ABCD",
            "00:1b:92:ff:fe:01:ab:cd",
        ] {
            assert_eq!(text.parse::<EntityId>(), Ok(id), "{text}");
        }
        assert!("0x".parse::<EntityId>().is_err());
        assert!("0x001b92fffe01abcd00".parse::<EntityId>().is_err());
        assert!("00:1b:92:ff:fe:01:ab".parse::<EntityId>().is_err());
        assert!(!EntityId(0).is_valid());
        assert!(!EntityId(u64::MAX).is_valid());
    }

    #[test]
    fn stream_id_splits_into_mac_and_unique_id() {
        let mac: MacAddress = "e8:f6:0a:e0:92:20".parse().unwrap();
        let stream = StreamId::new(mac, 0x0001);
        assert_eq!(stream, StreamId(0xe8f6_0ae0_9220_0001));
        assert_eq!(stream.mac(), mac);
        assert_eq!(stream.unique_id(), 1);
    }
}
