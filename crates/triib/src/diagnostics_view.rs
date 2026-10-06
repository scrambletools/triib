//! The inspector's diagnostics: how each clock domain and stream has
//! fared since the entity started, from the counters it keeps, with what
//! went wrong marked, and how each bound stream input's reservation
//! stands.

use atdecc::descriptor::DescriptorType;
use atdecc::model::EntityModel;
use iced::Element;
use scramble_ui::component;
use scramble_ui::font::{Type, styled};
use scramble_ui::style;

use crate::app::Message;
use crate::describe;

/// What a line says: what went fine, and what went wrong.
#[derive(Debug, Default, PartialEq)]
struct Facts {
    fine: Vec<String>,
    wrong: Vec<String>,
}

impl Facts {
    fn fine(&mut self, text: String) {
        self.fine.push(text);
    }

    /// A count of something that should not happen, in words, when it
    /// did.
    fn wrong_count(&mut self, count: Option<u32>, words: impl Fn(u32) -> String) {
        if let Some(count) = count.filter(|&count| count > 0) {
            self.wrong.push(words(count));
        }
    }

    fn is_empty(&self) -> bool {
        self.fine.is_empty() && self.wrong.is_empty()
    }
}

/// A count with thousands set apart, as 1,204,331.
fn number(count: u32) -> String {
    let digits = count.to_string();
    let mut text = String::new();
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// How often something happened: "not stopped", "stopped once",
/// "stopped twice" or "stopped 3 times".
fn happened(done: &str, count: u32) -> String {
    match count {
        0 => format!("not {done}"),
        1 => format!("{done} once"),
        2 => format!("{done} twice"),
        _ => format!("{done} {} times", number(count)),
    }
}

/// A count of things, singular or plural: "1 frame", "12 frames".
fn things(count: u32, one: &str, many: &str) -> String {
    format!("{} {}", number(count), if count == 1 { one } else { many })
}

/// The diagnostics section, or nothing when the entity reports no
/// counters and no reservations.
pub fn diagnostics<'a>(model: &EntityModel) -> Vec<Element<'a, Message>> {
    let mut items = Vec::new();
    for domain in model.clock_domains() {
        let Some(counters) = model.clock_domain_counters(domain.index) else {
            continue;
        };
        let mut facts = Facts::default();
        if let Some(locked) = counters.locked {
            facts.fine(happened("locked", locked));
        }
        facts.wrong_count(counters.unlocked, |count| happened("lost lock", count));
        let name = model
            .name_of(DescriptorType::CLOCK_DOMAIN, domain.index)
            .unwrap_or("Clock domain");
        items.extend(item(name, facts));
    }
    for stream in model.streams(true) {
        let mut facts = Facts::default();
        if let Some(counters) = model.stream_input_counters(stream.index) {
            if let Some(frames) = counters.frames_rx {
                facts.fine(format!("{} in", things(frames, "frame", "frames")));
            }
            if let Some(locked) = counters.media_locked {
                facts.fine(happened("media locked", locked));
            }
            facts.wrong_count(counters.media_unlocked, |count| {
                happened("lost media lock", count)
            });
            facts.wrong_count(counters.stream_interrupted, |count| {
                happened("interrupted", count)
            });
            facts.wrong_count(counters.seq_num_mismatch, |count| {
                format!("{} out of sequence", things(count, "frame", "frames"))
            });
            facts.wrong_count(counters.media_reset, |count| {
                things(count, "media reset", "media resets")
            });
            facts.wrong_count(counters.timestamp_uncertain, |count| {
                happened("timestamps uncertain", count)
            });
            facts.wrong_count(counters.timestamp_not_valid, |count| {
                format!("{} without a timestamp", things(count, "frame", "frames"))
            });
            facts.wrong_count(counters.unsupported_format, |count| {
                format!(
                    "{} in an unsupported format",
                    things(count, "frame", "frames")
                )
            });
            facts.wrong_count(counters.late_timestamp, |count| {
                format!("{} late", things(count, "frame", "frames"))
            });
            facts.wrong_count(counters.early_timestamp, |count| {
                format!("{} early", things(count, "frame", "frames"))
            });
        }
        if let Some(info) = model.stream_info(DescriptorType::STREAM_INPUT, stream.index)
            && info.bound()
        {
            if info.talker_failed() {
                facts.wrong.push(format!(
                    "the talker's reservation failed: {}",
                    describe::reservation_failure(info)
                ));
            } else if info.registering() {
                facts.fine(format!(
                    "{} µs accumulated latency",
                    number(info.msrp_accumulated_latency / 1000)
                ));
            }
        }
        let name = model
            .name_of(DescriptorType::STREAM_INPUT, stream.index)
            .unwrap_or("Stream input");
        items.extend(item(name, facts));
    }
    for stream in model.streams(false) {
        let Some(counters) = model.stream_output_counters(stream.index) else {
            continue;
        };
        let mut facts = Facts::default();
        if let Some(frames) = counters.frames_tx {
            facts.fine(format!("{} out", things(frames, "frame", "frames")));
        }
        if let Some(started) = counters.stream_start {
            facts.fine(happened("started", started));
        }
        if let Some(stopped) = counters.stream_stop {
            facts.fine(happened("stopped", stopped));
        }
        facts.wrong_count(counters.stream_interrupted, |count| {
            happened("interrupted", count)
        });
        facts.wrong_count(counters.media_reset, |count| {
            things(count, "media reset", "media resets")
        });
        facts.wrong_count(counters.timestamp_uncertain, |count| {
            happened("timestamps uncertain", count)
        });
        facts.wrong_count(counters.timestamp_not_valid, |count| {
            format!("{} without a timestamp", things(count, "frame", "frames"))
        });
        let name = model
            .name_of(DescriptorType::STREAM_OUTPUT, stream.index)
            .unwrap_or("Stream output");
        items.extend(item(name, facts));
    }
    if items.is_empty() {
        return items;
    }
    let mut section = vec![
        component::section("Diagnostics"),
        styled("Counted since the entity started.", Type::BodySmall)
            .style(style::on_surface_variant)
            .into(),
    ];
    section.extend(items);
    section
}

/// A name with what went fine under it, then what went wrong, marked.
fn item<'a>(name: &str, facts: Facts) -> Option<Element<'a, Message>> {
    if facts.is_empty() {
        return None;
    }
    let mut lines = iced::widget::column![styled(name.to_owned(), Type::BodyMedium)].spacing(2);
    if !facts.fine.is_empty() {
        lines = lines.push(
            styled(capitalized(&facts.fine.join(", ")), Type::BodySmall)
                .style(style::on_surface_variant),
        );
    }
    if !facts.wrong.is_empty() {
        lines = lines.push(
            styled(capitalized(&facts.wrong.join(", ")), Type::BodySmall).style(style::error_text),
        );
    }
    Some(lines.into())
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_read_naturally() {
        assert_eq!(number(7), "7");
        assert_eq!(number(1204), "1,204");
        assert_eq!(number(1_204_331), "1,204,331");
        assert_eq!(happened("stopped", 0), "not stopped");
        assert_eq!(happened("locked", 1), "locked once");
        assert_eq!(happened("locked", 2), "locked twice");
        assert_eq!(happened("interrupted", 1000), "interrupted 1,000 times");
        assert_eq!(things(1, "frame", "frames"), "1 frame");
        assert_eq!(things(12, "frame", "frames"), "12 frames");
        assert_eq!(capitalized("lost lock 2 times"), "Lost lock 2 times");
    }

    #[test]
    fn only_what_happened_is_wrong() {
        let mut facts = Facts::default();
        facts.wrong_count(Some(0), |count| format!("{count} late"));
        facts.wrong_count(None, |count| format!("{count} early"));
        assert!(facts.is_empty());
        facts.wrong_count(Some(3), |count| format!("{count} late"));
        assert_eq!(facts.wrong, ["3 late"]);
    }
}
