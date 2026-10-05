//! The IEEE 1722-2016 alternative AVTPDU header that ADP, AECP and ACMP
//! share, and the subtypes that tell them apart.

use crate::error::{DecodeError, EncodeError};

/// AVTPDU subtypes (IEEE 1722-2016, Table 6).
pub mod subtype {
    pub const ADP: u8 = 0xfa;
    pub const AECP: u8 = 0xfb;
    pub const ACMP: u8 = 0xfc;
    /// MAAP, which shares the ethertype and is not part of ATDECC.
    pub const MAAP: u8 = 0xfe;
}

/// The first four octets of an ATDECC PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlHeader {
    pub subtype: u8,
    /// The 4-bit message_type.
    pub message_type: u8,
    /// The 5-bit field ADP calls valid_time and AECP and ACMP call status.
    pub status: u8,
    /// Octets after the 8-octet ID that follows the header.
    pub control_data_length: u16,
}

impl ControlHeader {
    pub const LEN: usize = 4;
    /// The header and the entity or stream ID after it, which
    /// control_data_length does not count.
    pub const COMMON_LEN: usize = 12;

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let [subtype, second, third, fourth, ..] = *bytes else {
            return Err(DecodeError::Truncated {
                needed: Self::LEN,
                available: bytes.len(),
            });
        };
        // The h bit is set to zero by senders and has no meaning here.
        let version = (second >> 4) & 0x07;
        if version != 0 {
            return Err(DecodeError::UnsupportedVersion(version));
        }
        Ok(Self {
            subtype,
            message_type: second & 0x0f,
            status: third >> 3,
            control_data_length: u16::from(third & 0x07) << 8 | u16::from(fourth),
        })
    }

    /// Decodes the header of a PDU of `subtype` whose control data must be
    /// at least `minimum` octets, and checks that the frame holds them.
    pub(crate) fn decode_expecting(
        bytes: &[u8],
        subtype: u8,
        minimum: u16,
    ) -> Result<Self, DecodeError> {
        let header = Self::decode(bytes)?;
        if header.subtype != subtype {
            return Err(DecodeError::WrongSubtype(header.subtype));
        }
        if header.control_data_length < minimum {
            return Err(DecodeError::ShortControlData(header.control_data_length));
        }
        let needed = Self::COMMON_LEN + usize::from(minimum);
        if bytes.len() < needed {
            return Err(DecodeError::Truncated {
                needed,
                available: bytes.len(),
            });
        }
        Ok(header)
    }

    pub fn encode(&self, out: &mut [u8]) -> Result<(), EncodeError> {
        if self.message_type > 0x0f {
            return Err(EncodeError::OutOfRange("message_type"));
        }
        if self.status > 0x1f {
            return Err(EncodeError::OutOfRange("status"));
        }
        if self.control_data_length > 0x07ff {
            return Err(EncodeError::OutOfRange("control_data_length"));
        }
        let Some(header) = out.get_mut(..Self::LEN) else {
            return Err(EncodeError::BufferTooSmall {
                needed: Self::LEN,
                available: out.len(),
            });
        };
        header[0] = self.subtype;
        header[1] = self.message_type;
        header[2] = self.status << 3 | (self.control_data_length >> 8) as u8;
        header[3] = self.control_data_length as u8;
        Ok(())
    }
}

/// Big-endian fields at fixed offsets, read after the caller has checked
/// the length.
pub(crate) fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

pub(crate) fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

pub(crate) fn read_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(read_array(bytes, at))
}

pub(crate) fn read_array<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut array = [0; N];
    array.copy_from_slice(&bytes[at..at + N]);
    array
}

pub(crate) fn write(out: &mut [u8], at: usize, value: &[u8]) {
    out[at..at + value.len()].copy_from_slice(value);
}

/// The first `needed` octets of `out`, or the error saying it is too
/// small.
pub(crate) fn buffer(out: &mut [u8], needed: usize) -> Result<&mut [u8], EncodeError> {
    let available = out.len();
    out.get_mut(..needed)
        .ok_or(EncodeError::BufferTooSmall { needed, available })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trips() {
        let header = ControlHeader {
            subtype: subtype::ACMP,
            message_type: 11,
            status: 31,
            control_data_length: 84,
        };
        let mut bytes = [0; 4];
        header.encode(&mut bytes).unwrap();
        assert_eq!(bytes, [0xfc, 0x0b, 0xf8, 0x54]);
        assert_eq!(ControlHeader::decode(&bytes), Ok(header));
    }

    #[test]
    fn header_refuses_what_does_not_fit() {
        let mut header = ControlHeader {
            subtype: subtype::ADP,
            message_type: 16,
            status: 0,
            control_data_length: 56,
        };
        let mut bytes = [0; 4];
        assert_eq!(
            header.encode(&mut bytes),
            Err(EncodeError::OutOfRange("message_type"))
        );
        header.message_type = 0;
        header.control_data_length = 0x800;
        assert_eq!(
            header.encode(&mut bytes),
            Err(EncodeError::OutOfRange("control_data_length"))
        );
        assert!(matches!(
            header.encode(&mut [0; 3]),
            Err(EncodeError::OutOfRange(_) | EncodeError::BufferTooSmall { .. })
        ));
    }

    #[test]
    fn header_refuses_other_versions_and_short_input() {
        assert_eq!(
            ControlHeader::decode(&[0xfa, 0x10, 0x00, 0x38]),
            Err(DecodeError::UnsupportedVersion(1))
        );
        assert_eq!(
            ControlHeader::decode(&[0xfa, 0x00]),
            Err(DecodeError::Truncated {
                needed: 4,
                available: 2
            })
        );
        // The h bit is ignored.
        assert!(ControlHeader::decode(&[0xfa, 0x80, 0x00, 0x38]).is_ok());
    }
}
