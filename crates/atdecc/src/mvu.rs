//! Milan Vendor Unique (MVU) messages (Milan 1.3, 5.4.3 and 5.4.4),
//! carried in AECP vendor unique PDUs.

use crate::aecp::{AecpHeader, AecpMessageType, VendorUniquePdu};
use crate::avtp::{read_array, read_u16, read_u32};
use crate::error::{DecodeError, EncodeError};
use crate::id::EntityId;
use crate::macros::{code, flags};

/// Avnu's OUI-36 `00-1B-C5-0A-C` with the MVU protocol number `0x100`.
pub const MVU_PROTOCOL_ID: [u8; 6] = [0x00, 0x1b, 0xc5, 0x0a, 0xc1, 0x00];

code! {
    /// MVU command_type (Table 5.15).
    pub struct MvuCommandType(u16) {
        const GET_MILAN_INFO = 0x0000;
        const SET_SYSTEM_UNIQUE_ID = 0x0001;
        const GET_SYSTEM_UNIQUE_ID = 0x0002;
        const SET_MEDIA_CLOCK_REFERENCE_INFO = 0x0003;
        const GET_MEDIA_CLOCK_REFERENCE_INFO = 0x0004;
        const BIND_STREAM = 0x0005;
        const UNBIND_STREAM = 0x0006;
        const GET_STREAM_INPUT_INFO_EX = 0x0007;
    }
}

code! {
    /// MVU status (Table 5.16).
    pub struct MvuStatus(u8) {
        const SUCCESS = 0;
        const NOT_IMPLEMENTED = 1;
        const NO_SUCH_DESCRIPTOR = 2;
        const ENTITY_LOCKED = 3;
        const BAD_ARGUMENTS = 7;
        const ENTITY_MISBEHAVING = 10;
        const PAYLOAD_TOO_SHORT = 13;
    }
}

flags! {
    /// GET_MILAN_INFO features_flags (Table 5.17).
    pub struct MilanFeatures(u32) {
        const REDUNDANCY = 0x0000_0001;
        const TALKER_DYNAMIC_MAPPINGS_WHILE_RUNNING = 0x0000_0002;
        const MVU_BINDING = 0x0000_0004;
        const TALKER_SIGNAL_PRESENCE = 0x0000_0008;
    }
}

/// The MVU part of a vendor unique PDU: the u flag, the command type and
/// the command specific data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MvuMessage<'a> {
    pub unsolicited: bool,
    pub command_type: MvuCommandType,
    pub data: &'a [u8],
}

impl<'a> MvuMessage<'a> {
    /// The MVU message in a vendor unique PDU, if it is one.
    pub fn from_pdu(pdu: &VendorUniquePdu<'a>) -> Result<Self, DecodeError> {
        if pdu.protocol_id != MVU_PROTOCOL_ID {
            return Err(DecodeError::WrongProtocol);
        }
        let Some(word) = pdu.payload.get(..2) else {
            return Err(DecodeError::Truncated {
                needed: 2,
                available: pdu.payload.len(),
            });
        };
        let word = read_u16(word, 0);
        Ok(Self {
            unsolicited: word & 0x8000 != 0,
            command_type: MvuCommandType(word & 0x7fff),
            data: &pdu.payload[2..],
        })
    }
}

/// Encodes a GET_MILAN_INFO command.
pub fn encode_get_milan_info(
    target: EntityId,
    controller: EntityId,
    sequence_id: u16,
    out: &mut [u8],
) -> Result<usize, EncodeError> {
    let header = AecpHeader {
        message_type: AecpMessageType::VENDOR_UNIQUE_COMMAND,
        status: 0,
        target_entity_id: target,
        controller_entity_id: controller,
        sequence_id,
    };
    let command = MvuCommandType::GET_MILAN_INFO.0.to_be_bytes();
    header.encode(&[&MVU_PROTOCOL_ID, &command, &[0, 0]], out)
}

/// A GET_MILAN_INFO response (Figure 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MilanInfo {
    pub protocol_version: u32,
    pub features: MilanFeatures,
    /// The Milan certification the entity passed, as four dotted numbers;
    /// all zeros when not certified.
    pub certification_version: [u8; 4],
    /// The Milan specification the entity implements; absent from entities
    /// older than Milan 1.3.
    pub specification_version: Option<[u8; 4]>,
}

impl MilanInfo {
    /// Decodes the command specific data of a GET_MILAN_INFO response.
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 14 {
            return Err(DecodeError::Truncated {
                needed: 14,
                available: data.len(),
            });
        }
        Ok(Self {
            protocol_version: read_u32(data, 2),
            features: MilanFeatures(read_u32(data, 6)),
            certification_version: read_array(data, 10),
            specification_version: (data.len() >= 18).then(|| read_array(data, 14)),
        })
    }

    pub fn is_certified(&self) -> bool {
        self.certification_version != [0; 4]
    }
}

flags! {
    /// SET_MEDIA_CLOCK_REFERENCE_INFO flags (Table 5.18): which values are
    /// valid, which also means the entity lets a controller change them.
    pub struct MediaClockReferenceFlags(u8) {
        const PRIORITY_VALID = 0x01;
        const DOMAIN_NAME_VALID = 0x02;
    }
}

/// Encodes GET_MEDIA_CLOCK_REFERENCE_INFO for a CLOCK_DOMAIN (Figure 5.9).
pub fn encode_get_media_clock_reference(
    target: EntityId,
    controller: EntityId,
    sequence_id: u16,
    domain: u16,
    out: &mut [u8],
) -> Result<usize, EncodeError> {
    let header = AecpHeader {
        message_type: AecpMessageType::VENDOR_UNIQUE_COMMAND,
        status: 0,
        target_entity_id: target,
        controller_entity_id: controller,
        sequence_id,
    };
    let command = MvuCommandType::GET_MEDIA_CLOCK_REFERENCE_INFO
        .0
        .to_be_bytes();
    header.encode(&[&MVU_PROTOCOL_ID, &command, &domain.to_be_bytes()], out)
}

/// A GET_ or SET_MEDIA_CLOCK_REFERENCE_INFO response, or the same in a
/// notification (Figure 5.8): how readily a clock domain should be chosen
/// as media clock reference, and the media clock domain it belongs to
/// (Milan 1.3, 7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaClockReference {
    /// The CLOCK_DOMAIN.
    pub domain: u16,
    pub flags: MediaClockReferenceFlags,
    /// The vendor's priority for the device, by its kind (Table 7.3).
    pub default_priority: u8,
    user_priority: u8,
    domain_name: [u8; 64],
}

impl MediaClockReference {
    /// Decodes the command specific data after the command type.
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 74 {
            return Err(DecodeError::Truncated {
                needed: 74,
                available: data.len(),
            });
        }
        Ok(Self {
            domain: read_u16(data, 0),
            flags: MediaClockReferenceFlags(data[2]),
            default_priority: data[4],
            user_priority: data[5],
            domain_name: read_array(data, 10),
        })
    }

    /// The user's priority, when the entity keeps one; it starts as the
    /// default.
    pub fn user_priority(&self) -> Option<u8> {
        self.flags
            .contains(MediaClockReferenceFlags::PRIORITY_VALID)
            .then_some(self.user_priority)
    }

    /// The priority to elect by: the user's, else the default.
    pub fn priority(&self) -> u8 {
        self.user_priority().unwrap_or(self.default_priority)
    }

    /// The media clock domain's name, when the entity keeps one; it starts
    /// as "DEFAULT".
    pub fn domain_name(&self) -> Option<&str> {
        self.flags
            .contains(MediaClockReferenceFlags::DOMAIN_NAME_VALID)
            .then(|| crate::descriptor::aem_string(&self.domain_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_clock_reference_info() {
        let mut out = [0; 64];
        let length =
            encode_get_media_clock_reference(EntityId(1), EntityId(2), 7, 3, &mut out).unwrap();
        assert_eq!(length, 32);
        let pdu = VendorUniquePdu::decode(&out[..length]).unwrap();
        let message = MvuMessage::from_pdu(&pdu).unwrap();
        assert_eq!(
            message.command_type,
            MvuCommandType::GET_MEDIA_CLOCK_REFERENCE_INFO
        );
        assert_eq!(message.data, &[0, 3]);

        let mut data = [0u8; 74];
        data[0..2].copy_from_slice(&1u16.to_be_bytes());
        data[2] = 0x03;
        data[4] = 192;
        data[5] = 200;
        data[10..17].copy_from_slice(b"DEFAULT");
        let reference = MediaClockReference::decode(&data).unwrap();
        assert_eq!(reference.domain, 1);
        assert_eq!(reference.default_priority, 192);
        assert_eq!(reference.user_priority(), Some(200));
        assert_eq!(reference.priority(), 200);
        assert_eq!(reference.domain_name(), Some("DEFAULT"));
        // Values the entity does not keep are not shown.
        data[2] = 0;
        let fixed = MediaClockReference::decode(&data).unwrap();
        assert_eq!((fixed.user_priority(), fixed.priority()), (None, 192));
        assert_eq!(fixed.domain_name(), None);
        assert!(MediaClockReference::decode(&data[..73]).is_err());
    }

    #[test]
    fn get_milan_info_command_layout() {
        let mut out = [0; 64];
        let length = encode_get_milan_info(EntityId(1), EntityId(2), 0x0102, &mut out).unwrap();
        assert_eq!(length, 32);
        assert_eq!(&out[..4], &[0xfb, 0x06, 0x00, 0x14]);
        assert_eq!(&out[22..28], &MVU_PROTOCOL_ID);
        assert_eq!(&out[28..32], &[0, 0, 0, 0]);
        let pdu = VendorUniquePdu::decode(&out[..length]).unwrap();
        let message = MvuMessage::from_pdu(&pdu).unwrap();
        assert_eq!(message.command_type, MvuCommandType::GET_MILAN_INFO);
        assert_eq!(message.data, &[0, 0]);
    }

    #[test]
    fn milan_info_with_and_without_specification_version() {
        let mut data = [0u8; 18];
        data[2..6].copy_from_slice(&1u32.to_be_bytes());
        data[6..10].copy_from_slice(&0x0000_0005u32.to_be_bytes());
        data[14..18].copy_from_slice(&[1, 3, 0, 0]);
        let info = MilanInfo::decode(&data).unwrap();
        assert_eq!(info.protocol_version, 1);
        assert!(info.features.contains(MilanFeatures::REDUNDANCY));
        assert!(info.features.contains(MilanFeatures::MVU_BINDING));
        assert!(!info.is_certified());
        assert_eq!(info.specification_version, Some([1, 3, 0, 0]));
        assert_eq!(
            MilanInfo::decode(&data[..14])
                .unwrap()
                .specification_version,
            None
        );
        assert!(MilanInfo::decode(&data[..13]).is_err());
    }
}
