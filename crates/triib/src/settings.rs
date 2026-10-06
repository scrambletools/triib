//! The app's settings and window state, kept in `triib.toml`.

use std::collections::BTreeMap;

use scramble_ui::appearance::Appearance;
use serde::{Deserialize, Serialize};

/// The language setting's value for following the system's language.
pub const SYSTEM_LANGUAGE: &str = "system";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The interface's language: a tag such as "en", or
    /// [`SYSTEM_LANGUAGE`].
    pub language: String,
    pub appearance: Appearance,
    /// Follow the desktop's accent color.
    pub system_accent: bool,
    /// "#RRGGBB" picked in the settings, else triib's own.
    pub accent_color: Option<String>,
    pub animations: bool,
    /// The interface last used, chosen again at startup when present.
    pub interface: Option<String>,
    /// Leave bridges, tunnels and other virtual interfaces out of the
    /// interface picker.
    pub hide_virtual_interfaces: bool,
    pub view: View,
    pub inspector: bool,
    /// The streams the connection matrix shows.
    pub matrix_streams: Streams,
    /// Leave out of the matrix the streams nothing shown can connect to.
    pub matrix_connectable_only: bool,
    /// What the network view shows flowing along its wires.
    pub network_shows: NetworkShows,
    /// The entity list's columns after the name, in order.
    pub entity_columns: Vec<EntityField>,
    /// Widths the user dragged the entity list's columns to, by field,
    /// kept when a column is removed; the others fit their text.
    pub entity_column_widths: BTreeMap<EntityField, f32>,
    /// The width the user dragged the entity list's name column to.
    pub entity_name_width: Option<f32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: SYSTEM_LANGUAGE.to_owned(),
            appearance: Appearance::System,
            system_accent: true,
            accent_color: None,
            animations: true,
            interface: None,
            hide_virtual_interfaces: true,
            view: View::Matrix,
            inspector: false,
            matrix_streams: Streams::All,
            matrix_connectable_only: false,
            network_shows: NetworkShows::Audio,
            entity_columns: EntityField::DEFAULT_COLUMNS.to_vec(),
            entity_column_widths: BTreeMap::new(),
            entity_name_width: None,
        }
    }
}

/// What the content area shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    /// Talker streams against listener streams.
    #[default]
    Matrix,
    // Host talkers and listeners are entities like any other.
    #[serde(alias = "host")]
    Entities,
    /// The network as gPTP paths show it.
    Network,
    /// Each media clock reference and the clock domains following it.
    Clocks,
}

/// What a column of the entity list after the name shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntityField {
    Group,
    Product,
    Vendor,
    Model,
    Firmware,
    SerialNumber,
    Milan,
    Roles,
    MediaClock,
    SamplingRate,
    ClockSource,
    Btc,
    State,
    EntityId,
    MacAddress,
    EntityModelId,
    Configuration,
    TalkerStreams,
    ListenerStreams,
}

impl EntityField {
    /// Every field, in the order the column menus offer them.
    pub const ALL: [Self; 19] = [
        Self::Group,
        Self::Product,
        Self::Vendor,
        Self::Model,
        Self::Firmware,
        Self::SerialNumber,
        Self::Milan,
        Self::Roles,
        Self::MediaClock,
        Self::SamplingRate,
        Self::ClockSource,
        Self::Btc,
        Self::State,
        Self::EntityId,
        Self::MacAddress,
        Self::EntityModelId,
        Self::Configuration,
        Self::TalkerStreams,
        Self::ListenerStreams,
    ];

    /// The columns shown until the user changes them.
    pub const DEFAULT_COLUMNS: [Self; 9] = [
        Self::Group,
        Self::Product,
        Self::Firmware,
        Self::Milan,
        Self::Roles,
        Self::MediaClock,
        Self::Btc,
        Self::State,
        Self::EntityId,
    ];

    /// The column's heading.
    pub fn label(self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::Product => "Product",
            Self::Vendor => "Vendor",
            Self::Model => "Model",
            Self::Firmware => "Firmware",
            Self::SerialNumber => "Serial number",
            Self::Milan => "Milan",
            Self::Roles => "Roles",
            Self::MediaClock => "Media clock",
            Self::SamplingRate => "Sampling rate",
            Self::ClockSource => "Clock source",
            Self::Btc => "BTC",
            Self::State => "State",
            Self::EntityId => "Entity ID",
            Self::MacAddress => "MAC address",
            Self::EntityModelId => "Entity model ID",
            Self::Configuration => "Configuration",
            Self::TalkerStreams => "Talker streams",
            Self::ListenerStreams => "Listener streams",
        }
    }
}

/// Which streams the connection matrix shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Streams {
    #[default]
    All,
    Audio,
    /// Media clock streams, such as CRF.
    Clock,
}

/// What the network view shows flowing along its wires.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkShows {
    /// The gPTP clock, from the grandmaster down.
    Clock,
    /// The bound audio streams, each on a wire of its own.
    #[default]
    // Settings saved when the view showed all streams together.
    #[serde(alias = "streams")]
    Audio,
    /// The bound media clock streams, such as CRF.
    MediaClock,
}

impl Settings {
    /// Loads `triib.toml` (`triib-dev.toml` for development builds),
    /// writing it so it lists every setting; when it cannot be used, the
    /// defaults and why.
    pub fn load() -> (Self, Option<String>) {
        // Tests start from the defaults, whatever the user has.
        if cfg!(test) {
            return (Self::default(), None);
        }
        let Some(path) = triib_store::paths::config_file() else {
            return (Self::default(), Some(NO_PLACE.to_owned()));
        };
        match triib_store::load_or_create(&path) {
            Ok(settings) => (settings, None),
            Err(error) => (
                Self::default(),
                Some(format!("Could not use {}: {error}.", path.display())),
            ),
        }
    }

    /// Saves the settings, or says why they could not be saved.
    pub fn save(&self) -> Result<(), String> {
        // Tests drive the app, which saves as it goes; never over the
        // user's own settings.
        if cfg!(test) {
            return Ok(());
        }
        let path = triib_store::paths::config_file().ok_or(NO_PLACE)?;
        triib_store::save(&path, self)
            .map_err(|error| format!("Could not save {}: {error}.", path.display()))
    }
}

/// Why there is no settings file to read or write.
const NO_PLACE: &str = "There is nowhere to keep the settings: the home folder is not known.";
