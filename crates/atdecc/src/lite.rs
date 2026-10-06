//! AVB Lite's ATDECC messages (AVB Lite profile, 2.4 and 6): the status
//! query an endpoint reports its mode with, and the CVU SRP messages that
//! carry MSRP declarations where bridges do not. Both are AECP vendor
//! unique messages under the AVB Lite MA-S OUI `8C-1F-64-36-C`.

use crate::aecp::{AecpHeader, AecpMessageType, VendorUniquePdu};
use crate::avtp::{read_u16, read_u32, read_u64};
use crate::error::{DecodeError, EncodeError};
use crate::id::{ClockIdentity, EntityId};
use crate::macros::{code, flags};

/// CVU SRP, sub-protocol `0x002`.
pub const CVU_PROTOCOL_ID: [u8; 6] = [0x8c, 0x1f, 0x64, 0x36, 0xc0, 0x02];
/// The status query, sub-protocol `0x003`.
pub const STATUS_PROTOCOL_ID: [u8; 6] = [0x8c, 0x1f, 0x64, 0x36, 0xc0, 0x03];

code! {
    /// The status query's command_type.
    pub struct LiteCommandType(u16) {
        const GET_LITE_STATUS = 0x0000;
    }
}

flags! {
    /// GET_LITE_STATUS flags.
    pub struct LiteFlags(u8) {
        /// The endpoint supports AVB Lite.
        const CAPABLE = 0x01;
        /// The interface operates in AVB Lite.
        const ACTIVE = 0x02;
        const OFFSET_VALID = 0x04;
        const EGRESS_VALID = 0x08;
    }
}

code! {
    /// Why an interface operates in AVB Lite (profile 2.2).
    pub struct FallbackReason(u8) {
        /// It operates in standard AVB.
        const NONE = 0;
        const ENDPOINT_TLV = 1;
        const PDELAY_UNANSWERED = 2;
        const MULTIPLE_RESPONDERS = 3;
        /// The operator or a controller set it.
        const CONFIGURED = 4;
    }
}

code! {
    pub struct PtpProfile(u8) {
        const GPTP = 0;
        const AVB_LITE_PTP = 1;
    }
}

/// The status query part of a vendor unique PDU: the u flag, the command
/// type and the command specific data after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteMessage<'a> {
    pub unsolicited: bool,
    pub command_type: LiteCommandType,
    pub data: &'a [u8],
}

impl<'a> LiteMessage<'a> {
    /// The status query message in a vendor unique PDU, if it is one.
    pub fn from_pdu(pdu: &VendorUniquePdu<'a>) -> Result<Self, DecodeError> {
        if pdu.protocol_id != STATUS_PROTOCOL_ID {
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
            command_type: LiteCommandType(word & 0x7fff),
            data: &pdu.payload[2..],
        })
    }
}

/// Encodes a GET_LITE_STATUS command for an AVB_INTERFACE.
pub fn encode_get_lite_status(
    target: EntityId,
    controller: EntityId,
    sequence_id: u16,
    interface: u16,
    out: &mut [u8],
) -> Result<usize, EncodeError> {
    let header = AecpHeader {
        message_type: AecpMessageType::VENDOR_UNIQUE_COMMAND,
        status: 0,
        target_entity_id: target,
        controller_entity_id: controller,
        sequence_id,
    };
    let command = LiteCommandType::GET_LITE_STATUS.0.to_be_bytes();
    header.encode(
        &[&STATUS_PROTOCOL_ID, &command, &interface.to_be_bytes()],
        out,
    )
}

/// What an endpoint reports of an interface in a GET_LITE_STATUS
/// response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteStatus {
    pub interface: u16,
    pub flags: LiteFlags,
    pub fallback_reason: FallbackReason,
    pub ptp_profile: PtpProfile,
    pub ptp_domain: u8,
    /// 0 when streams are untagged.
    pub media_vlan_id: u16,
    pub unicast_fanout_limit: u8,
    /// Mb/s, 0 when not known.
    pub link_speed: u32,
    /// kb/s of admitted streams sent, each unicast copy counted.
    pub committed_egress: u32,
    pub grandmaster: ClockIdentity,
    /// Nanoseconds.
    pub offset_from_grandmaster: i32,
}

impl LiteStatus {
    /// The response's octets after the command_type.
    const LEN: usize = 30;

    /// Decodes the data after the command_type of a response.
    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < Self::LEN {
            return Err(DecodeError::Truncated {
                needed: Self::LEN,
                available: data.len(),
            });
        }
        Ok(Self {
            interface: read_u16(data, 0),
            flags: LiteFlags(data[2]),
            fallback_reason: FallbackReason(data[3]),
            ptp_profile: PtpProfile(data[4]),
            ptp_domain: data[5],
            media_vlan_id: read_u16(data, 6),
            unicast_fanout_limit: data[8],
            link_speed: read_u32(data, 10),
            committed_egress: read_u32(data, 14),
            grandmaster: ClockIdentity(read_u64(data, 18)),
            offset_from_grandmaster: read_u32(data, 26) as i32,
        })
    }

    /// The data after the command_type, as a response carries it.
    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut out = [0; Self::LEN];
        out[0..2].copy_from_slice(&self.interface.to_be_bytes());
        out[2] = self.flags.0;
        out[3] = self.fallback_reason.0;
        out[4] = self.ptp_profile.0;
        out[5] = self.ptp_domain;
        out[6..8].copy_from_slice(&self.media_vlan_id.to_be_bytes());
        out[8] = self.unicast_fanout_limit;
        out[10..14].copy_from_slice(&self.link_speed.to_be_bytes());
        out[14..18].copy_from_slice(&self.committed_egress.to_be_bytes());
        out[18..26].copy_from_slice(&self.grandmaster.0.to_be_bytes());
        out[26..30].copy_from_slice(&self.offset_from_grandmaster.to_be_bytes());
        out
    }

    /// The offset from the grandmaster, when the endpoint measured it.
    pub fn offset(&self) -> Option<i32> {
        self.flags
            .contains(LiteFlags::OFFSET_VALID)
            .then_some(self.offset_from_grandmaster)
    }

    /// The committed egress, when the endpoint keeps it.
    pub fn egress(&self) -> Option<u32> {
        self.flags
            .contains(LiteFlags::EGRESS_VALID)
            .then_some(self.committed_egress)
    }
}

/// A CVU SRP message: an MSRP message carried in a vendor unique command,
/// from the endpoint the PDU's controller_entity_id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CvuMessage<'a> {
    pub sender: EntityId,
    /// The MSRP attribute type of the message.
    pub command_type: u8,
    /// The MSRP message, from its AttributeType through its EndMark.
    pub msrp: &'a [u8],
}

impl<'a> CvuMessage<'a> {
    pub fn from_pdu(pdu: &VendorUniquePdu<'a>) -> Result<Self, DecodeError> {
        if pdu.protocol_id != CVU_PROTOCOL_ID {
            return Err(DecodeError::WrongProtocol);
        }
        let Some((&command_type, msrp)) = pdu.payload.split_first() else {
            return Err(DecodeError::Truncated {
                needed: 1,
                available: 0,
            });
        };
        Ok(Self {
            sender: pdu.header.controller_entity_id,
            command_type,
            msrp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> LiteStatus {
        LiteStatus {
            interface: 0,
            flags: LiteFlags::CAPABLE | LiteFlags::ACTIVE | LiteFlags::OFFSET_VALID,
            fallback_reason: FallbackReason::MULTIPLE_RESPONDERS,
            ptp_profile: PtpProfile::AVB_LITE_PTP,
            ptp_domain: 0,
            media_vlan_id: 2,
            unicast_fanout_limit: 2,
            link_speed: 1000,
            committed_egress: 6_336,
            grandmaster: ClockIdentity(0x0011_22ff_fe33_4455),
            offset_from_grandmaster: -320,
        }
    }

    #[test]
    fn status_round_trips_and_says_what_is_valid() {
        let status = status();
        let decoded = LiteStatus::decode(&status.to_bytes()).unwrap();
        assert_eq!(decoded, status);
        assert_eq!(decoded.offset(), Some(-320));
        assert_eq!(decoded.egress(), None);
        assert!(LiteStatus::decode(&status.to_bytes()[..29]).is_err());
    }

    #[test]
    fn queries_carry_the_status_protocol() {
        let mut out = [0; 64];
        let length = encode_get_lite_status(
            EntityId(0xe8f6_0ae0_9220_0000),
            EntityId(0x9c6b_00ff_fe30_9a2b),
            7,
            0,
            &mut out,
        )
        .unwrap();
        let pdu = VendorUniquePdu::decode(&out[..length]).unwrap();
        let message = LiteMessage::from_pdu(&pdu).unwrap();
        assert_eq!(message.command_type, LiteCommandType::GET_LITE_STATUS);
        assert!(!message.unsolicited);
        assert_eq!(message.data, [0, 0]);
        assert!(CvuMessage::from_pdu(&pdu).is_err());
    }
}
