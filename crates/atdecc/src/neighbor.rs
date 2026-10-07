//! The bridge port this computer is plugged into, from the gPTP messages
//! the bridge sends on the link, heard without taking part in gPTP.

use std::io;
use std::time::{Duration, Instant};

use avb_net::{MacAddress, Socket};

use crate::ClockIdentity;

const ETHERTYPE_GPTP: u16 = 0x88f7;
/// Where gPTP sends its messages, to the neighbor only.
const GPTP_MULTICAST: MacAddress = MacAddress([0x01, 0x80, 0xc2, 0x00, 0x00, 0x0e]);
/// How long the bridge is known after its last peer delay request; it
/// sends one a second.
const NEIGHBOR_TIMEOUT: Duration = Duration::from_secs(5);
/// How long gPTP counts as running after the bridge's last Sync, which it
/// sends eight times a second.
const SYNC_TIMEOUT: Duration = Duration::from_secs(2);

/// The bridge port this computer is plugged into, from the peer delay
/// requests the bridge sends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    pub clock: ClockIdentity,
    pub port: u16,
    /// The bridge sends Sync on the link, as it does once gPTP runs there.
    pub synced: bool,
}

/// Listens to the gPTP messages the neighboring bridge sends this
/// computer.
pub struct NeighborListener {
    socket: Socket,
    /// The bridge port and when it last sent a peer delay request.
    heard: Option<(ClockIdentity, u16, Instant)>,
    last_sync: Option<Instant>,
    reported: Option<Neighbor>,
}

impl NeighborListener {
    /// Listens on `interface`. It fails where the system keeps gPTP from
    /// other programs, or raw Ethernet is out of reach.
    pub fn open(interface: &str) -> io::Result<Self> {
        let socket = Socket::open(interface, ETHERTYPE_GPTP)?;
        socket.join_multicast(GPTP_MULTICAST)?;
        Ok(Self {
            socket,
            heard: None,
            last_sync: None,
            reported: None,
        })
    }

    /// Reads what arrived, returning the neighbor when it changed.
    pub fn poll(&mut self) -> Option<Option<Neighbor>> {
        let neighbor = self.neighbor();
        (neighbor != self.reported).then(|| {
            self.reported = neighbor;
            neighbor
        })
    }

    /// Reads what arrived and returns the neighbor heard lately, if any.
    pub fn neighbor(&mut self) -> Option<Neighbor> {
        let mut buffer = [0; 128];
        let now = Instant::now();
        let own = self.socket.mac();
        while let Ok(Some(received)) = self.socket.receive(&mut buffer, Some(Duration::ZERO)) {
            // The system's own gPTP, where it runs one, is not the bridge.
            if received.source == own {
                continue;
            }
            let Some(message) = GptpMessage::decode(&buffer[..received.length.min(buffer.len())])
            else {
                continue;
            };
            match message.message_type {
                GptpMessage::PDELAY_REQ => self.heard = Some((message.clock, message.port, now)),
                GptpMessage::SYNC => self.last_sync = Some(now),
                _ => {}
            }
        }
        self.heard
            .filter(|(_, _, at)| now.duration_since(*at) < NEIGHBOR_TIMEOUT)
            .map(|(clock, port, _)| Neighbor {
                clock,
                port,
                synced: self
                    .last_sync
                    .is_some_and(|at| now.duration_since(at) < SYNC_TIMEOUT),
            })
    }
}

/// The parts of a gPTP message header the neighbor is told by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GptpMessage {
    message_type: u8,
    clock: ClockIdentity,
    port: u16,
}

impl GptpMessage {
    const SYNC: u8 = 0x0;
    const PDELAY_REQ: u8 = 0x2;

    /// Decodes the common header: the message type, then the sending
    /// port's identity at octet 20.
    fn decode(bytes: &[u8]) -> Option<Self> {
        let header: &[u8; 34] = bytes.get(..34)?.try_into().ok()?;
        let clock: [u8; 8] = header[20..28].try_into().ok()?;
        Some(Self {
            message_type: header[0] & 0x0f,
            clock: ClockIdentity(u64::from_be_bytes(clock)),
            port: u16::from_be_bytes([header[28], header[29]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_delay_requests_name_the_bridge_port() {
        // A bridge's Pdelay_Req, as on the bench: port 6 of the switch.
        let mut frame = [0u8; 54];
        frame[0] = 0x12;
        frame[1] = 0x02;
        frame[20..28].copy_from_slice(&0x0001_f2ff_feff_3b14u64.to_be_bytes());
        frame[28..30].copy_from_slice(&6u16.to_be_bytes());
        let message = GptpMessage::decode(&frame).unwrap();
        assert_eq!(message.message_type, GptpMessage::PDELAY_REQ);
        assert_eq!(message.clock, ClockIdentity(0x0001_f2ff_feff_3b14));
        assert_eq!(message.port, 6);
        assert_eq!(GptpMessage::decode(&frame[..33]), None);
    }
}
