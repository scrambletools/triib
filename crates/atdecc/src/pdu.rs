//! Telling ATDECC PDUs apart by subtype.

use crate::acmp::Acmpdu;
use crate::adp::Adpdu;
use crate::aecp::{AecpHeader, AecpMessageType, AemPdu, VendorUniquePdu};
use crate::avtp::{ControlHeader, subtype};
use crate::error::DecodeError;

/// An AVTPDU received on the AVTP ethertype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pdu<'a> {
    Adp(Adpdu),
    Acmp(Acmpdu),
    Aem(AemPdu<'a>),
    VendorUnique(VendorUniquePdu<'a>),
    /// Another AECP message type, such as ADDRESS_ACCESS, not decoded
    /// further.
    Aecp(AecpHeader, &'a [u8]),
    /// Another AVTP subtype sharing the ethertype, such as MAAP or a
    /// stream.
    Other {
        subtype: u8,
    },
}

/// Decodes the octets after the Ethernet header of a frame on the AVTP
/// ethertype.
pub fn decode(bytes: &[u8]) -> Result<Pdu<'_>, DecodeError> {
    let Some(&first) = bytes.first() else {
        return Err(DecodeError::Truncated {
            needed: ControlHeader::LEN,
            available: 0,
        });
    };
    match first {
        subtype::ADP => Adpdu::decode(bytes).map(Pdu::Adp),
        subtype::ACMP => Acmpdu::decode(bytes).map(Pdu::Acmp),
        subtype::AECP => {
            let (header, body) = AecpHeader::decode(bytes)?;
            match header.message_type {
                AecpMessageType::AEM_COMMAND | AecpMessageType::AEM_RESPONSE => {
                    AemPdu::decode(bytes).map(Pdu::Aem)
                }
                AecpMessageType::VENDOR_UNIQUE_COMMAND
                | AecpMessageType::VENDOR_UNIQUE_RESPONSE => {
                    VendorUniquePdu::decode(bytes).map(Pdu::VendorUnique)
                }
                _ => Ok(Pdu::Aecp(header, body)),
            }
        }
        other => Ok(Pdu::Other { subtype: other }),
    }
}

/// Whether `bytes` are an ATDECC PDU, by subtype: ADP, AECP or ACMP.
pub fn is_atdecc(bytes: &[u8]) -> bool {
    matches!(
        bytes.first(),
        Some(&(subtype::ADP | subtype::AECP | subtype::ACMP))
    )
}

/// How many octets an ATDECC PDU's control_data_length claims past the
/// end of the frame: [`decode`] takes the octets there are, as some
/// entities count octets they do not send. Zero when the frame holds them
/// all; `None` for another subtype or a header that does not decode.
pub fn missing_octets(bytes: &[u8]) -> Option<usize> {
    if !is_atdecc(bytes) {
        return None;
    }
    let header = ControlHeader::decode(bytes).ok()?;
    let claimed = ControlHeader::COMMON_LEN + usize::from(header.control_data_length);
    Some(claimed.saturating_sub(bytes.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::EntityId;

    #[test]
    fn dispatches_by_subtype() {
        let discover = Adpdu::discover(EntityId(0)).to_bytes().unwrap();
        assert!(matches!(decode(&discover), Ok(Pdu::Adp(_))));
        assert_eq!(decode(&[0xfe, 0x01]), Ok(Pdu::Other { subtype: 0xfe }));
        assert!(matches!(
            decode(&[]),
            Err(DecodeError::Truncated {
                needed: 4,
                available: 0
            })
        ));
        let mut aem = [0u8; 24];
        aem[0] = 0xfb;
        aem[3] = 12;
        aem[23] = 0x03;
        assert!(matches!(decode(&aem), Ok(Pdu::Aem(_))));
        aem[1] = 0x02;
        assert!(matches!(decode(&aem), Ok(Pdu::Aecp(_, _))));
        assert!(decode(&[0xfb, 0x00, 0x00, 0x10]).is_err());
    }

    #[test]
    fn octets_claimed_past_the_frame_are_counted() {
        let discover = Adpdu::discover(EntityId(0)).to_bytes().unwrap();
        assert!(is_atdecc(&discover));
        assert_eq!(missing_octets(&discover), Some(0));
        // An AEM response claiming 8 octets more than it carries, as the
        // wired endpoint's GET_COUNTERS does.
        let mut aem = [0u8; 24];
        aem[0] = 0xfb;
        aem[3] = 20;
        assert_eq!(missing_octets(&aem), Some(8));
        // Ethernet padding after the PDU is no shortfall.
        aem[3] = 4;
        assert_eq!(missing_octets(&aem), Some(0));
        assert_eq!(missing_octets(&[0xfe, 0x01, 0, 0]), None);
        assert!(!is_atdecc(&[0x02]));
    }
}
