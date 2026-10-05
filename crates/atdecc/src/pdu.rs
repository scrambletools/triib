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
}
