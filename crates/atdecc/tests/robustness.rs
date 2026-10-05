//! Arbitrary input never panics the decoders, and whatever decodes
//! encodes to something that decodes to the same value.

use atdecc::{Acmpdu, Adpdu, Pdu};

/// A small deterministic generator, so failures reproduce.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn random_frames_never_panic_and_round_trip() {
    let mut random = XorShift(0x9e37_79b9_7f4a_7c15);
    let mut decoded = 0;
    for _ in 0..200_000 {
        let length = (random.next() % 112) as usize;
        let mut bytes: Vec<u8> = (0..length).map(|_| random.next() as u8).collect();
        // Steer most frames to ATDECC subtypes, version 0 and plausible
        // lengths, so the decoders get past the header.
        if let Some(first) = bytes.first_mut() {
            *first = [0xfa, 0xfb, 0xfc, *first][(random.next() % 4) as usize];
        }
        if bytes.len() > 3 && !random.next().is_multiple_of(4) {
            bytes[1] &= 0x8f;
            let control_data_length: u16 = [44, 56, 84, 0x7ff][(random.next() % 4) as usize];
            bytes[2] = (bytes[2] & 0xf8) | (control_data_length >> 8) as u8;
            bytes[3] = control_data_length as u8;
        }
        match atdecc::decode(&bytes) {
            Ok(Pdu::Adp(adpdu)) => {
                decoded += 1;
                let encoded = adpdu.to_bytes().unwrap();
                assert_eq!(Adpdu::decode(&encoded), Ok(adpdu));
            }
            Ok(Pdu::Acmp(acmpdu)) => {
                decoded += 1;
                let mut encoded = [0; Acmpdu::FULL_LEN];
                let written = acmpdu.encode(&mut encoded).unwrap();
                assert_eq!(Acmpdu::decode(&encoded[..written]), Ok(acmpdu));
            }
            Ok(_) | Err(_) => {}
        }
    }
    assert!(decoded > 10_000, "only {decoded} frames decoded");
}
