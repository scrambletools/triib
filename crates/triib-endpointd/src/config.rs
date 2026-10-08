//! The endpoints to run, from `endpoints.toml` in triib's data folder:
//! the interface, then each talker or listener with its channels and
//! where its audio comes from or goes.

use std::path::PathBuf;

use atdecc::EntityId;
use atdecc::stream_format::StreamFormat;
use serde::{Deserialize, Serialize};
use triib_stream::audio::{Sink, Source};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// The interface the entities are on: wired, with a PTP hardware clock
    /// that ptp4l keeps on gPTP time.
    pub interface: String,
    /// ptp4l's read-only management socket.
    pub ptp4l_socket: String,
    /// Whether the endpoints may fall back to AVB Lite.
    pub avb_lite: LiteChoice,
    #[serde(rename = "endpoint")]
    pub endpoints: Vec<EndpointConfig>,
}

/// When the endpoints run AVB Lite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiteChoice {
    /// When nothing on the interface answers as an AVB bridge would
    /// (AVB Lite profile, 2.2).
    #[default]
    Auto,
    /// Never.
    Off,
    /// Always, as an operator may set it.
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Talker,
    Listener,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub kind: Kind,
    /// Kept apart from the others' to keep its entity ID stable, from 0 to
    /// 239; its place in the list when left out.
    #[serde(default)]
    pub instance: Option<u8>,
    pub name: String,
    #[serde(default = "eight")]
    pub channels: u16,
    /// A talker's audio: "silence", "tone", "default" for the default
    /// input, or an input's name.
    #[serde(default)]
    pub source: Option<String>,
    /// A listener's audio: "discard", "default" for the default output,
    /// or an output's name.
    #[serde(default)]
    pub sink: Option<String>,
    /// The device channel the stream's first channel goes to or comes from.
    #[serde(default)]
    pub first_channel: u16,
    /// The stream format a controller last chose, in hex; the first one
    /// offered when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The talker's stream output a listener is bound to, as
    /// `<entity ID>:<output>`, bound again when it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<String>,
}

/// A binding as `bound` keeps it.
pub fn binding_text(talker: EntityId, output: u16) -> String {
    format!("{talker}:{output}")
}

fn eight() -> u16 {
    8
}

impl EndpointConfig {
    /// The talker and stream output a listener is bound to.
    pub fn bound_to(&self) -> Option<(EntityId, u16)> {
        let (talker, output) = self.bound.as_deref()?.rsplit_once(':')?;
        Some((talker.parse().ok()?, output.parse().ok()?))
    }

    /// The stream format a controller last chose.
    pub fn stream_format(&self) -> Option<StreamFormat> {
        let text = self.format.as_deref()?;
        u64::from_str_radix(text.trim_start_matches("0x"), 16)
            .ok()
            .map(StreamFormat)
    }

    /// Its instance, as its entity ID takes it, when it is `place` in the
    /// list.
    pub fn instance_at(&self, place: usize) -> u8 {
        self.instance.unwrap_or(place as u8).min(239)
    }

    pub fn source(&self) -> Source {
        match self.source.as_deref() {
            None | Some("silence") => Source::Silence,
            Some("tone") => Source::Tone {
                hertz: 1000.0,
                level: 0.1,
            },
            Some("default") => Source::Device {
                name: None,
                first_channel: self.first_channel,
            },
            Some(name) => Source::Device {
                name: Some(name.to_owned()),
                first_channel: self.first_channel,
            },
        }
    }

    pub fn sink(&self) -> Sink {
        match self.sink.as_deref() {
            None | Some("discard") => Sink::Discard,
            Some("default") => Sink::Device {
                name: None,
                first_channel: self.first_channel,
            },
            Some(name) => Sink::Device {
                name: Some(name.to_owned()),
                first_channel: self.first_channel,
            },
        }
    }
}

/// Where the endpoints are listed: `endpoints.toml` in the data folder.
pub fn path() -> Option<PathBuf> {
    triib_store::paths::data_dir().map(|dir| dir.join("endpoints.toml"))
}
