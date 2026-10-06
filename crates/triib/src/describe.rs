//! Words and icons for what an entity advertises, shared by the entity
//! list, the table and the inspector.

use atdecc::DescriptorType;
use atdecc::aecp::AemStatus;
use atdecc::aem::MappingChange;
use atdecc::controller::{Outcome, Refusal};
use atdecc::model::{EntityModel, EnumerationFailure, EnumerationState};

use crate::app::Triib;
use crate::network::Action;
use atdecc::{
    Adpdu, ControllerCapabilities, EntityCapabilities, ListenerCapabilities, TalkerCapabilities,
};
use scramble_ui::icon::Icon;

pub fn is_talker(adp: &Adpdu) -> bool {
    adp.talker_capabilities
        .contains(TalkerCapabilities::IMPLEMENTED)
}

pub fn is_listener(adp: &Adpdu) -> bool {
    adp.listener_capabilities
        .contains(ListenerCapabilities::IMPLEMENTED)
}

pub fn is_controller(adp: &Adpdu) -> bool {
    adp.controller_capabilities
        .contains(ControllerCapabilities::IMPLEMENTED)
}

/// An icon for the entity's roles.
pub fn glyph(adp: &Adpdu) -> Icon {
    match (is_talker(adp), is_listener(adp)) {
        (true, true) => Icon::GraphicEq,
        (true, false) => Icon::Mic,
        (false, true) => Icon::Speaker,
        (false, false) if is_controller(adp) => Icon::Computer,
        (false, false) => Icon::Lan,
    }
}

/// "Talker 2, listener 2", "Controller" and the like.
pub fn roles(adp: &Adpdu) -> String {
    let mut roles = Vec::new();
    if is_talker(adp) {
        roles.push(format!("talker {}", adp.talker_stream_sources));
    }
    if is_listener(adp) {
        roles.push(format!("listener {}", adp.listener_stream_sinks));
    }
    if is_controller(adp) {
        roles.push("controller".to_owned());
    }
    capitalized(&if roles.is_empty() {
        "no roles".to_owned()
    } else {
        roles.join(", ")
    })
}

/// The SR classes it supports.
pub fn classes(adp: &Adpdu) -> &'static str {
    let capabilities = adp.entity_capabilities;
    match (
        capabilities.contains(EntityCapabilities::CLASS_A_SUPPORTED),
        capabilities.contains(EntityCapabilities::CLASS_B_SUPPORTED),
    ) {
        (true, true) => "A and B",
        (true, false) => "A",
        (false, true) => "B",
        (false, false) => "None",
    }
}

/// Its gPTP BTC, or that it has none.
pub fn clock(adp: &Adpdu) -> String {
    if adp
        .entity_capabilities
        .contains(EntityCapabilities::GPTP_SUPPORTED)
    {
        adp.gptp_grandmaster_id.to_string()
    } else {
        "No gPTP".to_owned()
    }
}

/// The media clock the entity follows: its first clock domain's current
/// clock source, by name.
pub fn media_clock(model: &EntityModel) -> String {
    model
        .clock_domains()
        .next()
        .map(|domain| {
            model
                .name_of(DescriptorType::CLOCK_SOURCE, domain.clock_source_index)
                .map_or_else(
                    || format!("Source {}", domain.clock_source_index),
                    str::to_owned,
                )
        })
        .unwrap_or_default()
}

/// Nanoseconds as milliseconds, with no more decimals than they need:
/// "12 ms", "2.5 ms".
pub fn milliseconds(nanoseconds: u64) -> String {
    let text = format!("{:.3}", nanoseconds as f64 / 1e6);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text} ms")
}

/// How far reading an entity's model got, in a few words.
pub fn read_state(model: Option<&EntityModel>) -> String {
    let Some(model) = model else {
        return "Not read".to_owned();
    };
    match model.state {
        EnumerationState::NotRead => "Not read".to_owned(),
        EnumerationState::Reading => format!("Reading, {} so far", model.descriptor_count()),
        EnumerationState::Complete if model.failed_reads > 0 => {
            format!("Ready, {} unreadable", model.failed_reads)
        }
        EnumerationState::Complete if model.from_cache => "Ready, from cache".to_owned(),
        EnumerationState::Complete => "Ready".to_owned(),
        EnumerationState::Failed(why) => format!("Failed: {}", failure(why)),
    }
}

/// The vendor and model names from the entity's localized strings.
pub fn product(model: &EntityModel) -> String {
    let Some(entity) = model.entity() else {
        return String::new();
    };
    let names: Vec<&str> = [entity.vendor_name, entity.model_name]
        .into_iter()
        .filter_map(|reference| model.localized(reference))
        .collect();
    // Some entities point both at the same string.
    match names.as_slice() {
        [vendor, model] if vendor == model => (*vendor).to_owned(),
        names => names.join(" "),
    }
}

/// Milan support: the specification version and certification, or "No".
pub fn milan(model: &EntityModel) -> String {
    let Some(milan) = model.milan else {
        return "No".to_owned();
    };
    let dotted = |version: [u8; 4]| {
        let mut parts: Vec<String> = version.iter().map(u8::to_string).collect();
        while parts.len() > 2 && parts.last().is_some_and(|part| part == "0") {
            parts.pop();
        }
        parts.join(".")
    };
    let version = milan
        .specification_version
        .map_or_else(|| "before 1.3".to_owned(), dotted);
    if milan.is_certified() {
        format!(
            "{version}, certified {}",
            dotted(milan.certification_version)
        )
    } else {
        format!("{version}, not certified")
    }
}

/// What went wrong with an action, in words, or `None` when it worked.
pub fn outcome(triib: &Triib, action: Action, outcome: Outcome) -> Option<String> {
    let reason = match outcome {
        Outcome::Done => return None,
        Outcome::Refused(Refusal::Acmp(status)) => status
            .name()
            .map_or_else(|| format!("status {}", status.0), flag_name),
        Outcome::Refused(Refusal::Aem(status)) => status
            .name()
            .map_or_else(|| format!("status {}", status.0), flag_name),
        Outcome::NoResponse => "no response".to_owned(),
        Outcome::NotPossible => "not possible".to_owned(),
    };
    Some(match action {
        Action::Connect { talker, listener } => format!(
            "Could not connect {} to {}: {reason}.",
            triib.stream_name(talker.0, DescriptorType::STREAM_OUTPUT, talker.1),
            triib.stream_name(listener.0, DescriptorType::STREAM_INPUT, listener.1),
        ),
        Action::Disconnect { listener } => format!(
            "Could not disconnect {}: {reason}.",
            triib.stream_name(listener.0, DescriptorType::STREAM_INPUT, listener.1),
        ),
        Action::Identify(entity_id) => format!(
            "Could not identify {}: {reason}.",
            triib.entity_name_of(entity_id)
        ),
        Action::Rename { target, name } => format!(
            "Could not rename {} to \"{}\": {reason}.",
            match (target.descriptor_type, target.name_index) {
                (DescriptorType::ENTITY, 0) => triib.entity_name_of(target.entity),
                (DescriptorType::ENTITY, _) =>
                    format!("{}'s group", triib.entity_name_of(target.entity)),
                (descriptor_type, _) =>
                    triib.stream_name(target.entity, descriptor_type, target.index),
            },
            name.as_str()
        ),
        Action::SetStreamFormat {
            entity,
            descriptor_type,
            index,
            ..
        } if outcome == Outcome::Refused(Refusal::Aem(AemStatus::STREAM_IS_RUNNING)) => format!(
            "Could not change the format of {}: it is streaming. Disconnect it first.",
            triib.stream_name(entity, descriptor_type, index)
        ),
        Action::SetStreamFormat {
            entity,
            descriptor_type,
            index,
            ..
        } => format!(
            "Could not change the format of {}: {reason}.",
            triib.stream_name(entity, descriptor_type, index)
        ),
        Action::SetSamplingRate { entity, .. } => format!(
            "Could not change the sampling rate of {}: {reason}.",
            triib.entity_name_of(entity)
        ),
        Action::SetClockSource { entity, .. } => format!(
            "Could not change the clock source of {}: {reason}.",
            triib.entity_name_of(entity)
        ),
        Action::Map {
            entity,
            change: MappingChange::Add,
            ..
        } => format!(
            "Could not map the channel on {}: {reason}.",
            triib.entity_name_of(entity)
        ),
        Action::Map { entity, .. } => format!(
            "Could not unmap the channel on {}: {reason}.",
            triib.entity_name_of(entity)
        ),
        Action::SetControl { entity, index, .. } => format!(
            "Could not set {} on {}: {reason}.",
            triib
                .models
                .get(&entity)
                .and_then(|model| model.name_of(DescriptorType::CONTROL, index))
                .map_or_else(|| format!("control {index}"), |name| format!("\"{name}\"")),
            triib.entity_name_of(entity)
        ),
    })
}

/// A stream's state in words: what an input is bound to and how that is
/// going, or what an output is sending.
pub fn stream_state(
    triib: &Triib,
    model: &EntityModel,
    stream: &atdecc::descriptor::StreamDescriptor<'_>,
) -> Option<String> {
    let info = model.stream_info(stream.descriptor_type, stream.index);
    if stream.is_input() {
        let binding = model.binding(stream.index)?;
        let Some((talker, output)) = binding.talker_stream() else {
            return Some("Not connected".to_owned());
        };
        let mut text = format!(
            "From {}",
            triib.stream_name(talker, DescriptorType::STREAM_OUTPUT, output)
        );
        match info {
            Some(info) if info.talker_failed() => {
                text.push_str(&format!(
                    ", the talker's reservation failed: {}",
                    reservation_failure(info)
                ));
            }
            Some(info) if info.settled() => text.push_str(", receiving"),
            Some(_) => text.push_str(", waiting for the talker"),
            None => {}
        }
        Some(text)
    } else {
        let info = info?;
        info.settled()
            .then(|| format!("Sending to {}", info.stream_dest_mac))
    }
}

/// Why reading an entity model failed, in words.
pub fn failure(failure: EnumerationFailure) -> String {
    match failure {
        EnumerationFailure::NoResponse => "it did not respond".to_owned(),
        EnumerationFailure::Refused(status) => format!(
            "it refused with {}",
            status
                .name()
                .map_or_else(|| status.0.to_string(), flag_name)
        ),
        EnumerationFailure::Malformed => "its response did not decode".to_owned(),
    }
}

/// A capability's name from the standard in words: `AEM_SUPPORTED`
/// becomes "AEM supported".
pub fn flag_name(name: &str) -> String {
    name_words(name, true)
}

/// A standard's name in words, capitalized when it starts a sentence:
/// "READ_DESCRIPTOR" as "Read descriptor", "STREAM_INPUT" in a sentence
/// as "stream input", keeping acronyms such as AVB and gPTP.
pub fn name_words(name: &str, capital: bool) -> String {
    const ACRONYMS: [&str; 19] = [
        "AEM", "ACMP", "ATDECC", "AVB", "AVC", "EFU", "ID", "MIDI", "PTP", "SMPTE", "SRP", "MSRP",
        "MVU", "CRF", "RX", "TX", "AS", "A", "B",
    ];
    let words: Vec<String> = name
        .split('_')
        .map(|word| match word {
            "GPTP" => "gPTP".to_owned(),
            "UDPV4" => "UDPv4".to_owned(),
            "UDPV6" => "UDPv6".to_owned(),
            word if ACRONYMS.contains(&word) => word.to_owned(),
            word => word.to_lowercase(),
        })
        .collect();
    let text = words.join(" ");
    // Only a lowercased first word is capitalized; acronyms keep their
    // own case, as in "gPTP".
    match words.first() {
        Some(first) if capital && first.chars().all(|character| character.is_lowercase()) => {
            capitalized(&text)
        }
        _ => text,
    }
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

/// What an MSRP failure code means (IEEE 802.1Q, 35.2.2.8.7).
pub fn msrp_failure(code: u8) -> &'static str {
    match code {
        1 => "insufficient bandwidth",
        2 => "insufficient bridge resources",
        3 => "insufficient bandwidth for the traffic class",
        4 => "stream ID in use by another talker",
        5 => "destination address already in use",
        6 => "pre-empted by a stream of higher rank",
        7 => "reported latency has changed",
        8 => "egress port is not AVB capable",
        9 => "use a different destination address",
        10 => "out of MSRP resources",
        11 => "out of MMRP resources",
        12 => "cannot store the destination address",
        13 => "priority is not an SR class priority",
        14 => "frames too large for the medium",
        15 => "fan-in port limit reached",
        16 => "first value changed for a registered stream",
        17 => "VLAN blocked on the egress port",
        18 => "VLAN tagging disabled on the egress port",
        19 => "SR class priority mismatch",
        _ => "unknown reason",
    }
}

/// Where a talker's reservation failed, in words: the reason, and the
/// bridge reporting it by the MAC address in its bridge ID.
pub fn reservation_failure(info: &atdecc::aem::StreamInfo) -> String {
    let reason = msrp_failure(info.msrp_failure_code);
    let [_, _, mac @ ..] = info.msrp_failure_bridge_id.to_be_bytes();
    if mac == [0; 6] {
        reason.to_owned()
    } else {
        format!("{reason}, at the bridge {}", avb_net::MacAddress(mac))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_names_read_as_words() {
        assert_eq!(flag_name("AEM_SUPPORTED"), "AEM supported");
        assert_eq!(flag_name("CLASS_A_SUPPORTED"), "Class A supported");
        assert_eq!(flag_name("SUPPORTS_UDPV4_ATDECC"), "Supports UDPv4 ATDECC");
        assert_eq!(flag_name("GPTP_SUPPORTED"), "gPTP supported");
        assert_eq!(flag_name("ENTITY_NOT_READY"), "Entity not ready");
        assert_eq!(flag_name("AVB_INTERFACE"), "AVB interface");
        assert_eq!(
            name_words("GET_RX_STATE_RESPONSE", true),
            "Get RX state response"
        );
        assert_eq!(name_words("STREAM_INPUT", false), "stream input");
        assert_eq!(milliseconds(12_000_000), "12 ms");
        assert_eq!(milliseconds(2_500_000), "2.5 ms");
    }

    #[test]
    fn roles_and_classes() {
        let adp = Adpdu {
            talker_stream_sources: 2,
            talker_capabilities: TalkerCapabilities::IMPLEMENTED,
            listener_stream_sinks: 3,
            listener_capabilities: ListenerCapabilities::IMPLEMENTED,
            entity_capabilities: EntityCapabilities::CLASS_B_SUPPORTED,
            ..Adpdu::default()
        };
        assert_eq!(roles(&adp), "Talker 2, listener 3");
        assert_eq!(classes(&adp), "B");
        assert_eq!(glyph(&adp), Icon::GraphicEq);
        assert_eq!(roles(&Adpdu::default()), "No roles");
        assert_eq!(clock(&adp), "No gPTP");
    }
}
