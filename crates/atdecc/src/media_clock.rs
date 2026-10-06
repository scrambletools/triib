//! Media clock across entities (Milan 1.3, 7.6): the clock each entity's
//! clock domain follows, through the stream input its clock source names
//! and the talker that input is bound to, back to the domain keeping its
//! own clock, the media clock reference of them all.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::descriptor::{ClockSourceType, DescriptorType, StreamDescriptor};
use crate::id::EntityId;
use crate::model::EntityModel;

/// An entity's clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DomainId {
    pub entity: EntityId,
    /// The CLOCK_DOMAIN.
    pub domain: u16,
}

/// Where a clock domain takes its clock from, by its clock source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockFrom {
    /// The entity's own clock (INTERNAL).
    Internal,
    /// A clock input on the entity, such as word clock (EXTERNAL).
    External,
    /// A stream input bound to a talker's stream output, which carries the
    /// clock of the talker's clock domain.
    Stream {
        input: u16,
        talker: EntityId,
        output: u16,
    },
    /// A stream input bound to no talker, so no clock arrives.
    Unbound { input: u16 },
    /// A clock source the model does not hold or know the kind of, or a
    /// stream input whose binding is not known yet.
    Unknown,
}

/// Why a clock domain follows no media clock reference, naming the domain
/// where its chain breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Broken {
    /// Its clock comes from a stream input bound to nothing.
    Unbound(DomainId),
    /// Its clock comes from a talker whose model is not read, or whose
    /// stream output names no clock domain the model holds.
    TalkerUnknown(DomainId),
    /// Its clock source is not known.
    Unknown(DomainId),
    /// The chain comes back to a domain already on it.
    Loop(DomainId),
}

/// One clock domain's place in media clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainClock {
    pub id: DomainId,
    pub from: ClockFrom,
    /// The domain whose clock this one follows directly: the talker's
    /// domain, for a clock from a stream whose talker is known.
    pub parent: Option<DomainId>,
    /// The domain at the head of the chain: itself when it keeps its own
    /// clock.
    pub reference: Result<DomainId, Broken>,
    /// Streams between this domain and the end of its chain.
    pub hops: usize,
}

/// The clock domains of `models` and the clock each follows, by entity
/// and domain.
pub fn media_clocks<'a>(
    models: impl IntoIterator<Item = (EntityId, &'a EntityModel)>,
) -> Vec<DomainClock> {
    let models: BTreeMap<EntityId, &EntityModel> = models.into_iter().collect();
    let mut links: BTreeMap<DomainId, (ClockFrom, Option<DomainId>)> = BTreeMap::new();
    for (&entity, model) in &models {
        for domain in model.clock_domains() {
            let from = clock_from(model, domain.clock_source_index);
            let parent = match from {
                ClockFrom::Stream { talker, output, .. } => models
                    .get(&talker)
                    .and_then(|talker_model| talker_domain(talker_model, output))
                    .map(|domain| DomainId {
                        entity: talker,
                        domain,
                    }),
                _ => None,
            };
            let id = DomainId {
                entity,
                domain: domain.index,
            };
            links.insert(id, (from, parent));
        }
    }
    links
        .iter()
        .map(|(&id, &(from, parent))| {
            let (reference, hops) = follow(&links, id);
            DomainClock {
                id,
                from,
                parent,
                reference,
                hops,
            }
        })
        .collect()
}

/// Where the clock of a domain using `source` comes from.
fn clock_from(model: &EntityModel, source: u16) -> ClockFrom {
    let Some(source) = model.clock_sources().find(|found| found.index == source) else {
        return ClockFrom::Unknown;
    };
    match source.clock_source_type {
        ClockSourceType::INTERNAL => ClockFrom::Internal,
        ClockSourceType::EXTERNAL => ClockFrom::External,
        ClockSourceType::INPUT_STREAM if source.location_type == DescriptorType::STREAM_INPUT => {
            let input = source.location_index;
            match model.binding(input) {
                None => ClockFrom::Unknown,
                Some(binding) => match binding.talker_stream() {
                    Some((talker, output)) => ClockFrom::Stream {
                        input,
                        talker,
                        output,
                    },
                    None => ClockFrom::Unbound { input },
                },
            }
        }
        _ => ClockFrom::Unknown,
    }
}

/// The clock domain a talker's stream output carries the clock of.
fn talker_domain(model: &EntityModel, output: u16) -> Option<u16> {
    let stream =
        StreamDescriptor::decode(model.descriptor(DescriptorType::STREAM_OUTPUT, output)?).ok()?;
    let domain = stream.clock_domain_index;
    model
        .has(DescriptorType::CLOCK_DOMAIN, domain)
        .then_some(domain)
}

/// Follows a domain's chain to its reference, or to where it breaks,
/// counting the streams on the way.
fn follow(
    links: &BTreeMap<DomainId, (ClockFrom, Option<DomainId>)>,
    start: DomainId,
) -> (Result<DomainId, Broken>, usize) {
    let mut path: Vec<DomainId> = Vec::new();
    let mut current = start;
    loop {
        if path.contains(&current) {
            return (Err(Broken::Loop(current)), path.len() - 1);
        }
        path.push(current);
        let hops = path.len() - 1;
        let Some(&(from, parent)) = links.get(&current) else {
            return (Err(Broken::TalkerUnknown(current)), hops);
        };
        match (from, parent) {
            (ClockFrom::Internal | ClockFrom::External, _) => return (Ok(current), hops),
            (ClockFrom::Stream { .. }, Some(parent)) => current = parent,
            (ClockFrom::Stream { .. }, None) => {
                return (Err(Broken::TalkerUnknown(current)), hops);
            }
            (ClockFrom::Unbound { .. }, _) => return (Err(Broken::Unbound(current)), hops),
            (ClockFrom::Unknown, _) => return (Err(Broken::Unknown(current)), hops),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::model::Binding;

    const MASTER: EntityId = EntityId(0x0001_0000_0000_0001);
    const DESK: EntityId = EntityId(0x0001_0000_0000_0002);
    const STAGE: EntityId = EntityId(0x0001_0000_0000_0003);

    fn descriptor(descriptor_type: DescriptorType, index: u16, length: usize) -> Vec<u8> {
        let mut bytes = vec![0; length];
        bytes[0..2].copy_from_slice(&descriptor_type.0.to_be_bytes());
        bytes[2..4].copy_from_slice(&index.to_be_bytes());
        bytes
    }

    fn put(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    /// An entity with one clock domain using `source`: 0 its internal
    /// clock, 1 its stream input 0. Its stream output 0 carries the
    /// domain's clock.
    fn entity(source: u16, bound_to: Option<(EntityId, u16)>) -> EntityModel {
        let mut model = EntityModel::reading();
        let mut domain = descriptor(DescriptorType::CLOCK_DOMAIN, 0, 80);
        put(&mut domain, 70, source);
        put(&mut domain, 72, 76);
        put(&mut domain, 74, 2);
        put(&mut domain, 78, 1);
        model.store(DescriptorType::CLOCK_DOMAIN, 0, &domain);
        let internal = descriptor(DescriptorType::CLOCK_SOURCE, 0, 86);
        model.store(DescriptorType::CLOCK_SOURCE, 0, &internal);
        let mut stream = descriptor(DescriptorType::CLOCK_SOURCE, 1, 86);
        put(&mut stream, 72, ClockSourceType::INPUT_STREAM.0);
        put(&mut stream, 82, DescriptorType::STREAM_INPUT.0);
        model.store(DescriptorType::CLOCK_SOURCE, 1, &stream);
        for descriptor_type in [DescriptorType::STREAM_INPUT, DescriptorType::STREAM_OUTPUT] {
            let mut bytes = descriptor(descriptor_type, 0, 132);
            put(&mut bytes, 82, 132);
            model.store(descriptor_type, 0, &bytes);
        }
        let binding = bound_to.map_or_else(Binding::default, |(talker, output)| Binding {
            talker,
            talker_unique_id: output,
            connection_count: 1,
            ..Binding::default()
        });
        model.set_binding(0, binding);
        model
    }

    fn id(entity: EntityId) -> DomainId {
        DomainId { entity, domain: 0 }
    }

    #[test]
    fn a_chain_leads_to_its_reference() {
        let master = entity(0, None);
        let desk = entity(1, Some((MASTER, 0)));
        let stage = entity(1, Some((DESK, 0)));
        let clocks = media_clocks([(MASTER, &master), (DESK, &desk), (STAGE, &stage)]);
        let found: Vec<_> = clocks
            .iter()
            .map(|clock| (clock.id.entity, clock.reference, clock.hops, clock.parent))
            .collect();
        assert_eq!(
            found,
            [
                (MASTER, Ok(id(MASTER)), 0, None),
                (DESK, Ok(id(MASTER)), 1, Some(id(MASTER))),
                (STAGE, Ok(id(MASTER)), 2, Some(id(DESK))),
            ]
        );
        assert_eq!(clocks[0].from, ClockFrom::Internal);
        assert_eq!(
            clocks[2].from,
            ClockFrom::Stream {
                input: 0,
                talker: DESK,
                output: 0
            }
        );
    }

    #[test]
    fn broken_chains_say_where() {
        let desk = entity(1, None);
        let stage = entity(1, Some((DESK, 0)));
        let orphan = entity(1, Some((EntityId(0x99), 0)));
        let clocks = media_clocks([(DESK, &desk), (STAGE, &stage), (MASTER, &orphan)]);
        let references: Vec<_> = clocks.iter().map(|clock| clock.reference).collect();
        assert_eq!(
            references,
            [
                Err(Broken::TalkerUnknown(id(MASTER))),
                Err(Broken::Unbound(id(DESK))),
                Err(Broken::Unbound(id(DESK))),
            ]
        );
        assert_eq!(clocks[1].from, ClockFrom::Unbound { input: 0 });

        // Each clocked from the other.
        let desk = entity(1, Some((STAGE, 0)));
        let stage = entity(1, Some((DESK, 0)));
        let clocks = media_clocks([(DESK, &desk), (STAGE, &stage)]);
        assert!(
            clocks
                .iter()
                .all(|clock| matches!(clock.reference, Err(Broken::Loop(_))))
        );
    }

    #[test]
    fn a_binding_not_read_yet_is_unknown() {
        let mut desk = entity(1, None);
        desk = {
            let mut fresh = EntityModel::reading();
            for (descriptor_type, index, bytes) in desk.all_descriptors() {
                fresh.store(descriptor_type, index, bytes);
            }
            fresh
        };
        let clocks = media_clocks([(DESK, &desk)]);
        assert_eq!(clocks[0].from, ClockFrom::Unknown);
        assert_eq!(clocks[0].reference, Err(Broken::Unknown(id(DESK))));
    }
}
