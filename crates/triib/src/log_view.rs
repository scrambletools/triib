//! The log: the ATDECC frames triib sent and heard, newest first, each
//! with what it says and, marked, where its sender broke the rules: a
//! frame that does not decode, a control_data_length claiming octets the
//! frame does not carry, and the long ACMP form from a Milan entity.

use std::collections::VecDeque;
use std::time::SystemTime;

use atdecc::EntityId;
use atdecc::acmp::{AcmpMessageType, AcmpStatus, Acmpdu};
use atdecc::adp::AdpMessageType;
use atdecc::aecp::{AecpMessageType, AemCommandType, AemPdu, AemStatus};
use atdecc::blocking::Frame;
use atdecc::descriptor::DescriptorType;
use atdecc::mvu::MvuMessage;
use atdecc::pdu::{self, Pdu};
use iced::widget::{column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Font, Length};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::{self, Icon};
use scramble_ui::{component, style};

use crate::app::{Message, Triib};

/// The most frames kept, the oldest dropped past it.
pub const LIMIT: usize = 5000;
/// The most lines drawn at once, the newest.
const SHOWN: usize = 500;
const TIME_WIDTH: f32 = 76.0;
const ENTITY_WIDTH: f32 = 180.0;

/// Which frames the log shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilter {
    #[default]
    All,
    /// Frames whose sender broke the rules.
    Warnings,
    Adp,
    Aecp,
    Acmp,
}

#[derive(Debug, Clone)]
pub enum LogMessage {
    Filter(LogFilter),
    /// Stop taking new frames, or take them again.
    Paused(bool),
    Cleared,
    /// Show a line's octets, or close it.
    Opened(Option<u64>),
}

#[derive(Debug, Default)]
pub struct Log {
    entries: VecDeque<Entry>,
    pub filter: LogFilter,
    pub paused: bool,
    /// The line whose octets show.
    pub open: Option<u64>,
    /// When the first frame kept came, which times count from.
    began: Option<SystemTime>,
    next: u64,
}

#[derive(Debug, Clone)]
struct Entry {
    number: u64,
    frame: Frame,
    /// What it says, as it read when it came.
    line: Line,
}

impl Log {
    /// Takes frames with what each says, unless paused, keeping the newest
    /// [`LIMIT`].
    pub fn add(&mut self, frames: Vec<(Frame, Line)>) {
        if self.paused {
            return;
        }
        for (frame, line) in frames {
            self.began.get_or_insert(frame.at);
            self.entries.push_back(Entry {
                number: self.next,
                frame,
                line,
            });
            self.next += 1;
            if self.entries.len() > LIMIT {
                self.entries.pop_front();
            }
        }
    }

    pub fn update(&mut self, message: LogMessage) {
        match message {
            LogMessage::Filter(filter) => self.filter = filter,
            LogMessage::Paused(paused) => self.paused = paused,
            LogMessage::Cleared => {
                self.entries.clear();
                self.open = None;
                self.began = None;
            }
            LogMessage::Opened(open) => self.open = open,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Seconds from the first frame kept to `at`.
    fn seconds(&self, at: SystemTime) -> f64 {
        self.began
            .and_then(|began| at.duration_since(began).ok())
            .map_or(0.0, |since| since.as_secs_f64())
    }
}

/// The protocol a frame belongs to, for the filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Protocol {
    Adp,
    Aecp,
    Acmp,
}

/// What a frame says, for its line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    protocol: Option<Protocol>,
    /// The entity it is to, from or about.
    pub entity: Option<EntityId>,
    pub summary: String,
    /// An entity's refusal, shown in the error color.
    pub refusal: Option<String>,
    /// Where the sender broke the rules.
    pub warnings: Vec<String>,
}

impl Line {
    fn shown_by(&self, filter: LogFilter) -> bool {
        match filter {
            LogFilter::All => true,
            LogFilter::Warnings => !self.warnings.is_empty(),
            LogFilter::Adp => self.protocol == Some(Protocol::Adp),
            LogFilter::Aecp => self.protocol == Some(Protocol::Aecp),
            LogFilter::Acmp => self.protocol == Some(Protocol::Acmp),
        }
    }
}

/// A standard name in words: "READ_DESCRIPTOR" as "Read descriptor",
/// keeping acronyms.
fn words(name: &str) -> String {
    sentence(name, true)
}

/// A standard name in words, capitalized when it starts a sentence:
/// "AVB_INTERFACE" as "AVB interface", "STREAM_INPUT" as "stream input".
fn sentence(name: &str, capital: bool) -> String {
    const ACRONYMS: [&str; 9] = ["RX", "TX", "AVB", "AS", "ID", "MVU", "AEM", "ACMP", "MSRP"];
    let mut text = String::new();
    for (place, word) in name.split('_').enumerate() {
        if place > 0 {
            text.push(' ');
        }
        if ACRONYMS.contains(&word) {
            text.push_str(word);
        } else if place == 0 && capital {
            let mut letters = word.chars();
            if let Some(first) = letters.next() {
                text.push(first);
                text.push_str(&letters.as_str().to_lowercase());
            }
        } else {
            text.push_str(&word.to_lowercase());
        }
    }
    text
}

fn named(name: Option<&str>, raw: impl std::fmt::Display) -> String {
    name.map_or_else(|| format!("type {raw}"), words)
}

/// The descriptor an AEM command or response is about, where its
/// payload names one.
fn aem_target(aem: &AemPdu<'_>) -> Option<(DescriptorType, u16)> {
    let at = match aem.command_type {
        // configuration_index and a reserved field come first.
        AemCommandType::READ_DESCRIPTOR => 4,
        AemCommandType::SET_NAME
        | AemCommandType::GET_NAME
        | AemCommandType::SET_STREAM_FORMAT
        | AemCommandType::GET_STREAM_FORMAT
        | AemCommandType::SET_STREAM_INFO
        | AemCommandType::GET_STREAM_INFO
        | AemCommandType::SET_SAMPLING_RATE
        | AemCommandType::GET_SAMPLING_RATE
        | AemCommandType::SET_CLOCK_SOURCE
        | AemCommandType::GET_CLOCK_SOURCE
        | AemCommandType::SET_CONTROL
        | AemCommandType::GET_CONTROL
        | AemCommandType::GET_AVB_INFO
        | AemCommandType::GET_COUNTERS
        | AemCommandType::GET_AUDIO_MAP
        | AemCommandType::ADD_AUDIO_MAPPINGS
        | AemCommandType::REMOVE_AUDIO_MAPPINGS => 0,
        _ => return None,
    };
    let bytes = aem.payload.get(at..at + 4)?;
    Some((
        DescriptorType(u16::from_be_bytes([bytes[0], bytes[1]])),
        u16::from_be_bytes([bytes[2], bytes[3]]),
    ))
}

/// What `frame` says, the entities in it named as `triib` knows them.
pub fn describe(triib: &Triib, frame: &Frame) -> Line {
    let mut line = Line {
        protocol: None,
        entity: None,
        summary: String::new(),
        refusal: None,
        warnings: Vec::new(),
    };
    if let Some(missing) = pdu::missing_octets(&frame.bytes).filter(|&missing| missing > 0) {
        line.warnings.push(format!(
            "Its control_data_length claims {missing} octets past the end of the frame."
        ));
    }
    match pdu::decode(&frame.bytes) {
        Err(error) => {
            line.summary = "Not decoded".to_owned();
            line.warnings.push(format!("It does not decode: {error}."));
        }
        Ok(Pdu::Adp(adpdu)) => {
            line.protocol = Some(Protocol::Adp);
            line.summary = match adpdu.message_type {
                AdpMessageType::ENTITY_AVAILABLE => "Available".to_owned(),
                AdpMessageType::ENTITY_DEPARTING => "Departing".to_owned(),
                AdpMessageType::ENTITY_DISCOVER if adpdu.entity_id.0 == 0 => {
                    "Discover every entity".to_owned()
                }
                AdpMessageType::ENTITY_DISCOVER => "Discover".to_owned(),
                other => named(other.name(), other.0),
            };
            line.entity = (adpdu.entity_id.0 != 0).then_some(adpdu.entity_id);
        }
        Ok(Pdu::Aem(aem)) => {
            line.protocol = Some(Protocol::Aecp);
            line.entity = Some(aem.header.target_entity_id);
            let mut summary = named(aem.command_type.name(), aem.command_type.0);
            if let Some((descriptor_type, index)) = aem_target(&aem) {
                let kind = descriptor_type.name().map_or_else(
                    || format!("descriptor {}", descriptor_type.0),
                    |name| sentence(name, false),
                );
                summary.push_str(&format!(", {kind} {index}"));
            }
            let response = aem.header.message_type == AecpMessageType::AEM_RESPONSE;
            if aem.unsolicited {
                summary.push_str(", notification");
            } else if response {
                summary.push_str(", response");
            }
            if response && !aem.status().is_success() {
                line.refusal = Some(named(aem.status().name(), aem.status().0));
            }
            line.summary = summary;
        }
        Ok(Pdu::VendorUnique(vendor_unique)) => {
            line.protocol = Some(Protocol::Aecp);
            line.entity = Some(vendor_unique.header.target_entity_id);
            let response =
                vendor_unique.header.message_type == AecpMessageType::VENDOR_UNIQUE_RESPONSE;
            line.summary = match MvuMessage::from_pdu(&vendor_unique) {
                Ok(message) => {
                    let mut summary = format!(
                        "Milan: {}",
                        named(message.command_type.name(), message.command_type.0)
                    );
                    if message.unsolicited {
                        summary.push_str(", notification");
                    } else if response {
                        summary.push_str(", response");
                    }
                    summary
                }
                Err(_) => "Vendor unique".to_owned(),
            };
            if response && vendor_unique.header.status != 0 {
                let status = AemStatus(vendor_unique.header.status);
                line.refusal = Some(named(status.name(), status.0));
            }
        }
        Ok(Pdu::Aecp(header, _)) => {
            line.protocol = Some(Protocol::Aecp);
            line.entity = Some(header.target_entity_id);
            line.summary = named(header.message_type.name(), header.message_type.0);
        }
        Ok(Pdu::Acmp(acmpdu)) => {
            line.protocol = Some(Protocol::Acmp);
            describe_acmp(triib, frame, &acmpdu, &mut line);
        }
        Ok(Pdu::Other { subtype }) => line.summary = format!("Subtype {subtype:#04x}"),
    }
    line
}

fn describe_acmp(triib: &Triib, frame: &Frame, acmpdu: &Acmpdu, line: &mut Line) {
    let message = acmpdu.message_type;
    // RX commands and responses are about the listener, TX ones the talker.
    let to_talker = matches!(
        message,
        AcmpMessageType::CONNECT_TX_COMMAND
            | AcmpMessageType::CONNECT_TX_RESPONSE
            | AcmpMessageType::DISCONNECT_TX_COMMAND
            | AcmpMessageType::DISCONNECT_TX_RESPONSE
            | AcmpMessageType::GET_TX_STATE_COMMAND
            | AcmpMessageType::GET_TX_STATE_RESPONSE
            | AcmpMessageType::GET_TX_CONNECTION_COMMAND
            | AcmpMessageType::GET_TX_CONNECTION_RESPONSE
    );
    let addressed = if to_talker {
        acmpdu.talker_entity_id
    } else {
        acmpdu.listener_entity_id
    };
    line.entity = (addressed.0 != 0).then_some(addressed);
    let stream = |entity: EntityId, descriptor_type, index| {
        if entity.0 == 0 {
            None
        } else {
            Some(triib.stream_name(entity, descriptor_type, index))
        }
    };
    let talker = stream(
        acmpdu.talker_entity_id,
        DescriptorType::STREAM_OUTPUT,
        acmpdu.talker_unique_id,
    );
    let listener = stream(
        acmpdu.listener_entity_id,
        DescriptorType::STREAM_INPUT,
        acmpdu.listener_unique_id,
    );
    let mut summary = named(message.name(), message.0);
    match (talker, listener) {
        (Some(talker), Some(listener)) => summary.push_str(&format!(", {talker} to {listener}")),
        (Some(talker), None) => summary.push_str(&format!(", {talker}")),
        (None, Some(listener)) => summary.push_str(&format!(", {listener}")),
        (None, None) => {}
    }
    line.summary = summary;
    let response = message.0 % 2 == 1;
    if response && acmpdu.status != AcmpStatus::SUCCESS {
        line.refusal = Some(named(acmpdu.status.name(), acmpdu.status.0));
    }
    // A Milan entity sends the short form (Milan 1.3, 5.5.2.2).
    if !frame.sent && acmpdu.ip.is_some() {
        let sender = if response {
            addressed
        } else {
            acmpdu.controller_entity_id
        };
        let milan = triib
            .models
            .get(&sender)
            .is_some_and(|model| model.milan.is_some());
        if milan {
            line.warnings.push(
                "It is in the long ACMP form, which a Milan entity may not send (Milan 1.3, \
                 5.5.2.2)."
                    .to_owned(),
            );
        }
    }
}

pub fn view(triib: &Triib) -> Element<'_, Message> {
    let log = &triib.log;
    let lines: Vec<&Entry> = log
        .entries
        .iter()
        .rev()
        .filter(|entry| entry.line.shown_by(log.filter))
        .collect();
    let filter = |label: &'static str, value: LogFilter| {
        button::button(Kind::Filled, label)
            .size(Size::ExtraSmall)
            .selected(log.filter == value)
            .on_press(Message::Log(LogMessage::Filter(value)))
    };
    let pause = if log.paused {
        button::with_icon(Kind::Tonal, Icon::ProgressActivity, "Resume")
    } else {
        button::with_icon(Kind::Filled, Icon::Stop, "Pause").selected(false)
    }
    .size(Size::ExtraSmall)
    .on_press(Message::Log(LogMessage::Paused(!log.paused)));
    let count = match (lines.len(), log.len()) {
        (shown, all) if shown == all => frames(all),
        (shown, all) => format!("{} of {}", frames(shown), all),
    };
    let header = row![
        component::connected(vec![
            filter("All", LogFilter::All),
            filter("Warnings", LogFilter::Warnings),
            filter("ADP", LogFilter::Adp),
            filter("AECP", LogFilter::Aecp),
            filter("ACMP", LogFilter::Acmp),
        ]),
        styled(count, Type::BodyMedium).style(style::on_surface_variant),
        space::horizontal(),
        pause,
        button::with_icon(Kind::Text, Icon::Delete, "Clear")
            .size(Size::ExtraSmall)
            .on_press_maybe((log.len() > 0).then_some(Message::Log(LogMessage::Cleared))),
    ]
    .spacing(12)
    .align_y(Center);
    let body: Element<'_, Message> = if lines.is_empty() {
        component::empty_state(
            Icon::History,
            if log.len() == 0 {
                "No frames yet"
            } else {
                "No frames to show"
            },
            if log.len() == 0 {
                "Every ATDECC frame triib sends and hears appears here, newest first."
            } else {
                "No frame kept matches the filter."
            },
        )
    } else {
        let mut list = column![].spacing(2);
        for entry in lines.into_iter().take(SHOWN) {
            list = list.push(log_line(triib, entry));
        }
        component::scroll(list.padding([0, 4])).height(Fill).into()
    };
    column![header, body]
        .spacing(12)
        .padding([16, 24])
        .width(Fill)
        .height(Fill)
        .into()
}

fn frames(count: usize) -> String {
    if count == 1 {
        "1 frame".to_owned()
    } else {
        format!("{count} frames")
    }
}

/// A frame's line: when, which way, the entity and what it says, then
/// its warnings, and its octets when opened.
fn log_line<'a>(triib: &'a Triib, entry: &Entry) -> Element<'a, Message> {
    let line = entry.line.clone();
    let log = &triib.log;
    let opened = log.open == Some(entry.number);
    let (glyph, way) = if entry.frame.sent {
        (Icon::ArrowForward, "Sent")
    } else {
        (Icon::ArrowBack, "Heard")
    };
    // The entity's name with the tag the matrix gives it, telling apart
    // entities of the same name.
    let entity: Element<'a, Message> = match line.entity {
        Some(entity_id) => match triib.entities.get(&entity_id) {
            Some(found) => row![
                styled(triib.entity_name(found), Type::BodyMedium)
                    .wrapping(iced::widget::text::Wrapping::None),
                styled(crate::matrix::tag(found), Type::BodySmall)
                    .font(Font::MONOSPACE)
                    .style(style::on_surface_variant)
                    .wrapping(iced::widget::text::Wrapping::None),
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
            None => styled(entity_id.to_string(), Type::BodyMedium).into(),
        },
        None => space().into(),
    };
    let mut summary = row![styled(line.summary, Type::BodyMedium)].spacing(8);
    if let Some(refusal) = line.refusal {
        summary = summary.push(styled(refusal, Type::BodyMedium).style(style::error_text));
    }
    let top = row![
        styled(
            format!("{:.3}", log.seconds(entry.frame.at)),
            Type::BodySmall
        )
        .font(Font::MONOSPACE)
        .style(style::on_surface_variant)
        .width(Length::Fixed(TIME_WIDTH)),
        component::tip(icon::icon(glyph, 16).style(style::on_surface_variant), way),
        container(entity)
            .width(Length::Fixed(ENTITY_WIDTH))
            .clip(true),
        summary.width(Fill),
    ]
    .spacing(12)
    .align_y(Center);
    let mut lines = column![top].spacing(4);
    for warning in line.warnings {
        lines = lines.push(
            row![
                space().width(Length::Fixed(TIME_WIDTH + 12.0 + 16.0 + 12.0)),
                icon::icon(Icon::Warning, 16).style(style::error_text),
                styled(warning, Type::BodySmall).style(style::error_text),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    if opened {
        lines = lines.push(
            container(
                styled(hex(&entry.frame.bytes), Type::BodySmall)
                    .font(Font::MONOSPACE)
                    .style(style::on_surface_variant),
            )
            .padding(iced::Padding {
                left: TIME_WIDTH + 12.0,
                ..iced::Padding::ZERO
            }),
        );
    }
    let toggle = Message::Log(LogMessage::Opened((!opened).then_some(entry.number)));
    mouse_area(container(lines).padding([4, 8]).width(Fill))
        .on_press(toggle)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

/// Octets as rows of 16 in hex, each row with its offset.
fn hex(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let octets: Vec<String> = chunk.iter().map(|octet| format!("{octet:02x}")).collect();
            format!("{:04x}  {}", row * 16, octets.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use atdecc::acmp::AcmpIp;
    use atdecc::aecp::AecpHeader;
    use avb_net::MacAddress;

    const WIRED_ESP: EntityId = EntityId(0xe8f6_0ae0_9220_0000);
    const CONTROLLER: EntityId = EntityId(0x9c6b_00ff_fe30_9a2b);

    fn sample() -> Triib {
        let (entities, models) = crate::view::tests::bench();
        let interface = avb_net::Interface {
            name: "enp6s0".to_owned(),
            mac: MacAddress([0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b]),
            up: true,
            speed: None,
            physical: true,
            wireless: false,
            hardware_clock: None,
        };
        Triib::sample(
            crate::settings::Settings::default(),
            interface,
            entities,
            models,
        )
    }

    fn frame(sent: bool, bytes: Vec<u8>) -> Frame {
        Frame {
            at: SystemTime::UNIX_EPOCH,
            sent,
            peer: MacAddress([0; 6]),
            bytes,
        }
    }

    fn aem(
        message_type: AecpMessageType,
        status: AemStatus,
        command_type: AemCommandType,
        payload: &[u8],
    ) -> Vec<u8> {
        let pdu = AemPdu {
            header: AecpHeader {
                message_type,
                status: status.0,
                target_entity_id: WIRED_ESP,
                controller_entity_id: CONTROLLER,
                sequence_id: 7,
            },
            unsolicited: false,
            controller_request: false,
            command_type,
            payload,
        };
        let mut out = [0; 128];
        let length = pdu.encode(&mut out).unwrap();
        out[..length].to_vec()
    }

    /// A few minutes of traffic on the bench, for pictures: discovery,
    /// reading, a refusal, a short frame and a connection.
    pub(crate) fn bench_frames(triib: &Triib) -> Vec<(Frame, Line)> {
        let at = |millis: u64| SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(millis);
        let mut frames = Vec::new();
        let mut push = |millis, sent, bytes: Vec<u8>| {
            frames.push(Frame {
                at: at(millis),
                ..frame(sent, bytes)
            });
        };
        push(
            0,
            true,
            atdecc::Adpdu::discover(EntityId(0))
                .to_bytes()
                .unwrap()
                .to_vec(),
        );
        for (place, found) in triib.entities.values().enumerate() {
            push(
                20 + 7 * place as u64,
                false,
                found.adp.to_bytes().unwrap().to_vec(),
            );
        }
        push(
            140,
            true,
            aem(
                AecpMessageType::AEM_COMMAND,
                AemStatus::SUCCESS,
                AemCommandType::READ_DESCRIPTOR,
                &[0, 0, 0, 0, 0, 5, 0, 1],
            ),
        );
        push(
            143,
            false,
            aem(
                AecpMessageType::AEM_RESPONSE,
                AemStatus::NOT_IMPLEMENTED,
                AemCommandType::GET_COUNTERS,
                &[0, 9, 0, 0],
            ),
        );
        let mut short = aem(
            AecpMessageType::AEM_RESPONSE,
            AemStatus::SUCCESS,
            AemCommandType::GET_COUNTERS,
            &[0, 9, 0, 0],
        );
        short[3] += 8;
        push(151, false, short);
        let connect = Acmpdu {
            message_type: AcmpMessageType::CONNECT_RX_COMMAND,
            talker_entity_id: EntityId(0xd111_e597_f544_8000),
            listener_entity_id: WIRED_ESP,
            controller_entity_id: CONTROLLER,
            ..Acmpdu::default()
        };
        let mut out = [0; 128];
        let length = connect.encode(&mut out).unwrap();
        push(2_310, true, out[..length].to_vec());
        frames
            .into_iter()
            .map(|frame| {
                let line = describe(triib, &frame);
                (frame, line)
            })
            .collect()
    }

    #[test]
    fn frames_read_as_what_they_say() {
        let triib = sample();
        let available = triib.entities[&WIRED_ESP].adp.to_bytes().unwrap().to_vec();
        let line = describe(&triib, &frame(false, available));
        assert_eq!(
            (line.summary.as_str(), line.entity),
            ("Available", Some(WIRED_ESP))
        );
        assert!(line.warnings.is_empty());

        // Configuration 0, reserved, then STREAM_INPUT 1.
        let read = aem(
            AecpMessageType::AEM_COMMAND,
            AemStatus::SUCCESS,
            AemCommandType::READ_DESCRIPTOR,
            &[0, 0, 0, 0, 0, 5, 0, 1],
        );
        let line = describe(&triib, &frame(true, read));
        assert_eq!(line.summary, "Read descriptor, stream input 1");

        let refused = aem(
            AecpMessageType::AEM_RESPONSE,
            AemStatus::NOT_IMPLEMENTED,
            AemCommandType::GET_COUNTERS,
            &[0, 9, 0, 0],
        );
        let line = describe(&triib, &frame(false, refused));
        assert_eq!(line.summary, "Get counters, AVB interface 0, response");
        assert_eq!(line.refusal.as_deref(), Some("Not implemented"));
    }

    #[test]
    fn rule_breaking_frames_are_marked() {
        let triib = sample();
        // A response claiming 8 octets more than it carries.
        let mut short = aem(
            AecpMessageType::AEM_RESPONSE,
            AemStatus::SUCCESS,
            AemCommandType::GET_COUNTERS,
            &[0, 9, 0, 0],
        );
        short[3] += 8;
        let line = describe(&triib, &frame(false, short));
        assert_eq!(
            line.warnings,
            ["Its control_data_length claims 8 octets past the end of the frame."]
        );
        let line = describe(&triib, &frame(false, vec![0xfb, 0x00, 0x00, 0x10]));
        assert_eq!(line.summary, "Not decoded");
        assert_eq!(line.warnings.len(), 2);

        // The long ACMP form from the wired endpoint, a Milan entity.
        let response = Acmpdu {
            message_type: AcmpMessageType::GET_RX_STATE_RESPONSE,
            listener_entity_id: WIRED_ESP,
            controller_entity_id: CONTROLLER,
            ip: Some(AcmpIp::default()),
            ..Acmpdu::default()
        };
        let mut out = [0; 128];
        let length = response.encode(&mut out).unwrap();
        let line = describe(&triib, &frame(false, out[..length].to_vec()));
        assert!(
            line.summary.starts_with("Get RX state response"),
            "{}",
            line.summary
        );
        assert_eq!(line.entity, Some(WIRED_ESP));
        assert!(
            line.warnings[0].contains("long ACMP form"),
            "{:?}",
            line.warnings
        );
        // The short form is as Milan wants.
        let short = Acmpdu {
            ip: None,
            ..response
        };
        let length = short.encode(&mut out).unwrap();
        assert!(
            describe(&triib, &frame(false, out[..length].to_vec()))
                .warnings
                .is_empty()
        );
    }

    #[test]
    fn the_log_keeps_the_newest_frames_until_paused_or_cleared() {
        let mut log = Log::default();
        let discover = atdecc::Adpdu::discover(EntityId(0))
            .to_bytes()
            .unwrap()
            .to_vec();
        let triib = sample();
        let line = describe(&triib, &frame(true, discover.clone()));
        let lines = |count| vec![(frame(true, discover.clone()), line.clone()); count];
        log.add(lines(LIMIT + 5));
        assert_eq!(log.len(), LIMIT);
        assert_eq!(log.entries.front().map(|entry| entry.number), Some(5));
        log.update(LogMessage::Paused(true));
        log.add(lines(1));
        assert_eq!(log.len(), LIMIT);
        log.update(LogMessage::Cleared);
        log.update(LogMessage::Paused(false));
        log.add(lines(1));
        assert_eq!(log.len(), 1);
    }
}
