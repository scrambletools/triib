//! MAC addresses.

use core::fmt;
use core::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: MacAddress = MacAddress([0xff; 6]);

    pub fn is_multicast(self) -> bool {
        self.0[0] & 0x01 != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, f] = self.0;
        write!(formatter, "{a:02x}:{b:02x}:{c:02x}:{d:02x}:{e:02x}:{f:02x}")
    }
}

impl FromStr for MacAddress {
    type Err = ParseError;

    /// `00:1b:92:01:ab:cd`, with `:` or `-` between the octets.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut octets = [0; 6];
        let mut parts = text.trim().split([':', '-']);
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
        Ok(MacAddress(octets))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a valid MAC address")
    }
}

impl core::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mac: MacAddress = "00:01:F2:01:62:e5".parse().unwrap();
        assert_eq!(mac.to_string(), "00:01:f2:01:62:e5");
        assert_eq!("00-01-f2-01-62-e5".parse::<MacAddress>(), Ok(mac));
        assert!(!mac.is_multicast());
        assert!(MacAddress::BROADCAST.is_multicast());
    }

    #[test]
    fn bad_addresses_are_refused() {
        for text in [
            "",
            "00:01:f2:01:62",
            "00:01:f2:01:62:e5:00",
            "0:01:f2:01:62:e5",
            "zz:01:f2:01:62:e5",
        ] {
            assert!(text.parse::<MacAddress>().is_err(), "{text}");
        }
    }
}
