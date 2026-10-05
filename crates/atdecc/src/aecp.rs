//! ATDECC Enumeration and Control Protocol PDUs (IEEE 1722.1-2021, 9.2
//! and 9.3): the common header, ATDECC Entity Model (AEM) commands and
//! responses, and vendor unique messages such as Milan's MVU.

use crate::avtp::{self, ControlHeader, read_array, read_u16, read_u64, subtype};
use crate::error::{DecodeError, EncodeError};
use crate::id::EntityId;
use crate::macros::code;

code! {
    /// AECP message_type (Table 9-1).
    pub struct AecpMessageType(u8) {
        const AEM_COMMAND = 0;
        const AEM_RESPONSE = 1;
        const ADDRESS_ACCESS_COMMAND = 2;
        const ADDRESS_ACCESS_RESPONSE = 3;
        const AVC_COMMAND = 4;
        const AVC_RESPONSE = 5;
        const VENDOR_UNIQUE_COMMAND = 6;
        const VENDOR_UNIQUE_RESPONSE = 7;
        const HDCP_APM_COMMAND = 8;
        const HDCP_APM_RESPONSE = 9;
        const EXTENDED_COMMAND = 14;
        const EXTENDED_RESPONSE = 15;
    }
}

impl AecpMessageType {
    pub const fn is_response(self) -> bool {
        self.0 & 1 == 1
    }
}

code! {
    /// AEM status (Table 7-141); SUCCESS in commands.
    pub struct AemStatus(u8) {
        const SUCCESS = 0;
        const NOT_IMPLEMENTED = 1;
        const NO_SUCH_DESCRIPTOR = 2;
        const ENTITY_LOCKED = 3;
        const ENTITY_ACQUIRED = 4;
        const NOT_AUTHENTICATED = 5;
        const AUTHENTICATION_DISABLED = 6;
        const BAD_ARGUMENTS = 7;
        const NO_RESOURCES = 8;
        /// The entity is still working on the command and will answer
        /// again; each one restarts the controller's timeout.
        const IN_PROGRESS = 9;
        const ENTITY_MISBEHAVING = 10;
        const NOT_SUPPORTED = 11;
        const STREAM_IS_RUNNING = 12;
    }
}

impl AemStatus {
    pub const fn is_success(self) -> bool {
        self.0 == 0
    }
}

code! {
    /// AEM command_type (Table 7-140).
    pub struct AemCommandType(u16) {
        const ACQUIRE_ENTITY = 0x0000;
        const LOCK_ENTITY = 0x0001;
        const ENTITY_AVAILABLE = 0x0002;
        const CONTROLLER_AVAILABLE = 0x0003;
        const READ_DESCRIPTOR = 0x0004;
        const WRITE_DESCRIPTOR = 0x0005;
        const SET_CONFIGURATION = 0x0006;
        const GET_CONFIGURATION = 0x0007;
        const SET_STREAM_FORMAT = 0x0008;
        const GET_STREAM_FORMAT = 0x0009;
        const SET_VIDEO_FORMAT = 0x000a;
        const GET_VIDEO_FORMAT = 0x000b;
        const SET_SENSOR_FORMAT = 0x000c;
        const GET_SENSOR_FORMAT = 0x000d;
        const SET_STREAM_INFO = 0x000e;
        const GET_STREAM_INFO = 0x000f;
        const SET_NAME = 0x0010;
        const GET_NAME = 0x0011;
        const SET_ASSOCIATION_ID = 0x0012;
        const GET_ASSOCIATION_ID = 0x0013;
        const SET_SAMPLING_RATE = 0x0014;
        const GET_SAMPLING_RATE = 0x0015;
        const SET_CLOCK_SOURCE = 0x0016;
        const GET_CLOCK_SOURCE = 0x0017;
        const SET_CONTROL = 0x0018;
        const GET_CONTROL = 0x0019;
        const INCREMENT_CONTROL = 0x001a;
        const DECREMENT_CONTROL = 0x001b;
        const SET_SIGNAL_SELECTOR = 0x001c;
        const GET_SIGNAL_SELECTOR = 0x001d;
        const SET_MIXER = 0x001e;
        const GET_MIXER = 0x001f;
        const SET_MATRIX = 0x0020;
        const GET_MATRIX = 0x0021;
        const START_STREAMING = 0x0022;
        const STOP_STREAMING = 0x0023;
        const REGISTER_UNSOLICITED_NOTIFICATION = 0x0024;
        const DEREGISTER_UNSOLICITED_NOTIFICATION = 0x0025;
        const IDENTIFY_NOTIFICATION = 0x0026;
        const GET_AVB_INFO = 0x0027;
        const GET_AS_PATH = 0x0028;
        const GET_COUNTERS = 0x0029;
        const REBOOT = 0x002a;
        const GET_AUDIO_MAP = 0x002b;
        const ADD_AUDIO_MAPPINGS = 0x002c;
        const REMOVE_AUDIO_MAPPINGS = 0x002d;
        const GET_VIDEO_MAP = 0x002e;
        const ADD_VIDEO_MAPPINGS = 0x002f;
        const REMOVE_VIDEO_MAPPINGS = 0x0030;
        const GET_SENSOR_MAP = 0x0031;
        const ADD_SENSOR_MAPPINGS = 0x0032;
        const REMOVE_SENSOR_MAPPINGS = 0x0033;
        const START_OPERATION = 0x0034;
        const ABORT_OPERATION = 0x0035;
        const OPERATION_STATUS = 0x0036;
        const AUTH_ADD_KEY = 0x0037;
        const AUTH_DELETE_KEY = 0x0038;
        const AUTH_GET_KEY_LIST = 0x0039;
        const AUTH_GET_KEY = 0x003a;
        const AUTH_ADD_KEY_TO_CHAIN = 0x003b;
        const AUTH_DELETE_KEY_FROM_CHAIN = 0x003c;
        const AUTH_GET_KEYCHAIN_LIST = 0x003d;
        const AUTH_GET_IDENTITY = 0x003e;
        const AUTH_ADD_TOKEN = 0x003f;
        const AUTH_DELETE_TOKEN = 0x0040;
        const AUTHENTICATE = 0x0041;
        const DEAUTHENTICATE = 0x0042;
        const ENABLE_TRANSPORT_SECURITY = 0x0043;
        const DISABLE_TRANSPORT_SECURITY = 0x0044;
        const ENABLE_STREAM_ENCRYPTION = 0x0045;
        const DISABLE_STREAM_ENCRYPTION = 0x0046;
        const SET_MEMORY_OBJECT_LENGTH = 0x0047;
        const GET_MEMORY_OBJECT_LENGTH = 0x0048;
        const SET_STREAM_BACKUP = 0x0049;
        const GET_STREAM_BACKUP = 0x004a;
        const GET_DYNAMIC_INFO = 0x004b;
        const SET_MAX_TRANSIT_TIME = 0x004c;
        const GET_MAX_TRANSIT_TIME = 0x004d;
    }
}

/// The fields every AECPDU starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AecpHeader {
    pub message_type: AecpMessageType,
    /// An [`AemStatus`] for AEM messages; vendor unique protocols define
    /// their own.
    pub status: u8,
    pub target_entity_id: EntityId,
    pub controller_entity_id: EntityId,
    pub sequence_id: u16,
}

impl AecpHeader {
    /// Octets of the header, up to and including sequence_id.
    pub const LEN: usize = 22;

    /// Decodes the header and returns it with the octets after it, up to
    /// the end of the control data (Ethernet padding left out). A
    /// control_data_length longer than the frame gives the octets the frame
    /// holds, as some entities count octets they do not send; the command's
    /// own decoder checks it has what it needs.
    pub fn decode(bytes: &[u8]) -> Result<(Self, &[u8]), DecodeError> {
        // controller_entity_id and sequence_id follow the target.
        let header = ControlHeader::decode_expecting(bytes, subtype::AECP, 10)?;
        let end =
            (ControlHeader::COMMON_LEN + usize::from(header.control_data_length)).min(bytes.len());
        Ok((
            Self {
                message_type: AecpMessageType(header.message_type),
                status: header.status,
                target_entity_id: EntityId(read_u64(bytes, 4)),
                controller_entity_id: EntityId(read_u64(bytes, 12)),
                sequence_id: read_u16(bytes, 20),
            },
            &bytes[Self::LEN..end],
        ))
    }

    /// Encodes the header and `body` after it into `out`, returning the
    /// octets written.
    pub fn encode(&self, body: &[&[u8]], out: &mut [u8]) -> Result<usize, EncodeError> {
        let body_length: usize = body.iter().map(|part| part.len()).sum();
        let length = Self::LEN + body_length;
        let control_data_length = u16::try_from(length - ControlHeader::COMMON_LEN)
            .map_err(|_| EncodeError::OutOfRange("control_data_length"))?;
        let out = avtp::buffer(out, length)?;
        ControlHeader {
            subtype: subtype::AECP,
            message_type: self.message_type.0,
            status: self.status,
            control_data_length,
        }
        .encode(out)?;
        avtp::write(out, 4, &self.target_entity_id.to_bytes());
        avtp::write(out, 12, &self.controller_entity_id.to_bytes());
        avtp::write(out, 20, &self.sequence_id.to_be_bytes());
        let mut at = Self::LEN;
        for part in body {
            avtp::write(out, at, part);
            at += part.len();
        }
        Ok(length)
    }
}

/// An AEM command or response: the header, the unsolicited and controller
/// request flags, the command type and the command specific data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AemPdu<'a> {
    pub header: AecpHeader,
    /// An unsolicited notification rather than a response to a command.
    pub unsolicited: bool,
    /// An unsolicited response asking the controller to carry out the
    /// command (the 2021 cr flag).
    pub controller_request: bool,
    pub command_type: AemCommandType,
    pub payload: &'a [u8],
}

impl<'a> AemPdu<'a> {
    pub fn status(&self) -> AemStatus {
        AemStatus(self.header.status)
    }

    /// Decodes an AEM_COMMAND or AEM_RESPONSE.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        let (header, body) = AecpHeader::decode(bytes)?;
        if !matches!(
            header.message_type,
            AecpMessageType::AEM_COMMAND | AecpMessageType::AEM_RESPONSE
        ) {
            return Err(DecodeError::WrongMessageType(header.message_type.0));
        }
        if body.len() < 2 {
            return Err(DecodeError::ShortControlData(
                (body.len() + AecpHeader::LEN - ControlHeader::COMMON_LEN) as u16,
            ));
        }
        let word = read_u16(body, 0);
        Ok(Self {
            header,
            unsolicited: word & 0x8000 != 0,
            controller_request: word & 0x4000 != 0,
            command_type: AemCommandType(word & 0x3fff),
            payload: &body[2..],
        })
    }

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        if self.command_type.0 > 0x3fff {
            return Err(EncodeError::OutOfRange("command_type"));
        }
        let word = self.command_type.0
            | if self.unsolicited { 0x8000 } else { 0 }
            | if self.controller_request { 0x4000 } else { 0 };
        self.header
            .encode(&[&word.to_be_bytes(), self.payload], out)
    }
}

/// A vendor unique command or response: the header, the protocol ID and
/// the protocol's own data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorUniquePdu<'a> {
    pub header: AecpHeader,
    pub protocol_id: [u8; 6],
    pub payload: &'a [u8],
}

impl<'a> VendorUniquePdu<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        let (header, body) = AecpHeader::decode(bytes)?;
        if !matches!(
            header.message_type,
            AecpMessageType::VENDOR_UNIQUE_COMMAND | AecpMessageType::VENDOR_UNIQUE_RESPONSE
        ) {
            return Err(DecodeError::WrongMessageType(header.message_type.0));
        }
        if body.len() < 6 {
            return Err(DecodeError::ShortControlData(
                (body.len() + AecpHeader::LEN - ControlHeader::COMMON_LEN) as u16,
            ));
        }
        Ok(Self {
            header,
            protocol_id: read_array(body, 0),
            payload: &body[6..],
        })
    }

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        self.header.encode(&[&self.protocol_id, self.payload], out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A READ_DESCRIPTOR command for the ENTITY descriptor, laid out by
    /// hand from Figure 7-30.
    const READ_ENTITY: [u8; 32] = [
        0xfb, 0x00, 0x00, 0x14, // AEM_COMMAND, status 0, length 20
        0xe8, 0xf6, 0x0a, 0xe0, 0x92, 0x20, 0x00, 0x00, // target_entity_id
        0x9c, 0x6b, 0x00, 0xff, 0xfe, 0x30, 0x9a, 0x2b, // controller_entity_id
        0x12, 0x34, // sequence_id
        0x00, 0x04, // u 0, cr 0, READ_DESCRIPTOR
        0x00, 0x00, 0x00, 0x00, // configuration_index, reserved
        0x00, 0x00, 0x00, 0x00, // descriptor_type ENTITY, descriptor_index 0
    ];

    #[test]
    fn aem_round_trips() {
        let pdu = AemPdu::decode(&READ_ENTITY).unwrap();
        assert_eq!(pdu.header.message_type, AecpMessageType::AEM_COMMAND);
        assert_eq!(pdu.header.target_entity_id, EntityId(0xe8f6_0ae0_9220_0000));
        assert_eq!(
            pdu.header.controller_entity_id,
            EntityId(0x9c6b_00ff_fe30_9a2b)
        );
        assert_eq!(pdu.header.sequence_id, 0x1234);
        assert_eq!(pdu.command_type, AemCommandType::READ_DESCRIPTOR);
        assert!(!pdu.unsolicited);
        assert_eq!(pdu.payload, &[0; 8]);
        let mut out = [0; 64];
        assert_eq!(pdu.encode(&mut out), Ok(32));
        assert_eq!(out[..32], READ_ENTITY);
    }

    #[test]
    fn unsolicited_and_request_flags() {
        let mut bytes = READ_ENTITY;
        bytes[1] = 0x01;
        bytes[22] = 0xc0;
        let pdu = AemPdu::decode(&bytes).unwrap();
        assert!(pdu.unsolicited);
        assert!(pdu.controller_request);
        assert_eq!(pdu.command_type, AemCommandType::READ_DESCRIPTOR);
        assert!(pdu.header.message_type.is_response());
    }

    #[test]
    fn length_bounds_the_payload_and_padding_is_ignored() {
        let mut padded = [0xee; 40];
        padded[..32].copy_from_slice(&READ_ENTITY);
        assert_eq!(AemPdu::decode(&padded).unwrap().payload.len(), 8);
        // A length longer than the frame gives what the frame holds.
        assert_eq!(AemPdu::decode(&READ_ENTITY[..30]).unwrap().payload.len(), 6);
        assert!(matches!(
            AemPdu::decode(&READ_ENTITY[..20]),
            Err(DecodeError::Truncated { .. })
        ));
    }

    #[test]
    fn vendor_unique_keeps_its_protocol() {
        let header = AecpHeader {
            message_type: AecpMessageType::VENDOR_UNIQUE_COMMAND,
            sequence_id: 7,
            ..AecpHeader::default()
        };
        let pdu = VendorUniquePdu {
            header,
            protocol_id: [0x00, 0x1b, 0xc5, 0x0a, 0xc1, 0x00],
            payload: &[0x00, 0x00, 0x00, 0x00],
        };
        let mut out = [0; 64];
        let length = pdu.encode(&mut out).unwrap();
        assert_eq!(length, 32);
        assert_eq!(out[3], 20);
        assert_eq!(VendorUniquePdu::decode(&out[..length]), Ok(pdu));
        assert!(AemPdu::decode(&out[..length]).is_err());
    }
}
