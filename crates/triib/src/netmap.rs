//! The network view: the gPTP tree with each bridge's devices in a column
//! under it, showing the clock, the audio streams or the media clock
//! streams, each stream on a wire of its own coloured by its talker, and
//! beside it the details of what is brought forward.

mod map;

use std::collections::HashMap;

use atdecc::descriptor::DescriptorType;
use atdecc::model::EntityModel;
use atdecc::{ClockIdentity, DiscoveredEntity, EntityId};
use iced::widget::{button as plain_button, column, container, mouse_area, row, scrollable, space};
use iced::{Center, Element, Fill, Length, Theme};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::component;
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::{self, Icon};
use scramble_ui::{Scheme, shape, style};

use crate::app::{Message, NetworkState, Triib};
use crate::describe;
use crate::settings::NetworkShows;
use crate::topology::{
    Apart, EntityReport, InterfaceReport, Kind as NodeKind, NodeId, Route, Topology,
};

const PANEL_WIDTH: f32 = 360.0;
const NARROW_PANEL_WIDTH: f32 = 300.0;
/// Below this width the side panel gives way to the map, a button in the
/// header switching between the two.
const WIDE: f32 = 880.0;
/// From this width the side panel takes its full width.
const ROOMY: f32 = 1200.0;
/// Below this width the header leaves out its title and summary.
const TITLED: f32 = 1040.0;

/// What the view brings forward: a stream, or a node with its streams or
/// its clock path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    /// A stream, by its listener's stream input.
    Stream(EntityId, u16),
    Node(NodeKey),
}

/// A node, by what stays the same while the topology is built again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKey {
    Entity(EntityId),
    Clock(ClockIdentity),
    Host,
}

/// A colour on the map or in the details, the theme deciding the exact
/// colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Paint {
    /// The gPTP clock.
    Clock,
    /// A shade of its talker's hue, `shade` from 0 (darkest) to 100.
    Talker { hue: u16, shade: u8 },
    /// A failed reservation, or a problem.
    Failed,
    /// Advertised with no listener ready.
    Idle,
    /// A link gPTP does not run on.
    Unsynced,
    /// The theme's text colour.
    Text,
    /// The theme's softer text colour.
    Soft,
    /// The theme's muted colour, for secondary text.
    Muted,
    /// The theme's outline colour, for borders.
    Plain,
}

/// How far a stream, or the clock on a link, gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Audio or the clock is flowing.
    Flowing,
    /// Bound and announced, but no listener is ready, so nothing is sent.
    Advertised,
    /// The talker's reservation failed.
    Failed,
    /// gPTP does not run on the link.
    Unsynced,
}

/// A wire on the map: a stream, or in the gPTP view the link up from a
/// node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Line {
    /// The links it crosses in order, each named by its lower node, and
    /// whether it goes up that link.
    pub hops: Vec<(NodeId, bool)>,
    pub status: Status,
    pub paint: Paint,
    /// Lit under what is brought forward.
    pub related: bool,
    /// The stream brought forward.
    pub picked: bool,
    /// What clicking it brings forward.
    pub focus: Focus,
}

/// A node as its card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Card {
    pub icon: char,
    pub name: String,
    /// The end of the node's MAC address, for telling alike names apart.
    pub tag: String,
    pub subtitle: String,
    pub subtitle_paint: Paint,
    pub border: Paint,
    /// A two pixel border, for what is brought forward.
    pub strong: bool,
    pub dashed: bool,
    /// Faded, taking no part in what is brought forward.
    pub faded: bool,
    pub focus: Focus,
}

/// Everything the map draws.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Map {
    pub topology: Topology,
    /// By node.
    pub cards: Vec<Card>,
    pub lines: Vec<Line>,
    /// What is brought forward, so clicking it again lets it go.
    pub focus: Option<Focus>,
}

/// A bound stream.
#[derive(Debug, Clone)]
struct Stream {
    input: (EntityId, u16),
    talker: EntityId,
    name: String,
    talker_name: String,
    listener_name: String,
    media_clock: bool,
    status: Status,
    reason: String,
    route: Route,
}

impl Stream {
    fn focus(&self) -> Focus {
        Focus::Stream(self.input.0, self.input.1)
    }
}

pub fn view(triib: &Triib) -> Element<'_, Message> {
    iced::widget::responsive(move |size| view_at(triib, size.width)).into()
}

/// The view for `width` pixels: the map with the side panel beside it, or
/// on narrow windows one of the two at a time.
fn view_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    let wide = width >= WIDE;
    let list_open = !wide && triib.network_list;
    let padding = if wide { [16, 24] } else { [12, 12] };
    let topology = topology_of(triib);
    let shows = triib.settings.network_shows;
    let streams: Vec<Stream> = match shows {
        NetworkShows::Clock => Vec::new(),
        _ => streams(triib, &topology)
            .into_iter()
            .filter(|stream| stream.media_clock == (shows == NetworkShows::MediaClock))
            .collect(),
    };
    let paints = paints(
        &streams
            .iter()
            .map(|stream| (stream.talker, stream.status))
            .collect::<Vec<_>>(),
    );
    // What is brought forward, as long as it is still there.
    let focus = triib.network_focus.filter(|focus| match focus {
        Focus::Stream(..) => streams.iter().any(|stream| stream.focus() == *focus),
        Focus::Node(key) => find(&topology, *key).is_some(),
    });
    let header = header(&topology, shows, &streams, focus, width, list_open);

    if topology.nodes.is_empty() {
        return column![
            header,
            component::empty_state(
                Icon::Hub,
                "No network to show yet",
                "Entities appear here once they have been read and have said where they sit in \
                 the gPTP tree.",
            )
        ]
        .spacing(16)
        .padding(padding)
        .into();
    }

    let lines = lines(&topology, shows, &streams, &paints, focus);
    let cards = cards(triib, &topology, shows, &streams, &paints, &lines, focus);
    let data = Map {
        topology: topology.clone(),
        cards,
        lines,
        focus,
    };
    // The map fits the room it is given, so it is laid out for that room.
    // A press on its empty space clears the selection.
    let canvas = mouse_area(
        container(iced::widget::responsive(move |area| {
            scrollable(map::map(data.clone(), area))
                .direction(scrollable::Direction::Both {
                    vertical: component::thin_scrollbar(),
                    horizontal: component::thin_scrollbar(),
                })
                .style(style::scrollbar)
                .width(Fill)
                .height(Fill)
                .into()
        }))
        .width(Fill)
        .height(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Scheme::of(theme).surface_container_low.into()),
            border: iced::border::rounded(shape::LARGE),
            ..container::Style::default()
        }),
    )
    .on_press(Message::SelectionCleared);
    let panel_width = if !wide {
        Length::Fill
    } else if width >= ROOMY {
        Length::Fixed(PANEL_WIDTH)
    } else {
        Length::Fixed(NARROW_PANEL_WIDTH)
    };
    let panel = || details(triib, &topology, shows, &streams, &paints, focus).view(panel_width);
    let body: Element<'_, Message> = if wide {
        row![canvas, panel()].spacing(16).height(Fill).into()
    } else if list_open {
        panel()
    } else {
        canvas.into()
    };
    column![header, body]
        .spacing(16)
        .padding(padding)
        .width(Fill)
        .height(Fill)
        .into()
}

/// The title, the choice of what to show, a summary and what is brought
/// forward; on narrow windows the last on a row of its own.
fn header<'a>(
    topology: &Topology,
    shows: NetworkShows,
    streams: &[Stream],
    focus: Option<Focus>,
    width: f32,
    list_open: bool,
) -> Element<'a, Message> {
    let wide = width >= WIDE;
    let choice = |glyph: Icon, label: &'static str, value: NetworkShows| {
        button::with_icon(Kind::Filled, glyph, label)
            .size(Size::ExtraSmall)
            .selected(shows == value)
            .on_press(Message::NetworkShows(value))
    };
    let choices = component::connected(vec![
        choice(Icon::Schedule, "gPTP", NetworkShows::Clock),
        choice(Icon::GraphicEq, "Audio", NetworkShows::Audio),
        choice(Icon::Timer, "CRF", NetworkShows::MediaClock),
    ]);
    let focused = focus.map(|focus| {
        let what = match focus {
            Focus::Stream(..) => streams
                .iter()
                .find(|stream| stream.focus() == focus)
                .map(|stream| stream.name.clone())
                .unwrap_or_default(),
            Focus::Node(key) => {
                let name = find(topology, key)
                    .map(|node| node_name(topology, node))
                    .unwrap_or_default();
                match shows {
                    NetworkShows::Clock => format!("{name}’s clock path"),
                    _ => format!("{name}’s streams"),
                }
            }
        };
        button::with_icon(Kind::Outlined, Icon::Close, format!("Showing {what}"))
            .size(Size::ExtraSmall)
            .on_press(Message::NetworkFocused(None))
    });
    let mut header = row![].spacing(12).align_y(Center);
    if width >= TITLED {
        header = header.push(styled("Network", Type::TitleLarge));
    }
    header = header.push(choices);
    if width >= TITLED {
        let bridges = topology
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Bridge)
            .count();
        let mut entities: Vec<EntityId> = topology
            .nodes
            .iter()
            .filter_map(|node| match node.kind {
                NodeKind::Entity { entity_id, .. } => Some(entity_id),
                _ => None,
            })
            .collect();
        entities.sort_by_key(|entity_id| entity_id.0);
        entities.dedup();
        let devices = entities.len();
        header = header.push(
            styled(
                format!(
                    "{devices} {}, {bridges} {}",
                    if devices == 1 { "device" } else { "devices" },
                    if bridges == 1 { "bridge" } else { "bridges" }
                ),
                Type::BodyMedium,
            )
            .style(style::on_surface_variant),
        );
    }
    header = header.push(space::horizontal());
    if wide {
        if let Some(focused) = focused {
            header = header.push(focused);
        }
        return header.into();
    }
    header = header.push(component::toggle_tool(
        Icon::ViewList,
        if list_open {
            "Show the map"
        } else {
            "Show the details"
        },
        list_open,
        Message::NetworkListToggled,
    ));
    match focused {
        Some(focused) => column![header, focused].spacing(8).into(),
        None => header.into(),
    }
}

/// The tree the entities' reports make, with this computer when the
/// network runs.
fn topology_of(triib: &Triib) -> Topology {
    let names: Vec<(EntityId, String)> = triib
        .entities
        .values()
        .map(|entity| (entity.entity_id(), triib.entity_name(entity)))
        .collect();
    let reports: Vec<EntityReport<'_>> = names
        .iter()
        .map(|(entity_id, name)| {
            let model = triib.models.get(entity_id);
            let interfaces = model
                .map(|model| {
                    model
                        .avb_interfaces()
                        .map(|interface| InterfaceReport {
                            index: interface.index,
                            clock: interface.clock_identity,
                            path: model.as_path(interface.index),
                            info: model.avb_info(interface.index),
                        })
                        .collect()
                })
                .unwrap_or_default();
            EntityReport {
                entity_id: *entity_id,
                name,
                interfaces,
            }
        })
        .collect();
    let host = matches!(triib.network_state, NetworkState::Running { .. }).then(|| {
        triib
            .neighbor
            .map(|neighbor| (neighbor.clock, neighbor.port, neighbor.synced))
    });
    Topology::build(&reports, host)
}

fn key_of(topology: &Topology, node: NodeId) -> NodeKey {
    match topology.nodes[node].kind {
        NodeKind::Entity { entity_id, .. } => NodeKey::Entity(entity_id),
        NodeKind::Host => NodeKey::Host,
        NodeKind::Bridge => NodeKey::Clock(topology.nodes[node].clock.unwrap_or(ClockIdentity(0))),
    }
}

fn find(topology: &Topology, key: NodeKey) -> Option<NodeId> {
    (0..topology.nodes.len()).find(|&node| key_of(topology, node) == key)
}

/// The last two octets of the entity's MAC address.
fn tag(entity: &DiscoveredEntity) -> String {
    let [.., fifth, sixth] = entity.mac.0;
    format!("{fifth:02x}:{sixth:02x}")
}

fn stream_name(model: Option<&EntityModel>, descriptor_type: DescriptorType, index: u16) -> String {
    model
        .and_then(|model| model.name_of(descriptor_type, index))
        .map_or_else(|| format!("Stream {index}"), str::to_owned)
}

/// A node's name on its card and in the details: a bridge's vendor, else
/// "Bridge".
fn node_name(topology: &Topology, node: NodeId) -> String {
    let entry = &topology.nodes[node];
    match entry.kind {
        NodeKind::Bridge => entry
            .clock
            .and_then(crate::topology::mac_of)
            .and_then(|mac| crate::vendor::name([mac[0], mac[1], mac[2]]))
            .unwrap_or_else(|| "Bridge".to_owned()),
        NodeKind::Host => "This computer".to_owned(),
        NodeKind::Entity { .. } => entry.name.clone(),
    }
}

/// Every bound stream input, by listener, with how far its stream gets.
fn streams(triib: &Triib, topology: &Topology) -> Vec<Stream> {
    let mut streams = Vec::new();
    for listener in triib.shown_entities() {
        let listener_id = listener.entity_id();
        let Some(model) = triib.models.get(&listener_id) else {
            continue;
        };
        for input in model.streams(true) {
            let Some((talker_id, output)) = model
                .binding(input.index)
                .and_then(|binding| binding.talker_stream())
            else {
                continue;
            };
            let info = model.stream_info(DescriptorType::STREAM_INPUT, input.index);
            let status = match info {
                Some(info) if info.talker_failed() => Status::Failed,
                Some(info) if info.settled() => Status::Flowing,
                _ => Status::Advertised,
            };
            let listener_name = triib.entity_name(listener);
            // A stream that is not flowing reaches its talker's bridge, or
            // a failed one the bridge that refused it.
            let talker_leg = match topology.placed(talker_id) {
                Some(talker) if topology.nodes[talker].parent.is_some() => Route {
                    hops: vec![(talker, true)],
                    stops: topology.nodes[talker].parent,
                },
                _ => Route::default(),
            };
            let (route, reason) = match status {
                Status::Flowing | Status::Unsynced => (
                    topology.route(talker_id, listener_id),
                    "Connected".to_owned(),
                ),
                Status::Advertised => (
                    talker_leg,
                    if topology.placed(listener_id).is_none() {
                        format!(
                            "Advertised, no listener ready ({listener_name} is not on the gPTP \
                             tree)"
                        )
                    } else {
                        "Advertised, no listener ready".to_owned()
                    },
                ),
                Status::Failed => {
                    let code = info.map_or(0, |info| info.msrp_failure_code);
                    let [_, _, mac @ ..] = info
                        .map_or(0, |info| info.msrp_failure_bridge_id)
                        .to_be_bytes();
                    let full = topology.route(talker_id, listener_id);
                    match topology.bridge_with_mac(mac) {
                        Some(bridge) => (
                            if via(topology, &full).contains(&bridge) {
                                topology.cut(&full, bridge)
                            } else {
                                talker_leg
                            },
                            format!(
                                "Reservation failed at {}: {}",
                                node_name(topology, bridge),
                                describe::msrp_failure(code)
                            ),
                        ),
                        None => (
                            talker_leg,
                            format!("Reservation failed: {}", describe::msrp_failure(code)),
                        ),
                    }
                }
            };
            let talker_model = triib.models.get(&talker_id);
            streams.push(Stream {
                input: (listener_id, input.index),
                talker: talker_id,
                name: format!(
                    "{} → {}",
                    stream_name(talker_model, DescriptorType::STREAM_OUTPUT, output),
                    stream_name(Some(model), DescriptorType::STREAM_INPUT, input.index)
                ),
                talker_name: triib.entity_name_of(talker_id),
                listener_name,
                media_clock: input.current_format.is_clock(),
                status,
                reason,
                route,
            });
        }
    }
    streams
}

/// A talker's hue by its place among the talkers: well apart for the first
/// few, then by the golden angle.
fn hue(place: usize) -> u16 {
    const HUES: [u16; 8] = [172, 205, 262, 318, 28, 108, 140, 235];
    HUES.get(place)
        .copied()
        .unwrap_or_else(|| ((172.0 + place as f32 * 137.5) % 360.0) as u16)
}

/// Each stream's colour, from its talker and status: every talker has a
/// hue and its streams are shades of it; streams that are not flowing lose
/// their hue.
fn paints(streams: &[(EntityId, Status)]) -> Vec<Paint> {
    let mut talkers: Vec<EntityId> = Vec::new();
    for &(talker, _) in streams {
        if !talkers.contains(&talker) {
            talkers.push(talker);
        }
    }
    let mut seen: HashMap<EntityId, usize> = HashMap::new();
    streams
        .iter()
        .map(|&(talker, status)| {
            let count = streams.iter().filter(|other| other.0 == talker).count();
            let index = seen.entry(talker).or_default();
            let shade = if count == 1 {
                50
            } else {
                (*index * 100 / (count - 1)) as u8
            };
            *index += 1;
            match status {
                Status::Flowing | Status::Unsynced => Paint::Talker {
                    hue: hue(talkers
                        .iter()
                        .position(|&known| known == talker)
                        .unwrap_or(0)),
                    shade,
                },
                Status::Failed => Paint::Failed,
                Status::Advertised => Paint::Idle,
            }
        })
        .collect()
}

/// The bridges and other nodes a route enters, the upper end of each link
/// it crosses.
fn via(topology: &Topology, route: &Route) -> Vec<NodeId> {
    let mut nodes = Vec::new();
    for &(node, _) in &route.hops {
        if let Some(parent) = topology.nodes[node].parent
            && !nodes.contains(&parent)
        {
            nodes.push(parent);
        }
    }
    nodes
}

/// Whether `node` is `ancestor` or below it.
fn below(topology: &Topology, node: NodeId, ancestor: NodeId) -> bool {
    let mut at = Some(node);
    while let Some(current) = at {
        if current == ancestor {
            return true;
        }
        at = topology.nodes[current].parent;
    }
    false
}

fn entity_of(topology: &Topology, node: NodeId) -> Option<EntityId> {
    match topology.nodes[node].kind {
        NodeKind::Entity { entity_id, .. } => Some(entity_id),
        _ => None,
    }
}

/// Whether a stream sends from, goes to or passes through the node.
fn touches(topology: &Topology, stream: &Stream, node: NodeId) -> bool {
    let entity = entity_of(topology, node);
    entity.is_some() && (entity == Some(stream.talker) || entity == Some(stream.input.0))
        || via(topology, &stream.route).contains(&node)
}

/// Whether a stream is lit under `focus`.
fn lit(topology: &Topology, stream: &Stream, focus: Option<Focus>) -> bool {
    match focus {
        None => true,
        Some(Focus::Stream(..)) => focus == Some(stream.focus()),
        Some(Focus::Node(key)) => {
            find(topology, key).is_some_and(|node| touches(topology, stream, node))
        }
    }
}

fn lines(
    topology: &Topology,
    shows: NetworkShows,
    streams: &[Stream],
    paints: &[Paint],
    focus: Option<Focus>,
) -> Vec<Line> {
    if shows == NetworkShows::Clock {
        // The link up from every node on a tree, the clock flowing down it.
        let focused = match focus {
            Some(Focus::Node(key)) => find(topology, key),
            _ => None,
        };
        return (0..topology.nodes.len())
            .filter(|&node| topology.nodes[node].parent.is_some())
            .map(|node| {
                let synced = topology.nodes[node].link.synced;
                Line {
                    hops: vec![(node, false)],
                    status: if synced {
                        Status::Flowing
                    } else {
                        Status::Unsynced
                    },
                    paint: if synced {
                        Paint::Clock
                    } else {
                        Paint::Unsynced
                    },
                    related: focused.is_none_or(|focused| {
                        below(topology, node, focused) || below(topology, focused, node)
                    }),
                    picked: false,
                    focus: Focus::Node(key_of(topology, node)),
                }
            })
            .collect();
    }
    streams
        .iter()
        .zip(paints)
        .map(|(stream, &paint)| Line {
            hops: stream.route.hops.clone(),
            status: stream.status,
            paint,
            related: lit(topology, stream, focus),
            picked: focus == Some(stream.focus()),
            focus: stream.focus(),
        })
        .collect()
}

fn cards(
    triib: &Triib,
    topology: &Topology,
    shows: NetworkShows,
    streams: &[Stream],
    paints: &[Paint],
    lines: &[Line],
    focus: Option<Focus>,
) -> Vec<Card> {
    let picked = streams
        .iter()
        .zip(paints)
        .find(|(stream, _)| focus == Some(stream.focus()));
    let interface = triib.settings.interface.as_deref().unwrap_or_default();
    // Whether a lit line enters or leaves the node.
    let on_lit_line = |node: NodeId, lit: &dyn Fn(&Line) -> bool| {
        lines.iter().any(|line| {
            lit(line)
                && line
                    .hops
                    .iter()
                    .any(|&(hop, _)| hop == node || topology.nodes[hop].parent == Some(node))
        })
    };
    (0..topology.nodes.len())
        .map(|node| {
            let entry = &topology.nodes[node];
            let key = key_of(topology, node);
            let entity = entity_of(topology, node);
            let (glyph, tag) = match entry.kind {
                NodeKind::Bridge => (Icon::Hub, String::new()),
                NodeKind::Host => (Icon::Computer, String::new()),
                NodeKind::Entity { entity_id, .. } => {
                    let found = triib.entities.get(&entity_id);
                    (
                        found.map_or(Icon::GraphicEq, |entity| {
                            crate::describe::glyph(&entity.adp)
                        }),
                        found.map(tag).unwrap_or_default(),
                    )
                }
            };
            let mut border = Paint::Plain;
            let (subtitle, subtitle_paint) = match (shows, entry.apart, entry.kind) {
                (_, Some(Apart::NoNeighbor), _) => {
                    (format!("No bridge heard on {interface}"), Paint::Muted)
                }
                (_, Some(apart), _) => (
                    match apart {
                        Apart::NoPath(_) => "Path not reported",
                        Apart::Unreported => "gPTP not reported",
                        _ => "Not on the gPTP tree",
                    }
                    .to_owned(),
                    Paint::Failed,
                ),
                (NetworkShows::Clock, None, _) if entry.parent.is_none() => {
                    border = Paint::Clock;
                    ("Grandmaster".to_owned(), Paint::Clock)
                }
                (NetworkShows::Clock, None, NodeKind::Host) => (
                    if entry.link.synced {
                        "Synced"
                    } else {
                        "Not synced"
                    }
                    .to_owned(),
                    Paint::Muted,
                ),
                (NetworkShows::Clock, None, _) if entry.link.synced => {
                    ("Synced".to_owned(), Paint::Muted)
                }
                (NetworkShows::Clock, None, _) => ("Not synced".to_owned(), Paint::Failed),
                (_, None, NodeKind::Host) => (format!("triib on {interface}"), Paint::Muted),
                (_, None, _) if entity.is_none() || !entry.children.is_empty() => {
                    let through = streams
                        .iter()
                        .filter(|stream| via(topology, &stream.route).contains(&node))
                        .count();
                    (format!("{through} through"), Paint::Muted)
                }
                (_, None, _) => {
                    let sending = streams
                        .iter()
                        .filter(|stream| entity == Some(stream.talker))
                        .count();
                    let receiving = streams
                        .iter()
                        .filter(|stream| entity == Some(stream.input.0))
                        .count();
                    let failed = streams
                        .iter()
                        .filter(|stream| {
                            entity == Some(stream.input.0) && stream.status == Status::Failed
                        })
                        .count();
                    let mut counts = Vec::new();
                    if sending > 0 {
                        counts.push(format!("{sending} out"));
                    }
                    if receiving > 0 {
                        counts.push(format!("{receiving} in"));
                    }
                    let mut text = if counts.is_empty() {
                        "None".to_owned()
                    } else {
                        counts.join(", ")
                    };
                    if failed > 0 {
                        text.push_str(&format!(", {failed} failed"));
                        border = Paint::Failed;
                        (text, Paint::Failed)
                    } else {
                        (text, Paint::Muted)
                    }
                }
            };
            let focused = focus == Some(Focus::Node(key));
            let in_picked = picked.is_some_and(|(stream, _)| {
                entity.is_some()
                    && (entity == Some(stream.talker) || entity == Some(stream.input.0))
            });
            let mut strong = false;
            if let Some((_, &paint)) = picked.filter(|_| in_picked) {
                border = paint;
                strong = true;
            }
            if focused {
                border = Paint::Text;
                strong = true;
            }
            // Under a focus, what takes no part in it fades.
            let faded = match focus {
                None => false,
                Some(_) if focused || in_picked => false,
                Some(Focus::Stream(..)) => !on_lit_line(node, &|line| line.picked),
                Some(Focus::Node(_)) if shows == NetworkShows::Clock => {
                    !on_lit_line(node, &|line| line.related)
                }
                Some(Focus::Node(_)) => !streams
                    .iter()
                    .any(|stream| lit(topology, stream, focus) && touches(topology, stream, node)),
            };
            Card {
                icon: glyph.codepoint(),
                name: node_name(topology, node),
                tag,
                subtitle,
                subtitle_paint,
                border,
                strong,
                dashed: entry.apart.is_some()
                    || (shows == NetworkShows::Clock
                        && entry.parent.is_some()
                        && !entry.link.synced),
                faded,
                focus: Focus::Node(key),
            }
        })
        .collect()
}

/// A stream or node listed in the details.
struct Item {
    name: String,
    detail: String,
    detail_paint: Paint,
    swatch: Paint,
    focus: Focus,
}

/// What the details show.
struct Page {
    kicker: &'static str,
    title: String,
    swatch: Option<Paint>,
    state: String,
    state_icon: Icon,
    state_filled: bool,
    state_paint: Paint,
    facts: Vec<(String, String)>,
    sections: Vec<(String, Vec<Item>)>,
    help: String,
}

const BACK: &str = "Click the background to go back to the overview.";

/// The panel beside the map: an overview, or the details of the stream or
/// node brought forward.
fn details(
    triib: &Triib,
    topology: &Topology,
    shows: NetworkShows,
    streams: &[Stream],
    paints: &[Paint],
    focus: Option<Focus>,
) -> Page {
    let focused = match focus {
        Some(Focus::Node(key)) => find(topology, key),
        _ => None,
    };
    match (shows, focus, focused) {
        (NetworkShows::Clock, _, Some(node)) => clock_details(triib, topology, node),
        (NetworkShows::Clock, ..) => clock_overview(topology),
        (_, Some(Focus::Stream(..)), _) => {
            let index = streams
                .iter()
                .position(|stream| Some(stream.focus()) == focus)
                .unwrap_or_default();
            stream_details(topology, shows, &streams[index], paints[index])
        }
        (_, _, Some(node)) => node_details(topology, streams, paints, node),
        _ => stream_overview(shows, streams, paints),
    }
}

fn stream_item(stream: &Stream, paint: Paint) -> Item {
    let (state, detail_paint) = match stream.status {
        Status::Flowing | Status::Unsynced => ("Connected", Paint::Muted),
        Status::Advertised => ("Advertised only", Paint::Soft),
        Status::Failed => ("Failed", Paint::Failed),
    };
    Item {
        name: stream.name.clone(),
        detail: format!(
            "{} → {} · {state}",
            stream.talker_name, stream.listener_name
        ),
        detail_paint,
        swatch: paint,
        focus: stream.focus(),
    }
}

/// Whether the node is on a tree with gPTP running up to it.
fn synced(topology: &Topology, node: NodeId) -> bool {
    let entry = &topology.nodes[node];
    entry.apart.is_none() && (entry.parent.is_none() || entry.link.synced)
}

fn clock_item(topology: &Topology, node: NodeId) -> Item {
    let ok = synced(topology, node);
    Item {
        name: node_name(topology, node),
        detail: if ok { "Synced" } else { "Not synced" }.to_owned(),
        detail_paint: if ok { Paint::Muted } else { Paint::Failed },
        swatch: if ok { Paint::Clock } else { Paint::Unsynced },
        focus: Focus::Node(key_of(topology, node)),
    }
}

/// Why a node is not on a tree, in words.
fn apart_reason(apart: Apart) -> String {
    match apart {
        Apart::OwnGrandmaster => "Not on the gPTP tree: it is its own grandmaster".to_owned(),
        Apart::NoPath(grandmaster) => {
            format!("Its path was not reported; it follows grandmaster {grandmaster}")
        }
        Apart::Unreported => "It has not reported its gPTP state".to_owned(),
        Apart::NoNeighbor => "No bridge heard on this computer's interface".to_owned(),
    }
}

fn clock_overview(topology: &Topology) -> Page {
    let unsynced: Vec<Item> = (0..topology.nodes.len())
        .filter(|&node| !synced(topology, node))
        .map(|node| clock_item(topology, node))
        .collect();
    let grandmasters: Vec<String> = topology
        .roots
        .iter()
        .map(|&root| node_name(topology, root))
        .collect();
    Page {
        kicker: "gPTP",
        title: "Clock tree".to_owned(),
        swatch: Some(Paint::Clock),
        state: if grandmasters.is_empty() {
            "No grandmaster heard".to_owned()
        } else {
            format!("Grandmaster: {}", grandmasters.join(", "))
        },
        state_icon: Icon::Schedule,
        state_filled: false,
        state_paint: Paint::Soft,
        facts: vec![
            (
                "Synced".to_owned(),
                (topology.nodes.len() - unsynced.len()).to_string(),
            ),
            ("Not synced".to_owned(), unsynced.len().to_string()),
        ],
        sections: if unsynced.is_empty() {
            Vec::new()
        } else {
            vec![("Needs attention".to_owned(), unsynced)]
        },
        help: "The clock flows from the grandmaster through each bridge to every node on the \
               tree. A broken grey line is a link gPTP does not run on. Click a device or its \
               wire to inspect its clock path; click the background to clear."
            .to_owned(),
    }
}

fn clock_details(triib: &Triib, topology: &Topology, node: NodeId) -> Page {
    let entry = &topology.nodes[node];
    let kicker = if entry.kind == NodeKind::Bridge {
        "Bridge"
    } else {
        "Device"
    };
    let title = node_name(topology, node);
    let below_here: Vec<NodeId> = (0..topology.nodes.len())
        .filter(|&other| other != node && below(topology, other, node))
        .collect();
    if entry.apart.is_none() && entry.parent.is_none() {
        let unsynced = below_here
            .iter()
            .filter(|&&other| !synced(topology, other))
            .count();
        let bridges: Vec<Item> = below_here
            .iter()
            .filter(|&&other| !topology.nodes[other].children.is_empty())
            .map(|&other| clock_item(topology, other))
            .collect();
        let (heading, items) = if bridges.is_empty() {
            (
                "Nodes below",
                entry
                    .children
                    .iter()
                    .map(|&child| clock_item(topology, child))
                    .collect(),
            )
        } else {
            ("Bridges below", bridges)
        };
        return Page {
            kicker,
            title,
            swatch: Some(Paint::Clock),
            state: "Grandmaster".to_owned(),
            state_icon: Icon::Schedule,
            state_filled: false,
            state_paint: Paint::Clock,
            facts: vec![
                (
                    "Synced".to_owned(),
                    (below_here.len() - unsynced).to_string(),
                ),
                ("Not synced".to_owned(), unsynced.to_string()),
            ],
            sections: vec![(heading.to_owned(), items)],
            help: BACK.to_owned(),
        };
    }
    let mut facts = Vec::new();
    if entry.apart.is_none() {
        let mut path = vec![node_name(topology, node)];
        let mut at = entry.parent;
        while let Some(parent) = at {
            path.push(node_name(topology, parent));
            at = topology.nodes[parent].parent;
        }
        path.reverse();
        facts.push(("Clock path".to_owned(), path.join(" → ")));
        facts.push((
            "Hops from grandmaster".to_owned(),
            (path.len() - 1).to_string(),
        ));
    }
    if let Some(delay) = entry.link.delay {
        facts.push(("Link delay".to_owned(), format!("{delay} ns")));
    }
    if let Some(port) = entry.link.port {
        facts.push(("Bridge port".to_owned(), port.to_string()));
    }
    if let NodeKind::Entity {
        entity_id,
        interface,
    } = entry.kind
        && let Some(counters) = triib
            .models
            .get(&entity_id)
            .and_then(|model| model.counters(DescriptorType::AVB_INTERFACE, interface))
            .and_then(|counters| counters.avb_interface())
    {
        if let Some(downs) = counters.link_down {
            facts.push(("Link drops".to_owned(), downs.to_string()));
        }
        if let Some(changes) = counters.gptp_gm_changed {
            facts.push(("Grandmaster changes".to_owned(), changes.to_string()));
        }
    }
    if let Some(clock) = entry.clock {
        facts.push(("Clock identity".to_owned(), clock.to_string()));
    }
    let ok = synced(topology, node);
    let state = match entry.apart {
        Some(apart) => apart_reason(apart),
        None if ok => "Synced to the grandmaster".to_owned(),
        None if entry.kind == NodeKind::Host => {
            "Not synced: this computer does not run gPTP".to_owned()
        }
        None => "Not synced: gPTP does not run on its link".to_owned(),
    };
    Page {
        kicker,
        title,
        swatch: Some(if ok { Paint::Clock } else { Paint::Unsynced }),
        state,
        state_icon: if ok { Icon::CheckCircle } else { Icon::Error },
        state_filled: true,
        state_paint: if ok || entry.kind == NodeKind::Host {
            Paint::Soft
        } else {
            Paint::Failed
        },
        facts,
        sections: if below_here.is_empty() {
            Vec::new()
        } else {
            vec![(
                "Nodes below".to_owned(),
                below_here
                    .iter()
                    .map(|&other| clock_item(topology, other))
                    .collect(),
            )]
        },
        help: BACK.to_owned(),
    }
}

fn stream_overview(shows: NetworkShows, streams: &[Stream], paints: &[Paint]) -> Page {
    let count = |status: Status| {
        streams
            .iter()
            .filter(|stream| stream.status == status)
            .count()
    };
    let attention: Vec<Item> = streams
        .iter()
        .zip(paints)
        .filter(|(stream, _)| stream.status != Status::Flowing)
        .map(|(stream, &paint)| stream_item(stream, paint))
        .collect();
    let media_clock = shows == NetworkShows::MediaClock;
    Page {
        kicker: if media_clock { "CRF" } else { "Audio" },
        title: if media_clock {
            "Media clock streams"
        } else {
            "Audio streams"
        }
        .to_owned(),
        swatch: None,
        state: format!("{} bound", streams.len()),
        state_icon: if media_clock {
            Icon::Timer
        } else {
            Icon::GraphicEq
        },
        state_filled: false,
        state_paint: Paint::Soft,
        facts: vec![
            ("Flowing".to_owned(), count(Status::Flowing).to_string()),
            (
                "Advertised".to_owned(),
                count(Status::Advertised).to_string(),
            ),
            ("Failed".to_owned(), count(Status::Failed).to_string()),
        ],
        sections: if attention.is_empty() {
            Vec::new()
        } else {
            vec![("Needs attention".to_owned(), attention)]
        },
        help: if media_clock {
            "Media clock (CRF) streams only, drawn the same way as audio: one wire per stream, \
             coloured by talker. Click a wire to inspect its stream, or a device to see its \
             streams; click the background to clear."
        } else {
            "Each stream has its own wire, entering and leaving every bridge it crosses. Colour \
             is by talker: each talker has a hue, and its streams are shades of it. Moving dots \
             mean audio is flowing; a still red line is a failed reservation and a still grey \
             line is advertised with no listener ready; both stop where the reservation stops. \
             Devices in the middle column connect straight to the grandmaster's bridge. Click a \
             wire to inspect its stream, or a device to see its streams; click the background \
             to clear."
        }
        .to_owned(),
    }
}

fn stream_details(topology: &Topology, shows: NetworkShows, stream: &Stream, paint: Paint) -> Page {
    // The talker, then the far end of each link the stream crosses.
    let mut names = vec![stream.talker_name.clone()];
    for &(node, up) in &stream.route.hops {
        let next = if up {
            topology.nodes[node].parent
        } else {
            Some(node)
        };
        if let Some(next) = next {
            names.push(node_name(topology, next));
        }
    }
    let (state_icon, state_filled, state_paint) = match stream.status {
        Status::Flowing | Status::Unsynced => (Icon::Link, true, Paint::Soft),
        Status::Advertised => (Icon::Pending, false, Paint::Soft),
        Status::Failed => (Icon::Error, true, Paint::Failed),
    };
    Page {
        kicker: if shows == NetworkShows::MediaClock {
            "Media clock stream"
        } else {
            "Audio stream"
        },
        title: stream.name.clone(),
        swatch: Some(paint),
        state: stream.reason.clone(),
        state_icon,
        state_filled,
        state_paint,
        facts: vec![
            ("Talker".to_owned(), stream.talker_name.clone()),
            ("Listener".to_owned(), stream.listener_name.clone()),
            (
                if stream.status == Status::Flowing {
                    "Path"
                } else {
                    "Reaches"
                }
                .to_owned(),
                names.join(" → "),
            ),
        ],
        sections: Vec::new(),
        help: BACK.to_owned(),
    }
}

fn node_details(topology: &Topology, streams: &[Stream], paints: &[Paint], node: NodeId) -> Page {
    let entry = &topology.nodes[node];
    let entity = entity_of(topology, node);
    let items = |test: &dyn Fn(&Stream) -> bool| -> Vec<Item> {
        streams
            .iter()
            .zip(paints)
            .filter(|(stream, _)| test(stream))
            .map(|(stream, &paint)| stream_item(stream, paint))
            .collect()
    };
    let bridge = entity.is_none() || !entry.children.is_empty();
    let glyph = match entry.kind {
        NodeKind::Bridge => Icon::Hub,
        NodeKind::Host => Icon::Computer,
        NodeKind::Entity { .. } => Icon::GraphicEq,
    };
    if bridge && entry.kind != NodeKind::Host {
        let through = items(&|stream| via(topology, &stream.route).contains(&node));
        return Page {
            kicker: "Bridge",
            title: node_name(topology, node),
            swatch: None,
            state: format!("{} streams passing through", through.len()),
            state_icon: glyph,
            state_filled: false,
            state_paint: Paint::Soft,
            facts: vec![("Through".to_owned(), through.len().to_string())],
            sections: if through.is_empty() {
                Vec::new()
            } else {
                vec![("Passing through".to_owned(), through)]
            },
            help: "Click a stream to inspect it, or the background to go back to the overview."
                .to_owned(),
        };
    }
    let sending = items(&|stream| entity.is_some() && entity == Some(stream.talker));
    let receiving = items(&|stream| entity.is_some() && entity == Some(stream.input.0));
    let problems = streams
        .iter()
        .filter(|stream| {
            entity.is_some()
                && (entity == Some(stream.talker) || entity == Some(stream.input.0))
                && stream.status != Status::Flowing
        })
        .count();
    let counts = [(sending.len(), "out"), (receiving.len(), "in")]
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, what)| format!("{count} {what}"))
        .collect::<Vec<_>>()
        .join(", ");
    let (state, state_icon, state_paint) = match entry.apart {
        Some(apart) => (apart_reason(apart), Icon::Error, Paint::Failed),
        None if counts.is_empty() => ("None".to_owned(), glyph, Paint::Soft),
        None => (counts, glyph, Paint::Soft),
    };
    Page {
        kicker: "Device",
        title: node_name(topology, node),
        swatch: None,
        state,
        state_icon,
        state_filled: false,
        state_paint,
        facts: vec![
            ("Sending".to_owned(), sending.len().to_string()),
            ("Receiving".to_owned(), receiving.len().to_string()),
            ("Problems".to_owned(), problems.to_string()),
        ],
        sections: [("Sending", sending), ("Receiving", receiving)]
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(title, items)| (title.to_owned(), items))
            .collect(),
        help: "Click a stream to inspect it, or the background to go back to the overview."
            .to_owned(),
    }
}

impl Page {
    fn view<'a>(self, width: Length) -> Element<'a, Message> {
        let swatch = |paint: Option<Paint>| {
            container(space())
                .width(5)
                .height(Fill)
                .style(move |theme: &Theme| container::Style {
                    background: paint
                        .map(|paint| map::paint_color(&Scheme::of(theme), paint).into()),
                    border: iced::border::rounded(3),
                    ..container::Style::default()
                })
        };
        let state_paint = self.state_paint;
        let state_icon = if self.state_filled {
            icon::filled(self.state_icon, 16)
        } else {
            icon::icon(self.state_icon, 16)
        };
        let tinted = move |theme: &Theme| iced::widget::text::Style {
            color: Some(map::text_color(&Scheme::of(theme), state_paint)),
        };
        let mut body = column![
            styled(self.kicker.to_uppercase(), Type::LabelSmall).style(style::on_surface_variant),
            row![
                swatch(self.swatch),
                column![
                    styled(self.title, Type::TitleMedium),
                    row![
                        container(state_icon.style(tinted)).padding(iced::Padding {
                            top: 2.0,
                            ..iced::Padding::ZERO
                        }),
                        styled(self.state, Type::BodyMedium).style(tinted),
                    ]
                    .spacing(4),
                ]
                .spacing(2)
                .width(Fill),
            ]
            .spacing(10)
            .height(Length::Shrink),
        ]
        .spacing(14);
        if !self.facts.is_empty() {
            body = body.push(
                column(self.facts.into_iter().map(|(label, value)| {
                    row![
                        styled(label, Type::BodyMedium)
                            .style(style::on_surface_variant)
                            .width(Length::Fixed(112.0)),
                        styled(value, Type::BodyMedium).width(Fill),
                    ]
                    .spacing(12)
                    .into()
                }))
                .spacing(8),
            );
        }
        for (title, items) in self.sections {
            let items = items.into_iter().map(|item| {
                let detail_paint = item.detail_paint;
                let content = row![
                    swatch(Some(item.swatch)),
                    column![
                        styled(item.name, Type::LabelLarge),
                        styled(item.detail, Type::BodySmall).style(move |theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(map::text_color(&Scheme::of(theme), detail_paint)),
                            }
                        }),
                    ]
                    .spacing(2)
                    .width(Fill),
                ]
                .spacing(10)
                .height(Length::Shrink);
                plain_button(content)
                    .padding([8, 10])
                    .width(Fill)
                    .style(|theme: &Theme, status| {
                        let scheme = Scheme::of(theme);
                        plain_button::Style {
                            background: Some(
                                match status {
                                    plain_button::Status::Hovered
                                    | plain_button::Status::Pressed => {
                                        scheme.surface_container_high
                                    }
                                    _ => scheme.surface_container,
                                }
                                .into(),
                            ),
                            text_color: scheme.on_surface,
                            border: iced::Border {
                                color: scheme.outline_variant,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            ..plain_button::Style::default()
                        }
                    })
                    .on_press(Message::NetworkFocused(Some(item.focus)))
                    .into()
            });
            body = body.push(
                column![styled(title, Type::LabelLarge).style(style::on_surface_variant)]
                    .push(column(items).spacing(6))
                    .spacing(6),
            );
        }
        body = body.push(styled(self.help, Type::BodySmall).style(style::on_surface_variant));
        container(
            scrollable(body.padding(16))
                .direction(scrollable::Direction::Vertical(component::thin_scrollbar()))
                .style(style::scrollbar)
                .height(Fill),
        )
        .width(width)
        .height(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Scheme::of(theme).surface_container_low.into()),
            border: iced::border::rounded(shape::LARGE),
            ..container::Style::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;

    use super::*;

    /// A switch with three entities under it, and four streams between
    /// them in each state.
    fn bench() -> (Topology, Vec<Stream>) {
        let switch = ClockIdentity(0x0001_f2ff_feff_3b14);
        let clocks = [1u64, 2, 3].map(|place| ClockIdentity(0x0011_22ff_fe00_0000 | place));
        let names = ["Stage box", "Console", "Amp"];
        let paths: Vec<[ClockIdentity; 2]> = clocks.iter().map(|&clock| [switch, clock]).collect();
        let reports: Vec<EntityReport<'_>> = (0..3)
            .map(|place| EntityReport {
                entity_id: EntityId(place as u64 + 1),
                name: names[place],
                interfaces: vec![InterfaceReport {
                    index: 0,
                    clock: clocks[place],
                    path: Some(&paths[place]),
                    info: None,
                }],
            })
            .collect();
        let topology = Topology::build(&reports, None);
        let stream = |talker: u64, listener: u64, input: u16, name: &str, status: Status| {
            let route = match status {
                Status::Flowing => topology.route(EntityId(talker), EntityId(listener)),
                _ => {
                    let node = topology.placed(EntityId(talker)).unwrap();
                    Route {
                        hops: vec![(node, true)],
                        stops: topology.nodes[node].parent,
                    }
                }
            };
            Stream {
                input: (EntityId(listener), input),
                talker: EntityId(talker),
                name: name.to_owned(),
                talker_name: names[talker as usize - 1].to_owned(),
                listener_name: names[listener as usize - 1].to_owned(),
                media_clock: false,
                status,
                reason: match status {
                    Status::Failed => "Reservation failed at Bridge: insufficient bandwidth",
                    Status::Advertised => "Advertised, no listener ready",
                    _ => "Connected",
                }
                .to_owned(),
                route,
            }
        };
        let streams = vec![
            stream(1, 2, 0, "Stage 1–8 → Console in 1–8", Status::Flowing),
            stream(1, 2, 1, "Stage 9–16 → Console in 9–16", Status::Flowing),
            stream(2, 3, 0, "Main L/R → Amp in", Status::Failed),
            stream(2, 3, 1, "Fill → Amp in 2", Status::Advertised),
        ];
        (topology, streams)
    }

    /// Writes pictures of the header and the details panel to the PNG files
    /// the variable `NETWORK_PANEL_PICTURE` names, with what each shows added.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn pictures() {
        let Ok(path) = std::env::var("NETWORK_PANEL_PICTURE") else {
            return;
        };
        let theme = scramble_ui::scheme::theme("triib".to_owned(), crate::app::TRIIB_SEED, true);
        let (topology, streams) = bench();
        let paints = paints(
            &streams
                .iter()
                .map(|stream| (stream.talker, stream.status))
                .collect::<Vec<_>>(),
        );
        let console = find(&topology, NodeKey::Entity(EntityId(2))).unwrap();
        // How to build each page, by the name its picture takes.
        type Build<'a> = Box<dyn Fn() -> Page + 'a>;
        let pages: [(&str, Build<'_>); 4] = [
            (
                "overview",
                Box::new(|| stream_overview(NetworkShows::Audio, &streams, &paints)),
            ),
            (
                "stream",
                Box::new(|| stream_details(&topology, NetworkShows::Audio, &streams[2], paints[2])),
            ),
            (
                "device",
                Box::new(|| node_details(&topology, &streams, &paints, console)),
            ),
            ("clock", Box::new(|| clock_overview(&topology))),
        ];
        for (suffix, page) in pages {
            let file = format!("{path}-{suffix}.png");
            for renderer in ["tiny-skia", "wgpu"] {
                let _ = std::fs::remove_file(format!("{path}-{suffix}-{renderer}.png"));
            }
            let settings = iced::Settings {
                fonts: scramble_ui::font::files().collect(),
                default_font: scramble_ui::font::TEXT,
                antialiasing: true,
                ..iced::Settings::default()
            };
            let focus = Some(streams[2].focus());
            let content: Element<'_, Message> = iced::widget::column![
                header(
                    &topology,
                    NetworkShows::Audio,
                    &streams,
                    focus,
                    1280.0,
                    false
                ),
                page().view(Length::Fixed(PANEL_WIDTH)),
            ]
            .spacing(16)
            .padding(16)
            .into();
            let mut simulator =
                iced_test::Simulator::with_size(settings, Size::new(1280.0, 640.0), content);
            let snapshot = simulator.snapshot(&theme).expect("draws");
            assert!(snapshot.matches_image(&file).expect("writes"));
        }
    }
}
