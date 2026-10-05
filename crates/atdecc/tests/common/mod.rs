//! Reading the golden captures: classic pcap files and tshark's field
//! exports beside them.

#![allow(dead_code)]

use std::collections::HashMap;

/// The frames of a classic pcap file.
pub fn pcap_frames(file: &[u8]) -> Vec<&[u8]> {
    pcap_records(file)
        .into_iter()
        .map(|(_, frame)| frame)
        .collect()
}

/// The capture time and octets of each frame of a classic pcap file.
pub fn pcap_records(file: &[u8]) -> Vec<(std::time::Duration, &[u8])> {
    let magic = u32::from_le_bytes(file[..4].try_into().unwrap());
    let (little_endian, nanoseconds) = match magic {
        0xa1b2_c3d4 => (true, false),
        0xa1b2_3c4d => (true, true),
        0xd4c3_b2a1 => (false, false),
        0x4d3c_b2a1 => (false, true),
        other => panic!("not a classic pcap file: magic {other:#x}"),
    };
    let read_u32 = |at: usize| {
        let bytes: [u8; 4] = file[at..at + 4].try_into().unwrap();
        if little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    };
    let mut records = Vec::new();
    let mut at = 24;
    while at + 16 <= file.len() {
        let seconds = u64::from(read_u32(at));
        let fraction = read_u32(at + 4);
        let time = std::time::Duration::from_secs(seconds)
            + if nanoseconds {
                std::time::Duration::from_nanos(u64::from(fraction))
            } else {
                std::time::Duration::from_micros(u64::from(fraction))
            };
        let captured = read_u32(at + 8) as usize;
        records.push((time, &file[at + 16..at + 16 + captured]));
        at += 16 + captured;
    }
    records
}

/// The ethertype and payload of an Ethernet frame, past any VLAN tag.
pub fn ethernet_payload(frame: &[u8]) -> (u16, &[u8]) {
    let mut ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    let mut payload = &frame[14..];
    if ethertype == 0x8100 {
        ethertype = u16::from_be_bytes([payload[2], payload[3]]);
        payload = &payload[4..];
    }
    (ethertype, payload)
}

/// tshark's values for each frame, by field name, empty ones left out.
pub fn expected_rows(expected: &'static str) -> Vec<HashMap<&'static str, &'static str>> {
    let mut lines = expected.lines();
    let names: Vec<&str> = lines.next().unwrap().split('\t').collect();
    lines
        .map(|line| {
            names
                .iter()
                .zip(line.split('\t'))
                .filter(|(_, value)| !value.is_empty())
                .map(|(name, value)| (*name, value))
                .collect()
        })
        .collect()
}

/// A tshark number, in hex with `0x` or in decimal.
pub fn number(text: &str) -> u64 {
    match text.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).unwrap(),
        None => text.parse().unwrap(),
    }
}
