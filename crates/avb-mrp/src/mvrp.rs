//! MVRP (IEEE 802.1Q-2022, 11.2): the VLANs a port asks to be a member of.

use alloc::vec::Vec;

use crate::mrpdu::{Format, add};

/// MVRP's one attribute type, the VLAN identifier (11.2.3.1.6).
pub const VID: u8 = 1;

/// The format of MVRP's MRPDUs, for a [`crate::Participant`].
pub const FORMAT: Format = Format {
    list_length: false,
    first_value_length: |attribute_type| (attribute_type == VID).then_some(2),
    key_length: |_| 2,
    four_packed: |_| false,
    nth_value: |_, first, index| {
        let mut value = Vec::from(first);
        add(&mut value, index);
        value
    },
};

/// The value naming VLAN `vlan_id`.
pub fn value(vlan_id: u16) -> Vec<u8> {
    Vec::from((vlan_id & 0x0fff).to_be_bytes())
}
