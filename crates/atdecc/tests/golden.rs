//! Every ADP and ACMP frame of a capture from real devices decodes to the
//! values tshark's independent dissector reads, and encodes back to the
//! same octets. See captures/README.md for how the capture was made.

mod common;

use std::collections::HashMap;

use common::{ethernet_payload, expected_rows, number, pcap_frames, pcap_records};

use atdecc::avtp::ControlHeader;
use atdecc::controller::{Config, Controller, Event, OfflineReason};
use atdecc::{Acmpdu, Adpdu, EntityId, Instant, MacAddress, Pdu};

const CAPTURE: &[u8] = include_bytes!("captures/bench-adp-acmp.pcap");
const EXPECTED: &str = include_str!("captures/bench-adp-acmp.tsv");

fn check(row: &HashMap<&str, &str>, frame: &str, field: &str, actual: u64) {
    let expected = row
        .get(field)
        .unwrap_or_else(|| panic!("frame {frame}: tshark has no {field}"));
    assert_eq!(number(expected), actual, "frame {frame}: {field}");
}

#[test]
fn bench_capture_matches_tshark() {
    let frames = pcap_frames(CAPTURE);
    let rows = expected_rows(EXPECTED);
    assert_eq!(frames.len(), rows.len(), "one tshark row per frame");
    let (mut adp, mut acmp, mut other) = (0, 0, 0);

    for (frame, row) in frames.iter().zip(&rows) {
        let number_text = row["frame.number"];
        let (ethertype, avtpdu) = ethernet_payload(frame);
        assert_eq!(ethertype, atdecc::ETHERTYPE_AVTP, "frame {number_text}");
        let header = ControlHeader::decode(avtpdu).unwrap();
        let length = ControlHeader::COMMON_LEN + usize::from(header.control_data_length);
        let complete = avtpdu.len() >= length;
        let pdu =
            atdecc::decode(avtpdu).unwrap_or_else(|error| panic!("frame {number_text}: {error}"));
        check(
            row,
            number_text,
            "ieee1722.subtype",
            u64::from(header.subtype),
        );

        match pdu {
            Pdu::Adp(adpdu) => {
                adp += 1;
                let check = |field, actual| check(row, number_text, field, actual);
                check("ieee17221.message_type", u64::from(adpdu.message_type.0));
                check("ieee17221.valid_time", u64::from(adpdu.valid_time));
                check("ieee17221.control_data_length", 56);
                check("ieee17221.entity_id", adpdu.entity_id.0);
                check("ieee17221.entity_model_id", adpdu.entity_model_id.0);
                check(
                    "ieee17221.entity_capabilities",
                    u64::from(adpdu.entity_capabilities.0),
                );
                check(
                    "ieee17221.talker_stream_sources",
                    u64::from(adpdu.talker_stream_sources),
                );
                check(
                    "ieee17221.talker_capabilities",
                    u64::from(adpdu.talker_capabilities.0),
                );
                check(
                    "ieee17221.listener_stream_sinks",
                    u64::from(adpdu.listener_stream_sinks),
                );
                check(
                    "ieee17221.listener_capabilities",
                    u64::from(adpdu.listener_capabilities.0),
                );
                check(
                    "ieee17221.controller_capabilities",
                    u64::from(adpdu.controller_capabilities.0),
                );
                check(
                    "ieee17221.available_index",
                    u64::from(adpdu.available_index),
                );
                check("ieee17221.gptp_grandmaster_id", adpdu.gptp_grandmaster_id.0);
                check("ieee17221.association_id", adpdu.association_id);
                assert!(
                    complete,
                    "frame {number_text}: ADPDU shorter than its length"
                );
                assert_eq!(
                    adpdu.to_bytes().unwrap(),
                    avtpdu[..Adpdu::LEN],
                    "frame {number_text}: re-encoded ADPDU differs"
                );
            }
            Pdu::Acmp(acmpdu) => {
                acmp += 1;
                let check = |field, actual| check(row, number_text, field, actual);
                check("ieee17221.message_type", u64::from(acmpdu.message_type.0));
                check("ieee17221.status_field", u64::from(acmpdu.status.0));
                check(
                    "ieee17221.control_data_length",
                    u64::from(header.control_data_length),
                );
                check("ieee17221.stream_id", acmpdu.stream_id.0);
                check("ieee17221.controller_guid", acmpdu.controller_entity_id.0);
                check("ieee17221.talker_guid", acmpdu.talker_entity_id.0);
                check("ieee17221.listener_guid", acmpdu.listener_entity_id.0);
                check(
                    "ieee17221.talker_unique_id",
                    u64::from(acmpdu.talker_unique_id),
                );
                check(
                    "ieee17221.listener_unique_id",
                    u64::from(acmpdu.listener_unique_id),
                );
                check(
                    "ieee17221.connection_count",
                    u64::from(acmpdu.connection_count),
                );
                check("ieee17221.sequence_id", u64::from(acmpdu.sequence_id));
                check("ieee17221.flags", u64::from(acmpdu.flags.0));
                check("ieee17221.vlan_id", u64::from(acmpdu.stream_vlan_id));
                assert_eq!(
                    row["ieee17221.dest_mac"],
                    acmpdu.stream_dest_mac.to_string(),
                    "frame {number_text}: stream_dest_mac"
                );

                let mut encoded = [0; Acmpdu::FULL_LEN];
                let written = acmpdu.encode(&mut encoded).unwrap();
                if complete {
                    assert_eq!(
                        encoded[..written],
                        avtpdu[..length],
                        "frame {number_text}: re-encoded ACMPDU differs"
                    );
                } else {
                    // A length promising IP fields the frame does not hold
                    // decodes, and encodes, in the short form.
                    assert!(acmpdu.ip.is_none(), "frame {number_text}");
                    assert_eq!(written, Acmpdu::SHORT_LEN);
                    assert_eq!(encoded[4..written], avtpdu[4..Acmpdu::SHORT_LEN]);
                }
            }
            Pdu::Aem(_) | Pdu::VendorUnique(_) | Pdu::Aecp(..) => {
                panic!("frame {number_text}: no AECP expected")
            }
            Pdu::Other { .. } => other += 1,
        }
    }

    // The capture holds ADP from three entities and a discover, ACMP
    // commands and responses in both forms, and MAAP.
    assert_eq!((adp, acmp, other), (10, 30, 2));
}

/// Replaying the capture, with its timing, through a controller discovers
/// the three entities and nothing else, then forgets them once their
/// valid time runs out.
#[test]
fn bench_capture_replays_through_discovery() {
    let records = pcap_records(CAPTURE);
    let start = records[0].0;
    let mut config = Config::new(EntityId(0x9c6b_00ff_fe30_9a2b));
    // Only discovery: the capture holds no responses to read descriptors.
    config.enumerate = false;
    let mut controller = Controller::new(config);
    let mut last = Instant::ZERO;
    for (time, frame) in &records {
        let now = Instant::from_nanos((*time - start).as_nanos() as u64);
        let source = MacAddress(frame[6..12].try_into().unwrap());
        let (_, avtpdu) = ethernet_payload(frame);
        controller.handle_frame(now, source, avtpdu).unwrap();
        controller.handle_timeout(now);
        last = now;
    }
    let online: Vec<Event> = std::iter::from_fn(|| controller.poll_event()).collect();
    let expected = [
        EntityId(0xd111_e597_f544_8000),
        EntityId(0xe8f6_0ae0_9220_0000),
        EntityId(0xfc01_2cfd_fe80_0000),
    ];
    let mut seen: Vec<EntityId> = online
        .iter()
        .map(|event| match event {
            Event::EntityOnline(entity_id) => *entity_id,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    seen.sort();
    assert_eq!(seen, expected);
    assert_eq!(controller.malformed_frames(), 0);
    let macos = controller.entity(expected[0]).unwrap();
    assert_eq!(macos.mac.to_string(), "d0:11:e5:97:f5:44");
    assert_eq!(macos.adp.valid_time, 8);

    // Nothing more is heard: each is forgotten at its own deadline.
    controller.handle_timeout(controller.poll_timeout().unwrap());
    let first_gone: Vec<Event> = std::iter::from_fn(|| controller.poll_event()).collect();
    assert_eq!(
        first_gone.len(),
        1,
        "the macOS entity has the shortest valid time"
    );
    controller.handle_timeout(last + std::time::Duration::from_secs(20));
    let rest: Vec<Event> = std::iter::from_fn(|| controller.poll_event()).collect();
    assert!(
        rest.iter()
            .all(|event| matches!(event, Event::EntityOffline(_, OfflineReason::TimedOut)))
    );
    assert_eq!(rest.len(), 2);
    assert_eq!(controller.entities().count(), 0);
}
