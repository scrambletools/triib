//! AVB Lite (the AVB Lite profile): telling from the Pdelay exchange
//! whether an AVB bridge is between this computer and its peers, falling
//! back to AVB Lite when none is, and the CVU SRP messages that carry
//! MSRP declarations there, as AECP vendor unique commands.

use std::collections::VecDeque;
use std::time::Duration;

use atdecc::aecp::{AecpHeader, AecpMessageType, VendorUniquePdu};
use atdecc::lite::{CVU_PROTOCOL_ID, CvuMessage, FallbackReason};
use atdecc::{EntityId, MacAddress};
use avb_mrp::msrp::{self, ListenerState, TalkerDeclaration};
use avb_mrp::{Event, mrpdu};

/// PTP's ethertype, and where gPTP's peer delay messages go.
pub const PTP_ETHERTYPE: u16 = 0x88f7;
pub const PDELAY_DESTINATION: MacAddress = MacAddress([0x01, 0x80, 0xc2, 0x00, 0x00, 0x0e]);

/// PTP message types.
const PDELAY_REQ: u8 = 0x2;
const PDELAY_RESP: u8 = 0x3;
const PDELAY_RESP_FOLLOW_UP: u8 = 0xa;
/// Octets of a Pdelay message before any TLV.
const PDELAY_LEN: usize = 54;
/// The Endpoint Declaration TLV (profile 2.1): an organization extension
/// under the AVB Lite MA-S, saying the sender is an endpoint.
const ENDPOINT_TLV: [u8; 12] = [
    0x00, 0x03, 0x00, 0x08, 0x8c, 0x1f, 0x64, 0x36, 0xc0, 0x01, 0x01, 0x00,
];

/// How often a Pdelay_Req goes out, and after fallback the beacon.
const PROBE_INTERVAL: Duration = Duration::from_secs(1);
const BEACON_INTERVAL: Duration = Duration::from_secs(3);
/// Unanswered requests that make the fallback (profile 2.2, condition 2).
const MOST_UNANSWERED: u32 = 9;
/// How long ptp4l may say it has no gPTP peer before that counts as the
/// same.
const PEERLESS: Duration = Duration::from_secs(10);

/// A Pdelay_Req from `identity` (clock identity, port 1) with the
/// Endpoint Declaration TLV, as esp_ptp sends its own and its beacons.
pub fn pdelay_request(identity: u64, sequence: u16) -> Vec<u8> {
    let mut message = vec![0; PDELAY_LEN];
    // gPTP (majorSdoId 1), Pdelay_Req; PTP version 2.
    message[0] = 0x10 | PDELAY_REQ;
    message[1] = 0x02;
    message[2..4].copy_from_slice(&((PDELAY_LEN + ENDPOINT_TLV.len()) as u16).to_be_bytes());
    // The PTP timescale flag.
    message[7] = 0x08;
    message[20..28].copy_from_slice(&identity.to_be_bytes());
    message[28..30].copy_from_slice(&1u16.to_be_bytes());
    message[30..32].copy_from_slice(&sequence.to_be_bytes());
    message[32] = 5;
    message.extend_from_slice(&ENDPOINT_TLV);
    message
}

/// Whether a PTP message's TLVs, after its first `start` octets, hold
/// the Endpoint Declaration TLV.
fn has_endpoint_tlv(message: &[u8], start: usize) -> bool {
    let length = message
        .get(2..4)
        .map_or(0, |octets| {
            usize::from(u16::from_be_bytes([octets[0], octets[1]]))
        })
        .min(message.len());
    let mut at = start;
    while at + 4 <= length {
        let kind = u16::from_be_bytes([message[at], message[at + 1]]);
        let size = usize::from(u16::from_be_bytes([message[at + 2], message[at + 3]]));
        let Some(body) = message.get(at + 4..at + 4 + size) else {
            return false;
        };
        // The identifiers and dataField; a pad octet may follow.
        if kind == 0x0003 && body.len() >= 7 && body[..7] == ENDPOINT_TLV[4..11] {
            return true;
        }
        at += 4 + size;
    }
    false
}

/// Decides between AVB and AVB Lite from the Pdelay exchange (profile
/// 2.2): an Endpoint Declaration TLV heard, nine requests unanswered, or
/// two responders to one request each make the fallback, which then
/// holds. Where ptp4l runs gPTP on the interface it asks the peer itself,
/// and ptp4l's word on the peer stands for the answers.
pub struct Fallback {
    identity: u64,
    sequence: u16,
    /// Whether this sends its own Pdelay_Req: when ptp4l does not.
    probing: bool,
    /// The request waiting for answers, and who answered it.
    waiting: Option<u16>,
    responders: Vec<[u8; 10]>,
    unanswered: u32,
    /// Since when ptp4l has said it has no peer.
    peerless_since: Option<Duration>,
    reason: Option<FallbackReason>,
    next: Duration,
    outgoing: VecDeque<Vec<u8>>,
}

impl Fallback {
    /// For the interface whose clock identity is `identity`; `configured`
    /// falls back at once, as an operator asked.
    pub fn new(identity: u64, configured: bool, now: Duration) -> Self {
        Fallback {
            identity,
            sequence: 0,
            probing: false,
            waiting: None,
            responders: Vec::new(),
            unanswered: 0,
            peerless_since: None,
            reason: configured.then_some(FallbackReason::CONFIGURED),
            next: now,
            outgoing: VecDeque::new(),
        }
    }

    pub fn reason(&self) -> Option<FallbackReason> {
        self.reason
    }

    /// What ptp4l says of the interface: `None` when it does not run
    /// gPTP there, which this then asks the peer itself for; else whether
    /// it has a gPTP peer.
    pub fn set_ptp4l(&mut self, now: Duration, peer: Option<bool>) {
        self.probing = peer.is_none();
        match peer {
            Some(false) => {
                let since = *self.peerless_since.get_or_insert(now);
                if now.saturating_sub(since) >= PEERLESS {
                    self.fall_back(FallbackReason::PDELAY_UNANSWERED);
                }
            }
            _ => self.peerless_since = None,
        }
    }

    fn fall_back(&mut self, reason: FallbackReason) {
        if self.reason.is_none() {
            self.reason = Some(reason);
        }
    }

    /// Hands it a PTP message heard on the interface, after the
    /// ethertype.
    pub fn handle_ptp(&mut self, message: &[u8]) {
        let Some(&first) = message.first() else {
            return;
        };
        if message.len() < PDELAY_LEN {
            return;
        }
        let kind = first & 0x0f;
        if matches!(kind, PDELAY_REQ | PDELAY_RESP | PDELAY_RESP_FOLLOW_UP)
            && has_endpoint_tlv(message, PDELAY_LEN)
        {
            self.fall_back(FallbackReason::ENDPOINT_TLV);
        }
        if kind != PDELAY_RESP {
            return;
        }
        let sequence = u16::from_be_bytes([message[30], message[31]]);
        let requester = u64::from_be_bytes(message[44..52].try_into().unwrap_or_default());
        if self.waiting != Some(sequence) || requester != self.identity {
            return;
        }
        let mut responder = [0; 10];
        responder.copy_from_slice(&message[20..30]);
        if !self.responders.contains(&responder) {
            self.responders.push(responder);
        }
        if self.responders.len() >= 2 {
            self.fall_back(FallbackReason::MULTIPLE_RESPONDERS);
        }
    }

    pub fn handle_timeout(&mut self, now: Duration) {
        if now < self.next {
            return;
        }
        if self.reason.is_some() {
            // An endpoint that fell back says so every 3 s, for peers
            // still in gPTP (profile 2.3).
            self.send_request();
            self.next = now + BEACON_INTERVAL;
            return;
        }
        if !self.probing {
            self.waiting = None;
            self.next = now + PROBE_INTERVAL;
            return;
        }
        if self.waiting.is_some() && self.responders.is_empty() {
            self.unanswered += 1;
            if self.unanswered >= MOST_UNANSWERED {
                self.fall_back(FallbackReason::PDELAY_UNANSWERED);
            }
        } else if self.waiting.is_some() {
            self.unanswered = 0;
        }
        self.responders.clear();
        self.waiting = Some(self.send_request());
        self.next = now + PROBE_INTERVAL;
    }

    fn send_request(&mut self) -> u16 {
        let sequence = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        self.outgoing
            .push_back(pdelay_request(self.identity, sequence));
        sequence
    }

    pub fn poll_timeout(&self) -> Duration {
        self.next
    }

    /// A PTP message to send to [`PDELAY_DESTINATION`].
    pub fn poll_transmit(&mut self) -> Option<Vec<u8>> {
        self.outgoing.pop_front()
    }
}

/// An AECP vendor unique command carrying one MSRP message as CVU SRP
/// (profile 6), from `sender`.
pub fn cvu_command(sender: EntityId, sequence: u16, message: &mrpdu::Message) -> Option<Vec<u8>> {
    let pdu = mrpdu::encode(std::slice::from_ref(message), &msrp::FORMAT);
    // The MSRP message alone: no ProtocolVersion before it, nor the
    // MRPDU's EndMark after.
    let msrp = pdu.get(1..pdu.len().checked_sub(2)?)?;
    let mut payload = vec![message.attribute_type];
    payload.extend_from_slice(msrp);
    let command = VendorUniquePdu {
        header: AecpHeader {
            message_type: AecpMessageType::VENDOR_UNIQUE_COMMAND,
            status: 0,
            target_entity_id: EntityId(0),
            controller_entity_id: sender,
            sequence_id: sequence,
        },
        protocol_id: CVU_PROTOCOL_ID,
        payload: &payload,
    };
    let mut out = vec![0; 1500];
    let length = command.encode(&mut out).ok()?;
    out.truncate(length);
    Some(out)
}

/// The response to a CVU SRP command, which the command's octets give
/// back with SUCCESS.
pub fn cvu_response(command: &[u8]) -> Option<Vec<u8>> {
    let pdu = VendorUniquePdu::decode(command).ok()?;
    let response = VendorUniquePdu {
        header: AecpHeader {
            message_type: AecpMessageType::VENDOR_UNIQUE_RESPONSE,
            status: 0,
            ..pdu.header
        },
        ..pdu
    };
    let mut out = vec![0; command.len().max(64)];
    let length = response.encode(&mut out).ok()?;
    out.truncate(length);
    Some(out)
}

/// What one CVU SRP command declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declared {
    /// A talker's stream, or that it is gone.
    Talker(TalkerDeclaration),
    TalkerGone(u64),
    /// A listener's word on a stream.
    Listener(u64, ListenerState),
    ListenerGone(u64),
}

/// The declarations of a CVU SRP command, when the AECP PDU is one.
pub fn cvu_declarations(pdu: &[u8]) -> Option<Vec<Declared>> {
    let pdu = VendorUniquePdu::decode(pdu).ok()?;
    if pdu.header.message_type != AecpMessageType::VENDOR_UNIQUE_COMMAND {
        return None;
    }
    let message = CvuMessage::from_pdu(&pdu).ok()?;
    let mut mrpdu = vec![0];
    mrpdu.extend_from_slice(message.msrp);
    mrpdu.extend_from_slice(&[0, 0]);
    let decoded = mrpdu::decode(&mrpdu, &msrp::FORMAT);
    let stream = |value: &[u8]| {
        value
            .get(..8)
            .and_then(|octets| octets.try_into().ok())
            .map(u64::from_be_bytes)
    };
    let declared = decoded
        .values
        .into_iter()
        .filter_map(|value| {
            let declares = matches!(value.event, Event::New | Event::JoinIn | Event::JoinMt);
            let leaves = value.event == Event::Lv;
            match value.attribute_type {
                msrp::attribute::TALKER_ADVERTISE | msrp::attribute::TALKER_FAILED => {
                    if declares {
                        msrp::decode_talker(value.attribute_type, &value.value, value.event)
                            .map(Declared::Talker)
                    } else if leaves {
                        stream(&value.value).map(Declared::TalkerGone)
                    } else {
                        None
                    }
                }
                msrp::attribute::LISTENER => {
                    let id = stream(&value.value)?;
                    if declares {
                        Some(Declared::Listener(
                            id,
                            ListenerState::from_number(value.four_packed.unwrap_or(0)),
                        ))
                    } else if leaves {
                        Some(Declared::ListenerGone(id))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect();
    Some(declared)
}

/// The MSRP message declaring a talker's stream, or withdrawing it.
pub fn talker_message(declaration: &TalkerDeclaration, event: Event) -> mrpdu::Message {
    mrpdu::Message {
        attribute_type: if declaration.failure.is_some() {
            msrp::attribute::TALKER_FAILED
        } else {
            msrp::attribute::TALKER_ADVERTISE
        },
        leave_all: false,
        values: vec![(msrp::talker_value(declaration), event, None)],
    }
}

/// The MSRP message giving a listener's word on `stream_id`, or
/// withdrawing it.
pub fn listener_message(stream_id: u64, state: ListenerState, event: Event) -> mrpdu::Message {
    mrpdu::Message {
        attribute_type: msrp::attribute::LISTENER,
        leave_all: false,
        values: vec![(
            stream_id.to_be_bytes().to_vec(),
            event,
            Some(state.number()),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: u64 = 0x9c6b_00ff_fe30_9a2b;

    /// A Pdelay_Resp from `responder` to request `sequence` of
    /// `requester`, with the Endpoint Declaration TLV or not.
    fn response(responder: u64, requester: u64, sequence: u16, endpoint: bool) -> Vec<u8> {
        let mut message = vec![0; PDELAY_LEN];
        message[0] = 0x10 | PDELAY_RESP;
        message[1] = 0x02;
        let length = PDELAY_LEN + if endpoint { ENDPOINT_TLV.len() } else { 0 };
        message[2..4].copy_from_slice(&(length as u16).to_be_bytes());
        message[20..28].copy_from_slice(&responder.to_be_bytes());
        message[28..30].copy_from_slice(&1u16.to_be_bytes());
        message[30..32].copy_from_slice(&sequence.to_be_bytes());
        message[44..52].copy_from_slice(&requester.to_be_bytes());
        message[52..54].copy_from_slice(&1u16.to_be_bytes());
        if endpoint {
            message.extend_from_slice(&ENDPOINT_TLV);
        }
        message
    }

    /// Runs the detector from second `from` to `until`, answering each
    /// request with `answer`.
    fn run(fallback: &mut Fallback, from: u64, until: u64, answer: impl Fn(u16) -> Vec<Vec<u8>>) {
        for tick in from * 10..until * 10 {
            let now = Duration::from_millis(tick * 100);
            fallback.handle_timeout(now);
            while let Some(request) = fallback.poll_transmit() {
                let sequence = u16::from_be_bytes([request[30], request[31]]);
                for message in answer(sequence) {
                    fallback.handle_ptp(&message);
                }
            }
        }
    }

    #[test]
    fn requests_carry_the_endpoint_declaration() {
        let request = pdelay_request(IDENTITY, 7);
        assert_eq!(request.len(), 66);
        assert_eq!(&request[..4], &[0x12, 0x02, 0x00, 0x42]);
        assert_eq!(&request[54..], &ENDPOINT_TLV);
        assert!(has_endpoint_tlv(&request, PDELAY_LEN));
        assert!(!has_endpoint_tlv(&request[..PDELAY_LEN], PDELAY_LEN));
        // An endpoint of the profile's first revision, its TLV seven
        // octets long with no pad, is still recognized.
        let mut older = request[..PDELAY_LEN].to_vec();
        older[2..4].copy_from_slice(&65u16.to_be_bytes());
        older.extend_from_slice(&[0x00, 0x03, 0x00, 0x07]);
        older.extend_from_slice(&ENDPOINT_TLV[4..11]);
        assert!(has_endpoint_tlv(&older, PDELAY_LEN));
    }

    #[test]
    fn a_single_bridge_answering_keeps_avb() {
        let mut fallback = Fallback::new(IDENTITY, false, Duration::ZERO);
        fallback.set_ptp4l(Duration::ZERO, None);
        run(&mut fallback, 0, 30, |sequence| {
            vec![response(0x0001_f2ff_feff_3b14, IDENTITY, sequence, false)]
        });
        assert_eq!(fallback.reason(), None);
    }

    #[test]
    fn nine_unanswered_requests_fall_back() {
        let mut fallback = Fallback::new(IDENTITY, false, Duration::ZERO);
        fallback.set_ptp4l(Duration::ZERO, None);
        run(&mut fallback, 0, 8, |_| Vec::new());
        assert_eq!(fallback.reason(), None);
        run(&mut fallback, 8, 11, |_| Vec::new());
        assert_eq!(fallback.reason(), Some(FallbackReason::PDELAY_UNANSWERED));
    }

    #[test]
    fn two_responders_or_an_endpoint_fall_back() {
        let mut fallback = Fallback::new(IDENTITY, false, Duration::ZERO);
        fallback.set_ptp4l(Duration::ZERO, None);
        run(&mut fallback, 0, 3, |sequence| {
            vec![
                response(0x1111_11ff_fe11_1111, IDENTITY, sequence, false),
                response(0x2222_22ff_fe22_2222, IDENTITY, sequence, false),
            ]
        });
        assert_eq!(fallback.reason(), Some(FallbackReason::MULTIPLE_RESPONDERS));

        let mut fallback = Fallback::new(IDENTITY, false, Duration::ZERO);
        fallback.set_ptp4l(Duration::ZERO, Some(true));
        // Another endpoint's request, heard through a plain switch.
        fallback.handle_ptp(&pdelay_request(0x3333_33ff_fe33_3333, 1));
        assert_eq!(fallback.reason(), Some(FallbackReason::ENDPOINT_TLV));
    }

    #[test]
    fn where_ptp4l_runs_it_neither_asks_nor_falls_back_while_ptp4l_has_a_peer() {
        let mut fallback = Fallback::new(IDENTITY, false, Duration::ZERO);
        for second in 0..30 {
            fallback.set_ptp4l(Duration::from_secs(second), Some(true));
        }
        run(&mut fallback, 0, 30, |_| Vec::new());
        assert_eq!(fallback.reason(), None);
        // Without a peer for 10 s, it falls back.
        for second in 30..41 {
            fallback.set_ptp4l(Duration::from_secs(second), Some(false));
        }
        assert_eq!(fallback.reason(), Some(FallbackReason::PDELAY_UNANSWERED));
    }

    #[test]
    fn after_falling_back_a_beacon_goes_out_every_3_s() {
        let mut fallback = Fallback::new(IDENTITY, true, Duration::ZERO);
        assert_eq!(fallback.reason(), Some(FallbackReason::CONFIGURED));
        let mut beacons = 0;
        for tick in 0..90 {
            fallback.handle_timeout(Duration::from_millis(tick * 100));
            while let Some(beacon) = fallback.poll_transmit() {
                assert!(has_endpoint_tlv(&beacon, PDELAY_LEN));
                beacons += 1;
            }
        }
        assert_eq!(beacons, 3);
    }

    #[test]
    fn cvu_commands_carry_one_msrp_message() {
        let declaration = TalkerDeclaration {
            stream_id: 0x9c6b_0030_9a2b_0000,
            destination: MacAddress([0x91, 0xe0, 0xf0, 0x00, 0x12, 0x34]),
            vlan_id: 2,
            max_frame_size: 224,
            max_interval_frames: 1,
            priority: 5,
            rank: true,
            accumulated_latency: 125_000,
            failure: None,
            event: Event::JoinIn,
        };
        let sender = EntityId(0x9c6b_00ff_0030_9a2b);
        let command = cvu_command(sender, 3, &talker_message(&declaration, Event::JoinIn)).unwrap();
        let pdu = VendorUniquePdu::decode(&command).unwrap();
        assert_eq!(pdu.protocol_id, CVU_PROTOCOL_ID);
        assert_eq!(pdu.header.target_entity_id, EntityId(0));
        // The command type, then the message: type, length, list length.
        assert_eq!(&pdu.payload[..3], &[1, 1, 25]);
        let declared = cvu_declarations(&command).unwrap();
        assert!(
            matches!(&declared[..], [Declared::Talker(found)] if found.stream_id == declaration.stream_id
                && found.destination == declaration.destination
                && found.max_frame_size == 224)
        );
        let ready = cvu_command(
            sender,
            4,
            &listener_message(declaration.stream_id, ListenerState::Ready, Event::JoinIn),
        )
        .unwrap();
        assert_eq!(
            cvu_declarations(&ready).unwrap(),
            [Declared::Listener(
                declaration.stream_id,
                ListenerState::Ready
            )]
        );
        let gone = cvu_command(
            sender,
            5,
            &listener_message(declaration.stream_id, ListenerState::Ready, Event::Lv),
        )
        .unwrap();
        assert_eq!(
            cvu_declarations(&gone).unwrap(),
            [Declared::ListenerGone(declaration.stream_id)]
        );
        // The response gives the command back.
        let response = cvu_response(&command).unwrap();
        let answered = VendorUniquePdu::decode(&response).unwrap();
        assert_eq!(
            answered.header.message_type,
            AecpMessageType::VENDOR_UNIQUE_RESPONSE
        );
        assert_eq!(answered.payload, pdu.payload);
        assert_eq!(cvu_declarations(&response), None);
    }
}
