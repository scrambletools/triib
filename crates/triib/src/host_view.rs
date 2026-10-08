//! This computer's own talkers and listeners in the views: the Entities
//! view's bar that adds them, and their section in the inspector, with
//! their audio, channels and a button that removes them.

use std::fmt;

use atdecc::EntityId;
use iced::widget::{container, space};
use iced::{Center, Element, Fill};
use scramble_ui::button::{self, Kind, Size};
use scramble_ui::font::{Type, styled};
use scramble_ui::icon::Icon;
use scramble_ui::{component, dir, dropdown, style};
use triib_endpointd::config::Kind as EndpointKind;

use crate::app::{Message, Triib};
use crate::fl;
use crate::view::{property, stacked};

/// The bar above the Entities view's table: why this computer cannot add
/// talkers and listeners, or the buttons that add them.
pub fn bar(triib: &Triib) -> Option<Element<'_, Message>> {
    if !crate::host::SUPPORTED || triib.interface().is_none() {
        return None;
    }
    let able = triib.hosts_endpoints();
    let note = if !able {
        Some(fl!("host-needs-clock"))
    } else if triib
        .endpoints
        .as_ref()
        .is_some_and(|status| status.gptp.starts_with("ptp4l"))
    {
        Some(fl!("host-no-ptp4l"))
    } else {
        None
    };
    let add = |kind, label: String| {
        button::with_icon(Kind::Tonal, Icon::Add, label)
            .size(Size::ExtraSmall)
            .on_press_maybe(able.then_some(Message::HostAdded(kind)))
    };
    let text: Element<'_, Message> = match note {
        Some(note) => styled(note, Type::BodySmall)
            .style(style::on_surface_variant)
            .width(Fill)
            .align_x(dir::text_start())
            .into(),
        None => space().width(Fill).into(),
    };
    Some(
        container(
            scramble_ui::row![
                text,
                component::toggle_tool(
                    Icon::Computer,
                    if triib.settings.host_entities_only {
                        fl!("host-show-all")
                    } else {
                        fl!("host-show-mine")
                    },
                    triib.settings.host_entities_only,
                    Message::HostEntitiesOnlyToggled,
                ),
                add(EndpointKind::Talker, fl!("host-add-talker")),
                add(EndpointKind::Listener, fl!("host-add-listener")),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([8, 16])
        .into(),
    )
}

/// The channel counts the inspector offers; six samples of each fit in a
/// frame up to 60.
const CHANNEL_COUNTS: [u16; 11] = [1, 2, 4, 6, 8, 12, 16, 24, 32, 48, 56];

/// One choice for an endpoint's audio: what endpoints.toml says, and the
/// words for it.
#[derive(Debug, Clone)]
struct Audio {
    spec: String,
    label: String,
}

impl PartialEq for Audio {
    fn eq(&self, other: &Self) -> bool {
        self.spec == other.spec
    }
}

impl fmt::Display for Audio {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

/// The inspector's section for one of this computer's endpoints.
pub fn sections(triib: &Triib, entity_id: EntityId) -> Vec<Element<'_, Message>> {
    let Some(endpoint) = crate::host::status_of(triib.endpoints.as_ref(), entity_id) else {
        return Vec::new();
    };
    let state = match endpoint.state.as_str() {
        "streaming" => fl!("host-streaming"),
        "waiting" => fl!("host-waiting"),
        "listening" => fl!("host-listening"),
        "bound" => fl!("host-bound"),
        _ => fl!("host-unbound"),
    };
    let audio = |spec: &str, label: String| Audio {
        spec: spec.to_owned(),
        label,
    };
    let (label, mut choices, devices) = match endpoint.kind {
        EndpointKind::Talker => (
            fl!("host-audio-from"),
            vec![
                audio("silence", fl!("host-silence")),
                audio("tone", fl!("host-tone")),
            ],
            &triib.audio_devices.0,
        ),
        EndpointKind::Listener => (
            fl!("host-audio-to"),
            vec![audio("discard", fl!("host-nowhere"))],
            &triib.audio_devices.1,
        ),
    };
    choices.push(audio("default", fl!("host-default-device")));
    choices.extend(
        devices
            .iter()
            .filter(|name| name.as_str() != "default")
            .map(|name| audio(name, name.clone())),
    );
    let current = choices
        .iter()
        .find(|choice| choice.spec == endpoint.audio)
        .cloned()
        .unwrap_or_else(|| audio(&endpoint.audio, endpoint.audio.clone()));
    let picker: Element<'_, Message> = {
        let _reading = dir::reading();
        dropdown::pick(choices, Some(current), move |choice: Audio| {
            Message::HostAudio(entity_id, choice.spec)
        })
        .size(button::Size::ExtraSmall)
        .width(Fill)
        .into()
    };
    let channels: Element<'_, Message> = {
        let mut counts = CHANNEL_COUNTS.to_vec();
        if !counts.contains(&endpoint.channels) {
            counts.push(endpoint.channels);
            counts.sort_unstable();
        }
        let _reading = dir::reading();
        dropdown::pick(counts, Some(endpoint.channels), move |count| {
            Message::HostChannels(entity_id, count)
        })
        .size(button::Size::ExtraSmall)
        .width(Fill)
        .into()
    };
    vec![
        component::section(fl!("netmap-this-computer")),
        property(fl!("host-state"), state),
        stacked(label, picker),
        stacked(fl!("host-channels"), channels),
        button::with_icon(Kind::Outlined, Icon::Delete, fl!("host-remove"))
            .size(Size::ExtraSmall)
            .on_press(Message::HostRemoved(entity_id))
            .into(),
    ]
}
