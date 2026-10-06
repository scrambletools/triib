//! The window: toolbar, the active view, the inspector on the selected
//! entity and the status bar.

use std::fmt;

use atdecc::aem::AvbInfoFlags;
use atdecc::descriptor::DescriptorType;
use atdecc::descriptor::{
    AudioUnitDescriptor, AvbInterfaceDescriptor, ClockDomainDescriptor, ClockSourceDescriptor,
    ConfigurationDescriptor, EntityDescriptor, LocaleDescriptor, SamplingRate, StreamDescriptor,
    StreamPortDescriptor, StringsDescriptor,
};
use atdecc::model::{EntityModel, EnumerationState};
use atdecc::stream_format::StreamFormat;
use atdecc::{DiscoveredEntity, EntityCapabilities, EntityId};
use avb_net::Interface;
use iced::widget::{container, pick_list, rule, scrollable, space, table, text_input};
use iced::{Center, Element, Fill, Length, Theme};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::component::{self, TOOLBAR_HEIGHT};
use scramble_ui::font::{TEXT, Type, styled};
use scramble_ui::icon::{self, Icon};
use scramble_ui::{Scheme, style};

use crate::app::{Message, NetworkState, Triib};
use crate::describe;
use crate::mapping_view;
use crate::network::{Action, Failure, NameTarget};
use crate::settings::View;
use crate::text::measure;

const STATUS_BAR_HEIGHT: f32 = 28.0;
const SEARCH_WIDTH: f32 = 280.0;
const INTERFACE_PICKER_WIDTH: f32 = 320.0;
/// Between the toolbar's ends and its first and last tools.
const TOOLBAR_PADDING: f32 = 12.0;
/// The narrowest window that has room for the inspector beside the view.
const INSPECTOR_BESIDE: f32 = 720.0;
/// The narrowest the interface picker gets, and below which its choices
/// leave out all but each interface's name and link.
const PICKER_NARROWEST: f32 = 160.0;
/// Below this width the status bar leaves out the interface.
const STATUS_FULL: f32 = 640.0;
/// The entity table's columns after the name.
const FIELDS: [&str; 9] = [
    "Group",
    "Product",
    "Firmware",
    "Milan",
    "Roles",
    "Media clock",
    "BTC",
    "State",
    "Entity ID",
];
/// Around each cell of the entity table, around the table, and spare room
/// for what measuring its text leaves out.
const TABLE_PADDING: f32 = 12.0;
const TABLE_MARGIN: f32 = 16.0;
const TABLE_SLACK: f32 = 24.0;
const MEDIUM: iced::Font = iced::Font {
    weight: iced::font::Weight::Medium,
    ..scramble_ui::font::TEXT
};

pub fn window(triib: &Triib) -> Element<'_, Message> {
    let window: Element<'_, Message> = iced::widget::column![
        toolbar(triib),
        container(iced::widget::responsive(move |size| {
            panels(triib, size.width)
        }))
        .height(Fill)
        .style(style::surface),
        status_bar(triib),
    ]
    .into();
    if triib.settings_open {
        crate::settings_view::dialog(triib, window)
    } else {
        window
    }
}

/// The active view with the inspector beside it, or on a narrow window the
/// inspector in its place until it is closed.
fn panels(triib: &Triib, width: f32) -> Element<'_, Message> {
    let view = container(content(triib)).width(Fill).height(Fill);
    if !triib.settings.inspector {
        return view.into();
    }
    if width < INSPECTOR_BESIDE {
        return inspector(triib, Length::Fill);
    }
    // The panels keep their sides in every language.
    iced::widget::row![view, inspector(triib, component::SIDE_SHEET_WIDTH.into())]
        .height(Fill)
        .into()
}

/// An interface as the picker lists it.
#[derive(Debug, Clone, PartialEq)]
struct Choice {
    name: String,
    label: String,
}

impl Choice {
    /// The interface with its link, speed and kind, or only its link when
    /// `brief`.
    fn new(interface: &Interface, brief: bool) -> Self {
        let mut label = interface.name.clone();
        if !interface.up {
            label.push_str(", link down");
        } else if brief {
            return Self {
                name: interface.name.clone(),
                label,
            };
        } else if let Some(speed) = interface.speed {
            label.push_str(&format!(", {}", speed_text(speed)));
        }
        if interface.wireless {
            label.push_str(", wireless");
        } else if interface.hardware_clock.is_some() {
            label.push_str(", hardware clock");
        }
        if !interface.physical {
            label.push_str(", virtual");
        }
        Self {
            name: interface.name.clone(),
            label,
        }
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

/// The toolbar, with groups that do not fit the window's width in a
/// "More" menu at its end.
fn toolbar(triib: &Triib) -> Element<'_, Message> {
    component::toolbar(iced::widget::responsive(move |size| {
        toolbar_at(triib, size.width)
    }))
}

fn toolbar_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    use component::{DIVIDER_WIDTH, TOOL_WIDTH, TOOLBAR_GAP};
    let tools = |count: f32| count * TOOL_WIDTH + (count - 1.0) * 4.0;
    // The space between the bar's two ends takes a gap of its own.
    let room = width - 2.0 * TOOLBAR_PADDING - TOOLBAR_GAP;
    // On a narrow window the picker leaves room for the inspector and
    // "More" buttons.
    let picker_width = INTERFACE_PICKER_WIDTH
        .min(room - 2.0 * (TOOL_WIDTH + TOOLBAR_GAP) - TOOLBAR_GAP)
        .max(PICKER_NARROWEST);
    let brief = picker_width < INTERFACE_PICKER_WIDTH;
    let choices: Vec<Choice> = triib
        .listed_interfaces()
        .map(|interface| Choice::new(interface, brief))
        .collect();
    let selected = triib
        .interface()
        .map(|interface| Choice::new(interface, brief));
    let picker = pick_list(choices, selected, |choice: Choice| {
        Message::InterfacePicked(choice.name)
    })
    .placeholder("Choose an interface")
    .width(picker_width)
    .padding([8, 12])
    .style(style::outlined_select)
    .menu_style(style::select_menu);

    let view = triib.settings.view;
    let view_tool = |glyph, label, target| {
        component::toggle_tool(glyph, label, view == target, Message::ViewPicked(target))
    };
    let inspector_glyph = if triib.settings.inspector {
        Icon::RightPanelClose
    } else {
        Icon::RightPanelOpen
    };

    // Each slot: its content, its width, when it moves into "More" (lowest
    // first), and whether a divider goes before it in the bar.
    let mut slots: Vec<(Element<'_, Message>, f32, Option<u8>, bool)> = vec![
        (
            component::tip(picker, "Network interface"),
            picker_width,
            None,
            false,
        ),
        (
            component::toggle_tool(
                Icon::FilterList,
                if triib.settings.hide_virtual_interfaces {
                    "Show virtual interfaces"
                } else {
                    "Hide virtual interfaces"
                },
                triib.settings.hide_virtual_interfaces,
                Message::VirtualInterfacesToggled,
            ),
            TOOL_WIDTH,
            Some(0),
            false,
        ),
        (
            component::group([
                view_tool(Icon::GridOn, "Connections", View::Matrix),
                view_tool(Icon::ViewList, "Entities", View::Entities),
                view_tool(Icon::Hub, "Network", View::Network),
                view_tool(Icon::Timer, "Media clock", View::Clocks),
            ]),
            DIVIDER_WIDTH + TOOLBAR_GAP + tools(4.0),
            Some(3),
            true,
        ),
        (
            component::tool(
                Icon::Refresh,
                "Ask every entity to announce itself",
                matches!(triib.network_state, NetworkState::Running { .. })
                    .then_some(Message::Rediscover),
            ),
            DIVIDER_WIDTH + TOOLBAR_GAP + TOOL_WIDTH,
            Some(1),
            true,
        ),
    ];
    let right = slots.len();
    slots.push((
        component::search_bar(
            text_input("Search entities and streams", &triib.search)
                .on_input(Message::SearchChanged),
            Vec::new(),
            SEARCH_WIDTH,
        ),
        SEARCH_WIDTH,
        Some(2),
        false,
    ));
    slots.push((
        component::tool(Icon::Settings, "Settings", Some(Message::SettingsOpened)),
        TOOL_WIDTH,
        Some(4),
        false,
    ));
    slots.push((
        component::toggle_tool(
            inspector_glyph,
            "Inspector",
            triib.settings.inspector,
            Message::InspectorToggled,
        ),
        TOOL_WIDTH,
        None,
        false,
    ));

    let widths: Vec<(f32, Option<u8>)> = slots
        .iter()
        .map(|(_, width, order, _)| (*width, *order))
        .collect();
    let shown = component::fitting_slots(room, &widths);
    let mut bar = iced::widget::row![].spacing(TOOLBAR_GAP).align_y(Center);
    let mut hidden = Vec::new();
    for (index, ((element, _, _, divider), shown)) in slots.into_iter().zip(shown).enumerate() {
        if index == right {
            bar = bar.push(space::horizontal());
        }
        if shown {
            if divider {
                bar = bar.push(component::toolbar_divider());
            }
            bar = bar.push(element);
        } else {
            hidden.push(element);
        }
    }
    if !hidden.is_empty() {
        bar = bar.push(component::overflow(
            hidden,
            triib.overflow_open,
            Message::OverflowToggled,
            Message::OverflowClosed,
        ));
    }
    container(bar)
        .padding([0.0, TOOLBAR_PADDING])
        .height(TOOLBAR_HEIGHT)
        .align_y(Center)
        .into()
}

/// What to show instead of entities: no interface, starting, a failure,
/// or listening with none heard yet.
fn network_state_view<'a>(triib: &'a Triib, state: &'a NetworkState) -> Element<'a, Message> {
    let interface = triib.settings.interface.as_deref().unwrap_or_default();
    match state {
        NetworkState::Idle => component::empty_state(
            Icon::Lan,
            "No interface",
            "Choose the interface on the AVB network to discover entities.",
        ),
        NetworkState::Starting => {
            component::empty_state(Icon::Lan, "Starting", format!("Opening {interface}."))
        }
        NetworkState::Running { .. } => component::empty_state(
            Icon::WifiTethering,
            "Listening",
            format!("Entities on {interface} appear here as they announce themselves."),
        ),
        NetworkState::Failed(failure) => failure_view(interface, failure),
    }
}

fn failure_view<'a>(interface: &str, failure: &'a Failure) -> Element<'a, Message> {
    let (headline, action) = match &failure.fix {
        Some(fix) => (
            "Permission needed".to_owned(),
            button::with_icon(Kind::Tonal, Icon::ContentCopy, "Copy the command")
                .on_press(Message::Copy(fix.clone())),
        ),
        None => (
            format!("Cannot use {interface}"),
            button::with_icon(Kind::Tonal, Icon::Refresh, "Try again")
                .on_press(Message::RetryNetwork),
        ),
    };
    let mut details = iced::widget::column![
        styled(failure.message.as_str(), Type::BodyMedium)
            .style(style::on_surface_variant)
            .center(),
    ]
    .spacing(12)
    .align_x(Center);
    if let Some(fix) = &failure.fix {
        details = details.push(
            styled(fix.as_str(), Type::BodySmall)
                .style(style::on_surface_variant)
                .center(),
        );
    }
    container(
        iced::widget::column![
            scramble_ui::icon::icon(Icon::Error, 48).style(style::error_text),
            styled(headline, Type::TitleLarge),
            details,
            action,
        ]
        .spacing(16)
        .align_x(Center)
        .padding(16)
        .max_width(420),
    )
    .center(Fill)
    .into()
}

fn content(triib: &Triib) -> Element<'_, Message> {
    match triib.settings.view {
        View::Matrix => match &triib.network_state {
            NetworkState::Running { .. } | NetworkState::Starting => crate::matrix::view(triib),
            state => network_state_view(triib, state),
        },
        View::Entities if !triib.entities.is_empty() => entity_table(triib),
        View::Entities => match &triib.network_state {
            NetworkState::Running { .. } | NetworkState::Starting => component::empty_state(
                Icon::ViewList,
                "No entities yet",
                "Every entity on the network, with its roles, SR classes and clock.",
            ),
            state => network_state_view(triib, state),
        },
        View::Network => match &triib.network_state {
            NetworkState::Running { .. } | NetworkState::Starting => crate::netmap::view(triib),
            state => network_state_view(triib, state),
        },
        View::Clocks => match &triib.network_state {
            NetworkState::Running { .. } | NetworkState::Starting => crate::clock_view::view(triib),
            state => network_state_view(triib, state),
        },
    }
}

fn entity_table(triib: &Triib) -> Element<'_, Message> {
    iced::widget::responsive(move |size| entity_table_at(triib, size.width)).into()
}

/// The entity table for `width` pixels: across the whole width when its
/// columns fit, each widened in proportion to its text, else at their own
/// widths, scrolling sideways.
fn entity_table_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    #[derive(Clone)]
    struct Row<'a> {
        entity: &'a DiscoveredEntity,
        name: String,
        fields: [String; FIELDS.len()],
    }
    let rows: Vec<Row<'_>> = triib
        .shown_entities()
        .into_iter()
        .map(|entity| {
            let model = triib.models.get(&entity.entity_id());
            Row {
                entity,
                name: triib.entity_name(entity),
                fields: [
                    model
                        .and_then(EntityModel::entity)
                        .map_or_else(String::new, |descriptor| descriptor.group_name.to_owned()),
                    model.map_or_else(String::new, describe::product),
                    model
                        .and_then(EntityModel::entity)
                        .map_or_else(String::new, |descriptor| {
                            descriptor.firmware_version.to_owned()
                        }),
                    model.map_or_else(String::new, describe::milan),
                    describe::roles(&entity.adp),
                    model.map_or_else(String::new, describe::media_clock),
                    describe::clock(&entity.adp),
                    describe::read_state(model),
                    entity.entity_id().to_string(),
                ],
            }
        })
        .collect();
    // Each column's own width: its widest text, and for the name its icon
    // and the row button's padding.
    let widest = |heading: &str, texts: &mut dyn Iterator<Item = &String>| {
        texts
            .map(|text| measure(text, TEXT, 14.0))
            .fold(measure(heading, MEDIUM, 14.0), f32::max)
    };
    let mut natural =
        vec![widest("Name", &mut rows.iter().map(|row| &row.name)) + 20.0 + 8.0 + 32.0];
    for (index, heading) in FIELDS.iter().enumerate() {
        natural.push(widest(
            heading,
            &mut rows.iter().map(|row| &row.fields[index]),
        ));
    }
    let needed = natural.iter().sum::<f32>()
        + natural.len() as f32 * (2.0 * TABLE_PADDING + 1.0)
        + 2.0 * TABLE_MARGIN
        + TABLE_SLACK;
    let fits = width >= needed;
    let column_width = |column: usize| {
        if fits {
            Length::FillPortion(natural[column].ceil().max(1.0) as u16)
        } else {
            Length::Shrink
        }
    };

    let header =
        |label: &'static str| styled(label, Type::LabelLarge).style(style::on_surface_variant);
    let mut columns = vec![
        table::column(header("Name"), |row: Row<'_>| {
            let entity_id = row.entity.entity_id();
            button::custom(
                Kind::Row,
                iced::widget::row![
                    icon::icon(describe::glyph(&row.entity.adp), 20),
                    styled(row.name, Type::BodyMedium),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .selected(triib.selected == Some(entity_id))
            .on_press(Message::EntitySelected(entity_id))
        })
        .width(column_width(0))
        .align_y(Center),
    ];
    for (index, heading) in FIELDS.iter().enumerate() {
        columns.push(
            table::column(header(heading), move |row: Row<'_>| {
                styled(row.fields[index].clone(), Type::BodyMedium)
            })
            .width(column_width(index + 1))
            .align_y(Center),
        );
    }
    let mut table = table(columns, rows).padding_x(TABLE_PADDING).padding_y(8);
    if fits {
        table = table.width(Fill);
    }
    let content = container(table).padding(TABLE_MARGIN);
    if fits {
        return component::scroll(content).height(Fill).into();
    }
    scrollable(content)
        .direction(scrollable::Direction::Both {
            vertical: component::thin_scrollbar(),
            horizontal: component::thin_scrollbar(),
        })
        .style(style::scrollbar)
        .width(Fill)
        .height(Fill)
        .into()
}

fn inspector(triib: &Triib, width: Length) -> Element<'_, Message> {
    let body: Element<'_, Message> = match triib.selected {
        None => styled("Select an entity to see its details.", Type::BodyMedium)
            .style(style::on_surface_variant)
            .into(),
        Some(entity_id) => match triib.entities.get(&entity_id) {
            None => styled(format!("{entity_id} is offline."), Type::BodyMedium)
                .style(style::on_surface_variant)
                .into(),
            Some(entity) => entity_details(triib, entity, triib.models.get(&entity_id)),
        },
    };
    component::side_sheet_sized("Inspector", Message::InspectorToggled, body, width)
}

/// A label and its value, in the inspector.
/// The ID of the field a name is edited in, for focusing it.
pub const NAME_FIELD: &str = "inspector-name";

/// A label and an element beside it, in the inspector.
fn labelled<'a>(
    label: impl Into<String>,
    value: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let label: String = label.into();
    iced::widget::row![
        styled(label, Type::BodyMedium)
            .style(style::on_surface_variant)
            .width(Length::Fixed(132.0)),
        container(value).width(Fill),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

/// A label with an element under it, for pickers too wide to sit beside
/// their label.
pub(crate) fn stacked<'a>(
    label: impl Into<String>,
    value: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let label: String = label.into();
    iced::widget::column![
        styled(label, Type::BodyMedium).style(style::on_surface_variant),
        container(value).width(Fill),
    ]
    .spacing(4)
    .into()
}

/// A name the entity holds with a button to edit it, or while it is
/// edited a field to type it in.
fn name_property<'a>(
    triib: &'a Triib,
    label: &'static str,
    target: NameTarget,
) -> Element<'a, Message> {
    if let Some((editing, text)) = &triib.editing
        && *editing == target
    {
        return iced::widget::column![
            styled(label, Type::BodyMedium).style(style::on_surface_variant),
            name_field(text),
        ]
        .spacing(4)
        .into();
    }
    labelled(label, name_line(triib, target, "Not set"))
}

/// A name with a button to edit it, `unset` standing in for none, or
/// while it is edited a field to type it in.
fn name_line<'a>(
    triib: &'a Triib,
    target: NameTarget,
    unset: &'static str,
) -> Element<'a, Message> {
    if let Some((editing, text)) = &triib.editing
        && *editing == target
    {
        return name_field(text);
    }
    let name = triib.current_name(target);
    let shown = if name.is_empty() {
        unset.to_owned()
    } else {
        name
    };
    iced::widget::row![
        styled(shown, Type::BodyMedium).width(Fill),
        component::tip(
            button::icon_button(Icon::Edit)
                .size(Size::ExtraSmall)
                .on_press(Message::EditName(target)),
            "Rename",
        ),
    ]
    .spacing(4)
    .align_y(Center)
    .into()
}

/// The field a name is typed in, with buttons to save and cancel; Enter
/// saves too.
fn name_field<'a>(text: &str) -> Element<'a, Message> {
    let icon = |glyph, label, message| {
        component::tip(
            button::icon_button(glyph)
                .size(Size::ExtraSmall)
                .on_press(message),
            label,
        )
    };
    iced::widget::row![
        text_input("Name", text)
            .id(iced::widget::Id::new(NAME_FIELD))
            .on_input(Message::EditChanged)
            .on_submit(Message::EditSubmitted)
            .padding([6, 10])
            .style(style::outlined_field)
            .width(Fill),
        icon(Icon::Check, "Save", Message::EditSubmitted),
        icon(Icon::Close, "Cancel", Message::EditCancelled),
    ]
    .spacing(4)
    .align_y(Center)
    .into()
}

/// A picker in the inspector's style, offering `choices` with `shown`
/// picked; a change already on its way holds it still.
fn picker<'a, T>(
    choices: Vec<T>,
    shown: T,
    busy: bool,
    pick: impl Fn(T) -> Action + 'a,
) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
{
    pick_list(choices, Some(shown), move |choice| {
        if busy {
            Message::Nothing
        } else {
            Message::Act(pick(choice))
        }
    })
    .text_size(13)
    .padding([4, 8])
    .width(Fill)
    .style(style::outlined_select)
    .menu_style(style::select_menu)
    .into()
}

/// A stream's format, with a picker of the formats it supports when it
/// has more than one.
fn format_picker<'a>(
    triib: &'a Triib,
    entity: EntityId,
    stream: &StreamDescriptor<'_>,
) -> Element<'a, Message> {
    let (descriptor_type, index) = (stream.descriptor_type, stream.index);
    let changing = triib
        .changing(|action| {
            matches!(action, Action::SetStreamFormat { entity: pending, descriptor_type: kind, index: place, .. }
                if *pending == entity && *kind == descriptor_type && *place == index)
        })
        .and_then(|action| match action {
            Action::SetStreamFormat { format, .. } => Some(*format),
            _ => None,
        });
    let shown = changing.unwrap_or(stream.current_format);
    let mut formats: Vec<StreamFormat> = stream.formats().collect();
    if !formats.contains(&stream.current_format) {
        formats.insert(0, stream.current_format);
    }
    if formats.len() < 2 {
        return styled(shown.to_string(), Type::BodySmall)
            .style(style::on_surface_variant)
            .into();
    }
    picker(formats, shown, changing.is_some(), move |format| {
        Action::SetStreamFormat {
            entity,
            descriptor_type,
            index,
            format,
        }
    })
}

/// A sampling rate in words, such as "48 kHz" or "44.1 kHz".
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rate(pub(crate) SamplingRate);

impl fmt::Display for Rate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hertz = self.0.base_frequency();
        if hertz.is_multiple_of(1000) {
            write!(formatter, "{} kHz", hertz / 1000)?;
        } else {
            write!(formatter, "{} kHz", f64::from(hertz) / 1000.0)?;
        }
        // The pull multiplies the base frequency (7.3.1.2).
        match self.0.pull() {
            0 => Ok(()),
            1 => formatter.write_str(" × 1/1.001"),
            2 => formatter.write_str(" × 1.001"),
            3 => formatter.write_str(" × 24/25"),
            4 => formatter.write_str(" × 25/24"),
            5 => formatter.write_str(" × 1/8"),
            pull => write!(formatter, ", pull {pull}"),
        }
    }
}

/// An audio unit's sampling rate, with a picker of the rates it supports
/// when it has more than one.
fn rate_picker<'a>(
    triib: &'a Triib,
    entity: EntityId,
    unit: &AudioUnitDescriptor<'_>,
) -> Element<'a, Message> {
    let index = unit.index;
    let changing = triib
        .changing(|action| {
            matches!(action, Action::SetSamplingRate { entity: pending, unit: place, .. }
                if *pending == entity && *place == index)
        })
        .and_then(|action| match action {
            Action::SetSamplingRate { rate, .. } => Some(Rate(*rate)),
            _ => None,
        });
    let current = Rate(unit.current_sampling_rate);
    let shown = changing.unwrap_or(current);
    let mut rates: Vec<Rate> = unit.sampling_rates().map(Rate).collect();
    if !rates.contains(&current) {
        rates.insert(0, current);
    }
    if rates.len() < 2 {
        return styled(shown.to_string(), Type::BodyMedium).into();
    }
    picker(rates, shown, changing.is_some(), move |rate| {
        Action::SetSamplingRate {
            entity,
            unit: index,
            rate: rate.0,
        }
    })
}

/// A clock source by its name.
#[derive(Debug, Clone, PartialEq)]
struct Source {
    index: u16,
    name: String,
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.name)
    }
}

/// A clock domain's clock source, with a picker of the sources it can use
/// when it has more than one.
pub(crate) fn source_picker<'a>(
    triib: &'a Triib,
    entity: EntityId,
    model: &EntityModel,
    domain: &ClockDomainDescriptor<'_>,
) -> Element<'a, Message> {
    let domain_index = domain.index;
    let source = |index: u16| Source {
        index,
        name: model
            .name_of(DescriptorType::CLOCK_SOURCE, index)
            .map_or_else(|| format!("Source {index}"), str::to_owned),
    };
    let changing = triib
        .changing(|action| {
            matches!(action, Action::SetClockSource { entity: pending, domain: place, .. }
                if *pending == entity && *place == domain_index)
        })
        .and_then(|action| match action {
            Action::SetClockSource { source, .. } => Some(*source),
            _ => None,
        });
    let shown = source(changing.unwrap_or(domain.clock_source_index));
    let mut sources: Vec<Source> = domain.clock_sources().map(source).collect();
    if !sources
        .iter()
        .any(|known| known.index == domain.clock_source_index)
    {
        sources.insert(0, source(domain.clock_source_index));
    }
    if sources.len() < 2 {
        return styled(shown.name, Type::BodyMedium).into();
    }
    picker(sources, shown, changing.is_some(), move |picked| {
        Action::SetClockSource {
            entity,
            domain: domain_index,
            source: picked.index,
        }
    })
}

fn property<'a>(label: impl Into<String>, value: String) -> Element<'a, Message> {
    let label: String = label.into();
    iced::widget::row![
        styled(label, Type::BodyMedium)
            .style(style::on_surface_variant)
            .width(Length::Fixed(132.0)),
        styled(value, Type::BodyMedium).width(Fill),
    ]
    .spacing(8)
    .into()
}

/// Everything read from an entity and everything it advertises, in
/// sections.
fn entity_details<'a>(
    triib: &'a Triib,
    entity: &'a DiscoveredEntity,
    model: Option<&'a EntityModel>,
) -> Element<'a, Message> {
    let mut column = iced::widget::column![].spacing(6);
    if entity
        .adp
        .entity_capabilities
        .contains(EntityCapabilities::AEM_IDENTIFY_CONTROL_INDEX_VALID)
    {
        let action = Action::Identify(entity.entity_id());
        let busy = triib.pending.contains(&action);
        column = column.push(
            button::with_icon(Kind::Tonal, Icon::Lightbulb, "Identify")
                .on_press_maybe((!busy).then_some(Message::Act(action))),
        );
    }
    if let Some(model) = model {
        column = column.extend(model_details(triib, entity.entity_id(), model));
    }
    column.extend(advertisement_details(entity)).into()
}

/// What the entity's descriptors say: its names, product, firmware,
/// Milan support, streams, channel mappings, interfaces and clocks, with
/// the names, stream formats, dynamic mappings, sampling rate and clock
/// source editable.
fn model_details<'a>(
    triib: &'a Triib,
    entity_id: EntityId,
    model: &'a EntityModel,
) -> Vec<Element<'a, Message>> {
    let mut items = Vec::new();
    match model.state {
        EnumerationState::Reading => items.push(
            styled(
                format!("Reading descriptors, {} so far.", model.descriptor_count()),
                Type::BodyMedium,
            )
            .style(style::on_surface_variant)
            .into(),
        ),
        EnumerationState::Failed(failure) => items.push(
            styled(
                format!(
                    "Could not read the entity model: {}.",
                    describe::failure(failure)
                ),
                Type::BodyMedium,
            )
            .style(style::error_text)
            .into(),
        ),
        EnumerationState::Complete | EnumerationState::NotRead => {}
    }
    let or_unset = |text: &str| {
        if text.is_empty() {
            "Not set".to_owned()
        } else {
            text.to_owned()
        }
    };
    let entity_name = |name_index| NameTarget {
        entity: entity_id,
        descriptor_type: DescriptorType::ENTITY,
        index: 0,
        name_index,
    };
    if let Some(descriptor) = model.entity() {
        items.push(component::section("Entity"));
        items.push(name_property(triib, "Name", entity_name(0)));
        items.push(name_property(triib, "Group", entity_name(1)));
        items.push(property("Product", describe::product(model)));
        items.push(property("Firmware", or_unset(descriptor.firmware_version)));
        items.push(property(
            "Serial number",
            or_unset(descriptor.serial_number),
        ));
        let configuration = model
            .name_of(
                DescriptorType::CONFIGURATION,
                descriptor.current_configuration,
            )
            .map(|name| {
                format!(
                    "{name} ({} of {})",
                    descriptor.current_configuration + 1,
                    descriptor.configurations_count
                )
            })
            .unwrap_or_else(|| (descriptor.current_configuration + 1).to_string());
        items.push(property("Configuration", configuration));
        items.push(property("Milan", describe::milan(model)));
    }
    for (input, title) in [(true, "Stream inputs"), (false, "Stream outputs")] {
        let streams: Vec<_> = model.streams(input).collect();
        if streams.is_empty() {
            continue;
        }
        items.push(component::section(title));
        for stream in streams {
            let target = NameTarget {
                entity: entity_id,
                descriptor_type: stream.descriptor_type,
                index: stream.index,
                name_index: 0,
            };
            let mut lines = iced::widget::column![
                name_line(triib, target, "Unnamed"),
                format_picker(triib, entity_id, &stream),
            ]
            .spacing(2);
            if let Some(state) = describe::stream_state(triib, model, &stream) {
                lines = lines.push(styled(state, Type::BodySmall).style(style::on_surface_variant));
            }
            items.push(lines.into());
        }
    }
    items.extend(mapping_view::mapping_details(triib, entity_id, model));
    let domains: Vec<_> = model.clock_domains().collect();
    if !domains.is_empty() {
        items.push(component::section("Media clock"));
        for domain in domains {
            let name = model
                .name_of(DescriptorType::CLOCK_DOMAIN, domain.index)
                .unwrap_or("Clock domain")
                .to_owned();
            items.push(stacked(
                name,
                source_picker(triib, entity_id, model, &domain),
            ));
        }
        for unit in model.audio_units() {
            items.push(stacked(
                "Sampling rate",
                rate_picker(triib, entity_id, &unit),
            ));
        }
    }
    let interfaces: Vec<_> = model.avb_interfaces().collect();
    if !interfaces.is_empty() {
        items.push(component::section("AVB interfaces"));
        for interface in interfaces {
            items.extend(interface_details(model, &interface));
        }
    }
    items.extend(descriptor_tree(triib, entity_id, model));
    items
}

/// An AVB interface: its address, and what it reports of gPTP, its path
/// to the grandmaster and its link.
fn interface_details<'a>(
    model: &EntityModel,
    interface: &AvbInterfaceDescriptor<'_>,
) -> Vec<Element<'a, Message>> {
    let name = model
        .name_of(DescriptorType::AVB_INTERFACE, interface.index)
        .unwrap_or("Interface")
        .to_owned();
    let mut items = vec![
        styled(name, Type::TitleSmall).into(),
        property("MAC address", interface.mac_address.to_string()),
        property("Clock identity", interface.clock_identity.to_string()),
    ];
    if let Some(info) = model.avb_info(interface.index) {
        items.push(property(
            "Grandmaster",
            format!(
                "{}, domain {}",
                info.gptp_grandmaster_id, info.gptp_domain_number
            ),
        ));
        items.push(property(
            "Peer delay",
            format!("{} ns", info.propagation_delay),
        ));
        let flags: Vec<String> = [
            (AvbInfoFlags::AS_CAPABLE, "asCapable"),
            (AvbInfoFlags::GPTP_ENABLED, "gPTP"),
            (AvbInfoFlags::SRP_ENABLED, "SRP"),
        ]
        .into_iter()
        .filter(|(flag, _)| info.flags.contains(*flag))
        .map(|(_, name)| name.to_owned())
        .collect();
        items.push(property(
            "Running",
            if flags.is_empty() {
                "None reported".to_owned()
            } else {
                flags.join(", ")
            },
        ));
    }
    if let Some(path) = model.as_path(interface.index) {
        let hops = path.len().saturating_sub(1);
        items.push(property(
            "Path",
            match hops {
                0 => "Its own grandmaster".to_owned(),
                1 => "1 hop from the grandmaster".to_owned(),
                hops => format!("{hops} hops from the grandmaster"),
            },
        ));
    }
    if let Some(counters) = model
        .counters(DescriptorType::AVB_INTERFACE, interface.index)
        .and_then(|counters| counters.avb_interface())
    {
        for (label, value) in [
            ("Link up", counters.link_up),
            ("Link down", counters.link_down),
            ("Grandmaster changes", counters.gptp_gm_changed),
            ("Frames sent", counters.frames_tx),
            ("Frames received", counters.frames_rx),
            ("CRC errors", counters.rx_crc_error),
        ] {
            if let Some(value) = value {
                items.push(property(label, value.to_string()));
            }
        }
    }
    items
}

/// The descriptor types the inspector's tree lists, in the standard's
/// order.
const TREE: [DescriptorType; 18] = [
    DescriptorType::ENTITY,
    DescriptorType::CONFIGURATION,
    DescriptorType::AUDIO_UNIT,
    DescriptorType::STREAM_INPUT,
    DescriptorType::STREAM_OUTPUT,
    DescriptorType::JACK_INPUT,
    DescriptorType::JACK_OUTPUT,
    DescriptorType::AVB_INTERFACE,
    DescriptorType::CLOCK_SOURCE,
    DescriptorType::MEMORY_OBJECT,
    DescriptorType::LOCALE,
    DescriptorType::STRINGS,
    DescriptorType::STREAM_PORT_INPUT,
    DescriptorType::STREAM_PORT_OUTPUT,
    DescriptorType::AUDIO_CLUSTER,
    DescriptorType::AUDIO_MAP,
    DescriptorType::CONTROL,
    DescriptorType::CLOCK_DOMAIN,
];

/// Every descriptor read from the entity, by type, each type opening to
/// its descriptors with their names and what matters most about them.
fn descriptor_tree<'a>(
    triib: &'a Triib,
    entity_id: EntityId,
    model: &'a EntityModel,
) -> Vec<Element<'a, Message>> {
    let mut items = vec![component::section("Descriptors")];
    for descriptor_type in TREE {
        let descriptors: Vec<(u16, &[u8])> = model.descriptors(descriptor_type).collect();
        if descriptors.is_empty() {
            continue;
        }
        let open = triib.tree_open.contains(&(entity_id, descriptor_type));
        let label = descriptor_type
            .name()
            .map_or_else(|| format!("{descriptor_type:?}"), describe::flag_name);
        let heading = iced::widget::row![
            icon::icon(
                if open {
                    Icon::ExpandMore
                } else {
                    Icon::ChevronRight
                },
                20
            ),
            styled(label, Type::BodyMedium).width(Fill),
            styled(descriptors.len().to_string(), Type::BodySmall).style(style::on_surface_variant),
        ]
        .spacing(8)
        .align_y(Center);
        items.push(
            button::custom(Kind::Row, heading)
                .width(Fill)
                .on_press(Message::TreeToggled(entity_id, descriptor_type))
                .into(),
        );
        if !open {
            continue;
        }
        for (index, bytes) in descriptors {
            let name = match descriptor_type {
                DescriptorType::ENTITY => model.entity_name().map(str::to_owned),
                _ => model.name_of(descriptor_type, index).map(str::to_owned),
            };
            let detail = descriptor_detail(model, descriptor_type, bytes);
            // Descriptors that have no name show what they hold instead.
            let (title, detail) = match (name, detail) {
                (Some(name), detail) => (name, detail),
                (None, Some(detail)) if !descriptor_type.has_object_name() => (detail, None),
                (None, detail) => ("Unnamed".to_owned(), detail),
            };
            let mut lines = iced::widget::column![styled(title, Type::BodyMedium)];
            if let Some(detail) = detail {
                lines =
                    lines.push(styled(detail, Type::BodySmall).style(style::on_surface_variant));
            }
            items.push(
                iced::widget::row![
                    styled(index.to_string(), Type::BodySmall)
                        .style(style::on_surface_variant)
                        .width(Length::Fixed(28.0)),
                    lines,
                ]
                .spacing(8)
                .padding(iced::Padding {
                    left: 28.0,
                    ..iced::Padding::ZERO
                })
                .into(),
            );
        }
    }
    items
}

/// What matters most about a descriptor, for the inspector's tree.
fn descriptor_detail(
    model: &EntityModel,
    descriptor_type: DescriptorType,
    bytes: &[u8],
) -> Option<String> {
    match descriptor_type {
        DescriptorType::ENTITY => EntityDescriptor::decode(bytes)
            .ok()
            .map(|entity| format!("Firmware {}", entity.firmware_version)),
        DescriptorType::CONFIGURATION => {
            ConfigurationDescriptor::decode(bytes)
                .ok()
                .map(|configuration| {
                    let types = configuration.descriptor_counts().count();
                    format!("{types} descriptor types")
                })
        }
        DescriptorType::AUDIO_UNIT => AudioUnitDescriptor::decode(bytes)
            .ok()
            .map(|unit| Rate(unit.current_sampling_rate).to_string()),
        DescriptorType::STREAM_INPUT | DescriptorType::STREAM_OUTPUT => {
            StreamDescriptor::decode(bytes)
                .ok()
                .map(|stream| stream.current_format.to_string())
        }
        DescriptorType::AVB_INTERFACE => AvbInterfaceDescriptor::decode(bytes)
            .ok()
            .map(|interface| interface.mac_address.to_string()),
        DescriptorType::CLOCK_SOURCE => ClockSourceDescriptor::decode(bytes).ok().map(|source| {
            let kind = source
                .clock_source_type
                .name()
                .map_or_else(|| "Clock".to_owned(), describe::flag_name);
            match source.location_type {
                DescriptorType::ENTITY => kind,
                location => format!(
                    "{kind}, from {} {}",
                    location
                        .name()
                        .map_or_else(|| format!("{location:?}"), describe::flag_name)
                        .to_lowercase(),
                    source.location_index
                ),
            }
        }),
        DescriptorType::CLOCK_DOMAIN => ClockDomainDescriptor::decode(bytes).ok().map(|domain| {
            let source = model
                .name_of(DescriptorType::CLOCK_SOURCE, domain.clock_source_index)
                .map_or_else(
                    || format!("source {}", domain.clock_source_index),
                    str::to_owned,
                );
            format!("Using {source}")
        }),
        DescriptorType::LOCALE => LocaleDescriptor::decode(bytes)
            .ok()
            .map(|locale| locale.locale_identifier.to_owned()),
        DescriptorType::STRINGS => StringsDescriptor::decode(bytes).ok().map(|strings| {
            let used: Vec<&str> = strings
                .strings
                .iter()
                .copied()
                .filter(|text| !text.is_empty())
                .collect();
            used.join(", ")
        }),
        DescriptorType::STREAM_PORT_INPUT | DescriptorType::STREAM_PORT_OUTPUT => {
            StreamPortDescriptor::decode(bytes).ok().map(|port| {
                let count = |count: u16, one: &str, many: &str| {
                    format!("{count} {}", if count == 1 { one } else { many })
                };
                format!(
                    "{}, {}",
                    count(port.number_of_clusters, "cluster", "clusters"),
                    count(port.number_of_maps, "map", "maps")
                )
            })
        }
        _ => None,
    }
}

/// What the entity advertises with ADP, in sections.
fn advertisement_details(entity: &DiscoveredEntity) -> Vec<Element<'_, Message>> {
    let adp = &entity.adp;
    let capabilities = adp.entity_capabilities;
    let indexed = |flag: EntityCapabilities, index: u16| {
        if capabilities.contains(flag) {
            index.to_string()
        } else {
            "Not advertised".to_owned()
        }
    };
    let mut items = vec![
        component::section("Identity"),
        property("Entity ID", adp.entity_id.to_string()),
        property("MAC address", entity.mac.to_string()),
        property("Entity model", adp.entity_model_id.to_string()),
        component::section("Roles"),
        property("Roles", describe::roles(adp)),
        property("Talker", flags_text(adp.talker_capabilities.names())),
        property("Listener", flags_text(adp.listener_capabilities.names())),
        component::section("Clock"),
        property("BTC", describe::clock(adp)),
        property("gPTP domain", adp.gptp_domain_number.to_string()),
        property("SR classes", describe::classes(adp).to_owned()),
        component::section("Entity model indexes"),
        property(
            "Configuration",
            indexed(
                EntityCapabilities::AEM_CONFIGURATION_INDEX_VALID,
                adp.current_configuration_index,
            ),
        ),
        property(
            "Identify control",
            indexed(
                EntityCapabilities::AEM_IDENTIFY_CONTROL_INDEX_VALID,
                adp.identify_control_index,
            ),
        ),
        property(
            "AVB interface",
            indexed(
                EntityCapabilities::AEM_INTERFACE_INDEX_VALID,
                adp.interface_index,
            ),
        ),
        component::section("Advertising"),
        property("Valid time", format!("{} s", adp.valid_seconds())),
        property("Available index", adp.available_index.to_string()),
        property(
            "Association",
            if capabilities.contains(EntityCapabilities::ASSOCIATION_ID_VALID) {
                format!("0x{:016x}", adp.association_id)
            } else {
                "None".to_owned()
            },
        ),
        component::section("Capabilities"),
    ];
    for name in capabilities.names() {
        items.push(styled(describe::flag_name(name), Type::BodyMedium).into());
    }
    items
}

/// A bit field's names in words, or "None".
fn flags_text<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let words: Vec<String> = names
        .filter(|name| *name != "IMPLEMENTED")
        .map(describe::flag_name)
        .collect();
    if words.is_empty() {
        "None".to_owned()
    } else {
        words.join(", ")
    }
}

fn status_bar(triib: &Triib) -> Element<'_, Message> {
    iced::widget::responsive(move |size| status_bar_at(triib, size.width))
        .height(STATUS_BAR_HEIGHT + 1.0)
        .into()
}

/// The status bar for `width` pixels, leaving out the interface and the
/// controller's ID when narrow.
fn status_bar_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    let full = width >= STATUS_FULL;
    let link = match triib.interface() {
        Some(interface) => {
            let mut text = interface.name.clone();
            text.push_str(if interface.up { ", up" } else { ", link down" });
            if let Some(clock) = interface.hardware_clock {
                text.push_str(&format!(", hardware clock ptp{clock}"));
            }
            text
        }
        None => "No interface".to_owned(),
    };
    let label =
        |content: String| styled(content, Type::LabelMedium).style(style::on_surface_variant);
    let count = triib.entities.len();
    let entities = format!("{count} {}", if count == 1 { "entity" } else { "entities" });
    let notice = triib.notice.clone();
    let state = match &triib.network_state {
        NetworkState::Idle => "Not discovering".to_owned(),
        NetworkState::Starting => "Starting".to_owned(),
        NetworkState::Running { controller } if full => format!("Discovering as {controller}"),
        NetworkState::Running { .. } => "Discovering".to_owned(),
        NetworkState::Failed(failure) if failure.fix.is_some() => "Permission needed".to_owned(),
        NetworkState::Failed(_) => "Stopped by an error".to_owned(),
    };
    iced::widget::column![
        rule::horizontal(1).style(style::divider),
        container(
            iced::widget::row![
                if full {
                    Element::from(label(link))
                } else {
                    Element::from(space())
                },
                space::horizontal(),
                match notice {
                    Some(notice) => iced::widget::row![
                        styled(notice, Type::LabelMedium).style(style::error_text),
                        button::icon_button(Icon::Close).on_press(Message::DismissNotice),
                    ]
                    .spacing(4)
                    .align_y(Center)
                    .into(),
                    None => Element::from(space()),
                },
                label(entities),
                label(state),
            ]
            .spacing(if full { 24 } else { 16 })
            .align_y(Center),
        )
        .padding([0, 16])
        .height(STATUS_BAR_HEIGHT)
        .align_y(Center)
        .style(|theme: &Theme| {
            let scheme = Scheme::of(theme);
            container::Style {
                background: Some(scheme.chrome.into()),
                ..Default::default()
            }
        }),
    ]
    .into()
}

fn speed_text(megabits: u32) -> String {
    if megabits >= 1000 && megabits.is_multiple_of(1000) {
        format!("{} Gb/s", megabits / 1000)
    } else {
        format!("{megabits} Mb/s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use atdecc::aecp::AemPdu;
    use atdecc::controller::{Config, Controller};
    use atdecc::{EntityId, Instant, MacAddress};
    use iced::Size;

    use crate::settings::Settings;

    /// The entities and models a controller reads from the bench, replaying
    /// the capture of a live run.
    fn bench() -> (
        BTreeMap<EntityId, DiscoveredEntity>,
        BTreeMap<EntityId, EntityModel>,
    ) {
        const CAPTURE: &[u8] = include_bytes!("../../atdecc/tests/captures/bench-enumeration.pcap");
        const HOST: [u8; 6] = [0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b];
        // Classic pcap, in microseconds or nanoseconds.
        let unit = match u32::from_le_bytes(CAPTURE[..4].try_into().unwrap()) {
            0xa1b2_3c4d => 1,
            _ => 1000,
        };
        let mut records = Vec::new();
        let mut at = 24;
        while at + 16 <= CAPTURE.len() {
            let word = |offset: usize| {
                u32::from_le_bytes(CAPTURE[at + offset..at + offset + 4].try_into().unwrap())
            };
            let time = u64::from(word(0)) * 1_000_000_000 + u64::from(word(4)) * unit;
            let length = word(8) as usize;
            records.push((time, &CAPTURE[at + 16..at + 16 + length]));
            at += 16 + length;
        }
        let ours = |frame: &[u8]| frame[6..12] == HOST;
        // The controller numbers its commands as the one that made the
        // capture did, so the answers match them.
        let first_sequence_id = records
            .iter()
            .filter(|(_, frame)| ours(frame) && frame[14] == 0xfb)
            .find_map(|(_, frame)| AemPdu::decode(&frame[14..]).ok())
            .map_or(0, |aem| aem.header.sequence_id);
        let mut config = Config::new(EntityId(0x9c6b_00ff_fe30_9a2b));
        config.register_unsolicited = false;
        config.network_info = false;
        config.media_clock_info = false;
        config.first_sequence_id = first_sequence_id;
        let mut controller = Controller::new(config);
        let mut buffer = [0; 1500];
        controller.discover(None);
        let start = records[0].0;
        for &(time, frame) in &records {
            let now = Instant::from_nanos(time.saturating_sub(start));
            while controller.poll_transmit(&mut buffer).unwrap().is_some() {}
            while let Some(deadline) = controller
                .poll_timeout()
                .filter(|deadline| *deadline <= now)
            {
                controller.handle_timeout(deadline);
            }
            let source = MacAddress(frame[6..12].try_into().unwrap());
            if ours(frame) || u16::from_be_bytes([frame[12], frame[13]]) != 0x22f0 {
                continue;
            }
            let _ = controller.handle_frame(now, source, &frame[14..]);
            controller.handle_timeout(now);
        }
        let entities: BTreeMap<EntityId, DiscoveredEntity> = controller
            .entities()
            .map(|entity| (entity.entity_id(), *entity))
            .collect();
        let models = entities
            .keys()
            .filter_map(|&entity_id| Some((entity_id, controller.model(entity_id)?.clone())))
            .collect();
        (entities, models)
    }

    /// A bench entity as it would read with its descriptors changed by
    /// `patch`: read again by a controller from those descriptors, its
    /// stream input port answering GET_AUDIO_MAP with `mappings` (stream
    /// channel, cluster channel) and its stream inputs bound as `bound`
    /// says (input, talker, output) and flowing.
    fn reread(
        entities: &BTreeMap<EntityId, DiscoveredEntity>,
        models: &BTreeMap<EntityId, EntityModel>,
        entity_id: EntityId,
        patch: impl FnOnce(&mut BTreeMap<(u16, u16), Vec<u8>>),
        mappings: &[(u16, u16)],
        bound: &[(u16, EntityId, u16)],
    ) -> EntityModel {
        use atdecc::aecp::{AecpHeader, AecpMessageType, AemStatus};
        use atdecc::aem::StreamInfoFlags;
        use atdecc::{AcmpMessageType, Acmpdu, AemCommandType, EntityCapabilities};

        let mut descriptors: BTreeMap<(u16, u16), Vec<u8>> = models[&entity_id]
            .all_descriptors()
            .map(|(descriptor_type, index, bytes)| ((descriptor_type.0, index), bytes.to_vec()))
            .collect();
        patch(&mut descriptors);
        let bound_to = |input: u16| {
            bound
                .iter()
                .find(|(place, _, _)| *place == input)
                .map(|&(_, talker, output)| (talker, output))
        };

        let mut config = Config::new(EntityId(0x9c6b_00ff_fe30_9a2b));
        config.register_unsolicited = false;
        config.network_info = false;
        let mut controller = Controller::new(config);
        let entity = entities[&entity_id];
        let mut adpdu = entity.adp;
        adpdu.entity_capabilities = EntityCapabilities::AEM_SUPPORTED;
        controller.handle_adpdu(Instant::from_millis(0), entity.mac, &adpdu);
        let mut buffer = [0; 1500];
        let mut out = [0; 1500];
        for step in 1..400 {
            let now = Instant::from_millis(step * 100);
            while let Some(transmit) = controller.poll_transmit(&mut buffer).unwrap() {
                let frame = &buffer[..transmit.length];
                if let Ok(command) = Acmpdu::decode(frame) {
                    if command.message_type != AcmpMessageType::GET_RX_STATE_COMMAND
                        || command.listener_entity_id != entity_id
                    {
                        continue;
                    }
                    let talker = bound_to(command.listener_unique_id);
                    let response = Acmpdu {
                        message_type: command.message_type.response(),
                        talker_entity_id: talker.map_or(EntityId(0), |(talker, _)| talker),
                        talker_unique_id: talker.map_or(0, |(_, output)| output),
                        connection_count: u16::from(talker.is_some()),
                        ..command
                    };
                    let length = response.encode(&mut out).unwrap();
                    controller
                        .handle_frame(now, entity.mac, &out[..length])
                        .unwrap();
                    continue;
                }
                let Ok(command) = AemPdu::decode(frame) else {
                    continue;
                };
                let mut payload = command.payload.to_vec();
                let key = |at: usize, payload: &[u8]| {
                    (
                        u16::from_be_bytes([payload[at], payload[at + 1]]),
                        u16::from_be_bytes([payload[at + 2], payload[at + 3]]),
                    )
                };
                let status = match command.command_type {
                    AemCommandType::READ_DESCRIPTOR => match descriptors.get(&key(4, &payload)) {
                        Some(bytes) => {
                            payload.truncate(4);
                            payload.extend_from_slice(bytes);
                            AemStatus::SUCCESS
                        }
                        None => AemStatus::NO_SUCH_DESCRIPTOR,
                    },
                    AemCommandType::GET_AUDIO_MAP if !mappings.is_empty() => {
                        payload.truncate(6);
                        payload.extend_from_slice(&1u16.to_be_bytes());
                        payload.extend_from_slice(&(mappings.len() as u16).to_be_bytes());
                        payload.extend_from_slice(&[0, 0]);
                        for &(stream_channel, cluster_channel) in mappings {
                            for field in [0, stream_channel, 0, cluster_channel] {
                                payload.extend_from_slice(&u16::to_be_bytes(field));
                            }
                        }
                        AemStatus::SUCCESS
                    }
                    AemCommandType::GET_STREAM_INFO => {
                        let (descriptor_type, index) = key(0, &payload);
                        let mut flags = StreamInfoFlags::STREAM_FORMAT_VALID;
                        if descriptor_type == DescriptorType::STREAM_INPUT.0
                            && bound_to(index).is_some()
                        {
                            flags |= StreamInfoFlags::CONNECTED
                                | StreamInfoFlags::STREAM_ID_VALID
                                | StreamInfoFlags::MSRP_ACC_LAT_VALID;
                        }
                        payload.resize(48, 0);
                        payload[4..8].copy_from_slice(&flags.0.to_be_bytes());
                        AemStatus::SUCCESS
                    }
                    _ => AemStatus::NOT_IMPLEMENTED,
                };
                let response = AemPdu {
                    header: AecpHeader {
                        message_type: AecpMessageType::AEM_RESPONSE,
                        status: status.0,
                        ..command.header
                    },
                    payload: &payload,
                    ..command
                };
                let length = response.encode(&mut out).unwrap();
                controller
                    .handle_frame(now, entity.mac, &out[..length])
                    .unwrap();
            }
            controller.handle_timeout(now);
            let model = controller.model(entity_id);
            if !controller.busy(entity_id)
                && model.is_some_and(|model| model.state == EnumerationState::Complete)
            {
                break;
            }
        }
        controller.model(entity_id).unwrap().clone()
    }

    const MAC_MINI: EntityId = EntityId(0xd111_e597_f544_8000);
    const WIRED_ESP: EntityId = EntityId(0xe8f6_0ae0_9220_0000);
    const WIFI_ESP: EntityId = EntityId(0xfc01_2cfd_fe80_0000);

    /// The bench's Mac mini as if its stream port input took dynamic
    /// mappings, with its first two stream channels crossed and the fifth
    /// mapped.
    fn dynamic_mac_mini(
        entities: &BTreeMap<EntityId, DiscoveredEntity>,
        models: &BTreeMap<EntityId, EntityModel>,
    ) -> EntityModel {
        let model = reread(
            entities,
            models,
            MAC_MINI,
            |descriptors| {
                let port = descriptors
                    .get_mut(&(DescriptorType::STREAM_PORT_INPUT.0, 0))
                    .unwrap();
                port[16..18].copy_from_slice(&0u16.to_be_bytes());
            },
            &[(0, 1), (1, 0), (4, 4)],
            &[],
        );
        assert!(
            model
                .dynamic_mappings(DescriptorType::STREAM_PORT_INPUT, 0)
                .is_some_and(|read| read.count() == 3),
            "the mappings read"
        );
        model
    }

    /// The bench with a media clock chain: the wired endpoint clocked from
    /// the Mac mini's CRF output through its CRF input, and the Wi-Fi
    /// endpoint clocked from a CRF input bound to nothing.
    fn clocked_bench(
        entities: &BTreeMap<EntityId, DiscoveredEntity>,
        models: &mut BTreeMap<EntityId, EntityModel>,
    ) {
        // Clock source 1 is the endpoints' CRF input, stream input 1.
        let from_crf_input = |descriptors: &mut BTreeMap<(u16, u16), Vec<u8>>| {
            let domain = descriptors
                .get_mut(&(DescriptorType::CLOCK_DOMAIN.0, 0))
                .unwrap();
            domain[70..72].copy_from_slice(&1u16.to_be_bytes());
        };
        let wired = reread(
            entities,
            models,
            WIRED_ESP,
            from_crf_input,
            &[],
            &[(1, MAC_MINI, 1)],
        );
        let wifi = reread(entities, models, WIFI_ESP, from_crf_input, &[], &[]);
        models.insert(WIRED_ESP, wired);
        models.insert(WIFI_ESP, wifi);
    }

    /// Writes pictures of the whole window on the bench, at a phone's width
    /// and a desktop's, to the PNG files the variable `WINDOW_PICTURE`
    /// names, with what each shows added.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn pictures() {
        let Ok(path) = std::env::var("WINDOW_PICTURE") else {
            return;
        };
        let theme = scramble_ui::scheme::theme("triib".to_owned(), crate::app::TRIIB_SEED, true);
        // The inspector in place, not sliding in.
        scramble_ui::motion::set_reduced(true);
        let (entities, models) = bench();
        assert_eq!(models.len(), 3, "the bench's models");
        assert_eq!(entities.len(), 3, "the bench's entities");
        let interface = Interface {
            name: "enp6s0".to_owned(),
            mac: MacAddress(HOST_MAC),
            up: true,
            speed: Some(2500),
            physical: true,
            wireless: false,
            hardware_clock: Some(0),
        };
        let phone = Size::new(360.0, 760.0);
        let desktop = Size::new(1280.0, 800.0);
        for (suffix, view, inspector, size) in [
            ("matrix-phone", View::Matrix, false, phone),
            ("entities-phone", View::Entities, false, phone),
            ("network-phone", View::Network, false, phone),
            ("inspector-phone", View::Matrix, true, phone),
            ("matrix-desktop", View::Matrix, true, desktop),
            ("entities-desktop", View::Entities, false, desktop),
            ("inspector-editing", View::Entities, true, desktop),
            (
                "inspector-tree",
                View::Entities,
                true,
                Size::new(1280.0, 3200.0),
            ),
            ("matrix-filtered", View::Matrix, false, desktop),
            ("clocks-desktop", View::Clocks, false, desktop),
            ("clocks-phone", View::Clocks, false, phone),
            (
                "media-clock-bench",
                View::Clocks,
                false,
                Size::new(1280.0, 500.0),
            ),
            (
                "inspector-mappings",
                View::Entities,
                true,
                Size::new(1280.0, 2000.0),
            ),
            (
                "inspector-mappings-phone",
                View::Entities,
                true,
                Size::new(360.0, 2400.0),
            ),
            ("settings-general", View::Matrix, false, desktop),
            ("settings-appearance", View::Matrix, false, desktop),
            ("settings-phone", View::Matrix, false, phone),
            (
                "entities-medium",
                View::Entities,
                false,
                Size::new(760.0, 600.0),
            ),
        ] {
            let file = format!("{path}-{suffix}.png");
            for renderer in ["tiny-skia", "wgpu"] {
                let _ = std::fs::remove_file(format!("{path}-{suffix}-{renderer}.png"));
            }
            let settings = Settings {
                interface: Some(interface.name.clone()),
                view,
                inspector,
                ..Settings::default()
            };
            let mut models = models.clone();
            if suffix.contains("mappings") {
                let model = dynamic_mac_mini(&entities, &models);
                models.insert(MAC_MINI, model);
            }
            if suffix.starts_with("clocks") {
                clocked_bench(&entities, &mut models);
            }
            let mut triib = Triib::sample(settings, interface.clone(), entities.clone(), models);
            triib.selected = entities.keys().next().copied();
            if suffix.contains("filtered") {
                triib.settings.matrix_connectable_only = true;
            }
            if suffix.starts_with("settings") {
                triib.settings_open = true;
                if suffix.contains("appearance") || suffix.contains("phone") {
                    triib.settings_tab = crate::settings_view::SettingsTab::Appearance;
                    triib.settings.system_accent = false;
                }
            }
            if suffix.contains("tree") {
                let entity = entities.keys().next().copied().unwrap();
                for descriptor_type in [
                    DescriptorType::CONFIGURATION,
                    DescriptorType::CLOCK_SOURCE,
                    DescriptorType::CLOCK_DOMAIN,
                    DescriptorType::STRINGS,
                    DescriptorType::STREAM_PORT_INPUT,
                ] {
                    triib.tree_open.insert((entity, descriptor_type));
                }
            }
            if suffix.contains("editing") {
                let target = NameTarget {
                    entity: entities.keys().next().copied().unwrap(),
                    descriptor_type: DescriptorType::ENTITY,
                    index: 0,
                    name_index: 0,
                };
                triib.editing = Some((target, "Studio Mac".to_owned()));
            }
            let iced_settings = iced::Settings {
                fonts: scramble_ui::font::files().collect(),
                default_font: scramble_ui::font::TEXT,
                antialiasing: true,
                ..iced::Settings::default()
            };
            let mut simulator =
                iced_test::Simulator::with_size(iced_settings, size, window(&triib));
            let snapshot = simulator.snapshot(&theme).expect("draws");
            assert!(snapshot.matches_image(&file).expect("writes"));
        }
    }

    const HOST_MAC: [u8; 6] = [0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b];

    #[test]
    fn rates_read_naturally() {
        assert_eq!(Rate(SamplingRate(48_000)).to_string(), "48 kHz");
        assert_eq!(Rate(SamplingRate(44_100)).to_string(), "44.1 kHz");
        assert_eq!(
            Rate(SamplingRate(1 << 29 | 48_000)).to_string(),
            "48 kHz × 1/1.001"
        );
    }

    #[test]
    fn edits_send_a_rename_only_when_the_name_changed() {
        let (entities, models) = bench();
        let interface = Interface {
            name: "enp6s0".to_owned(),
            mac: MacAddress(HOST_MAC),
            up: true,
            speed: None,
            physical: true,
            wireless: false,
            hardware_clock: None,
        };
        let mut triib = Triib::sample(Settings::default(), interface, entities, models);
        let mac_mini = EntityId(0xd111_e597_f544_8000);
        let target = NameTarget {
            entity: mac_mini,
            descriptor_type: DescriptorType::ENTITY,
            index: 0,
            name_index: 0,
        };
        let _ = triib.update(Message::EditName(target));
        assert_eq!(triib.editing, Some((target, "Mac mini".to_owned())));
        // Unchanged: nothing to send.
        let _ = triib.update(Message::EditSubmitted);
        assert_eq!(triib.editing, None);
        assert!(triib.pending.is_empty());
        // Longer than an entity holds: not taken.
        let _ = triib.update(Message::EditName(target));
        let _ = triib.update(Message::EditChanged("x".repeat(65)));
        assert_eq!(triib.editing, Some((target, "Mac mini".to_owned())));
        let _ = triib.update(Message::EditChanged("Studio Mac".to_owned()));
        // Selecting another entity drops the edit.
        let _ = triib.update(Message::EntitySelected(EntityId(1)));
        assert_eq!(triib.editing, None);
    }

    #[test]
    fn speeds_read_naturally() {
        assert_eq!(speed_text(100), "100 Mb/s");
        assert_eq!(speed_text(1000), "1 Gb/s");
        assert_eq!(speed_text(2500), "2500 Mb/s");
    }
}
