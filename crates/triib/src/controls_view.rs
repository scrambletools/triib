//! The inspector's controls: each of an entity's controls with what sets
//! it, a slider for a level, a switch for one that is on or off and a
//! picker for a selector, or its value alone when it only reports.

use std::fmt;

use atdecc::EntityId;
use atdecc::control::{
    ControlDescriptor, ControlType, Linear, Number, Scalar, Selector, Shape, decode_values,
    encode_values,
};
use atdecc::descriptor::{DescriptorType, LocalizedStringRef};
use atdecc::model::EntityModel;
use iced::widget::{slider, toggler};
use iced::{Center, Element, Fill, Length, Theme};
use scramble_ui::component::{self, Backdrop};
use scramble_ui::font::{Type, styled};
use scramble_ui::{Scheme, button, dropdown, style};

use crate::app::{Message, Triib};
use crate::network::{Action, ControlValues};
use crate::view::stacked;

/// The room a slider's value takes beside it.
const VALUE_WIDTH: f32 = 84.0;

/// A slider being dragged: the control, which of its values, and where
/// the slider is, in the value's unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlDrag {
    pub entity: EntityId,
    pub index: u16,
    pub value: usize,
    pub shown: f64,
}

/// The controls section, or nothing when the entity has no control to
/// show; an identify control the Identify button stands for is left out.
pub fn controls<'a>(
    triib: &'a Triib,
    entity: EntityId,
    model: &'a EntityModel,
) -> Vec<Element<'a, Message>> {
    let identify_button = triib.entities.get(&entity).is_some_and(|found| {
        found
            .adp
            .entity_capabilities
            .contains(atdecc::EntityCapabilities::AEM_IDENTIFY_CONTROL_INDEX_VALID)
    });
    let shown: Vec<ControlDescriptor<'_>> = model
        .controls()
        .filter(|control| !(identify_button && control.control_type == ControlType::IDENTIFY))
        .collect();
    if shown.is_empty() {
        return Vec::new();
    }
    let mut items = vec![component::section("Controls")];
    for control in shown {
        let name = control_name(model, &control);
        items.push(stacked(name, editor(triib, entity, model, &control)));
    }
    items
}

/// The control's name: the one it holds, else its kind.
fn control_name(model: &EntityModel, control: &ControlDescriptor<'_>) -> String {
    if let Some(name) = model.name_of(DescriptorType::CONTROL, control.index) {
        return name.to_owned();
    }
    control.control_type.name().map_or_else(
        || format!("Control {}", control.index),
        |kind| {
            let words = kind.to_lowercase().replace('_', " ");
            let mut letters = words.chars();
            letters
                .next()
                .map(|first| first.to_uppercase().chain(letters).collect())
                .unwrap_or_default()
        },
    )
}

/// What sets the control, value by value, or shows what it reports.
fn editor<'a>(
    triib: &'a Triib,
    entity: EntityId,
    model: &'a EntityModel,
    control: &ControlDescriptor<'_>,
) -> Element<'a, Message> {
    let index = control.index;
    let pending = pending_values(triib, entity, control);
    let busy = pending.is_some();
    let current: Vec<Number> = pending.unwrap_or_else(|| control.current().collect());
    match (control.value_type.shape(), control.value_type.scalar()) {
        (Shape::Linear, Some(scalar)) => {
            let values: Vec<Linear> = control.linear().collect();
            let mut lines = iced::widget::column![].spacing(4);
            for (place, value) in values.iter().enumerate() {
                let now = current.get(place).copied().unwrap_or(value.current);
                let label = model
                    .localized(value.string)
                    .filter(|_| values.len() > 1)
                    .map(str::to_owned);
                let line = if control.read_only {
                    reading(value.unit.show(now).to_string())
                } else if on_off(value) {
                    let on = now.to_f64() != value.minimum.to_f64();
                    let target = if on { value.minimum } else { value.maximum };
                    let toggled = set_message(entity, index, scalar, &current, place, target)
                        .filter(|_| !busy)
                        .map(|message| move |_: bool| message.clone());
                    toggler(on)
                        .on_toggle_maybe(toggled)
                        .size(24)
                        .style(style::switch)
                        .into()
                } else {
                    let place = (index, place);
                    level(triib, entity, place, value, scalar, now, busy)
                };
                lines = lines.push(match label {
                    Some(label) => stacked(label, line),
                    None => line,
                });
            }
            lines.into()
        }
        (Shape::Selector, Some(scalar)) if !control.read_only => {
            let Some(selector) = control.selector() else {
                return reading(String::new());
            };
            let options: Vec<Choice> = selector
                .options()
                .map(|option| Choice::new(model, &selector, option))
                .collect();
            let now = current.first().copied().unwrap_or(selector.current);
            let picked = Choice::new(model, &selector, now);
            dropdown::pick(options, Some(picked), move |choice: Choice| {
                if busy {
                    return Message::Nothing;
                }
                set_message(entity, index, scalar, &[choice.number], 0, choice.number)
                    .unwrap_or(Message::Nothing)
            })
            .size(button::Size::ExtraSmall)
            .width(Fill)
            .into()
        }
        (Shape::Selector, _) => {
            let text = control.selector().map_or_else(String::new, |selector| {
                Choice::new(model, &selector, selector.current).label
            });
            reading(text)
        }
        (Shape::Array, _) => {
            let text = control.array().map_or_else(String::new, |array| {
                array
                    .current()
                    .map(|value| array.unit.show(value).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            reading(text)
        }
        (Shape::Utf8, _) => reading(control.text().unwrap_or_default().to_owned()),
        _ => reading("Not shown here".to_owned()),
    }
}

/// A value the control only reports.
fn reading<'a>(text: String) -> Element<'a, Message> {
    styled(text, Type::BodyMedium).into()
}

/// Whether a linear value only goes between off and on, as a mute or an
/// identify control does: one step from its minimum to its maximum.
fn on_off(value: &Linear) -> bool {
    let (low, high) = (value.minimum.to_f64(), value.maximum.to_f64());
    high > low && value.step.to_f64() == high - low
}

/// A slider over a linear value's range in its unit, with the value
/// beside it; the value is sent once the slider is let go.
fn level<'a>(
    triib: &'a Triib,
    entity: EntityId,
    (index, place): (u16, usize),
    value: &Linear,
    scalar: Scalar,
    now: Number,
    busy: bool,
) -> Element<'a, Message> {
    let scale = value.unit.scale();
    let (low, high) = (
        value.minimum.to_f64() * scale,
        value.maximum.to_f64() * scale,
    );
    let (low, high) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    let step = match value.step.to_f64() * scale {
        step if step > 0.0 => step,
        _ => (high - low) / 100.0,
    };
    let dragged = triib
        .control_drag
        .filter(|drag| (drag.entity, drag.index, drag.value) == (entity, index, place));
    let shown = dragged.map_or(now.to_f64() * scale, |drag| drag.shown);
    let text = match dragged {
        Some(drag) => value
            .unit
            .show(value.nearest(value.unit.raw(drag.shown, scalar), scalar)),
        None => value.unit.show(now),
    };
    let rail = slider(low..=high, shown.clamp(low, high), move |shown| {
        if busy {
            Message::Nothing
        } else {
            Message::ControlDragged(ControlDrag {
                entity,
                index,
                value: place,
                shown,
            })
        }
    })
    .step(step)
    .on_release(Message::ControlReleased)
    .height(style::SLIDER_HEIGHT)
    .style(|theme: &Theme, status| {
        style::slider(Backdrop::ContainerLow.color(&Scheme::of(theme)))(theme, status)
    });
    iced::widget::row![
        rail,
        styled(text.to_string(), Type::BodyMedium)
            .style(style::on_surface_variant)
            .width(Length::Fixed(VALUE_WIDTH))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

/// The message setting value `place` of the control to `number`, the
/// others kept as they are.
fn set_message(
    entity: EntityId,
    index: u16,
    scalar: Scalar,
    current: &[Number],
    place: usize,
    number: Number,
) -> Option<Message> {
    let mut numbers = current.to_vec();
    *numbers.get_mut(place)? = number;
    let mut octets = [0; atdecc::aem::MAX_CONTROL_VALUES];
    let length = encode_values(scalar, numbers, &mut octets)?;
    let values = ControlValues::new(&octets[..length])?;
    Some(Message::Act(Action::SetControl {
        entity,
        index,
        values,
    }))
}

/// The message a let-go slider sends: its value, nearest a setting the
/// control takes.
pub fn released(triib: &Triib, drag: ControlDrag) -> Option<Message> {
    let control = triib.models.get(&drag.entity)?.control(drag.index)?;
    let scalar = control.value_type.scalar()?;
    let value = control.linear().nth(drag.value)?;
    let current: Vec<Number> = control.current().collect();
    let number = value.nearest(value.unit.raw(drag.shown, scalar), scalar);
    if current.get(drag.value) == Some(&number) {
        return None;
    }
    set_message(
        drag.entity,
        drag.index,
        scalar,
        &current,
        drag.value,
        number,
    )
}

/// The values a change on its way sets the control to.
fn pending_values(
    triib: &Triib,
    entity: EntityId,
    control: &ControlDescriptor<'_>,
) -> Option<Vec<Number>> {
    let scalar = control.value_type.scalar()?;
    let action = triib.changing(|action| {
        matches!(action, Action::SetControl { entity: pending, index, .. }
            if *pending == entity && *index == control.index)
    })?;
    match action {
        Action::SetControl { values, .. } => {
            Some(decode_values(scalar, values.as_slice()).collect())
        }
        _ => None,
    }
}

/// A selector's option, by its number, named for the picker.
#[derive(Debug, Clone)]
struct Choice {
    number: Number,
    label: String,
}

impl Choice {
    fn new(model: &EntityModel, selector: &Selector<'_>, number: Number) -> Self {
        let label = if selector.strings {
            model
                .localized(LocalizedStringRef(number.to_f64() as u16))
                .map_or_else(|| format!("Option {}", number.to_f64()), str::to_owned)
        } else {
            selector.unit.show(number).to_string()
        };
        Self { number, label }
    }
}

impl PartialEq for Choice {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRED_ESP: EntityId = EntityId(0xe8f6_0ae0_9220_0000);

    fn sample() -> Triib {
        let (entities, models) = crate::view::tests::bench();
        let interface = avb_net::Interface {
            name: "enp6s0".to_owned(),
            mac: avb_net::MacAddress([0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b]),
            up: true,
            speed: None,
            physical: true,
            wireless: false,
            hardware_clock: None,
        };
        let settings = crate::settings::Settings::default();
        Triib::sample(settings, interface, entities, models)
    }

    /// The wired endpoint's speaker volume: -95.5 dB to 32 dB in half
    /// decibels, as tenths of a decibel.
    fn volume(triib: &Triib) -> ControlDescriptor<'_> {
        let model = &triib.models[&WIRED_ESP];
        model
            .controls()
            .find(|control| control_name(model, control) == "Speaker Volume")
            .unwrap()
    }

    #[test]
    fn a_let_go_slider_sends_the_nearest_setting() {
        let triib = sample();
        let index = volume(&triib).index;
        let drag = |shown| ControlDrag {
            entity: WIRED_ESP,
            index,
            value: 0,
            shown,
        };
        // -12.3 dB goes to -12.5 dB, the nearest half decibel.
        let Some(Message::Act(Action::SetControl {
            entity,
            index: set,
            values,
        })) = released(&triib, drag(-12.3))
        else {
            panic!("a change to send");
        };
        assert_eq!((entity, set), (WIRED_ESP, index));
        assert_eq!(values.as_slice(), (-125i16).to_be_bytes());
        // Let go where it already is: nothing to send.
        let now = volume(&triib).current().next().unwrap().to_f64() / 10.0;
        assert!(released(&triib, drag(now)).is_none());
    }

    #[test]
    fn switches_stand_for_values_that_are_off_or_on() {
        let triib = sample();
        let model = &triib.models[&WIRED_ESP];
        let identify = model
            .controls()
            .find(|control| control.control_type == ControlType::IDENTIFY)
            .and_then(|control| control.linear().next())
            .unwrap();
        assert!(on_off(&identify), "0 to 255 in one step");
        assert!(!on_off(&volume(&triib).linear().next().unwrap()));
    }

    #[test]
    fn the_identify_button_stands_for_the_identify_control() {
        let triib = sample();
        let model = &triib.models[&WIRED_ESP];
        // The section's heading, then volume and gain without identify.
        assert_eq!(controls(&triib, WIRED_ESP, model).len(), 3);
        assert_eq!(model.controls().count(), 3);
    }
}
