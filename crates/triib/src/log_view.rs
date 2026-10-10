//! The log, in a panel under the view: the ATDECC frames triib sent and
//! heard, newest first, each with what it says and, marked, where its
//! sender broke the rules: a frame that does not decode, a
//! control_data_length claiming octets the frame does not carry, and the
//! long ACMP form from a Milan entity.

use std::collections::VecDeque;
use std::time::SystemTime;

use atdecc::EntityId;
use atdecc::acmp::{AcmpMessageType, AcmpStatus, Acmpdu};
use atdecc::adp::AdpMessageType;
use atdecc::aecp::{AecpMessageType, AemCommandType, AemPdu, AemStatus};
use atdecc::blocking::Frame;
use atdecc::descriptor::DescriptorType;
use atdecc::lite::{CvuMessage, LiteFlags, LiteMessage, LiteStatus};
use atdecc::mvu::MvuMessage;
use atdecc::pdu::{self, Pdu};
use atdecc::wireless::{TimeMode, WirelessFlags, WirelessMessage, WirelessStatus};
use avb_mrp::msrp;
use iced::widget::{container, mouse_area, space};
use iced::{Center, Element, Fill, Font, Length};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::{self, Icon};
use scramble_ui::{column, component, dir, line, resize, row, style};

use crate::app::{Message, Triib};
use crate::describe;
use crate::fl;

/// The most frames kept, the oldest dropped past it.
pub const LIMIT: usize = 5000;
/// The most lines drawn at once, the newest.
const SHOWN: usize = 500;
/// The least height the log panel takes: its edge, its header and a few
/// lines.
pub const LEAST_HEIGHT: f32 = 140.0;
/// The narrowest panel whose filters fit beside its title.
const HEADER_IN_ONE_ROW: f32 = 760.0;
/// The narrowest panel whose lines have what they say beside their entity.
const LINE_IN_ONE_ROW: f32 = 600.0;
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

fn named(name: Option<&str>, raw: impl std::fmt::Display) -> String {
    name.map_or_else(|| format!("type {raw}"), describe::flag_name)
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
        line.warnings
            .push(fl!("log-warning-short", missing = missing));
    }
    match pdu::decode(&frame.bytes) {
        Err(error) => {
            line.summary = fl!("log-not-decoded");
            line.warnings
                .push(fl!("log-warning-undecodable", error = error.to_string()));
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
                    |name| describe::name_words(name, false),
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
            line.summary = if let Ok(message) = LiteMessage::from_pdu(&vendor_unique) {
                describe_lite(message, response)
            } else if let Ok(message) = WirelessMessage::from_pdu(&vendor_unique) {
                describe_wireless(message, response)
            } else if let Ok(cvu) = CvuMessage::from_pdu(&vendor_unique) {
                // A talker sends CVU SRP about itself, to no one in
                // particular.
                line.entity = Some(cvu.sender);
                describe_cvu(&cvu)
            } else if let Ok(message) = MvuMessage::from_pdu(&vendor_unique) {
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
            } else {
                "Vendor unique".to_owned()
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

/// An AVB Lite status query or its answer, with what the answer says.
fn describe_lite(message: LiteMessage<'_>, response: bool) -> String {
    let mut summary = format!(
        "AVB Lite: {}",
        named(message.command_type.name(), message.command_type.0)
    );
    if message.unsolicited {
        summary.push_str(", notification");
    } else if response {
        summary.push_str(", response");
    }
    if let Ok(status) = LiteStatus::decode(message.data) {
        let mode = if status.flags.contains(LiteFlags::ACTIVE) {
            "active"
        } else if status.flags.contains(LiteFlags::CAPABLE) {
            "capable"
        } else {
            "not capable"
        };
        summary.push_str(&format!(", {mode}"));
        if let Some(offset) = status.offset() {
            summary.push_str(&format!(", offset {offset} ns"));
        }
    }
    summary
}

/// An AVB Wireless status query or its answer, with what the answer says.
fn describe_wireless(message: WirelessMessage<'_>, response: bool) -> String {
    let mut summary = format!(
        "AVB Wireless: {}",
        named(message.command_type.name(), message.command_type.0)
    );
    if message.unsolicited {
        summary.push_str(", notification");
    } else if response {
        summary.push_str(", response");
    }
    let Ok(status) = WirelessStatus::decode(message.data) else {
        return summary;
    };
    if status.access_point() {
        let plural = if status.station_count == 1 { "" } else { "s" };
        summary.push_str(&format!(
            ", access point, {} station{plural}",
            status.station_count
        ));
        return summary;
    }
    let mode = match status.time_mode {
        TimeMode::NONE => "no time".to_owned(),
        TimeMode::MODE_A_FTM => "Mode A over FTM".to_owned(),
        TimeMode::MODE_A_TM => "Mode A over TM".to_owned(),
        TimeMode::MODE_B => "Mode B".to_owned(),
        TimeMode(other) => format!("time mode {other}"),
    };
    let time = if status.flags.contains(WirelessFlags::LOCKED) {
        "locked"
    } else if status.flags.contains(WirelessFlags::HOLDOVER) {
        "holding over"
    } else {
        "not locked"
    };
    summary.push_str(&format!(", station, {mode}, {time}"));
    if let Some(rssi) = status.signal() {
        summary.push_str(&format!(", {rssi} dBm"));
    }
    summary
}

/// A CVU SRP message: the streams a talker declares or withdraws.
fn describe_cvu(cvu: &CvuMessage<'_>) -> String {
    let declarations: Vec<_> = msrp::talker_declarations(cvu.msrp).collect();
    let kind = match cvu.command_type {
        msrp::attribute::TALKER_ADVERTISE => "talker advertise",
        msrp::attribute::TALKER_FAILED => "talker failed",
        msrp::attribute::LISTENER => "listener",
        msrp::attribute::DOMAIN => "domain",
        _ => "declaration",
    };
    let mut summary = format!("CVU SRP: {kind}");
    if let Some(first) = declarations.first() {
        let verb = if first.event.declares() {
            ""
        } else {
            " withdrawn"
        };
        summary.push_str(&format!(
            ", stream {:#018x}{verb} to {}, VLAN {}, {}",
            first.stream_id,
            first.destination,
            first.vlan_id,
            crate::lite_view::rate(first.bandwidth(8000))
        ));
        if declarations.len() > 1 {
            summary.push_str(&format!(" and {} more", declarations.len() - 1));
        }
    }
    summary
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
            line.warnings.push(fl!("log-warning-long-acmp"));
        }
    }
}

/// The log panel under the view, `width` wide and `height` tall of the
/// `most` it may take, its top edge dragged to resize it: a header with
/// the filters, then the frames, newest first.
pub fn panel(triib: &Triib, width: f32, height: f32, most: f32) -> Element<'_, Message> {
    let log = &triib.log;
    let lines: Vec<&Entry> = log
        .entries
        .iter()
        .rev()
        .filter(|entry| entry.line.shown_by(log.filter))
        .collect();
    let filter = |label: String, value: LogFilter| {
        button::button(Kind::Filled, label)
            .size(Size::ExtraSmall)
            .selected(log.filter == value)
            .on_press(Message::Log(LogMessage::Filter(value)))
    };
    let wide = width >= HEADER_IN_ONE_ROW;
    let pause_message = Message::Log(LogMessage::Paused(!log.paused));
    let (pause_icon, pause_label) = if log.paused {
        (Icon::ProgressActivity, fl!("log-resume"))
    } else {
        (Icon::Stop, fl!("log-pause"))
    };
    let clear_message = (log.len() > 0).then_some(Message::Log(LogMessage::Cleared));
    // On a narrow window the buttons give up their labels.
    let (pause, clear): (Element<'_, Message>, Element<'_, Message>) = if wide {
        let pause = if log.paused {
            button::with_icon(Kind::Tonal, pause_icon, pause_label)
        } else {
            button::with_icon(Kind::Filled, pause_icon, pause_label).selected(false)
        };
        (
            pause.size(Size::ExtraSmall).on_press(pause_message).into(),
            button::with_icon(Kind::Text, Icon::Delete, fl!("log-clear"))
                .size(Size::ExtraSmall)
                .on_press_maybe(clear_message)
                .into(),
        )
    } else {
        (
            component::tip(
                button::icon_button(pause_icon)
                    .selected(log.paused)
                    .on_press(pause_message),
                pause_label,
            ),
            component::tip(
                button::icon_button(Icon::Delete).on_press_maybe(clear_message),
                fl!("log-clear"),
            ),
        )
    };
    let count = match (lines.len(), log.len()) {
        (shown, all) if shown == all => frames(all),
        (shown, all) => fl!("log-shown-of", shown = shown, all = all),
    };
    let title = styled(fl!("toolbar-log"), Type::TitleLarge);
    let filters = component::connected(vec![
        filter(fl!("log-all"), LogFilter::All),
        filter(fl!("log-warnings"), LogFilter::Warnings),
        filter("ADP".to_owned(), LogFilter::Adp),
        filter("AECP".to_owned(), LogFilter::Aecp),
        filter("ACMP".to_owned(), LogFilter::Acmp),
    ]);
    let count = styled(count, Type::BodyMedium)
        .style(style::on_surface_variant)
        .wrapping(iced::widget::text::Wrapping::None);
    let actions = row![
        pause,
        clear,
        component::tip(
            button::icon_button(Icon::Close).on_press(Message::LogToggled),
            scramble_ui::labels::get().close,
        ),
    ]
    .spacing(8)
    .align_y(Center);
    // On a narrow window the filters go under the title.
    let header: Element<'_, Message> = if wide {
        row![title, filters, count, space::horizontal(), actions]
            .spacing(16)
            .align_y(Center)
            .into()
    } else {
        column![
            row![title, count, space::horizontal(), actions]
                .spacing(12)
                .align_y(Center),
            filters,
        ]
        .spacing(4)
        .into()
    };
    let body: Element<'_, Message> = if lines.is_empty() {
        container(
            styled(
                if log.len() == 0 {
                    fl!("log-empty")
                } else {
                    fl!("log-none-match")
                },
                Type::BodyMedium,
            )
            .style(style::on_surface_variant)
            .center(),
        )
        .center(Fill)
        .into()
    } else {
        let mut list = column![].spacing(2);
        for entry in lines.into_iter().take(SHOWN) {
            list = list.push(log_line(triib, entry, width >= LINE_IN_ONE_ROW));
        }
        component::scroll(list.padding([0, 4])).height(Fill).into()
    };
    let handle = resize::handle(move |drag| Message::LogResized(drag, height, most)).height();
    container(column![
        handle,
        container(column![header, body].spacing(4))
            .padding(dir::padding(0.0, 12.0, 8.0, 24.0))
            .height(Fill),
    ])
    .width(Fill)
    .height(height)
    .style(style::chrome)
    .into()
}

fn frames(count: usize) -> String {
    fl!("log-frames", count = count)
}

/// A frame's line: when, which way, the entity and what it says, then
/// its warnings, and its octets when opened; what it says goes under the
/// entity unless `one_row`.
fn log_line<'a>(triib: &'a Triib, entry: &Entry, one_row: bool) -> Element<'a, Message> {
    let line = entry.line.clone();
    let log = &triib.log;
    let opened = log.open == Some(entry.number);
    // Out from triib, or in to it, pointing the way the line reads.
    let glyph = if entry.frame.sent != dir::mirrored() {
        Icon::ArrowForward
    } else {
        Icon::ArrowBack
    };
    let way = if entry.frame.sent {
        fl!("log-sent")
    } else {
        fl!("log-heard")
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
    let mut summary = line![styled(line.summary, Type::BodyMedium)].spacing(8);
    if let Some(refusal) = line.refusal {
        summary = summary.push(styled(refusal, Type::BodyMedium).style(style::error_text));
    }
    // What it says starts on the line's start side.
    let summary = summary.push(space::horizontal());
    let time = styled(
        format!("{:.3}", log.seconds(entry.frame.at)),
        Type::BodySmall,
    )
    .font(Font::MONOSPACE)
    .style(style::on_surface_variant)
    .width(Length::Fixed(TIME_WIDTH))
    .align_x(dir::text_start());
    let way = component::tip(icon::icon(glyph, 16).style(style::on_surface_variant), way);
    // Under the time, or beside the entity.
    let indent = if one_row {
        TIME_WIDTH + 12.0 + 16.0 + 12.0
    } else {
        TIME_WIDTH + 12.0
    };
    let mut lines = if one_row {
        column![
            row![
                time,
                way,
                container(entity)
                    .width(Length::Fixed(ENTITY_WIDTH))
                    .align_x(dir::horizontal_start())
                    .clip(true),
                summary.width(Fill),
            ]
            .spacing(12)
            .align_y(Center)
        ]
    } else {
        column![
            row![
                time,
                way,
                container(entity)
                    .width(Fill)
                    .align_x(dir::horizontal_start())
                    .clip(true)
            ]
            .spacing(12)
            .align_y(Center),
            row![space().width(Length::Fixed(indent)), summary.width(Fill)],
        ]
    }
    .spacing(4);
    for warning in line.warnings {
        lines = lines.push(
            row![
                space().width(Length::Fixed(indent)),
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
            .padding(dir::padding(
                0.0,
                0.0,
                0.0,
                if one_row { TIME_WIDTH + 12.0 } else { 0.0 },
            )),
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
    fn avb_lite_frames_say_what_they_report() {
        let triib = sample();
        let mut out = [0; 64];
        let length =
            atdecc::lite::encode_get_lite_status(WIRED_ESP, CONTROLLER, 3, 0, &mut out).unwrap();
        let line = describe(&triib, &frame(true, out[..length].to_vec()));
        assert_eq!(line.summary, "AVB Lite: Get lite status");
        assert_eq!(line.entity, Some(WIRED_ESP));

        let [notification, declaration] =
            <[Vec<u8>; 2]>::try_from(crate::view::tests::lite_frames(-180, 6_336)).unwrap();
        let line = describe(&triib, &frame(false, notification));
        assert_eq!(
            line.summary,
            "AVB Lite: Get lite status, notification, active, offset -180 ns"
        );
        assert_eq!(line.entity, Some(WIRED_ESP));
        // CVU SRP is about the talker that sends it.
        let line = describe(&triib, &frame(false, declaration));
        assert_eq!(
            line.summary,
            "CVU SRP: talker advertise, stream 0xd111e597f5448000 to 91:e0:f0:00:6a:20, \
             VLAN 2, 17 Mb/s"
        );
        assert_eq!(line.entity, Some(EntityId(0xd111_e597_f544_8000)));
        assert!(line.warnings.is_empty());
    }

    #[test]
    fn avb_wireless_frames_say_what_they_report() {
        let triib = sample();
        let mut out = [0; 64];
        let length =
            atdecc::wireless::encode_get_wireless_status(WIRED_ESP, CONTROLLER, 4, 0, &mut out)
                .unwrap();
        let line = describe(&triib, &frame(true, out[..length].to_vec()));
        assert_eq!(line.summary, "AVB Wireless: Get wireless status");
        let [notification] =
            <[Vec<u8>; 1]>::try_from(crate::view::tests::wireless_frames(WirelessFlags::LOCKED))
                .unwrap();
        let line = describe(&triib, &frame(false, notification));
        assert_eq!(
            line.summary,
            "AVB Wireless: Get wireless status, notification, station, Mode A over FTM, locked, \
             -52 dBm"
        );
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
