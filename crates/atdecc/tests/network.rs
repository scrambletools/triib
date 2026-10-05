//! The bench's answers to GET_AVB_INFO, GET_AS_PATH and GET_COUNTERS,
//! decoded and checked against tshark's reading of the same frames.

mod common;

use std::collections::HashMap;

use atdecc::aecp::{AecpMessageType, AemPdu, AemStatus};
use atdecc::aem::{AsPath, AvbInfo, AvbInfoFlags, Counters};
use atdecc::avtp::ControlHeader;
use atdecc::{AemCommandType, ClockIdentity, MacAddress};
use common::{ethernet_payload, expected_rows, number, pcap_frames};

const CAPTURE: &[u8] = include_bytes!("captures/bench-network.pcap");
const EXPECTED: &str = include_str!("captures/bench-network.tsv");

/// The switch, grandmaster of every wired entity on the bench.
const SWITCH: ClockIdentity = ClockIdentity(0x0001_f2ff_feff_3b14);
const MACOS: MacAddress = MacAddress([0xd0, 0x11, 0xe5, 0x97, 0xf5, 0x44]);
const WIRED: MacAddress = MacAddress([0xe8, 0xf6, 0x0a, 0xe0, 0x92, 0x20]);

fn field<'a>(row: &HashMap<&str, &'a str>, frame: usize, name: &str) -> &'a str {
    row.get(name)
        .copied()
        .unwrap_or_else(|| panic!("frame {frame}: tshark has no {name}"))
}

fn flag(row: &HashMap<&str, &str>, frame: usize, name: &str) -> bool {
    field(row, frame, name) == "True"
}

/// tshark's comma separated values of a field that repeats.
fn numbers(row: &HashMap<&str, &str>, frame: usize, name: &str) -> Vec<u64> {
    field(row, frame, name).split(',').map(number).collect()
}

#[test]
fn network_responses_match_tshark() {
    let frames = pcap_frames(CAPTURE);
    let rows = expected_rows(EXPECTED);
    assert_eq!(frames.len(), rows.len());
    let mut answered = Vec::new();
    for (index, (frame, row)) in frames.iter().zip(&rows).enumerate() {
        let number_of_frame = index + 1;
        let (ethertype, avtpdu) = ethernet_payload(frame);
        if ethertype != 0x22f0 || avtpdu[0] != 0xfb {
            continue;
        }
        let Ok(aem) = AemPdu::decode(avtpdu) else {
            continue;
        };
        if aem.header.message_type != AecpMessageType::AEM_RESPONSE
            || !matches!(
                aem.command_type,
                AemCommandType::GET_AVB_INFO
                    | AemCommandType::GET_AS_PATH
                    | AemCommandType::GET_COUNTERS
            )
        {
            continue;
        }
        let source = MacAddress(frame[6..12].try_into().unwrap());
        assert_eq!(
            number(field(row, number_of_frame, "ieee17221.command_type")),
            u64::from(aem.command_type.0)
        );
        answered.push((source, aem.command_type, aem.status()));
        if !aem.status().is_success() {
            continue;
        }
        match aem.command_type {
            AemCommandType::GET_AVB_INFO => {
                let info = AvbInfo::decode(aem.payload).unwrap();
                let prefix = "ieee17221.avb_info_";
                assert_eq!(
                    number(field(
                        row,
                        number_of_frame,
                        &format!("{prefix}gptp_grandmaster_id")
                    )),
                    info.gptp_grandmaster_id.0
                );
                assert_eq!(info.gptp_grandmaster_id, SWITCH);
                assert_eq!(
                    number(field(
                        row,
                        number_of_frame,
                        &format!("{prefix}propagation_delay")
                    )),
                    u64::from(info.propagation_delay)
                );
                assert_eq!(
                    number(field(
                        row,
                        number_of_frame,
                        &format!("{prefix}gptp_domain_number")
                    )),
                    u64::from(info.gptp_domain_number)
                );
                for (name, bit) in [
                    ("ieee17221.as_capable_flag", AvbInfoFlags::AS_CAPABLE),
                    ("ieee17221.gptp_enabled_flag", AvbInfoFlags::GPTP_ENABLED),
                    ("ieee17221.srp_enabled_flag", AvbInfoFlags::SRP_ENABLED),
                ] {
                    assert_eq!(
                        flag(row, number_of_frame, name),
                        info.flags.contains(bit),
                        "frame {number_of_frame}: {name}"
                    );
                }
                let mappings = info.msrp_mappings();
                assert_eq!(
                    number(field(row, number_of_frame, "ieee17221.msrp_mappings_count")),
                    mappings.len() as u64
                );
                let classes: Vec<u64> = mappings
                    .iter()
                    .map(|mapping| u64::from(mapping.traffic_class))
                    .collect();
                let priorities: Vec<u64> = mappings
                    .iter()
                    .map(|mapping| u64::from(mapping.priority))
                    .collect();
                let vlans: Vec<u64> = mappings
                    .iter()
                    .map(|mapping| u64::from(mapping.vlan_id))
                    .collect();
                assert_eq!(
                    numbers(row, number_of_frame, "ieee17221.msrp_mapping_traffic_class"),
                    classes
                );
                assert_eq!(
                    numbers(row, number_of_frame, "ieee17221.msrp_mapping_priority"),
                    priorities
                );
                assert_eq!(
                    numbers(row, number_of_frame, "ieee17221.msrp_vlan_id"),
                    vlans
                );
            }
            AemCommandType::GET_AS_PATH => {
                let path = AsPath::decode(aem.payload).unwrap();
                assert_eq!(
                    number(field(row, number_of_frame, "ieee17221.as_path_count")),
                    path.len() as u64
                );
                // tshark does not show the identities; both entities
                // name the switch, then themselves.
                let identities: Vec<ClockIdentity> = path.clock_identities().collect();
                assert_eq!(identities.len(), 2, "frame {number_of_frame}");
                assert_eq!(identities[0], SWITCH, "frame {number_of_frame}");
            }
            AemCommandType::GET_COUNTERS => {
                let counters = Counters::decode(aem.payload).unwrap();
                let interface = counters.avb_interface().unwrap();
                for (name, value) in [
                    ("link_up", interface.link_up),
                    ("link_down", interface.link_down),
                    ("gptp_gm_changed", interface.gptp_gm_changed),
                ] {
                    assert_eq!(
                        flag(
                            row,
                            number_of_frame,
                            &format!("ieee17221.flags.{name}_valid")
                        ),
                        value.is_some(),
                        "frame {number_of_frame}: {name} valid"
                    );
                    assert_eq!(
                        Some(number(field(
                            row,
                            number_of_frame,
                            &format!("ieee17221.{name}")
                        ))),
                        value.map(u64::from),
                        "frame {number_of_frame}: {name}"
                    );
                }
                // The response claims 8 octets more than it carries
                // (esp_avb.md, finding 13) and decodes all the same.
                let header = ControlHeader::decode(avtpdu).unwrap();
                assert_eq!(header.control_data_length, 156);
                assert_eq!(avtpdu.len(), ControlHeader::COMMON_LEN + 148);
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(
        answered,
        [
            (MACOS, AemCommandType::GET_AVB_INFO, AemStatus::SUCCESS),
            (MACOS, AemCommandType::GET_AS_PATH, AemStatus::SUCCESS),
            (
                MACOS,
                AemCommandType::GET_COUNTERS,
                AemStatus::NOT_IMPLEMENTED
            ),
            (WIRED, AemCommandType::GET_AVB_INFO, AemStatus::SUCCESS),
            (WIRED, AemCommandType::GET_AS_PATH, AemStatus::SUCCESS),
            (WIRED, AemCommandType::GET_COUNTERS, AemStatus::SUCCESS),
        ]
    );
}
