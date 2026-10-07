//! This computer's own talkers and listeners, which triib-endpointd runs:
//! adding, changing and removing them in its `endpoints.toml`, starting
//! it, and what it says each is doing.

use atdecc::{EntityId, MacAddress};
use triib_endpointd::config::{self, Config, EndpointConfig, Kind};
use triib_endpointd::status::{DaemonStatus, EndpointStatus};

/// Whether this system runs host talkers and listeners.
pub const SUPPORTED: bool = cfg!(target_os = "linux");

/// The endpoints file, or a new one for `interface`, with the interface
/// set to it either way.
fn load(interface: &str) -> Result<(std::path::PathBuf, Config), String> {
    let path = config::path().ok_or("no data folder")?;
    let mut config = if path.exists() {
        triib_store::load::<Config>(&path).map_err(|error| error.to_string())?
    } else {
        Config {
            ptp4l_socket: "/var/run/ptp4lro".into(),
            ..Config::default()
        }
    };
    config.interface = interface.to_owned();
    Ok((path, config))
}

fn save(path: &std::path::Path, config: &Config) -> Result<(), String> {
    triib_store::save(path, config).map_err(|error| error.to_string())
}

/// The place in the file of the endpoint whose entity is `entity_id`.
fn place_of(config: &Config, mac: MacAddress, entity_id: EntityId) -> Option<usize> {
    config
        .endpoints
        .iter()
        .enumerate()
        .position(|(place, endpoint)| {
            triib_endpointd::runtime::entity_id(mac, endpoint.instance_at(place)) == entity_id
        })
}

/// Adds a talker or listener named `name`, with the default device for
/// its audio.
pub fn add(interface: &str, kind: Kind, name: String) -> Result<(), String> {
    let (path, mut config) = load(interface)?;
    let taken: Vec<u8> = config
        .endpoints
        .iter()
        .enumerate()
        .map(|(place, endpoint)| endpoint.instance_at(place))
        .collect();
    let free = (0..240u8)
        .find(|candidate| !taken.contains(candidate))
        .ok_or("no room")?;
    let (source, sink) = match kind {
        Kind::Talker => (Some("default".to_owned()), None),
        Kind::Listener => (None, Some("default".to_owned())),
    };
    config.endpoints.push(EndpointConfig {
        kind,
        instance: Some(free),
        name,
        channels: 8,
        source,
        sink,
        first_channel: 0,
    });
    save(&path, &config)
}

/// Removes the endpoint whose entity is `entity_id`; the others keep
/// their entity IDs.
pub fn remove(interface: &str, mac: MacAddress, entity_id: EntityId) -> Result<(), String> {
    let (path, mut config) = load(interface)?;
    let place = place_of(&config, mac, entity_id).ok_or("not one of this computer's")?;
    config.endpoints = pinned(std::mem::take(&mut config.endpoints));
    config.endpoints.remove(place);
    save(&path, &config)
}

/// Sends the endpoint's audio from or to `audio`: "silence", "tone",
/// "discard", "default" or a device's name.
pub fn set_audio(
    interface: &str,
    mac: MacAddress,
    entity_id: EntityId,
    audio: String,
) -> Result<(), String> {
    let (path, mut config) = load(interface)?;
    let place = place_of(&config, mac, entity_id).ok_or("not one of this computer's")?;
    let endpoint = &mut config.endpoints[place];
    match endpoint.kind {
        Kind::Talker => endpoint.source = Some(audio),
        Kind::Listener => endpoint.sink = Some(audio),
    }
    save(&path, &config)
}

/// The endpoints on `interface`, each with its instance written down so
/// its entity ID stays the same, for a preset to keep.
pub fn endpoints(interface: &str) -> Vec<EndpointConfig> {
    if cfg!(test) {
        return Vec::new();
    }
    let Some(config) = config::path().and_then(|path| triib_store::load::<Config>(&path).ok())
    else {
        return Vec::new();
    };
    if config.interface != interface {
        return Vec::new();
    }
    pinned(config.endpoints)
}

fn pinned(mut endpoints: Vec<EndpointConfig>) -> Vec<EndpointConfig> {
    for (place, endpoint) in endpoints.iter_mut().enumerate() {
        endpoint.instance = Some(endpoint.instance_at(place));
    }
    endpoints
}

/// Runs `endpoints` on `interface` in place of the ones there, starting
/// the daemon when it isn't running. Says whether they will start again,
/// which they do when anything changed.
pub fn restore(interface: &str, endpoints: &[EndpointConfig]) -> Result<bool, String> {
    if cfg!(test) {
        return Ok(false);
    }
    let path = config::path().ok_or("no data folder")?;
    let running = triib_endpointd::status::read().is_some();
    let now = triib_store::load::<Config>(&path).ok();
    let same = now.as_ref().is_some_and(|config| {
        config.interface == interface && pinned(config.endpoints.clone()) == endpoints
    });
    if !same {
        let (path, mut config) = load(interface)?;
        config.endpoints = endpoints.to_vec();
        save(&path, &config)?;
    }
    ensure_running()?;
    Ok(!same || !running)
}

/// Starts triib-endpointd, from beside triib, unless it runs already.
pub fn ensure_running() -> Result<(), String> {
    if triib_endpointd::status::read().is_some() {
        return Ok(());
    }
    start()
}

#[cfg(unix)]
fn start() -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    // Beside triib, else wherever the PATH finds it.
    let program = std::env::current_exe()
        .map(|path| path.with_file_name("triib-endpointd"))
        .ok()
        .filter(|path| path.exists())
        .unwrap_or_else(|| "triib-endpointd".into());
    let log = triib_store::paths::state_dir()
        .map(|dir| {
            let _ = std::fs::create_dir_all(&dir);
            dir.join("endpointd.log")
        })
        .and_then(|path| std::fs::File::create(path).ok());
    let (out, error) = match log {
        Some(file) => match file.try_clone() {
            Ok(copy) => (Stdio::from(file), Stdio::from(copy)),
            Err(_) => (Stdio::null(), Stdio::null()),
        },
        None => (Stdio::null(), Stdio::null()),
    };
    Command::new(program)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(error)
        // Its own process group, so it outlives the app.
        .process_group(0)
        .spawn()
        .map(drop)
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn start() -> Result<(), String> {
    Err("not on this system".into())
}

/// What the daemon says of the endpoint whose entity is `entity_id`.
pub fn status_of(status: Option<&DaemonStatus>, entity_id: EntityId) -> Option<&EndpointStatus> {
    let id = entity_id.to_string();
    status?
        .endpoints
        .iter()
        .find(|endpoint| endpoint.entity_id == id)
}
