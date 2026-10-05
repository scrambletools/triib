//! ATDECC Connection Management Protocol PDUs (IEEE 1722.1-2021, 8.2.1),
//! with the names Milan 1.3 gives some of them (5.5.2.2).

use avb_net::MacAddress;

use crate::avtp::{self, ControlHeader, read_array, read_u16, read_u64, subtype};
use crate::error::{DecodeError, EncodeError};
use crate::id::{EntityId, StreamId};
use crate::macros::{code, flags};

code! {
    /// ACMP message_type (Table 8-2). Commands are even, and each
    /// response is its command plus one.
    pub struct AcmpMessageType(u8) {
        const CONNECT_TX_COMMAND = 0;
        const CONNECT_TX_RESPONSE = 1;
        const DISCONNECT_TX_COMMAND = 2;
        const DISCONNECT_TX_RESPONSE = 3;
        const GET_TX_STATE_COMMAND = 4;
        const GET_TX_STATE_RESPONSE = 5;
        const CONNECT_RX_COMMAND = 6;
        const CONNECT_RX_RESPONSE = 7;
        const DISCONNECT_RX_COMMAND = 8;
        const DISCONNECT_RX_RESPONSE = 9;
        const GET_RX_STATE_COMMAND = 10;
        const GET_RX_STATE_RESPONSE = 11;
        const GET_TX_CONNECTION_COMMAND = 12;
        const GET_TX_CONNECTION_RESPONSE = 13;
    }
}

impl AcmpMessageType {
    /// Milan's name for CONNECT_TX_COMMAND: a listener asking a talker for
    /// its stream's parameters.
    pub const PROBE_TX_COMMAND: Self = Self::CONNECT_TX_COMMAND;
    pub const PROBE_TX_RESPONSE: Self = Self::CONNECT_TX_RESPONSE;
    /// Milan's name for CONNECT_RX_COMMAND.
    pub const BIND_RX_COMMAND: Self = Self::CONNECT_RX_COMMAND;
    pub const BIND_RX_RESPONSE: Self = Self::CONNECT_RX_RESPONSE;
    /// Milan's name for DISCONNECT_RX_COMMAND.
    pub const UNBIND_RX_COMMAND: Self = Self::DISCONNECT_RX_COMMAND;
    pub const UNBIND_RX_RESPONSE: Self = Self::DISCONNECT_RX_RESPONSE;

    pub const fn is_command(self) -> bool {
        self.0 & 1 == 0
    }

    pub const fn is_response(self) -> bool {
        self.0 & 1 == 1
    }

    /// The response to this command, or this message if it is a response.
    pub const fn response(self) -> Self {
        Self(self.0 | 1)
    }

    /// How long a controller or listener waits for the response to this
    /// command before retrying once (Table 8-1). `None` for responses
    /// and reserved values.
    pub const fn timeout_ms(self) -> Option<u32> {
        match self {
            Self::CONNECT_TX_COMMAND => Some(2000),
            Self::DISCONNECT_TX_COMMAND => Some(200),
            Self::GET_TX_STATE_COMMAND => Some(200),
            Self::CONNECT_RX_COMMAND => Some(4500),
            Self::DISCONNECT_RX_COMMAND => Some(500),
            Self::GET_RX_STATE_COMMAND => Some(200),
            Self::GET_TX_CONNECTION_COMMAND => Some(200),
            _ => None,
        }
    }

    /// The timeout with Milan devices, which answer probes and binds
    /// without waiting on the talker (Milan 1.3, Table 5.26). Commands
    /// Milan does not use keep the standard's timeout.
    pub const fn milan_timeout_ms(self) -> Option<u32> {
        match self {
            Self::PROBE_TX_COMMAND
            | Self::BIND_RX_COMMAND
            | Self::UNBIND_RX_COMMAND
            | Self::GET_TX_STATE_COMMAND
            | Self::GET_RX_STATE_COMMAND => Some(200),
            other => other.timeout_ms(),
        }
    }
}

code! {
    /// ACMP status (Table 8-3); SUCCESS in commands.
    pub struct AcmpStatus(u8) {
        const SUCCESS = 0;
        const LISTENER_UNKNOWN_ID = 1;
        const TALKER_UNKNOWN_ID = 2;
        const TALKER_DEST_MAC_FAIL = 3;
        const TALKER_NO_STREAM_INDEX = 4;
        const TALKER_NO_BANDWIDTH = 5;
        const TALKER_EXCLUSIVE = 6;
        const LISTENER_TALKER_TIMEOUT = 7;
        const LISTENER_EXCLUSIVE = 8;
        const STATE_UNAVAILABLE = 9;
        const NOT_CONNECTED = 10;
        const NO_SUCH_CONNECTION = 11;
        const COULD_NOT_SEND_MESSAGE = 12;
        const TALKER_MISBEHAVING = 13;
        const LISTENER_MISBEHAVING = 14;
        const CONTROLLER_NOT_AUTHORIZED = 16;
        const INCOMPATIBLE_REQUEST = 17;
        const LISTENER_INVALID_CONNECTION = 18;
        const LISTENER_CAN_ONLY_LISTEN_ONCE = 19;
        const NOT_SUPPORTED = 31;
    }
}

impl AcmpStatus {
    pub const fn is_success(self) -> bool {
        self.0 == 0
    }
}

flags! {
    /// ACMP flags (Table 8-4). The standard numbers bits from the most
    /// significant, so its bit 15 is `0x0001`.
    pub struct AcmpFlags(u16) {
        const CLASS_B = 0x0001;
        const FAST_CONNECT = 0x0002;
        const SAVED_STATE = 0x0004;
        const STREAMING_WAIT = 0x0008;
        const SUPPORTS_ENCRYPTED = 0x0010;
        const ENCRYPTED_PDU = 0x0020;
        const SRP_REGISTRATION_FAILED = 0x0040;
        const CL_ENTRIES_VALID = 0x0080;
        const NO_SRP = 0x0100;
        const UDP = 0x0200;
    }
}

impl AcmpFlags {
    /// Milan's name for SRP_REGISTRATION_FAILED (Milan 1.3, Table 5.23).
    pub const REGISTERING_FAILED: Self = Self::SRP_REGISTRATION_FAILED;
}

/// The IP transport fields IEEE 1722.1-2021 appended to the ACMPDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcmpIp {
    pub ip_flags: u16,
    pub source_port: u16,
    pub destination_port: u16,
    /// IPv6, or IPv4 mapped to IPv6.
    pub source_address: [u8; 16],
    pub destination_address: [u8; 16],
}

/// An ACMPDU. The reserved field is zero when encoded and ignored when
/// decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Acmpdu {
    pub message_type: AcmpMessageType,
    pub status: AcmpStatus,
    pub stream_id: StreamId,
    pub controller_entity_id: EntityId,
    pub talker_entity_id: EntityId,
    pub listener_entity_id: EntityId,
    /// The talker's STREAM_OUTPUT index.
    pub talker_unique_id: u16,
    /// The listener's STREAM_INPUT index.
    pub listener_unique_id: u16,
    pub stream_dest_mac: MacAddress,
    pub connection_count: u16,
    pub sequence_id: u16,
    pub flags: AcmpFlags,
    /// Zero when the SRP domain's VLAN applies.
    pub stream_vlan_id: u16,
    pub connected_listeners_entries: u16,
    /// The 2021 IP fields. `None` for the 56-octet form of IEEE
    /// 1722.1-2013, which Milan devices send and must accept; the PDU is
    /// encoded in that form when this is `None`.
    pub ip: Option<AcmpIp>,
}

impl Acmpdu {
    /// Control data octets of the 2013 and Milan form.
    pub const SHORT_CONTROL_DATA_LENGTH: u16 = 44;
    /// Control data octets of the 2021 form, with the IP fields.
    pub const FULL_CONTROL_DATA_LENGTH: u16 = 84;
    pub const SHORT_LEN: usize =
        ControlHeader::COMMON_LEN + Self::SHORT_CONTROL_DATA_LENGTH as usize;
    pub const FULL_LEN: usize = ControlHeader::COMMON_LEN + Self::FULL_CONTROL_DATA_LENGTH as usize;

    /// A command from `controller` with every other field zero, in the
    /// short form.
    pub fn command(
        message_type: AcmpMessageType,
        controller: EntityId,
        talker: (EntityId, u16),
        listener: (EntityId, u16),
        sequence_id: u16,
    ) -> Self {
        Self {
            message_type,
            controller_entity_id: controller,
            talker_entity_id: talker.0,
            talker_unique_id: talker.1,
            listener_entity_id: listener.0,
            listener_unique_id: listener.1,
            sequence_id,
            ..Self::default()
        }
    }

    /// Whether `response` answers this command: the response type, from
    /// the same controller, with the same sequence ID.
    pub fn is_answered_by(&self, response: &Acmpdu) -> bool {
        self.message_type.is_command()
            && response.message_type == self.message_type.response()
            && response.controller_entity_id == self.controller_entity_id
            && response.sequence_id == self.sequence_id
    }

    /// The octets [`encode`](Self::encode) writes.
    pub fn encoded_len(&self) -> usize {
        if self.ip.is_some() {
            Self::FULL_LEN
        } else {
            Self::SHORT_LEN
        }
    }

    /// Decodes an ACMPDU from the octets after the Ethernet header, in
    /// either form. When the frame ends before the IP fields its length
    /// promises, as some senders' do, it decodes in the short form.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let header =
            ControlHeader::decode_expecting(bytes, subtype::ACMP, Self::SHORT_CONTROL_DATA_LENGTH)?;
        let has_ip = header.control_data_length >= Self::FULL_CONTROL_DATA_LENGTH
            && bytes.len() >= Self::FULL_LEN;
        Ok(Self {
            message_type: AcmpMessageType(header.message_type),
            status: AcmpStatus(header.status),
            stream_id: StreamId(read_u64(bytes, 4)),
            controller_entity_id: EntityId(read_u64(bytes, 12)),
            talker_entity_id: EntityId(read_u64(bytes, 20)),
            listener_entity_id: EntityId(read_u64(bytes, 28)),
            talker_unique_id: read_u16(bytes, 36),
            listener_unique_id: read_u16(bytes, 38),
            stream_dest_mac: MacAddress(read_array(bytes, 40)),
            connection_count: read_u16(bytes, 46),
            sequence_id: read_u16(bytes, 48),
            flags: AcmpFlags(read_u16(bytes, 50)),
            stream_vlan_id: read_u16(bytes, 52),
            connected_listeners_entries: read_u16(bytes, 54),
            ip: has_ip.then(|| AcmpIp {
                ip_flags: read_u16(bytes, 56),
                source_port: read_u16(bytes, 60),
                destination_port: read_u16(bytes, 62),
                source_address: read_array(bytes, 64),
                destination_address: read_array(bytes, 80),
            }),
        })
    }

    /// Encodes into the start of `out`, returning the octets written.
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, EncodeError> {
        let length = self.encoded_len();
        let out = avtp::buffer(out, length)?;
        ControlHeader {
            subtype: subtype::ACMP,
            message_type: self.message_type.0,
            status: self.status.0,
            control_data_length: if self.ip.is_some() {
                Self::FULL_CONTROL_DATA_LENGTH
            } else {
                Self::SHORT_CONTROL_DATA_LENGTH
            },
        }
        .encode(out)?;
        avtp::write(out, 4, &self.stream_id.to_bytes());
        avtp::write(out, 12, &self.controller_entity_id.to_bytes());
        avtp::write(out, 20, &self.talker_entity_id.to_bytes());
        avtp::write(out, 28, &self.listener_entity_id.to_bytes());
        avtp::write(out, 36, &self.talker_unique_id.to_be_bytes());
        avtp::write(out, 38, &self.listener_unique_id.to_be_bytes());
        avtp::write(out, 40, &self.stream_dest_mac.0);
        avtp::write(out, 46, &self.connection_count.to_be_bytes());
        avtp::write(out, 48, &self.sequence_id.to_be_bytes());
        avtp::write(out, 50, &self.flags.0.to_be_bytes());
        avtp::write(out, 52, &self.stream_vlan_id.to_be_bytes());
        avtp::write(out, 54, &self.connected_listeners_entries.to_be_bytes());
        if let Some(ip) = &self.ip {
            avtp::write(out, 56, &ip.ip_flags.to_be_bytes());
            avtp::write(out, 58, &[0; 2]);
            avtp::write(out, 60, &ip.source_port.to_be_bytes());
            avtp::write(out, 62, &ip.destination_port.to_be_bytes());
            avtp::write(out, 64, &ip.source_address);
            avtp::write(out, 80, &ip.destination_address);
        }
        Ok(length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GET_RX_STATE_RESPONSE in the short form with a distinct value in
    /// every field, laid out by hand from Figure 8-1.
    const SHORT: [u8; Acmpdu::SHORT_LEN] = [
        0xfc, 0x0b, 0x08, 0x2c, // subtype, GET_RX_STATE_RESPONSE, status 1, length 44
        0xe8, 0xf6, 0x0a, 0xe0, 0x92, 0x20, 0x00, 0x01, // stream_id
        0x9c, 0x6b, 0x00, 0xff, 0xfe, 0x30, 0x9a, 0x2b, // controller_entity_id
        0xe8, 0xf6, 0x0a, 0xe0, 0x92, 0x20, 0x00, 0x00, // talker_entity_id
        0xd1, 0x11, 0xe5, 0x97, 0xf5, 0x44, 0x80, 0x00, // listener_entity_id
        0x00, 0x01, 0x00, 0x02, // talker_unique_id, listener_unique_id
        0x91, 0xe0, 0xf0, 0x00, 0x6a, 0x20, // stream_dest_mac
        0x00, 0x03, // connection_count
        0x41, 0x0d, 0x00, 0x48, // sequence_id, flags
        0x00, 0x02, 0x00, 0x04, // stream_vlan_id, connected_listeners_entries
    ];

    fn short() -> Acmpdu {
        Acmpdu {
            message_type: AcmpMessageType::GET_RX_STATE_RESPONSE,
            status: AcmpStatus::LISTENER_UNKNOWN_ID,
            stream_id: StreamId(0xe8f6_0ae0_9220_0001),
            controller_entity_id: EntityId(0x9c6b_00ff_fe30_9a2b),
            talker_entity_id: EntityId(0xe8f6_0ae0_9220_0000),
            listener_entity_id: EntityId(0xd111_e597_f544_8000),
            talker_unique_id: 1,
            listener_unique_id: 2,
            stream_dest_mac: MacAddress([0x91, 0xe0, 0xf0, 0x00, 0x6a, 0x20]),
            connection_count: 3,
            sequence_id: 0x410d,
            flags: AcmpFlags::REGISTERING_FAILED | AcmpFlags::STREAMING_WAIT,
            stream_vlan_id: 2,
            connected_listeners_entries: 4,
            ip: None,
        }
    }

    fn ip() -> AcmpIp {
        let mut source_address = [0; 16];
        source_address[10..].copy_from_slice(&[0xff, 0xff, 192, 168, 4, 10]);
        let mut destination_address = [0; 16];
        destination_address[10..].copy_from_slice(&[0xff, 0xff, 239, 1, 2, 3]);
        AcmpIp {
            ip_flags: 0x0001,
            source_port: 17220,
            destination_port: 17221,
            source_address,
            destination_address,
        }
    }

    #[test]
    fn short_form_decodes_and_encodes_every_field() {
        assert_eq!(Acmpdu::decode(&SHORT), Ok(short()));
        let mut bytes = [0; Acmpdu::FULL_LEN];
        assert_eq!(short().encode(&mut bytes), Ok(Acmpdu::SHORT_LEN));
        assert_eq!(bytes[..Acmpdu::SHORT_LEN], SHORT);
    }

    #[test]
    fn full_form_carries_the_ip_fields() {
        let full = Acmpdu {
            ip: Some(ip()),
            ..short()
        };
        let mut bytes = [0; Acmpdu::FULL_LEN];
        assert_eq!(full.encode(&mut bytes), Ok(Acmpdu::FULL_LEN));
        assert_eq!(bytes[3], 84);
        assert_eq!(
            &bytes[56..64],
            &[0x00, 0x01, 0x00, 0x00, 0x43, 0x44, 0x43, 0x45]
        );
        assert_eq!(&bytes[76..80], &[192, 168, 4, 10]);
        assert_eq!(&bytes[92..96], &[239, 1, 2, 3]);
        assert_eq!(Acmpdu::decode(&bytes), Ok(full));
    }

    #[test]
    fn a_full_length_that_the_frame_does_not_hold_decodes_short() {
        let mut bytes = [0; Acmpdu::FULL_LEN - 8];
        bytes[..Acmpdu::SHORT_LEN].copy_from_slice(&SHORT);
        bytes[3] = 84;
        assert_eq!(Acmpdu::decode(&bytes), Ok(short()));
    }

    #[test]
    fn refuses_short_frames_and_lengths() {
        assert_eq!(
            Acmpdu::decode(&SHORT[..55]),
            Err(DecodeError::Truncated {
                needed: 56,
                available: 55
            })
        );
        let mut bytes = SHORT;
        bytes[3] = 43;
        assert_eq!(
            Acmpdu::decode(&bytes),
            Err(DecodeError::ShortControlData(43))
        );
        assert_eq!(
            short().encode(&mut [0; 40]),
            Err(EncodeError::BufferTooSmall {
                needed: 56,
                available: 40
            })
        );
        let mut status = short();
        status.status = AcmpStatus(32);
        assert_eq!(
            status.encode(&mut [0; 56]),
            Err(EncodeError::OutOfRange("status"))
        );
    }

    #[test]
    fn commands_pair_with_their_responses() {
        let command = Acmpdu::command(
            AcmpMessageType::BIND_RX_COMMAND,
            EntityId(1),
            (EntityId(2), 0),
            (EntityId(3), 1),
            7,
        );
        let mut response = command;
        response.message_type = AcmpMessageType::BIND_RX_RESPONSE;
        assert!(command.is_answered_by(&response));
        response.sequence_id = 8;
        assert!(!command.is_answered_by(&response));
        assert!(!response.is_answered_by(&command));
        assert_eq!(AcmpMessageType::CONNECT_RX_COMMAND.timeout_ms(), Some(4500));
        assert_eq!(
            AcmpMessageType::BIND_RX_COMMAND.milan_timeout_ms(),
            Some(200)
        );
        assert_eq!(
            AcmpMessageType::DISCONNECT_TX_COMMAND.milan_timeout_ms(),
            Some(200)
        );
        assert_eq!(AcmpMessageType::CONNECT_RX_RESPONSE.timeout_ms(), None);
    }
}
