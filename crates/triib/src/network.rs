//! The network thread: one per chosen interface, running the ATDECC
//! controller and posting what it learns to the app.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use atdecc::aem::{AudioMapping, MappingChange, NAME_LENGTH, aem_name};
use atdecc::blocking::Driver;
use atdecc::controller::{Advertise, CommandId, Outcome};
use atdecc::descriptor::{DescriptorType, SamplingRate};
use atdecc::model::EntityModel;
use atdecc::stream_format::StreamFormat;
use atdecc::{ClockIdentity, DiscoveredEntity, EntityId, EntityModelId, Event, OfflineReason};
use avb_net::{MacAddress, Socket};

use crate::External;

/// The longest the thread waits before looking for commands and the stop
/// flag.
const TURN: Duration = Duration::from_millis(200);

/// triib's controller entity model ID, under the Scramble Tools MA-S
/// `8C-1F-64-36-C`.
const ENTITY_MODEL_ID: EntityModelId = EntityModelId(0x8c1f_6436_c000_0001);

/// How long other controllers keep triib after its last advertisement, in
/// units of 2 s: the standard's default of 62 s.
const VALID_TIME: u8 = 31;

const ETHERTYPE_GPTP: u16 = 0x88f7;
/// Where gPTP sends its messages, to the neighbor only.
const GPTP_MULTICAST: MacAddress = MacAddress([0x01, 0x80, 0xc2, 0x00, 0x00, 0x0e]);
/// How long the bridge is known after its last peer delay request; it
/// sends one a second.
const NEIGHBOR_TIMEOUT: Duration = Duration::from_secs(5);
/// How long gPTP counts as running after the bridge's last Sync, which it
/// sends eight times a second.
const SYNC_TIMEOUT: Duration = Duration::from_secs(2);

/// What the network thread reports, tagged with the generation of the
/// thread that sent it so reports from a replaced thread can be ignored.
#[derive(Debug, Clone)]
pub struct Report {
    pub generation: u64,
    pub kind: ReportKind,
}

#[derive(Debug, Clone)]
pub enum ReportKind {
    Started {
        controller: EntityId,
    },
    Failed(Failure),
    Online(DiscoveredEntity),
    Changed(DiscoveredEntity),
    Restarted(DiscoveredEntity),
    Offline(EntityId, OfflineReason),
    /// What has been read of an entity, sent when reading starts, ends or
    /// fails, and when a notification changes it.
    Model(EntityId, Box<EntityModel>),
    /// An action the app asked for finished.
    Finished(Action, Outcome),
    /// The bridge this computer is plugged into, as its gPTP messages show,
    /// or none heard.
    Neighbor(Option<Neighbor>),
}

/// The bridge port this computer is plugged into, from the peer delay
/// requests the bridge sends on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    pub clock: ClockIdentity,
    pub port: u16,
    /// The bridge sends Sync on the link, as it does once gPTP runs there.
    pub synced: bool,
}

/// Something the app asks the network to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Bind a listener's stream input to a talker's stream output.
    Connect {
        talker: (EntityId, u16),
        listener: (EntityId, u16),
    },
    /// Unbind a listener's stream input.
    Disconnect { listener: (EntityId, u16) },
    /// Make an entity identify itself for a few seconds.
    Identify(EntityId),
    /// Rename an entity, its group or one of its descriptors.
    Rename { target: NameTarget, name: Name },
    /// Change a stream input's or output's format.
    SetStreamFormat {
        entity: EntityId,
        descriptor_type: DescriptorType,
        index: u16,
        format: StreamFormat,
    },
    /// Change an audio unit's sampling rate.
    SetSamplingRate {
        entity: EntityId,
        unit: u16,
        rate: SamplingRate,
    },
    /// Pick a clock domain's clock source.
    SetClockSource {
        entity: EntityId,
        domain: u16,
        source: u16,
    },
    /// Add or remove one of a stream port's dynamic mappings.
    Map {
        entity: EntityId,
        port: (DescriptorType, u16),
        change: MappingChange,
        mapping: AudioMapping,
    },
    /// Set a control's current values.
    SetControl {
        entity: EntityId,
        index: u16,
        values: ControlValues,
    },
}

/// The most octets of a control's values an action carries: a linear
/// control's at most (IEEE 1722.1-2021, Table 7-39).
const CONTROL_VALUES: usize = 72;

/// A control's new values, as SET_CONTROL carries them.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlValues {
    octets: [u8; CONTROL_VALUES],
    length: u8,
}

impl ControlValues {
    /// `octets` as values, or `None` when there are none or too many.
    pub fn new(octets: &[u8]) -> Option<Self> {
        if octets.is_empty() || octets.len() > CONTROL_VALUES {
            return None;
        }
        let mut values = Self {
            octets: [0; CONTROL_VALUES],
            length: octets.len() as u8,
        };
        values.octets[..octets.len()].copy_from_slice(octets);
        Some(values)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.octets[..usize::from(self.length)]
    }
}

impl std::fmt::Debug for ControlValues {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "ControlValues({:02x?})", self.as_slice())
    }
}

/// A name an entity holds: for the entity, `name_index` 0 is its name and
/// 1 its group; for its other descriptors 0 is their name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NameTarget {
    pub entity: EntityId,
    pub descriptor_type: DescriptorType,
    pub index: u16,
    pub name_index: u16,
}

/// A name as an entity holds it: up to 64 octets of UTF-8.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Name([u8; NAME_LENGTH]);

impl Name {
    /// `text` as a name, or `None` when it is longer than an entity holds.
    pub fn new(text: &str) -> Option<Self> {
        aem_name(text).map(Self)
    }

    pub fn as_str(&self) -> &str {
        let end = self
            .0
            .iter()
            .position(|&octet| octet == 0)
            .unwrap_or(NAME_LENGTH);
        std::str::from_utf8(&self.0[..end]).unwrap_or_default()
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self.as_str())
    }
}

/// How long an entity identifies itself.
const IDENTIFY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    /// The command that grants the missing permission, when that is the
    /// problem.
    pub fix: Option<String>,
}

impl Failure {
    fn from_io(error: &io::Error) -> Self {
        let fix = (error.kind() == io::ErrorKind::PermissionDenied).then(|| {
            let program = std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "triib".to_owned());
            format!("sudo setcap cap_net_raw+ep {program}")
        });
        let message = match fix {
            Some(_) => "triib needs permission to send and receive raw Ethernet frames.".to_owned(),
            None => error.to_string(),
        };
        Self { message, fix }
    }
}

enum Command {
    Discover,
    Act(Action),
}

/// A running network thread. Dropping it stops the thread.
pub struct Network {
    stop: Arc<AtomicBool>,
    commands: Sender<Command>,
}

impl Network {
    pub fn start(interface: String, generation: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (commands, received) = mpsc::channel();
        let stopping = stop.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("atdecc-{interface}"))
            .spawn(move || run(&interface, generation, &stopping, &received));
        if let Err(error) = spawned {
            report(generation, ReportKind::Failed(Failure::from_io(&error)));
        }
        Self { stop, commands }
    }

    /// Asks every entity to advertise itself now.
    pub fn discover(&self) {
        let _ = self.commands.send(Command::Discover);
    }

    /// Carries out `action`, reporting how it went.
    pub fn act(&self, action: Action) {
        let _ = self.commands.send(Command::Act(action));
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn report(generation: u64, kind: ReportKind) {
    crate::post(External::Network(Report { generation, kind }));
}

fn run(interface: &str, generation: u64, stop: &AtomicBool, commands: &Receiver<Command>) {
    let opened = Driver::open_with(interface, |config| {
        config.advertise = Some(Advertise {
            entity_model_id: ENTITY_MODEL_ID,
            valid_time: VALID_TIME,
        });
    });
    let mut driver = match opened {
        Ok(driver) => driver,
        Err(error) => {
            report(generation, ReportKind::Failed(Failure::from_io(&error)));
            return;
        }
    };
    report(
        generation,
        ReportKind::Started {
            controller: driver.controller().entity_id(),
        },
    );
    for model in models::load() {
        driver.controller_mut().remember_model(model);
    }
    driver.controller_mut().discover(None);
    let mut actions: HashMap<CommandId, Action> = HashMap::new();
    // Without it the network view only lacks this computer's place.
    let mut gptp = Gptp::open(interface);
    loop {
        if stop.load(Ordering::Relaxed) {
            // Tell other controllers triib is leaving.
            let _ = driver.close();
            return;
        }
        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Discover => driver.controller_mut().discover(None),
                Command::Act(action) => {
                    let now = driver.now();
                    let controller = driver.controller_mut();
                    let command = match action {
                        Action::Connect { talker, listener } => {
                            controller.connect(now, talker, listener)
                        }
                        Action::Disconnect { listener } => controller.disconnect(now, listener),
                        Action::Identify(entity_id) => {
                            controller.identify(now, entity_id, IDENTIFY)
                        }
                        Action::Rename { target, name } => controller.set_name(
                            now,
                            target.entity,
                            target.descriptor_type,
                            target.index,
                            target.name_index,
                            name.as_str(),
                        ),
                        Action::SetStreamFormat {
                            entity,
                            descriptor_type,
                            index,
                            format,
                        } => controller.set_stream_format(
                            now,
                            entity,
                            descriptor_type,
                            index,
                            format,
                        ),
                        Action::SetSamplingRate { entity, unit, rate } => controller
                            .set_sampling_rate(now, entity, DescriptorType::AUDIO_UNIT, unit, rate),
                        Action::SetClockSource {
                            entity,
                            domain,
                            source,
                        } => controller.set_clock_source(now, entity, domain, source),
                        Action::Map {
                            entity,
                            port,
                            change: MappingChange::Add,
                            mapping,
                        } => controller.add_audio_mappings(now, entity, port, &[mapping]),
                        Action::Map {
                            entity,
                            port,
                            change: MappingChange::Remove,
                            mapping,
                        } => controller.remove_audio_mappings(now, entity, port, &[mapping]),
                        Action::SetControl {
                            entity,
                            index,
                            values,
                        } => controller.set_control(now, entity, index, values.as_slice()),
                    };
                    actions.insert(command, action);
                }
            }
        }
        if let Err(error) = driver.turn(TURN) {
            report(generation, ReportKind::Failed(Failure::from_io(&error)));
            return;
        }
        if let Some(gptp) = &mut gptp
            && let Some(neighbor) = gptp.poll()
        {
            report(generation, ReportKind::Neighbor(neighbor));
        }
        while let Some(event) = driver.controller_mut().poll_event() {
            let controller = driver.controller();
            let snapshot = |entity_id| controller.entity(entity_id).copied();
            let kind = match event {
                Event::EntityOnline(entity_id) => snapshot(entity_id).map(ReportKind::Online),
                Event::EntityChanged(entity_id) => snapshot(entity_id).map(ReportKind::Changed),
                Event::EntityRestarted(entity_id) => snapshot(entity_id).map(ReportKind::Restarted),
                Event::EntityOffline(entity_id, reason) => {
                    Some(ReportKind::Offline(entity_id, reason))
                }
                Event::CommandFinished(command, outcome) => actions
                    .remove(&command)
                    .map(|action| ReportKind::Finished(action, outcome)),
                Event::EntityEnumerated(entity_id) => {
                    // Read in full: kept, so entities of its model read
                    // faster from here on and next time.
                    if let Some(model) = controller.static_model(entity_id) {
                        models::save(&model);
                        driver.controller_mut().remember_model(model);
                    }
                    let controller = driver.controller();
                    controller
                        .model(entity_id)
                        .map(|model| ReportKind::Model(entity_id, Box::new(model.clone())))
                }
                Event::EnumerationStarted(entity_id)
                | Event::EnumerationFailed(entity_id, _)
                | Event::EntityModelChanged(entity_id) => controller
                    .model(entity_id)
                    .map(|model| ReportKind::Model(entity_id, Box::new(model.clone()))),
            };
            if let Some(kind) = kind {
                report(generation, kind);
            }
        }
    }
}

/// Listens to the gPTP messages the neighboring bridge sends this
/// computer, without taking part in gPTP.
struct Gptp {
    socket: Socket,
    /// The bridge port and when it last sent a peer delay request.
    heard: Option<(ClockIdentity, u16, Instant)>,
    last_sync: Option<Instant>,
    reported: Option<Neighbor>,
}

impl Gptp {
    fn open(interface: &str) -> Option<Self> {
        let socket = Socket::open(interface, ETHERTYPE_GPTP).ok()?;
        socket.join_multicast(GPTP_MULTICAST).ok()?;
        Some(Self {
            socket,
            heard: None,
            last_sync: None,
            reported: None,
        })
    }

    /// Reads what arrived, returning the neighbor when it changed.
    fn poll(&mut self) -> Option<Option<Neighbor>> {
        let mut buffer = [0; 128];
        let now = Instant::now();
        while let Ok(Some(received)) = self.socket.receive(&mut buffer, Some(Duration::ZERO)) {
            let Some(message) = GptpMessage::decode(&buffer[..received.length.min(buffer.len())])
            else {
                continue;
            };
            match message.message_type {
                GptpMessage::PDELAY_REQ => self.heard = Some((message.clock, message.port, now)),
                GptpMessage::SYNC => self.last_sync = Some(now),
                _ => {}
            }
        }
        let neighbor = self
            .heard
            .filter(|(_, _, at)| now.duration_since(*at) < NEIGHBOR_TIMEOUT)
            .map(|(clock, port, _)| Neighbor {
                clock,
                port,
                synced: self
                    .last_sync
                    .is_some_and(|at| now.duration_since(at) < SYNC_TIMEOUT),
            });
        (neighbor != self.reported).then(|| {
            self.reported = neighbor;
            neighbor
        })
    }
}

/// The parts of a gPTP message header the network view uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GptpMessage {
    message_type: u8,
    clock: ClockIdentity,
    port: u16,
}

impl GptpMessage {
    const SYNC: u8 = 0x0;
    const PDELAY_REQ: u8 = 0x2;

    /// Decodes the common header: the message type, then the sending
    /// port's identity at octet 20.
    fn decode(bytes: &[u8]) -> Option<Self> {
        let header: &[u8; 34] = bytes.get(..34)?.try_into().ok()?;
        let clock: [u8; 8] = header[20..28].try_into().ok()?;
        Some(Self {
            message_type: header[0] & 0x0f,
            clock: ClockIdentity(u64::from_be_bytes(clock)),
            port: u16::from_be_bytes([header[28], header[29]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_delay_requests_name_the_bridge_port() {
        // A bridge's Pdelay_Req, as on the bench: port 6 of the switch.
        let mut frame = [0u8; 54];
        frame[0] = 0x12;
        frame[1] = 0x02;
        frame[20..28].copy_from_slice(&0x0001_f2ff_feff_3b14u64.to_be_bytes());
        frame[28..30].copy_from_slice(&6u16.to_be_bytes());
        let message = GptpMessage::decode(&frame).unwrap();
        assert_eq!(message.message_type, GptpMessage::PDELAY_REQ);
        assert_eq!(message.clock, ClockIdentity(0x0001_f2ff_feff_3b14));
        assert_eq!(message.port, 6);
        assert_eq!(GptpMessage::decode(&frame[..33]), None);
    }
}

/// Entity models kept between runs in the cache folder, one file each.
mod models {
    use std::path::PathBuf;

    use atdecc::cache::{CachedModel, ModelKey};

    fn folder() -> Option<PathBuf> {
        triib_store::paths::cache_dir().map(|dir| dir.join("models"))
    }

    /// The file a model is kept in, named by what it is for.
    fn file(key: &ModelKey) -> Option<PathBuf> {
        // FNV-1a, which stays the same between builds, unlike the hasher
        // the standard library picks.
        let firmware = key
            .firmware
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |hash, octet| {
                (hash ^ u64::from(octet)).wrapping_mul(0x0100_0000_01b3)
            });
        folder().map(|folder| {
            folder.join(format!(
                "{:016x}-{}-{firmware:016x}.aem",
                key.entity_model_id.0, key.configuration
            ))
        })
    }

    /// Every model kept, leaving out files that do not read as one.
    pub fn load() -> Vec<CachedModel> {
        let Some(entries) = folder().and_then(|folder| std::fs::read_dir(folder).ok()) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "aem")
            })
            .filter_map(|entry| std::fs::read(entry.path()).ok())
            .filter_map(|bytes| CachedModel::decode(&bytes).ok())
            .collect()
    }

    /// Keeps a model, replacing one kept for the same entities.
    pub fn save(model: &CachedModel) {
        let Some(path) = file(&model.key) else {
            return;
        };
        if let Some(folder) = path.parent()
            && std::fs::create_dir_all(folder).is_err()
        {
            return;
        }
        if let Err(error) = triib_store::atomic::write(&path, &model.encode()) {
            eprintln!("triib: could not keep {}: {error}", path.display());
        }
    }
}
