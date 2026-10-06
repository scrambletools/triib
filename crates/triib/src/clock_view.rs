//! The media clock view: each media clock reference with the clock
//! domains following it, as a tree through the streams that carry its
//! clock, then the domains whose chain breaks and why. Each domain's clock
//! source can be picked where it shows.

use std::collections::BTreeMap;

use atdecc::EntityId;
use atdecc::descriptor::DescriptorType;
use atdecc::media_clock::{Broken, ClockFrom, DomainClock, DomainId, media_clocks};
use atdecc::model::EntityModel;
use iced::widget::{column, container, mouse_area, row, scrollable, space};
use iced::{Center, Element, Fill, Length, Theme, mouse};
use scramble_ui::component;
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::Icon;
use scramble_ui::{Scheme, shape, style};

use crate::app::{Message, Triib};
use crate::view::{Rate, source_picker};

/// From this width the pickers sit beside what they set, and the title
/// shows.
const WIDE: f32 = 720.0;
/// The widest the cards get, so lines stay readable on wide windows.
const CONTENT_WIDTH: f32 = 880.0;
/// How far each stream further from the reference indents.
const INDENT: f32 = 20.0;
const PICKER_WIDTH: f32 = 280.0;

pub fn view(triib: &Triib) -> Element<'_, Message> {
    iced::widget::responsive(move |size| view_at(triib, size.width)).into()
}

fn view_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    let wide = width >= WIDE;
    let padding = if wide { [16, 24] } else { [12, 12] };
    let clocks = media_clocks(triib.models.iter().map(|(&entity, model)| (entity, model)));
    let references: Vec<&DomainClock> = {
        let mut references: Vec<&DomainClock> = clocks
            .iter()
            .filter(|clock| clock.reference == Ok(clock.id))
            .collect();
        // The largest domains first, then by name.
        references.sort_by_key(|reference| {
            let followers = clocks
                .iter()
                .filter(|clock| clock.reference == Ok(reference.id))
                .count();
            (
                std::cmp::Reverse(followers),
                domain_name(triib, reference.id),
            )
        });
        references
    };
    let broken: Vec<&DomainClock> = clocks
        .iter()
        .filter(|clock| clock.reference.is_err())
        .collect();
    let following = clocks.len() - references.len() - broken.len();

    let mut header = row![].spacing(12).align_y(Center);
    if wide {
        header = header.push(styled("Media clock", Type::TitleLarge));
    }
    header = header.push(
        styled(
            summary(references.len(), following, broken.len()),
            Type::BodyMedium,
        )
        .style(style::on_surface_variant),
    );
    if clocks.is_empty() {
        return column![
            header,
            component::empty_state(
                Icon::Timer,
                "No media clocks yet",
                "Each entity's clock domains appear here once read, under the media clock \
                 reference they follow.",
            )
        ]
        .spacing(16)
        .padding(padding)
        .into();
    }

    let mut cards = column![].spacing(16).width(Fill).max_width(CONTENT_WIDTH);
    for reference in references {
        cards = cards.push(reference_card(triib, &clocks, reference, wide));
    }
    if !broken.is_empty() {
        cards = cards.push(broken_card(triib, &broken, wide));
    }
    column![
        header,
        scrollable(container(cards).center_x(Fill))
            .style(style::scrollbar)
            .height(Fill)
    ]
    .spacing(16)
    .padding(padding)
    .width(Fill)
    .height(Fill)
    .into()
}

fn summary(references: usize, following: usize, broken: usize) -> String {
    let plural = |count: usize, one: &str, many: &str| {
        format!("{count} {}", if count == 1 { one } else { many })
    };
    let mut parts = vec![
        plural(references, "reference", "references"),
        format!("{following} following"),
    ];
    if broken > 0 {
        parts.push(format!("{broken} not clocked"));
    }
    parts.join(", ")
}

/// An entity's name, with its entity ID when another entity has the same
/// name, and its clock domain's when it has more than one.
fn domain_name(triib: &Triib, id: DomainId) -> String {
    let mut entity = triib.entity_name_of(id.entity);
    let shared = triib
        .entities
        .keys()
        .any(|&other| other != id.entity && triib.entity_name_of(other) == entity);
    if shared {
        entity = format!("{entity} ({})", id.entity);
    }
    let Some(model) = triib.models.get(&id.entity) else {
        return entity;
    };
    if model.clock_domains().count() > 1 {
        let domain = model
            .name_of(DescriptorType::CLOCK_DOMAIN, id.domain)
            .map_or_else(|| format!("Clock domain {}", id.domain + 1), str::to_owned);
        format!("{entity}, {domain}")
    } else {
        entity
    }
}

/// A card with a reference on top and the domains following it below.
fn reference_card<'a>(
    triib: &'a Triib,
    clocks: &[DomainClock],
    reference: &DomainClock,
    wide: bool,
) -> Element<'a, Message> {
    let rate = triib
        .models
        .get(&reference.id.entity)
        .and_then(|model| model.sampling_rate(reference.id.domain));
    let mut body = column![
        styled("REFERENCE", Type::LabelSmall).style(style::on_surface_variant),
        domain_row(triib, reference, None, wide),
    ]
    .spacing(8);
    let mut followers = Vec::new();
    tree(clocks, reference.id, &mut followers);
    if !followers.is_empty() {
        body = body.push(space().height(4));
        body = body.push(
            styled(format!("FOLLOWING · {}", followers.len()), Type::LabelSmall)
                .style(style::on_surface_variant),
        );
        for follower in followers {
            body = body.push(domain_row(triib, follower, rate, wide));
        }
    }
    card(body)
}

/// The domains clocked from `parent`, each before those clocked from it.
fn tree<'c>(clocks: &'c [DomainClock], parent: DomainId, out: &mut Vec<&'c DomainClock>) {
    for clock in clocks
        .iter()
        .filter(|clock| clock.parent == Some(parent) && clock.reference.is_ok())
    {
        out.push(clock);
        tree(clocks, clock.id, out);
    }
}

/// A card listing the domains that follow no reference, each with why.
fn broken_card<'a>(triib: &'a Triib, broken: &[&DomainClock], wide: bool) -> Element<'a, Message> {
    let mut body =
        column![styled("NOT CLOCKED", Type::LabelSmall).style(style::error_text)].spacing(8);
    for clock in broken {
        body = body.push(domain_row(triib, clock, None, wide));
    }
    card(body)
}

fn card<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body)
        .padding(16)
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Scheme::of(theme).surface_container_low.into()),
            border: iced::border::rounded(shape::LARGE),
            ..container::Style::default()
        })
        .into()
}

/// One domain: its name, which selects the entity, where its clock comes
/// from and how that is going, and a picker of its clock source.
fn domain_row<'a>(
    triib: &'a Triib,
    clock: &DomainClock,
    reference_rate: Option<atdecc::descriptor::SamplingRate>,
    wide: bool,
) -> Element<'a, Message> {
    let models = &triib.models;
    let model = models.get(&clock.id.entity);
    let name = mouse_area(
        styled(domain_name(triib, clock.id), Type::BodyLarge).style(style::primary_text),
    )
    .interaction(mouse::Interaction::Pointer)
    .on_press(Message::EntitySelected(clock.id.entity));
    let mut facts = column![].spacing(2);
    for (text, trouble) in facts_of(triib, models, clock, reference_rate) {
        facts = facts.push(styled(text, Type::BodySmall).style(if trouble {
            style::error_text
        } else {
            style::on_surface_variant
        }));
    }
    let picker: Option<Element<'a, Message>> = model.and_then(|model| {
        let domain = model
            .clock_domains()
            .find(|domain| domain.index == clock.id.domain)?;
        Some(
            container(source_picker(triib, clock.id.entity, model, &domain))
                .width(Length::Fixed(PICKER_WIDTH))
                .into(),
        )
    });
    let text = column![name, facts].spacing(2).width(Fill);
    let line: Element<'a, Message> = match picker {
        Some(picker) if wide => row![text, picker].spacing(12).align_y(Center).into(),
        Some(picker) => column![text, picker].spacing(6).into(),
        None => text.into(),
    };
    let depth = clock.hops.saturating_sub(1) as f32;
    row![space().width(Length::Fixed(depth * INDENT)), line]
        .width(Fill)
        .into()
}

/// What a domain's line says under its name, each marked when it is
/// trouble: where its clock comes from, whether that stream flows, its
/// rate against the reference's, and its media clock reference priority.
fn facts_of(
    triib: &Triib,
    models: &BTreeMap<EntityId, EntityModel>,
    clock: &DomainClock,
    reference_rate: Option<atdecc::descriptor::SamplingRate>,
) -> Vec<(String, bool)> {
    let mut facts = Vec::new();
    let Some(model) = models.get(&clock.id.entity) else {
        return facts;
    };
    let source = model
        .clock_domains()
        .find(|domain| domain.index == clock.id.domain)
        .map(|domain| domain.clock_source_index);
    let source_name = source
        .and_then(|index| model.name_of(DescriptorType::CLOCK_SOURCE, index))
        .map(|name| format!(", {name}"))
        .unwrap_or_default();
    match clock.from {
        ClockFrom::Internal => facts.push((format!("Own clock{source_name}"), false)),
        ClockFrom::External => facts.push((format!("External clock{source_name}"), false)),
        ClockFrom::Stream {
            input,
            talker,
            output,
        } => {
            facts.push((
                format!(
                    "From {}, fed by {}",
                    model
                        .name_of(DescriptorType::STREAM_INPUT, input)
                        .unwrap_or("a stream input"),
                    triib.stream_name(talker, DescriptorType::STREAM_OUTPUT, output),
                ),
                false,
            ));
            let flowing = model
                .stream_info(DescriptorType::STREAM_INPUT, input)
                .is_some_and(|info| info.settled());
            if !flowing {
                facts.push(("The stream is not flowing".to_owned(), true));
            }
        }
        ClockFrom::Unbound { input } => facts.push((
            format!(
                "Clocked from {}, which is not connected",
                model
                    .name_of(DescriptorType::STREAM_INPUT, input)
                    .unwrap_or("a stream input"),
            ),
            true,
        )),
        ClockFrom::Unknown => facts.push(("Clock source not known yet".to_owned(), true)),
    }
    if let Err(broken) = clock.reference {
        let upstream = |at: DomainId| (at != clock.id).then(|| domain_name(triib, at));
        let reason = match broken {
            Broken::Unbound(at) => {
                upstream(at).map(|name| format!("{name}, upstream, is not connected"))
            }
            Broken::TalkerUnknown(at) => Some(match upstream(at) {
                Some(name) => format!("The talker feeding {name} is not known"),
                None => "The talker feeding it is not known".to_owned(),
            }),
            Broken::Unknown(at) => {
                upstream(at).map(|name| format!("The clock source of {name} is not known"))
            }
            Broken::Loop(_) => Some("Its clock goes round in a loop".to_owned()),
        };
        facts.extend(reason.map(|reason| (reason, true)));
    }
    let rate = model.sampling_rate(clock.id.domain);
    match (rate, reference_rate) {
        (Some(rate), Some(reference)) if rate != reference => facts.push((
            format!("{}, not the reference's {}", Rate(rate), Rate(reference)),
            true,
        )),
        (Some(rate), _) => facts.push((Rate(rate).to_string(), false)),
        (None, _) => {}
    }
    if let Some(reference) = model.media_clock_reference(clock.id.domain) {
        let domain = reference
            .domain_name()
            .map(|name| format!(", domain {name}"))
            .unwrap_or_default();
        facts.push((format!("Priority {}{domain}", reference.priority()), false));
    }
    facts
}
