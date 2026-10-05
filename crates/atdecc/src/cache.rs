//! Entity models kept between runs: the descriptors read from an entity,
//! by its entity model, firmware and configuration, so that when an entity
//! of that model comes online again only what changes on it is read.

use alloc::string::String;
use alloc::vec::Vec;

use crate::avtp::{read_u16, read_u64};
use crate::descriptor::DescriptorType;
use crate::error::DecodeError;
use crate::id::EntityModelId;

/// Which entities a cached model is for. Entities report a new entity
/// model ID when their descriptors change (Milan 1.3, 5.3.1); the firmware
/// is part of the key in case one does not.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelKey {
    pub entity_model_id: EntityModelId,
    pub firmware: String,
    pub configuration: u16,
}

/// An entity's descriptors, other than its ENTITY descriptor, which each
/// entity has its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedModel {
    pub key: ModelKey,
    pub descriptors: Vec<(DescriptorType, u16, Vec<u8>)>,
}

/// The start of an encoded model and the version of its layout.
const MAGIC: &[u8; 8] = b"ATDECC\0\x01";

impl CachedModel {
    /// The model as octets to keep: the magic, the key, then each
    /// descriptor's type, index and length before its octets.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.key.entity_model_id.0.to_be_bytes());
        out.extend_from_slice(&self.key.configuration.to_be_bytes());
        out.extend_from_slice(&(self.key.firmware.len() as u16).to_be_bytes());
        out.extend_from_slice(self.key.firmware.as_bytes());
        out.extend_from_slice(&(self.descriptors.len() as u16).to_be_bytes());
        for (descriptor_type, index, bytes) in &self.descriptors {
            out.extend_from_slice(&descriptor_type.0.to_be_bytes());
            out.extend_from_slice(&index.to_be_bytes());
            out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut at = 0;
        let mut take = |length: usize| -> Result<&[u8], DecodeError> {
            let part = bytes.get(at..at + length).ok_or(DecodeError::Truncated {
                needed: at + length,
                available: bytes.len(),
            })?;
            at += length;
            Ok(part)
        };
        if take(MAGIC.len())? != MAGIC {
            return Err(DecodeError::UnsupportedVersion(0));
        }
        let entity_model_id = EntityModelId(read_u64(take(8)?, 0));
        let configuration = read_u16(take(2)?, 0);
        let firmware_length = usize::from(read_u16(take(2)?, 0));
        let firmware = String::from_utf8_lossy(take(firmware_length)?).into_owned();
        let count = read_u16(take(2)?, 0);
        let mut descriptors = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let descriptor_type = DescriptorType(read_u16(take(2)?, 0));
            let index = read_u16(take(2)?, 0);
            let length = usize::from(read_u16(take(2)?, 0));
            descriptors.push((descriptor_type, index, take(length)?.to_vec()));
        }
        Ok(Self {
            key: ModelKey {
                entity_model_id,
                firmware,
                configuration,
            },
            descriptors,
        })
    }

    /// The descriptor stored for a type and index.
    pub fn descriptor(&self, descriptor_type: DescriptorType, index: u16) -> Option<&[u8]> {
        self.descriptors
            .iter()
            .find(|(kind, place, _)| *kind == descriptor_type && *place == index)
            .map(|(_, _, bytes)| bytes.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let model = CachedModel {
            key: ModelKey {
                entity_model_id: EntityModelId(0x0000_0074_6869_6e67),
                firmware: "1.0.0".into(),
                configuration: 0,
            },
            descriptors: alloc::vec![
                (DescriptorType::CONFIGURATION, 0, alloc::vec![0, 1, 0, 0, 7]),
                (DescriptorType::STREAM_INPUT, 1, alloc::vec![0, 5, 0, 1]),
            ],
        };
        let bytes = model.encode();
        assert_eq!(CachedModel::decode(&bytes), Ok(model.clone()));
        assert_eq!(
            model.descriptor(DescriptorType::STREAM_INPUT, 1),
            Some(&[0, 5, 0, 1][..])
        );
        // Cut short, or not a model at all.
        assert!(CachedModel::decode(&bytes[..bytes.len() - 1]).is_err());
        assert!(CachedModel::decode(b"not a model").is_err());
    }
}
