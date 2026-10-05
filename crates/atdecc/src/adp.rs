//! ATDECC Discovery Protocol PDUs (IEEE 1722.1-2021, 6.2.2).

use crate::avtp::{self, ControlHeader, read_u16, read_u32, read_u64, subtype};
use crate::error::{DecodeError, EncodeError};
use crate::id::{ClockIdentity, EntityId, EntityModelId};
use crate::macros::{code, flags};

code! {
    /// ADP message_type (Table 6-1).
    pub struct AdpMessageType(u8) {
        const ENTITY_AVAILABLE = 0;
        const ENTITY_DEPARTING = 1;
        const ENTITY_DISCOVER = 2;
    }
}

flags! {
    /// entity_capabilities (Table 6-2). The standard numbers bits from the
    /// most significant, so its bit 31 is `0x0000_0001`.
    pub struct EntityCapabilities(u32) {
        const EFU_MODE = 0x0000_0001;
        const ADDRESS_ACCESS_SUPPORTED = 0x0000_0002;
        const GATEWAY_ENTITY = 0x0000_0004;
        const AEM_SUPPORTED = 0x0000_0008;
        const LEGACY_AVC = 0x0000_0010;
        const ASSOCIATION_ID_SUPPORTED = 0x0000_0020;
        const ASSOCIATION_ID_VALID = 0x0000_0040;
        const VENDOR_UNIQUE_SUPPORTED = 0x0000_0080;
        const CLASS_A_SUPPORTED = 0x0000_0100;
        const CLASS_B_SUPPORTED = 0x0000_0200;
        const GPTP_SUPPORTED = 0x0000_0400;
        const AEM_AUTHENTICATION_SUPPORTED = 0x0000_0800;
        const AEM_AUTHENTICATION_REQUIRED = 0x0000_1000;
        const AEM_PERSISTENT_ACQUIRE_SUPPORTED = 0x0000_2000;
        const AEM_IDENTIFY_CONTROL_INDEX_VALID = 0x0000_4000;
        const AEM_INTERFACE_INDEX_VALID = 0x0000_8000;
        const GENERAL_CONTROLLER_IGNORE = 0x0001_0000;
        const ENTITY_NOT_READY = 0x0002_0000;
        const ACMP_ACQUIRE_WITH_AEM = 0x0004_0000;
        const ACMP_AUTHENTICATE_WITH_AEM = 0x0008_0000;
        const SUPPORTS_UDPV4_ATDECC = 0x0010_0000;
        const SUPPORTS_UDPV4_STREAMING = 0x0020_0000;
        const SUPPORTS_UDPV6_ATDECC = 0x0040_0000;
        const SUPPORTS_UDPV6_STREAMING = 0x0080_0000;
        const MULTIPLE_PTP_INSTANCES = 0x0100_0000;
        const AEM_CONFIGURATION_INDEX_VALID = 0x0200_0000;
    }
}

flags! {
    /// talker_capabilities (Table 6-3).
    pub struct TalkerCapabilities(u16) {
        const IMPLEMENTED = 0x0001;
        const OTHER_SOURCE = 0x0200;
        const CONTROL_SOURCE = 0x0400;
        const MEDIA_CLOCK_SOURCE = 0x0800;
        const SMPTE_SOURCE = 0x1000;
        const MIDI_SOURCE = 0x2000;
        const AUDIO_SOURCE = 0x4000;
        const VIDEO_SOURCE = 0x8000;
    }
}

flags! {
    /// listener_capabilities (Table 6-4).
    pub struct ListenerCapabilities(u16) {
        const IMPLEMENTED = 0x0001;
        const OTHER_SINK = 0x0200;
        const CONTROL_SINK = 0x0400;
        const MEDIA_CLOCK_SINK = 0x0800;
        const SMPTE_SINK = 0x1000;
        const MIDI_SINK = 0x2000;
        const AUDIO_SINK = 0x4000;
        const VIDEO_SINK = 0x8000;
    }
}

flags! {
    /// controller_capabilities (Table 6-5).
    pub struct ControllerCapabilities(u32) {
        const IMPLEMENTED = 0x0000_0001;
    }
}

/// An ADPDU. The reserved fields are zero when encoded and ignored when
/// decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Adpdu {
    pub message_type: AdpMessageType,
    /// How long the advertisement stays valid, in units of 2 seconds; zero
    /// in messages other than ENTITY_AVAILABLE.
    pub valid_time: u8,
    pub entity_id: EntityId,
    pub entity_model_id: EntityModelId,
    pub entity_capabilities: EntityCapabilities,
    /// The most streams the talker can source at once.
    pub talker_stream_sources: u16,
    pub talker_capabilities: TalkerCapabilities,
    /// The most streams the listener can sink at once.
    pub listener_stream_sinks: u16,
    pub listener_capabilities: ListenerCapabilities,
    pub controller_capabilities: ControllerCapabilities,
    /// Counts ENTITY_AVAILABLE messages since the entity started; a drop
    /// means it restarted.
    pub available_index: u32,
    pub gptp_grandmaster_id: ClockIdentity,
    pub gptp_domain_number: u8,
    pub current_configuration_index: u16,
    pub identify_control_index: u16,
    pub interface_index: u16,
    pub association_id: u64,
}

impl Adpdu {
    /// Control data octets after the entity ID.
    pub const CONTROL_DATA_LENGTH: u16 = 56;
    /// Octets of an encoded ADPDU.
    pub const LEN: usize = ControlHeader::COMMON_LEN + Self::CONTROL_DATA_LENGTH as usize;

    /// An ENTITY_DISCOVER for `entity_id`, or for every entity when it is
    /// zero.
    pub fn discover(entity_id: EntityId) -> Self {
        Self {
            message_type: AdpMessageType::ENTITY_DISCOVER,
            entity_id,
            ..Self::default()
        }
    }

    /// The valid time in seconds.
    pub fn valid_seconds(&self) -> u32 {
        u32::from(self.valid_time) * 2
    }

    /// Decodes an ADPDU from the octets after the Ethernet header. Octets
    /// past the ADPDU, such as Ethernet padding, are ignored.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let header =
            ControlHeader::decode_expecting(bytes, subtype::ADP, Self::CONTROL_DATA_LENGTH)?;
        Ok(Self {
            message_type: AdpMessageType(header.message_type),
            valid_time: header.status,
            entity_id: EntityId(read_u64(bytes, 4)),
            entity_model_id: EntityModelId(read_u64(bytes, 12)),
            entity_capabilities: EntityCapabilities(read_u32(bytes, 20)),
            talker_stream_sources: read_u16(bytes, 24),
            talker_capabilities: TalkerCapabilities(read_u16(bytes, 26)),
            listener_stream_sinks: read_u16(bytes, 28),
            listener_capabilities: ListenerCapabilities(read_u16(bytes, 30)),
            controller_capabilities: ControllerCapabilities(read_u32(bytes, 32)),
            available_index: read_u32(bytes, 36),
            gptp_grandmaster_id: ClockIdentity(read_u64(bytes, 40)),
            gptp_domain_number: bytes[48],
            current_configuration_index: read_u16(bytes, 50),
            identify_control_index: read_u16(bytes, 52),
            interface_index: read_u16(bytes, 54),
            association_id: read_u64(bytes, 56),
        })
    }

    /// Encodes into the start of `out`, returning the octets written,
    /// [`Self::LEN`].
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let out = avtp::buffer(out, Self::LEN)?;
        ControlHeader {
            subtype: subtype::ADP,
            message_type: self.message_type.0,
            status: self.valid_time,
            control_data_length: Self::CONTROL_DATA_LENGTH,
        }
        .encode(out)
        .map_err(|error| match error {
            EncodeError::OutOfRange("status") => EncodeError::OutOfRange("valid_time"),
            other => other,
        })?;
        avtp::write(out, 4, &self.entity_id.to_bytes());
        avtp::write(out, 12, &self.entity_model_id.to_bytes());
        avtp::write(out, 20, &self.entity_capabilities.0.to_be_bytes());
        avtp::write(out, 24, &self.talker_stream_sources.to_be_bytes());
        avtp::write(out, 26, &self.talker_capabilities.0.to_be_bytes());
        avtp::write(out, 28, &self.listener_stream_sinks.to_be_bytes());
        avtp::write(out, 30, &self.listener_capabilities.0.to_be_bytes());
        avtp::write(out, 32, &self.controller_capabilities.0.to_be_bytes());
        avtp::write(out, 36, &self.available_index.to_be_bytes());
        avtp::write(out, 40, &self.gptp_grandmaster_id.to_bytes());
        out[48] = self.gptp_domain_number;
        out[49] = 0;
        avtp::write(out, 50, &self.current_configuration_index.to_be_bytes());
        avtp::write(out, 52, &self.identify_control_index.to_be_bytes());
        avtp::write(out, 54, &self.interface_index.to_be_bytes());
        avtp::write(out, 56, &self.association_id.to_be_bytes());
        avtp::write(out, 64, &[0; 4]);
        Ok(Self::LEN)
    }

    /// The encoded ADPDU.
    pub fn to_bytes(&self) -> Result<[u8; Self::LEN], EncodeError> {
        let mut bytes = [0; Self::LEN];
        self.encode(&mut bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An ENTITY_AVAILABLE with a distinct value in every field, laid out
    /// by hand from Figure 6-1.
    const AVAILABLE: [u8; Adpdu::LEN] = [
        0xfa, 0x00, 0x50, 0x38, // subtype, message_type 0, valid_time 10, length 56
        0x00, 0x1b, 0x92, 0xff, 0xfe, 0x01, 0xab, 0xcd, // entity_id
        0x00, 0x1b, 0x92, 0x00, 0x00, 0x00, 0x00, 0x2a, // entity_model_id
        0x02, 0x00, 0xc5, 0x8a, // entity_capabilities
        0x00, 0x02, 0x40, 0x01, // talker_stream_sources, talker_capabilities
        0x00, 0x03, 0x48, 0x01, // listener_stream_sinks, listener_capabilities
        0x00, 0x00, 0x00, 0x01, // controller_capabilities
        0x00, 0x00, 0x01, 0x07, // available_index
        0x00, 0x01, 0xf2, 0xff, 0xfe, 0xff, 0x3b, 0x14, // gptp_grandmaster_id
        0x05, 0x00, 0x00, 0x01, // gptp_domain_number, reserved0, current_configuration_index
        0x00, 0x02, 0x00, 0x03, // identify_control_index, interface_index
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // association_id
        0x00, 0x00, 0x00, 0x00, // reserved1
    ];

    fn available() -> Adpdu {
        Adpdu {
            message_type: AdpMessageType::ENTITY_AVAILABLE,
            valid_time: 10,
            entity_id: EntityId(0x001b_92ff_fe01_abcd),
            entity_model_id: EntityModelId(0x001b_9200_0000_002a),
            entity_capabilities: EntityCapabilities::AEM_CONFIGURATION_INDEX_VALID
                | EntityCapabilities::AEM_INTERFACE_INDEX_VALID
                | EntityCapabilities::AEM_IDENTIFY_CONTROL_INDEX_VALID
                | EntityCapabilities::GPTP_SUPPORTED
                | EntityCapabilities::CLASS_A_SUPPORTED
                | EntityCapabilities::VENDOR_UNIQUE_SUPPORTED
                | EntityCapabilities::AEM_SUPPORTED
                | EntityCapabilities::ADDRESS_ACCESS_SUPPORTED,
            talker_stream_sources: 2,
            talker_capabilities: TalkerCapabilities::AUDIO_SOURCE | TalkerCapabilities::IMPLEMENTED,
            listener_stream_sinks: 3,
            listener_capabilities: ListenerCapabilities::AUDIO_SINK
                | ListenerCapabilities::MEDIA_CLOCK_SINK
                | ListenerCapabilities::IMPLEMENTED,
            controller_capabilities: ControllerCapabilities::IMPLEMENTED,
            available_index: 0x107,
            gptp_grandmaster_id: ClockIdentity(0x0001_f2ff_feff_3b14),
            gptp_domain_number: 5,
            current_configuration_index: 1,
            identify_control_index: 2,
            interface_index: 3,
            association_id: 0x1122_3344_5566_7788,
        }
    }

    #[test]
    fn decodes_every_field_at_its_offset() {
        assert_eq!(Adpdu::decode(&AVAILABLE), Ok(available()));
        assert_eq!(available().valid_seconds(), 20);
    }

    #[test]
    fn encodes_every_field_at_its_offset() {
        assert_eq!(available().to_bytes(), Ok(AVAILABLE));
    }

    #[test]
    fn ignores_padding_and_reserved_fields() {
        let mut padded = [0xee; Adpdu::LEN + 6];
        padded[..Adpdu::LEN].copy_from_slice(&AVAILABLE);
        padded[49] = 0xff;
        padded[64..68].copy_from_slice(&[0xff; 4]);
        assert_eq!(Adpdu::decode(&padded), Ok(available()));
    }

    #[test]
    fn discover_is_all_zero_but_the_entity() {
        let bytes = Adpdu::discover(EntityId(0)).to_bytes().unwrap();
        assert_eq!(&bytes[..4], &[0xfa, 0x02, 0x00, 0x38]);
        assert!(bytes[4..].iter().all(|&octet| octet == 0));
    }

    #[test]
    fn refuses_short_and_foreign_frames() {
        assert_eq!(
            Adpdu::decode(&AVAILABLE[..60]),
            Err(DecodeError::Truncated {
                needed: 68,
                available: 60
            })
        );
        let mut acmp = AVAILABLE;
        acmp[0] = subtype::ACMP;
        assert_eq!(Adpdu::decode(&acmp), Err(DecodeError::WrongSubtype(0xfc)));
        let mut short = AVAILABLE;
        short[3] = 40;
        assert_eq!(
            Adpdu::decode(&short),
            Err(DecodeError::ShortControlData(40))
        );
    }

    #[test]
    fn refuses_values_too_wide_to_encode() {
        let mut adpdu = available();
        adpdu.valid_time = 32;
        assert_eq!(adpdu.to_bytes(), Err(EncodeError::OutOfRange("valid_time")));
        assert_eq!(
            available().encode(&mut [0; 67]),
            Err(EncodeError::BufferTooSmall {
                needed: 68,
                available: 67
            })
        );
    }

    #[test]
    fn debug_names_known_values() {
        assert_eq!(
            format!("{:?}", AdpMessageType::ENTITY_DEPARTING),
            "ENTITY_DEPARTING"
        );
        assert_eq!(format!("{:?}", AdpMessageType(9)), "AdpMessageType(9)");
        assert_eq!(
            format!("{:?}", TalkerCapabilities(0x4003)),
            "TalkerCapabilities(IMPLEMENTED | AUDIO_SOURCE | 0x2)"
        );
        let names: Vec<&str> = TalkerCapabilities(0x4003).names().collect();
        assert_eq!(names, ["IMPLEMENTED", "AUDIO_SOURCE"]);
    }
}
