//! MRPDUs (IEEE 802.1Q-2022, 10.8): messages of vector attributes, each a
//! first value and the events of the values counted on from it.

use alloc::vec::Vec;

/// The version every MRPDU carries (10.8.1.2).
pub const PROTOCOL_VERSION: u8 = 0;

/// The VectorHeader's LeaveAllEvent, in place (10.8.2.6).
const LEAVE_ALL: u16 = 0x2000;
const NUMBER_OF_VALUES: u16 = 0x1fff;

/// An attribute event (10.8.2.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Event {
    New,
    JoinIn,
    In,
    JoinMt,
    Mt,
    /// The declaration is withdrawn.
    Lv,
}

impl Event {
    pub(crate) fn from_number(number: u8) -> Option<Self> {
        Some(match number {
            0 => Event::New,
            1 => Event::JoinIn,
            2 => Event::In,
            3 => Event::JoinMt,
            4 => Event::Mt,
            5 => Event::Lv,
            _ => return None,
        })
    }

    fn number(self) -> u8 {
        match self {
            Event::New => 0,
            Event::JoinIn => 1,
            Event::In => 2,
            Event::JoinMt => 3,
            Event::Mt => 4,
            Event::Lv => 5,
        }
    }

    /// Whether the sender declares the attribute with this event.
    pub fn declares(self) -> bool {
        matches!(self, Event::New | Event::JoinIn | Event::JoinMt)
    }
}

/// What sets one MRP application's messages apart from another's.
#[derive(Clone, Copy)]
pub struct Format {
    /// MSRP's messages carry an AttributeListLength; MVRP's do not.
    pub list_length: bool,
    /// The octets of an attribute type's first value, `None` for a type
    /// the application does not know.
    pub first_value_length: fn(u8) -> Option<usize>,
    /// The leading octets of a value that name the attribute; the rest
    /// may change while it stays the same attribute.
    pub key_length: fn(u8) -> usize,
    /// Whether an attribute type's values carry a four-packed event too,
    /// as MSRP's Listener declarations do.
    pub four_packed: fn(u8) -> bool,
    /// The value `index` places after `first` in a vector.
    pub nth_value: fn(u8, &[u8], u16) -> Vec<u8>,
}

/// One value a received MRPDU holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub attribute_type: u8,
    pub value: Vec<u8>,
    pub event: Event,
    /// The four-packed event, for the types that carry one.
    pub four_packed: Option<u8>,
}

/// What a received MRPDU holds: the attribute types it asks to declare
/// again (LeaveAll), then its values, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decoded {
    pub leave_all: Vec<u8>,
    pub values: Vec<Value>,
}

/// Decodes an MRPDU's payload, from its ProtocolVersion on. What follows
/// a part that does not decode is left out, as are unknown attribute
/// types.
pub fn decode(pdu: &[u8], format: &Format) -> Decoded {
    let mut decoded = Decoded::default();
    let Some((&_version, mut rest)) = pdu.split_first() else {
        return decoded;
    };
    // An EndMark, or nothing left, ends the messages.
    while let [attribute_type, attribute_length, after @ ..] = rest {
        if *attribute_type == 0 {
            break;
        }
        let (list, next) = if format.list_length {
            let [high, low, list @ ..] = after else {
                break;
            };
            let length = usize::from(u16::from_be_bytes([*high, *low]));
            if length > list.len() {
                break;
            }
            (&list[..length], &list[length..])
        } else {
            (after, &[][..])
        };
        let length = usize::from(*attribute_length);
        let known = (format.first_value_length)(*attribute_type) == Some(length);
        let four = (format.four_packed)(*attribute_type);
        let used = vectors(list, length, four, |leave_all, first, events, fours, count| {
            if !known {
                return;
            }
            if leave_all && !decoded.leave_all.contains(attribute_type) {
                decoded.leave_all.push(*attribute_type);
            }
            for index in 0..count {
                let packed = events[usize::from(index / 3)];
                let number = packed / [36, 6, 1][usize::from(index % 3)] % 6;
                let Some(event) = Event::from_number(number) else {
                    continue;
                };
                let four_packed = four.then(|| {
                    let packed = fours[usize::from(index / 4)];
                    (packed >> (6 - 2 * (index % 4))) & 0x3
                });
                decoded.values.push(Value {
                    attribute_type: *attribute_type,
                    value: (format.nth_value)(*attribute_type, first, index),
                    event,
                    four_packed,
                });
            }
        });
        rest = if format.list_length {
            next
        } else {
            match used {
                Some(used) => &list[used..],
                None => break,
            }
        };
    }
    decoded
}

/// Calls `each` with every vector attribute of an attribute list: its
/// LeaveAll, first value, three- and four-packed events and number of
/// values. Returns how many octets the list took through its EndMark, or
/// `None` when it ran out first.
fn vectors(
    mut list: &[u8],
    length: usize,
    four: bool,
    mut each: impl FnMut(bool, &[u8], &[u8], &[u8], u16),
) -> Option<usize> {
    let total = list.len();
    loop {
        let [high, low, rest @ ..] = list else {
            // Without an EndMark the list ends where its octets do.
            return list.is_empty().then_some(total);
        };
        let header = u16::from_be_bytes([*high, *low]);
        if header == 0 {
            return Some(total - rest.len());
        }
        let count = header & NUMBER_OF_VALUES;
        let threes = usize::from(count).div_ceil(3);
        let fours = if four {
            usize::from(count).div_ceil(4)
        } else {
            0
        };
        let size = length + threes + fours;
        if rest.len() < size {
            return None;
        }
        let (first, packed) = rest[..size].split_at(length);
        let (events, four_events) = packed.split_at(threes);
        each(header & LEAVE_ALL != 0, first, events, four_events, count);
        list = &rest[size..];
    }
}

/// A message to encode: one attribute type's values, each sent as a
/// vector of its own, and whether it asks to declare them all again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub attribute_type: u8,
    pub leave_all: bool,
    /// Each value with its event and, for the types that carry one, its
    /// four-packed event.
    pub values: Vec<(Vec<u8>, Event, Option<u8>)>,
}

/// Encodes an MRPDU's payload, from its ProtocolVersion on. A message
/// with LeaveAll and no values still carries a vector, with a first
/// value of zeros, as bridges send it.
pub fn encode(messages: &[Message], format: &Format) -> Vec<u8> {
    let mut pdu = Vec::from([PROTOCOL_VERSION]);
    for message in messages {
        let Some(length) = (format.first_value_length)(message.attribute_type) else {
            continue;
        };
        pdu.push(message.attribute_type);
        pdu.push(length as u8);
        let list_at = pdu.len();
        if format.list_length {
            pdu.extend_from_slice(&[0, 0]);
        }
        let mut leave_all = message.leave_all;
        let mut vector = |pdu: &mut Vec<u8>, value: &[u8], event: Option<(Event, Option<u8>)>| {
            let count = u16::from(event.is_some());
            let flag = if leave_all { LEAVE_ALL } else { 0 };
            leave_all = false;
            pdu.extend_from_slice(&(flag | count).to_be_bytes());
            let mut first = Vec::from(value);
            first.resize(length, 0);
            pdu.extend_from_slice(&first);
            if let Some((event, four_packed)) = event {
                pdu.push(event.number() * 36);
                if let Some(four_packed) = four_packed {
                    pdu.push((four_packed & 0x3) << 6);
                }
            }
        };
        if message.values.is_empty() {
            vector(&mut pdu, &[], None);
        }
        for (value, event, four_packed) in &message.values {
            vector(&mut pdu, value, Some((*event, *four_packed)));
        }
        pdu.extend_from_slice(&[0, 0]);
        if format.list_length {
            let list = (pdu.len() - list_at - 2) as u16;
            pdu[list_at..list_at + 2].copy_from_slice(&list.to_be_bytes());
        }
    }
    pdu.extend_from_slice(&[0, 0]);
    pdu
}

/// Adds `index` to the big-endian number in `octets`, in place.
pub(crate) fn add(octets: &mut [u8], index: u16) {
    let mut carry = u32::from(index);
    for octet in octets.iter_mut().rev() {
        if carry == 0 {
            break;
        }
        let sum = u32::from(*octet) + carry;
        *octet = sum as u8;
        carry = sum >> 8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{msrp, mvrp};

    #[test]
    fn msrp_messages_round_trip() {
        let talker = Vec::from([0x11u8; 25]);
        let messages = [
            Message {
                attribute_type: msrp::attribute::TALKER_ADVERTISE,
                leave_all: true,
                values: Vec::from([(talker.clone(), Event::New, None)]),
            },
            Message {
                attribute_type: msrp::attribute::LISTENER,
                leave_all: false,
                values: Vec::from([(Vec::from([0x22; 8]), Event::JoinMt, Some(2))]),
            },
        ];
        let pdu = encode(&messages, &msrp::FORMAT);
        let decoded = decode(&pdu, &msrp::FORMAT);
        assert_eq!(decoded.leave_all, [msrp::attribute::TALKER_ADVERTISE]);
        assert_eq!(
            decoded.values,
            [
                Value {
                    attribute_type: msrp::attribute::TALKER_ADVERTISE,
                    value: talker,
                    event: Event::New,
                    four_packed: None,
                },
                Value {
                    attribute_type: msrp::attribute::LISTENER,
                    value: Vec::from([0x22; 8]),
                    event: Event::JoinMt,
                    four_packed: Some(2),
                },
            ]
        );
    }

    #[test]
    fn a_leave_all_without_values_decodes_as_bridges_send_it() {
        // A Domain message with only LeaveAll, its first value zeroed.
        let pdu = [0, 4, 4, 0, 8, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let decoded = decode(&pdu, &msrp::FORMAT);
        assert_eq!(decoded.leave_all, [msrp::attribute::DOMAIN]);
        assert!(decoded.values.is_empty());
        let message = Message {
            attribute_type: msrp::attribute::DOMAIN,
            leave_all: true,
            values: Vec::new(),
        };
        assert_eq!(encode(&[message], &msrp::FORMAT), pdu);
    }

    #[test]
    fn vectors_count_values_on_from_the_first() {
        // Two listeners in one vector: Ready, then AskingFailed.
        let mut pdu = Vec::from([0, msrp::attribute::LISTENER, 8, 0, 0]);
        pdu.extend_from_slice(&[0x00, 0x02]);
        pdu.extend_from_slice(&0x0001_0203_0405_06ffu64.to_be_bytes());
        pdu.push(36 + 6); // JoinIn, JoinIn
        pdu.push((2 << 6) | (1 << 4));
        pdu.extend_from_slice(&[0, 0]);
        let list = (pdu.len() - 5) as u16;
        pdu[3..5].copy_from_slice(&list.to_be_bytes());
        pdu.extend_from_slice(&[0, 0]);
        let decoded = decode(&pdu, &msrp::FORMAT);
        assert_eq!(decoded.values.len(), 2);
        assert_eq!(decoded.values[0].four_packed, Some(2));
        assert_eq!(
            decoded.values[1].value,
            0x0001_0203_0405_0700u64.to_be_bytes()
        );
        assert_eq!(decoded.values[1].four_packed, Some(1));
    }

    #[test]
    fn mvrp_messages_have_no_list_length() {
        let message = Message {
            attribute_type: mvrp::VID,
            leave_all: false,
            values: Vec::from([(mvrp::value(2), Event::JoinIn, None)]),
        };
        let pdu = encode(&[message], &mvrp::FORMAT);
        assert_eq!(pdu, [0, 1, 2, 0, 1, 0, 2, 36, 0, 0, 0, 0]);
        let decoded = decode(&pdu, &mvrp::FORMAT);
        assert_eq!(decoded.values[0].value, [0, 2]);
        assert_eq!(decoded.values[0].event, Event::JoinIn);
    }

    #[test]
    fn short_or_strange_pdus_decode_to_what_they_hold() {
        assert_eq!(decode(&[], &msrp::FORMAT), Decoded::default());
        assert_eq!(decode(&[0, 1, 25, 0, 99], &msrp::FORMAT), Decoded::default());
        assert_eq!(decode(&[0, 9, 3, 0, 0, 0, 0], &msrp::FORMAT), Decoded::default());
    }
}
