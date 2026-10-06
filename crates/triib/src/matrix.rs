//! The connection matrix: talkers' stream outputs along the top,
//! listeners' stream inputs down the right, each talker and listener
//! collapsible to one column or row, and in each cell whether the input
//! is bound to the output and how that is going.

mod grid;

use atdecc::model::EntityModel;
use atdecc::stream_format::{Fit, fit};
use atdecc::{DescriptorType, DiscoveredEntity, EntityId, StreamFormat};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row};
use iced::{Center, Color, Element, Fill, Theme};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::component;
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::{self, Icon};
use scramble_ui::{Scheme, faded, shape, style};

use crate::app::{self, Message, Triib};
use crate::network::Action;
use crate::settings::Streams;

const STATUS_HEIGHT: f32 = 48.0;

/// Which end of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    Talker,
    Listener,
}

/// A column or row: a talker or listener, or one of its streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Line {
    Entity(EntityId),
    Stream(EntityId, u16),
}

impl Line {
    pub fn entity(self) -> EntityId {
        match self {
            Line::Entity(entity_id) | Line::Stream(entity_id, _) => entity_id,
        }
    }
}

/// What the pointer is over: a cell, or a column's or row's heading.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Hover {
    pub column: Option<Line>,
    pub row: Option<Line>,
}

/// A column's or row's heading.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Heading {
    pub line: Line,
    pub name: String,
    /// For an entity, the end of its MAC address, telling apart entities
    /// of the same name; for a stream, its format in short.
    pub detail: String,
    /// For an entity, whether it is collapsed to this line.
    pub collapsed: Option<bool>,
}

/// What a cell for one stream output and one stream input shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Connected,
    /// Bound, the listener has not found the talker's stream yet.
    Waiting,
    /// Bound, but the talker's reservation failed or the formats differ.
    Trouble,
    /// A bind or unbind is on its way.
    Working,
    /// Not bound to this output; the formats match.
    Open,
    /// The input takes the talker's format but is set to another.
    Change,
    /// The input does not take the talker's format.
    Incompatible,
}

impl State {
    /// The icon, and whether it is filled.
    pub fn icon(self) -> (Icon, bool) {
        match self {
            State::Connected => (Icon::Link, true),
            State::Waiting => (Icon::Pending, false),
            State::Trouble => (Icon::Error, true),
            State::Working => (Icon::ProgressActivity, false),
            State::Open => (Icon::RadioButtonUnchecked, false),
            State::Change => (Icon::ChangeCircle, false),
            State::Incompatible => (Icon::Remove, false),
        }
    }

    pub fn color(self, scheme: &Scheme) -> Color {
        match self {
            State::Connected => scheme.primary,
            State::Waiting => scheme.tertiary,
            State::Trouble => scheme.error,
            State::Working => scheme.on_surface_variant,
            State::Open => faded(scheme.on_surface_variant, 0.45),
            State::Change => faded(scheme.on_surface_variant, 0.6),
            State::Incompatible => faded(scheme.on_surface_variant, 0.22),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cell {
    /// Under an expanded talker or beside an expanded listener.
    Blank,
    /// How many bindings join a collapsed talker's or listener's streams
    /// to the other line.
    Count(usize),
    Stream {
        state: State,
        /// What clicking does.
        action: Option<Action>,
    },
}

/// Everything the matrix widget draws.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Grid {
    pub columns: Vec<Heading>,
    pub rows: Vec<Heading>,
    /// Row by row.
    pub cells: Vec<Cell>,
    /// The entity the inspector shows.
    pub selected: Option<EntityId>,
}

impl Grid {
    pub fn cell(&self, row: usize, column: usize) -> Cell {
        self.cells
            .get(row * self.columns.len() + column)
            .copied()
            .unwrap_or(Cell::Blank)
    }

    fn row_of(&self, line: Line) -> Option<usize> {
        self.rows.iter().position(|heading| heading.line == line)
    }

    fn column_of(&self, line: Line) -> Option<usize> {
        self.columns.iter().position(|heading| heading.line == line)
    }
}

/// A stream of an entity, as the matrix lists it.
struct Stream<'a> {
    entity_id: EntityId,
    index: u16,
    name: String,
    format: StreamFormat,
    /// For an input, the formats it takes.
    supported: Vec<StreamFormat>,
    model: &'a EntityModel,
}

impl Stream<'_> {
    fn key(&self) -> (EntityId, u16) {
        (self.entity_id, self.index)
    }

    /// For an input, the talker's stream output it is bound to.
    fn bound_to(&self) -> Option<(EntityId, u16)> {
        self.model
            .binding(self.index)
            .and_then(|binding| binding.talker_stream())
    }
}

/// A talker or listener with the streams the matrix shows.
struct Group<'a> {
    entity_id: EntityId,
    name: String,
    /// The end of its MAC address.
    tag: String,
    streams: Vec<Stream<'a>>,
}

impl Group<'_> {
    fn label(&self) -> String {
        format!("{} {}", self.name, self.tag)
    }
}

pub fn view(triib: &Triib) -> Element<'_, Message> {
    let mut talkers = groups(triib, Side::Talker);
    let mut listeners = groups(triib, Side::Listener);
    let count = |groups: &[Group<'_>]| {
        groups
            .iter()
            .map(|group| group.streams.len())
            .sum::<usize>()
    };
    // How many streams leaving out what cannot connect hides, when it is on.
    let hidden = triib.settings.matrix_connectable_only.then(|| {
        let before = count(&talkers) + count(&listeners);
        connectable_only(&mut talkers, &mut listeners);
        before - count(&talkers) - count(&listeners)
    });
    let body: Element<'_, Message> = if talkers.is_empty() || listeners.is_empty() {
        let filtered = !triib.search.trim().is_empty()
            || triib.settings.matrix_streams != Streams::All
            || triib.settings.matrix_connectable_only;
        if filtered {
            component::empty_state(
                Icon::FilterAlt,
                "No streams to show",
                "Change the search or the filters to see more streams.",
            )
        } else {
            component::empty_state(
                Icon::GridOn,
                "No streams to connect",
                "Talker streams and listener streams meet here once entities with them have \
                 been read.",
            )
        }
    } else {
        let grid = grid(triib, &talkers, &listeners);
        let status = status(triib, &grid, &talkers, &listeners);
        column![
            container(grid::matrix(grid, triib.hover))
                .center_x(Fill)
                .height(Fill),
            status,
            legend()
        ]
        .spacing(12)
        .into()
    };
    column![filters(triib, hidden), body]
        .spacing(16)
        .padding([16, 24])
        .width(Fill)
        .height(Fill)
        .into()
}

/// The streams shown, and whether only connectable ones are, with how
/// many that hides.
fn filters(triib: &Triib, hidden: Option<usize>) -> Element<'_, Message> {
    let streams = triib.settings.matrix_streams;
    let choice = |label: &'static str, value: Streams| {
        button::button(Kind::Filled, label)
            .size(Size::ExtraSmall)
            .selected(streams == value)
            .on_press(Message::MatrixStreams(value))
    };
    let mut filters = row![
        component::connected(vec![
            choice("All streams", Streams::All),
            choice("Audio", Streams::Audio),
            choice("Media clock", Streams::Clock),
        ]),
        button::with_icon(Kind::Filled, Icon::FilterAlt, "Hide what cannot connect")
            .size(Size::ExtraSmall)
            .selected(hidden.is_some())
            .on_press(Message::MatrixConnectableOnlyToggled),
    ];
    if let Some(hidden) = hidden {
        filters = filters.push(
            styled(
                match hidden {
                    0 => "Every stream shown can connect".to_owned(),
                    1 => "1 stream hidden".to_owned(),
                    hidden => format!("{hidden} streams hidden"),
                },
                Type::BodyMedium,
            )
            .style(style::on_surface_variant),
        );
    }
    filters
        .spacing(12)
        .align_y(Center)
        .wrap()
        .vertical_spacing(8)
        .into()
}

/// The talkers or listeners with streams the search and the stream filter
/// let through, by name.
fn groups(triib: &Triib, side: Side) -> Vec<Group<'_>> {
    let input = side == Side::Listener;
    let search = triib.search_text();
    let streams = triib.settings.matrix_streams;
    triib
        .shown_entities()
        .into_iter()
        .filter_map(|entity| {
            let entity_id = entity.entity_id();
            let model = triib.models.get(&entity_id)?;
            // An entity found by its own name shows all its streams.
            let whole = search.is_empty() || triib.entity_matches(entity, &search);
            let shown: Vec<Stream<'_>> = model
                .streams(input)
                .filter(|stream| whole || app::stream_matches(model, stream, &search))
                .filter(|stream| match streams {
                    Streams::All => true,
                    Streams::Audio => !stream.current_format.is_clock(),
                    Streams::Clock => stream.current_format.is_clock(),
                })
                .map(|stream| Stream {
                    entity_id,
                    index: stream.index,
                    name: model
                        .name_of(stream.descriptor_type, stream.index)
                        .map_or_else(|| format!("Stream {}", stream.index), str::to_owned),
                    format: stream.current_format,
                    supported: if input {
                        stream.formats().collect()
                    } else {
                        Vec::new()
                    },
                    model,
                })
                .collect();
            (!shown.is_empty()).then(|| Group {
                entity_id,
                name: triib.entity_name(entity),
                tag: tag(entity),
                streams: shown,
            })
        })
        .collect()
}

/// The last two octets of the entity's MAC address.
fn tag(entity: &DiscoveredEntity) -> String {
    let [.., fifth, sixth] = entity.mac.0;
    format!("{fifth:02x}:{sixth:02x}")
}

/// Leaves out the streams that nothing on the other side could connect
/// to, keeping bound ones.
fn connectable_only<'a>(talkers: &mut Vec<Group<'a>>, listeners: &mut Vec<Group<'a>>) {
    let reaches = |output: &Stream<'_>, input: &Stream<'_>| {
        input.bound_to() == Some(output.key())
            || fit(output.format, input.format, input.supported.iter().copied())
                != Fit::Incompatible
    };
    let outputs: Vec<(EntityId, u16)> = talkers
        .iter()
        .flat_map(|group| &group.streams)
        .filter(|output| {
            listeners
                .iter()
                .flat_map(|group| &group.streams)
                .any(|input| reaches(output, input))
        })
        .map(Stream::key)
        .collect();
    let inputs: Vec<(EntityId, u16)> = listeners
        .iter()
        .flat_map(|group| &group.streams)
        .filter(|input| {
            talkers
                .iter()
                .flat_map(|group| &group.streams)
                .any(|output| reaches(output, input))
        })
        .map(Stream::key)
        .collect();
    for (groups, kept) in [(talkers, outputs), (listeners, inputs)] {
        for group in groups.iter_mut() {
            group.streams.retain(|stream| kept.contains(&stream.key()));
        }
        groups.retain(|group| !group.streams.is_empty());
    }
}

/// A stream format in a few characters, such as "48k 8ch" or "CRF 48k".
fn short(format: StreamFormat) -> String {
    let rate = format.sample_rate().map(|hertz| {
        if hertz.is_multiple_of(1000) {
            format!("{}k", hertz / 1000)
        } else {
            format!("{:.1}k", hertz as f32 / 1000.0)
        }
    });
    if format.is_clock() {
        return rate.map_or_else(|| "CRF".to_owned(), |rate| format!("CRF {rate}"));
    }
    match (rate, format.channels()) {
        (Some(rate), Some(channels)) => format!("{rate} {channels}ch"),
        (Some(rate), None) => rate,
        (None, Some(channels)) => format!("{channels}ch"),
        (None, None) => String::new(),
    }
}

/// The streams one column or row stands for: one stream, or every stream
/// of a collapsed talker or listener. `None` for an expanded one.
type Members<'s, 'a> = Option<Vec<&'s Stream<'a>>>;

fn lines<'s, 'a>(
    triib: &Triib,
    groups: &'s [Group<'a>],
    side: Side,
) -> Vec<(Heading, Members<'s, 'a>)> {
    let mut lines = Vec::new();
    for group in groups {
        let collapsed = triib.collapsed.contains(&(side, group.entity_id));
        lines.push((
            Heading {
                line: Line::Entity(group.entity_id),
                name: group.name.clone(),
                detail: group.tag.clone(),
                collapsed: Some(collapsed),
            },
            collapsed.then(|| group.streams.iter().collect()),
        ));
        if collapsed {
            continue;
        }
        for stream in &group.streams {
            lines.push((
                Heading {
                    line: Line::Stream(stream.entity_id, stream.index),
                    name: stream.name.clone(),
                    detail: short(stream.format),
                    collapsed: None,
                },
                Some(vec![stream]),
            ));
        }
    }
    lines
}

fn grid(triib: &Triib, talkers: &[Group<'_>], listeners: &[Group<'_>]) -> Grid {
    let (columns, outputs): (Vec<_>, Vec<_>) =
        lines(triib, talkers, Side::Talker).into_iter().unzip();
    let (rows, inputs): (Vec<_>, Vec<_>) =
        lines(triib, listeners, Side::Listener).into_iter().unzip();
    let mut cells = Vec::with_capacity(rows.len() * columns.len());
    for inputs in &inputs {
        for outputs in &outputs {
            cells.push(match (inputs, outputs) {
                (Some(inputs), Some(outputs)) => match (inputs.as_slice(), outputs.as_slice()) {
                    ([input], [output]) => {
                        let (state, action) = assess(triib, output, input);
                        Cell::Stream { state, action }
                    }
                    _ => Cell::Count(
                        inputs
                            .iter()
                            .filter(|input| {
                                input.bound_to().is_some_and(|bound| {
                                    outputs.iter().any(|output| output.key() == bound)
                                })
                            })
                            .count(),
                    ),
                },
                _ => Cell::Blank,
            });
        }
    }
    Grid {
        columns,
        rows,
        cells,
        selected: triib.selected,
    }
}

/// How `output` and `input` stand, and what clicking their cell does.
fn assess(triib: &Triib, output: &Stream<'_>, input: &Stream<'_>) -> (State, Option<Action>) {
    let talker = output.key();
    let listener = input.key();
    let bound_here = input.bound_to() == Some(talker);
    let fits = fit(output.format, input.format, input.supported.iter().copied());
    let state = if bound_here {
        let info = input
            .model
            .stream_info(DescriptorType::STREAM_INPUT, input.index);
        match (fits, info) {
            (Fit::Matches, Some(info)) if info.talker_failed() => State::Trouble,
            (Fit::Matches, Some(info)) if !info.settled() => State::Waiting,
            (Fit::Matches, _) => State::Connected,
            _ => State::Trouble,
        }
    } else {
        match fits {
            Fit::Matches => State::Open,
            Fit::InputMustChange => State::Change,
            Fit::Incompatible => State::Incompatible,
        }
    };
    let pending = triib.pending.iter().find_map(|action| match *action {
        Action::Connect {
            talker: target,
            listener: pending,
        } if pending == listener => Some(target == talker),
        Action::Disconnect { listener: pending } if pending == listener => Some(bound_here),
        _ => None,
    });
    match pending {
        Some(true) => (State::Working, None),
        // Another change to this input is on its way.
        Some(false) => (state, None),
        None if bound_here => (state, Some(Action::Disconnect { listener })),
        None if fits == Fit::Incompatible => (state, None),
        None => (state, Some(Action::Connect { talker, listener })),
    }
}

/// A column's or row's line found among the talkers or listeners.
enum Found<'s, 'a> {
    Group(&'s Group<'a>),
    Stream(&'s Group<'a>, &'s Stream<'a>),
}

fn find<'s, 'a>(groups: &'s [Group<'a>], line: Line) -> Option<Found<'s, 'a>> {
    match line {
        Line::Entity(entity_id) => groups
            .iter()
            .find(|group| group.entity_id == entity_id)
            .map(Found::Group),
        Line::Stream(entity_id, index) => groups
            .iter()
            .find(|group| group.entity_id == entity_id)
            .and_then(|group| {
                group
                    .streams
                    .iter()
                    .find(|stream| stream.index == index)
                    .map(|stream| Found::Stream(group, stream))
            }),
    }
}

impl Found<'_, '_> {
    fn label(&self) -> String {
        match self {
            Found::Group(group) => group.label(),
            Found::Stream(group, stream) => format!("{}, {}", group.label(), stream.name),
        }
    }
}

/// What the status line says about the pointer's place.
struct Said {
    glyph: (Icon, bool),
    color: Color,
    title: String,
    detail: String,
}

/// What the pointer is over, in words: a cell's talker and listener and
/// what clicking does, or a heading's stream.
fn status<'a>(
    triib: &Triib,
    grid: &Grid,
    talkers: &[Group<'_>],
    listeners: &[Group<'_>],
) -> Element<'a, Message> {
    let said = say(triib, grid, talkers, listeners);
    let color = said.color;
    let glyph = if said.glyph.1 {
        icon::filled(said.glyph.0, 20)
    } else {
        icon::icon(said.glyph.0, 20)
    };
    container(
        row![
            glyph.color(color),
            // On a narrow window what does not fit is cut off.
            styled(said.title, Type::BodyMedium).wrapping(Wrapping::None),
            styled(said.detail, Type::BodyMedium)
                .style(style::on_surface_variant)
                .wrapping(Wrapping::None),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .padding([0, 16])
    .height(STATUS_HEIGHT)
    .width(Fill)
    .clip(true)
    .align_y(Center)
    .style(|theme: &Theme| container::Style {
        background: Some(Scheme::of(theme).surface_container.into()),
        border: iced::border::rounded(shape::MEDIUM),
        ..container::Style::default()
    })
    .into()
}

fn say(triib: &Triib, grid: &Grid, talkers: &[Group<'_>], listeners: &[Group<'_>]) -> Said {
    let scheme = Scheme::of(&triib.theme());
    let info = |title: String, detail: String| Said {
        glyph: (Icon::Info, false),
        color: scheme.on_surface_variant,
        title,
        detail,
    };
    let hover = triib.hover;
    let column = hover.column.and_then(|line| find(talkers, line));
    let row = hover.row.and_then(|line| find(listeners, line));
    let cell = hover
        .row
        .and_then(|line| grid.row_of(line))
        .zip(hover.column.and_then(|line| grid.column_of(line)))
        .map(|(row, column)| grid.cell(row, column));
    match (column, row, cell) {
        (Some(Found::Stream(talker, output)), Some(Found::Stream(listener, input)), _) => {
            let (state, action) = assess(triib, output, input);
            let route = format!(
                "{}, {}  →  {}, {}",
                talker.label(),
                output.name,
                listener.label(),
                input.name
            );
            let detail = match state {
                State::Working => "Working on it.".to_owned(),
                _ if action.is_none() && state != State::Incompatible => {
                    "Waiting for the last change to this input.".to_owned()
                }
                State::Connected => "Connected and receiving. Click to disconnect.".to_owned(),
                State::Waiting => {
                    "Bound, waiting for the talker's stream. Click to disconnect.".to_owned()
                }
                State::Trouble => match input
                    .model
                    .stream_info(DescriptorType::STREAM_INPUT, input.index)
                {
                    Some(info) if info.talker_failed() => format!(
                        "Bound, but the talker's reservation failed: {}. Click to disconnect.",
                        crate::describe::reservation_failure(info)
                    ),
                    _ => format!(
                        "Bound, but the formats differ: the talker sends {}, the input is set \
                         to {}. Click to disconnect.",
                        output.format, input.format
                    ),
                },
                State::Open => format!("Formats match ({}). Click to connect.", output.format),
                State::Change => format!(
                    "The input takes {} but is set to {}, so it may not play until its format \
                     changes. Click to connect anyway.",
                    output.format, input.format
                ),
                State::Incompatible => format!(
                    "The input does not take {}. It is set to {}.",
                    output.format, input.format
                ),
            };
            Said {
                glyph: state.icon(),
                color: state.color(&scheme),
                title: route,
                detail,
            }
        }
        (Some(column), Some(row), Some(Cell::Count(count))) => info(
            format!("{}  →  {}", column.label(), row.label()),
            if count == 0 {
                "Not connected. Expand to connect streams one by one.".to_owned()
            } else {
                format!("{count} connected. Expand to see each one.")
            },
        ),
        (column, row, _) => {
            // A heading, or a blank cell, said as its stream's heading.
            let heading = match (row, column) {
                (Some(row @ Found::Stream(..)), _) | (Some(row), None) => {
                    Some((row, Side::Listener))
                }
                (_, Some(column)) => Some((column, Side::Talker)),
                (None, None) => None,
            };
            match heading {
                Some((Found::Group(group), side)) => {
                    let count = group.streams.len();
                    let kind = match (side, count) {
                        (Side::Talker, 1) => "stream output",
                        (Side::Talker, _) => "stream outputs",
                        (Side::Listener, 1) => "stream input",
                        (Side::Listener, _) => "stream inputs",
                    };
                    let collapsed = triib.collapsed.contains(&(side, group.entity_id));
                    info(
                        group.label(),
                        format!(
                            "{count} {kind}. Click the arrow to {}, the name to inspect it.",
                            if collapsed { "expand" } else { "collapse" }
                        ),
                    )
                }
                Some((Found::Stream(group, stream), side)) => {
                    let mut detail = format!("{}.", stream.format);
                    if side == Side::Listener
                        && let Some(state) = stream
                            .model
                            .streams(true)
                            .find(|descriptor| descriptor.index == stream.index)
                            .and_then(|descriptor| {
                                crate::describe::stream_state(triib, stream.model, &descriptor)
                            })
                    {
                        detail = format!("{detail} {state}.");
                    }
                    info(
                        format!("{}, {}", group.label(), stream.name),
                        format!("{detail} Click to inspect {}.", group.name),
                    )
                }
                None => info(
                    "Point at a cell".to_owned(),
                    "to see its talker and listener and whether their formats meet.".to_owned(),
                ),
            }
        }
    }
}

/// What each cell icon means.
fn legend<'a>() -> Element<'a, Message> {
    let entry = |state: State, label: &'static str| -> Element<'a, Message> {
        let (glyph, filled) = state.icon();
        let glyph = if filled {
            icon::filled(glyph, 20)
        } else {
            icon::icon(glyph, 20)
        };
        row![
            glyph.style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(state.color(&Scheme::of(theme))),
            }),
            styled(label, Type::BodySmall).style(style::on_surface_variant),
        ]
        .spacing(6)
        .align_y(Center)
        .into()
    };
    row![
        entry(State::Connected, "Connected"),
        entry(State::Waiting, "Bound, waiting for the stream"),
        entry(State::Trouble, "Bound, something is wrong"),
        entry(State::Open, "Can connect"),
        entry(State::Change, "Input format must change first"),
        entry(State::Incompatible, "Formats cannot meet"),
    ]
    .spacing(20)
    .wrap()
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_shorten() {
        // AAF, 32-bit integer, 48 kHz, 8 channels of 32 bits, 6 per frame.
        let aaf = StreamFormat(0x0205_0220_0200_6000);
        assert_eq!(short(aaf), "48k 8ch");
        // CRF audio sample, 48 kHz.
        let crf = StreamFormat(0x0410_6001_0000_bb80);
        assert_eq!(short(crf), "CRF 48k");
    }
}
