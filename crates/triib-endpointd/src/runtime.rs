//! The running endpoints: one thread taking frames from the sockets and
//! ptp4l's state, driving each entity and the MSRP and MVRP participants,
//! and starting and stopping the streams as reservations come and go.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::SystemTime;
use std::time::{Duration, Instant as Clock};

use atdecc::entity::{
    EndpointModel, Entity, EntityEvent, InputReservation, OutputReservation, StreamModel,
};
use atdecc::stream_format::StreamFormat;
use atdecc::{
    ADP_ACMP_MULTICAST, ClockIdentity, EntityId, EntityModelId, Instant, MacAddress, StreamId,
};
use avb_mrp::msrp::{self, Domain, ListenerState, TalkerDeclaration};
use avb_mrp::{Participant, Registration, mvrp};
use avb_net::Socket;
use triib_stream::audio::{Sink, Source};
use triib_stream::{Listener, ListenerConfig, MediaClock, Talker, TalkerConfig};

use crate::config::{Config, EndpointConfig, Kind};
use crate::gptp;
use crate::status::{DaemonStatus, EndpointStatus, status_path};

/// Entity model IDs under Scramble Tools' MA-S `8C-1F-64-36-C`.
const TALKER_MODEL: EntityModelId = EntityModelId(0x8c1f_6436_c000_0002);
const LISTENER_MODEL: EntityModelId = EntityModelId(0x8c1f_6436_c000_0003);
/// Class A's priority and VLAN.
const PRIORITY: u8 = 3;
const VLAN: u16 = 2;
/// The latency a talker declares for itself, in nanoseconds.
const TALKER_LATENCY: u32 = 125_000;
/// How often the streams' counters are logged.
const REPORT: Duration = Duration::from_secs(30);

/// What the sockets and ptp4l hand the runtime.
enum Input {
    Avtp(MacAddress, Vec<u8>),
    Msrp(Vec<u8>),
    Mvrp(Vec<u8>),
    Gptp(Option<gptp::Status>),
}

/// AAF at 48 kHz, 6 samples a frame, 32-bit, in `bits` of them.
fn aaf_48k(channels: u16, bits: u8) -> StreamFormat {
    StreamFormat(
        (0x02 << 56)
            | (0x05 << 48)
            | (0x02 << 40)
            | (u64::from(bits) << 32)
            | (u64::from(channels & 0x3ff) << 22)
            | (6 << 12),
    )
}

/// An entity ID from the interface's address and the endpoint's instance,
/// apart from the app's (FF-FE) and triib-cli's (FF-FD).
pub fn entity_id(mac: MacAddress, instance: u8) -> EntityId {
    let [a, b, c, d, e, f] = mac.0;
    EntityId(u64::from_be_bytes([a, b, c, 0xff, instance, d, e, f]))
}

/// The stream a talker's output sends, and its destination, from the
/// locally administered range 91:E0:F0:00 is in.
fn output_stream(mac: MacAddress, instance: u8, index: u16) -> (StreamId, MacAddress) {
    let unique = (u16::from(instance) << 8) | index;
    let destination = MacAddress([
        0x91,
        0xe0,
        0xf0,
        0x00,
        mac.0[5],
        instance.wrapping_add(index as u8),
    ]);
    (StreamId::new(mac, unique), destination)
}

struct Endpoint {
    config: EndpointConfig,
    entity: Entity,
    talker: Option<Talker>,
    listener: Option<Listener>,
    /// The stream its Listener declaration names, while it declares one.
    declared: Option<Vec<u8>>,
    /// Its place in the configuration's list.
    place: usize,
}

impl Endpoint {
    fn new(
        config: EndpointConfig,
        instance: u8,
        mac: MacAddress,
        interface: &str,
        clock: ClockIdentity,
    ) -> Self {
        let channels = config.channels.clamp(1, 64);
        let formats = vec![aaf_48k(channels, 32), aaf_48k(channels, 24)];
        let stream = StreamModel {
            name: config.name.clone(),
            current_format: formats[0],
            formats,
            channels,
        };
        let talker = config.kind == Kind::Talker;
        let model = EndpointModel {
            entity_id: entity_id(mac, instance),
            entity_model_id: if talker { TALKER_MODEL } else { LISTENER_MODEL },
            entity_name: config.name.clone(),
            group_name: String::new(),
            vendor_name: "Scramble Tools".into(),
            model_name: if talker {
                "triib talker"
            } else {
                "triib listener"
            }
            .into(),
            firmware_version: env!("CARGO_PKG_VERSION").into(),
            serial_number: String::new(),
            mac,
            interface_name: interface.into(),
            clock_identity: clock,
            sampling_rates: vec![48_000],
            current_sampling_rate: 48_000,
            outputs: if talker { vec![stream.clone()] } else { vec![] },
            inputs: if talker { vec![] } else { vec![stream] },
            clock_source: 0,
        };
        let outputs = if talker {
            vec![output_stream(mac, instance, 0)]
        } else {
            vec![]
        };
        Endpoint {
            config,
            entity: Entity::new(model, &outputs),
            talker: None,
            listener: None,
            declared: None,
            place: 0,
        }
    }
}

pub struct Runtime {
    interface: String,
    mac: MacAddress,
    start: Clock,
    avtp: Arc<Socket>,
    msrp: Arc<Socket>,
    mvrp: Arc<Socket>,
    endpoints: Vec<Endpoint>,
    msrp_participant: Participant,
    mvrp_participant: Participant,
    clock: Arc<MediaClock>,
    /// The talker declarations registered, by stream ID.
    talkers: HashMap<u64, TalkerDeclaration>,
    /// Our talkers' streams a listener on the network is ready for.
    ready_remotely: HashMap<u64, bool>,
    stop: Arc<AtomicBool>,
    inputs: Receiver<Input>,
    last_report: Clock,
    last_counters: Clock,
    /// The configuration running, its file, and when that last changed.
    config: Config,
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    last_look: Clock,
    gptp_text: String,
    /// When it started on this configuration, in milliseconds since the
    /// Unix epoch.
    started: u64,
}

/// Why the runtime ended.
pub enum Exit {
    Stop,
    /// The configuration changed; start again with this one.
    Reload(Config),
}

/// The modification time of `path`.
fn modified(path: Option<&PathBuf>) -> Option<SystemTime> {
    std::fs::metadata(path?).ok()?.modified().ok()
}

fn log(message: impl AsRef<str>) {
    println!("{}", message.as_ref());
}

/// Hands every frame `socket` receives to the runtime as `wrap` makes it.
fn read_into(
    socket: Arc<Socket>,
    stop: Arc<AtomicBool>,
    sender: Sender<Input>,
    name: &str,
    wrap: fn(MacAddress, Vec<u8>) -> Input,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let mut buffer = vec![0u8; 1522];
            while !stop.load(Ordering::Relaxed) {
                match socket.receive(&mut buffer, Some(Duration::from_millis(200))) {
                    Ok(Some(received)) => {
                        let bytes = buffer[..received.length].to_vec();
                        if sender.send(wrap(received.source, bytes)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => {}
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        })?;
    Ok(())
}

impl Runtime {
    pub fn new(config: Config, path: Option<PathBuf>, stop: Arc<AtomicBool>) -> io::Result<Self> {
        let interface = config.interface.clone();
        let found = avb_net::interfaces()
            .into_iter()
            .find(|found| found.name == interface)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no interface named {interface}"),
                )
            })?;
        let clock = Arc::new(MediaClock::open(found.hardware_clock)?);
        if found.hardware_clock.is_none() {
            log(format!(
                "{interface} has no PTP hardware clock: presentation times will mean nothing"
            ));
        }
        let avtp = Arc::new(Socket::open(&interface, atdecc::ETHERTYPE_AVTP)?);
        avtp.join_multicast(ADP_ACMP_MULTICAST)?;
        let _ = avtp.keep_payloads_starting(0xfa..=0xfc);
        let msrp = Arc::new(Socket::open(&interface, avb_mrp::ETHERTYPE_MSRP)?);
        msrp.join_multicast(avb_mrp::MSRP_DESTINATION)?;
        let mvrp = Arc::new(Socket::open(&interface, avb_mrp::ETHERTYPE_MVRP)?);
        mvrp.join_multicast(avb_mrp::MVRP_DESTINATION)?;

        let (sender, inputs) = mpsc::channel();
        read_into(
            avtp.clone(),
            stop.clone(),
            sender.clone(),
            "avtp",
            Input::Avtp,
        )?;
        read_into(
            msrp.clone(),
            stop.clone(),
            sender.clone(),
            "msrp",
            |_, bytes| Input::Msrp(bytes),
        )?;
        read_into(
            mvrp.clone(),
            stop.clone(),
            sender.clone(),
            "mvrp",
            |_, bytes| Input::Mvrp(bytes),
        )?;
        let socket = if config.ptp4l_socket.is_empty() {
            "/var/run/ptp4lro".to_owned()
        } else {
            config.ptp4l_socket.clone()
        };
        gptp::watch(socket, stop.clone(), sender, Input::Gptp);

        let mac = avtp.mac();
        let own_clock = ClockIdentity(u64::from_be_bytes([
            mac.0[0], mac.0[1], mac.0[2], 0xff, 0xfe, mac.0[3], mac.0[4], mac.0[5],
        ]));
        let mut used = Vec::new();
        let endpoints: Vec<Endpoint> = config
            .endpoints
            .iter()
            .enumerate()
            .filter_map(|(place, endpoint)| {
                let instance = endpoint.instance_at(place);
                if used.contains(&instance) {
                    log(format!(
                        "instance {instance} is taken; leaving out {}",
                        endpoint.name
                    ));
                    return None;
                }
                used.push(instance);
                let mut made =
                    Endpoint::new(endpoint.clone(), instance, mac, &interface, own_clock);
                made.place = place;
                Some(made)
            })
            .collect();
        let seed = mac
            .0
            .iter()
            .fold(0u64, |seed, octet| (seed << 8) | u64::from(*octet));
        Ok(Runtime {
            interface,
            mac,
            start: Clock::now(),
            avtp,
            msrp,
            mvrp,
            endpoints,
            msrp_participant: Participant::new(msrp::FORMAT, Duration::ZERO, seed),
            mvrp_participant: Participant::new(mvrp::FORMAT, Duration::ZERO, seed ^ 0x5555),
            clock,
            talkers: HashMap::new(),
            ready_remotely: HashMap::new(),
            stop,
            inputs,
            last_report: Clock::now(),
            last_counters: Clock::now(),
            modified: modified(path.as_ref()),
            path,
            config,
            last_look: Clock::now(),
            gptp_text: "ptp4l not asked yet".into(),
            started: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |since| since.as_millis() as u64),
        })
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn now(&self) -> Instant {
        Instant::from_nanos(self.elapsed().as_nanos() as u64)
    }

    /// Runs until told to stop or the configuration changes, then says
    /// goodbye.
    pub fn run(mut self) -> io::Result<Exit> {
        let now = self.now();
        let since = self.elapsed();
        for endpoint in &mut self.endpoints {
            endpoint.entity.start(now);
            log(format!(
                "{} {} as {}",
                match endpoint.config.kind {
                    Kind::Talker => "talker",
                    Kind::Listener => "listener",
                },
                endpoint.config.name,
                endpoint.entity.entity_id()
            ));
        }
        // Every endpoint takes part in class A on VLAN 2, and each talker
        // declares its stream.
        self.mvrp_participant
            .declare(since, mvrp::VID, mvrp::value(VLAN), None);
        self.msrp_participant.declare(
            since,
            msrp::attribute::DOMAIN,
            Domain::CLASS_A.value(),
            None,
        );
        for index in 0..self.endpoints.len() {
            self.declare_talker(index);
        }
        while !self.stop.load(Ordering::Relaxed) {
            let wait = self.next_wait();
            match self.inputs.recv_timeout(wait) {
                Ok(input) => self.handle(input),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            self.turn();
            if self.last_look.elapsed() >= Duration::from_secs(2) {
                self.last_look = Clock::now();
                self.write_status();
                if let Some(config) = self.changed_config() {
                    log("endpoints.toml changed, starting again");
                    self.shut_down();
                    return Ok(Exit::Reload(config));
                }
            }
        }
        self.shut_down();
        if let Some(path) = status_path() {
            let _ = std::fs::remove_file(path);
        }
        Ok(Exit::Stop)
    }

    /// The configuration file, when it changed into something else.
    fn changed_config(&mut self) -> Option<Config> {
        let now = modified(self.path.as_ref());
        if now == self.modified {
            return None;
        }
        self.modified = now;
        match triib_store::load::<Config>(self.path.as_ref()?) {
            Ok(config) if config != self.config => Some(config),
            Ok(_) => None,
            Err(error) => {
                log(format!("endpoints.toml: {error}"));
                None
            }
        }
    }

    /// Keeps a name a controller gave in the configuration.
    fn keep_name(&mut self, index: usize) {
        let endpoint = &self.endpoints[index];
        let name = endpoint.entity.model().entity_name.clone();
        let Some(entry) = self.config.endpoints.get_mut(endpoint.place) else {
            return;
        };
        entry.name = name;
        if let Some(path) = &self.path {
            if let Err(error) = triib_store::save(path, &self.config) {
                log(format!("endpoints.toml: {error}"));
            }
            self.modified = modified(Some(path));
        }
    }

    fn write_status(&self) {
        let Some(path) = status_path() else {
            return;
        };
        let endpoints = self
            .endpoints
            .iter()
            .map(|endpoint| EndpointStatus {
                entity_id: endpoint.entity.entity_id().to_string(),
                name: endpoint.entity.model().entity_name.clone(),
                kind: endpoint.config.kind,
                audio: match endpoint.config.kind {
                    Kind::Talker => endpoint
                        .config
                        .source
                        .clone()
                        .unwrap_or_else(|| "silence".into()),
                    Kind::Listener => endpoint
                        .config
                        .sink
                        .clone()
                        .unwrap_or_else(|| "discard".into()),
                },
                state: match endpoint.config.kind {
                    Kind::Talker if endpoint.talker.is_some() => "streaming",
                    Kind::Talker => "waiting",
                    Kind::Listener if endpoint.listener.is_some() => "listening",
                    Kind::Listener if endpoint.entity.input_binding(0).is_some() => "bound",
                    Kind::Listener => "unbound",
                }
                .into(),
            })
            .collect();
        let status = DaemonStatus {
            pid: std::process::id(),
            started: self.started,
            interface: self.interface.clone(),
            gptp: self.gptp_text.clone(),
            endpoints,
        };
        let _ = triib_store::save(&path, &status);
    }

    fn next_wait(&self) -> Duration {
        let now = self.now();
        let since = self.elapsed();
        let entity = self
            .endpoints
            .iter()
            .filter_map(|endpoint| endpoint.entity.poll_timeout())
            .min()
            .map(|at| at.saturating_duration_since(now));
        let mrp = [
            self.msrp_participant.poll_timeout(),
            self.mvrp_participant.poll_timeout(),
        ]
        .into_iter()
        .min()
        .map(|at| at.saturating_sub(since));
        [entity, mrp, Some(Duration::from_millis(250))]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(Duration::from_millis(250))
    }

    fn handle(&mut self, input: Input) {
        let now = self.now();
        let since = self.elapsed();
        match input {
            Input::Avtp(source, bytes) => {
                for endpoint in &mut self.endpoints {
                    endpoint.entity.handle_frame(now, source, &bytes);
                }
            }
            Input::Msrp(bytes) => self.msrp_participant.handle_pdu(since, &bytes),
            Input::Mvrp(bytes) => self.mvrp_participant.handle_pdu(since, &bytes),
            Input::Gptp(status) => {
                self.gptp_text = match &status {
                    Some(status) => format!(
                        "grandmaster {}, {}",
                        status.gptp.grandmaster,
                        if status.gptp.as_capable {
                            "asCapable"
                        } else {
                            "not asCapable"
                        }
                    ),
                    None => "ptp4l does not answer".into(),
                };
                match &status {
                    Some(status) => log(format!(
                        "gPTP: grandmaster {}, peer delay {} ns, {}",
                        status.gptp.grandmaster,
                        status.gptp.propagation_delay,
                        if status.gptp.as_capable {
                            "asCapable"
                        } else {
                            "not asCapable"
                        }
                    )),
                    None => log("gPTP: ptp4l does not answer"),
                }
                let gptp = status.map(|status| status.gptp).unwrap_or_default();
                for endpoint in &mut self.endpoints {
                    endpoint.entity.set_gptp(now, gptp.clone());
                }
            }
        }
    }

    /// Runs the timers, then acts on what the entities and participants
    /// report and sends what they queued.
    fn turn(&mut self) {
        let now = self.now();
        let since = self.elapsed();
        for endpoint in &mut self.endpoints {
            endpoint.entity.handle_timeout(now);
        }
        self.msrp_participant.handle_timeout(since);
        self.mvrp_participant.handle_timeout(since);
        loop {
            for index in 0..self.endpoints.len() {
                while let Some(event) = self.endpoints[index].entity.poll_event() {
                    self.entity_event(index, event);
                }
            }
            while let Some(registration) = self.msrp_participant.poll_event() {
                self.registration(registration);
            }
            while self.mvrp_participant.poll_event().is_some() {}
            if !self.flush() {
                break;
            }
        }
        if self.last_counters.elapsed() >= Duration::from_secs(1) {
            self.last_counters = Clock::now();
            self.update_counters();
        }
        if self.last_report.elapsed() >= REPORT {
            self.last_report = Clock::now();
            self.report();
        }
    }

    /// Sends what the entities and participants queued, saying whether an
    /// entity had a frame from another of ours to handle.
    fn flush(&mut self) -> bool {
        let now = self.now();
        let mut handed = false;
        // The socket never brings back its own frames, so our entities'
        // frames for each other go to them here as well as on the wire.
        loop {
            let mut sent = Vec::new();
            for (index, endpoint) in self.endpoints.iter_mut().enumerate() {
                while let Some((destination, bytes)) = endpoint.entity.poll_transmit() {
                    let _ = self.avtp.send(destination, &bytes);
                    if destination == ADP_ACMP_MULTICAST || destination == self.mac {
                        sent.push((index, bytes));
                    }
                }
            }
            if sent.is_empty() || self.endpoints.len() < 2 {
                break;
            }
            for (from, bytes) in sent {
                for (index, endpoint) in self.endpoints.iter_mut().enumerate() {
                    if index != from {
                        endpoint.entity.handle_frame(now, self.mac, &bytes);
                    }
                }
            }
            handed = true;
        }
        while let Some(pdu) = self.msrp_participant.poll_transmit() {
            let _ = self.msrp.send(avb_mrp::MSRP_DESTINATION, &pdu);
        }
        while let Some(pdu) = self.mvrp_participant.poll_transmit() {
            let _ = self.mvrp.send(avb_mrp::MVRP_DESTINATION, &pdu);
        }
        handed
    }

    /// The Talker Advertise for a talker endpoint's stream, from its
    /// current format.
    fn declaration(&self, index: usize) -> Option<TalkerDeclaration> {
        let endpoint = &self.endpoints[index];
        let (stream_id, destination) = endpoint.entity.output_stream(0)?;
        let format = endpoint.entity.model().outputs.first()?.current_format;
        let layout = triib_stream::aaf::Layout::of(format)?;
        Some(TalkerDeclaration {
            stream_id: stream_id.0,
            destination,
            vlan_id: VLAN,
            max_frame_size: layout.pdu_length() as u16,
            max_interval_frames: 1,
            priority: PRIORITY,
            rank: true,
            accumulated_latency: TALKER_LATENCY,
            failure: None,
            event: avb_mrp::Event::New,
        })
    }

    fn declare_talker(&mut self, index: usize) {
        let Some(declaration) = self.declaration(index) else {
            return;
        };
        let since = self.elapsed();
        self.msrp_participant.declare(
            since,
            msrp::attribute::TALKER_ADVERTISE,
            msrp::talker_value(&declaration),
            None,
        );
        self.endpoints[index].entity.set_output_reservation(
            0,
            OutputReservation {
                ready_listeners: 0,
                registering: true,
            },
        );
    }

    fn registration(&mut self, registration: Registration) {
        match registration {
            Registration::Joined {
                attribute_type: msrp::attribute::LISTENER,
                value,
                four_packed,
            } => {
                let state = ListenerState::from_number(four_packed.unwrap_or(0));
                self.listener_heard(stream_of(&value), Some(state));
            }
            Registration::Left {
                attribute_type: msrp::attribute::LISTENER,
                value,
            } => self.listener_heard(stream_of(&value), None),
            Registration::Joined {
                attribute_type,
                value,
                ..
            } if is_talker(attribute_type) => {
                if let Some(declaration) =
                    msrp::decode_talker(attribute_type, &value, avb_mrp::Event::JoinIn)
                {
                    self.talkers.insert(declaration.stream_id, declaration);
                    self.talker_heard(declaration.stream_id);
                }
            }
            Registration::Left {
                attribute_type,
                value,
            } if is_talker(attribute_type) => {
                let stream_id = stream_of(&value);
                if self.talkers.remove(&stream_id).is_some() {
                    self.talker_heard(stream_id);
                }
            }
            _ => {}
        }
    }

    /// Which of our talkers sends `stream_id`.
    fn own_talker(&self, stream_id: u64) -> Option<usize> {
        self.endpoints.iter().position(|endpoint| {
            endpoint
                .entity
                .output_stream(0)
                .is_some_and(|(id, _)| id.0 == stream_id)
        })
    }

    /// A listener's declaration for one of our talkers' streams changed.
    fn listener_heard(&mut self, stream_id: u64, state: Option<ListenerState>) {
        let ready = state.is_some_and(ListenerState::ready);
        self.ready_remotely.insert(stream_id, ready);
        if let Some(index) = self.own_talker(stream_id) {
            self.update_talker(index);
        }
    }

    /// Starts a talker's stream when a listener is ready for it, on the
    /// network or here, and stops it when none is.
    fn update_talker(&mut self, index: usize) {
        let Some((stream_id, _)) = self.endpoints[index].entity.output_stream(0) else {
            return;
        };
        let remote = self
            .ready_remotely
            .get(&stream_id.0)
            .copied()
            .unwrap_or(false);
        let local = self
            .endpoints
            .iter()
            .filter(|endpoint| {
                endpoint.listener.is_some()
                    && endpoint
                        .entity
                        .input_stream(0)
                        .is_some_and(|stream| stream.stream_id == stream_id)
            })
            .count();
        let listeners = usize::from(remote) + local;
        let ready = listeners > 0;
        self.endpoints[index].entity.set_output_reservation(
            0,
            OutputReservation {
                ready_listeners: listeners.min(usize::from(u16::MAX)) as u16,
                registering: true,
            },
        );
        let endpoint = &mut self.endpoints[index];
        if ready && endpoint.talker.is_none() {
            log(format!(
                "{}: a listener is ready, streaming",
                endpoint.config.name
            ));
            self.start_talker(index);
        } else if !ready && endpoint.talker.is_some() {
            log(format!(
                "{}: no listener ready, stopping",
                endpoint.config.name
            ));
            endpoint.talker = None;
        }
    }

    fn start_talker(&mut self, index: usize) {
        let endpoint = &self.endpoints[index];
        let Some((stream_id, destination)) = endpoint.entity.output_stream(0) else {
            return;
        };
        let Some(stream) = endpoint.entity.model().outputs.first() else {
            return;
        };
        let transit = endpoint.entity.max_transit_time(0).unwrap_or(2_000_000);
        let config = TalkerConfig {
            interface: self.interface.clone(),
            mac: self.mac,
            stream_id: stream_id.0,
            destination,
            vlan_id: VLAN,
            priority: PRIORITY,
            format: stream.current_format,
            max_transit_time: Duration::from_nanos(u64::from(transit)),
            source: endpoint.config.source(),
        };
        // An audio device that isn't there, as on another computer than
        // the one its name came from, leaves the stream running silent.
        let started = Talker::start(config.clone(), self.clock.clone()).or_else(|error| {
            if config.source == Source::Silence {
                return Err(error);
            }
            log(format!(
                "{}: {error}, sending silence",
                endpoint.config.name
            ));
            let silent = TalkerConfig {
                source: Source::Silence,
                ..config
            };
            Talker::start(silent, self.clock.clone())
        });
        match started {
            Ok(talker) => self.endpoints[index].talker = Some(talker),
            Err(error) => log(format!(
                "{}: could not start the stream: {error}",
                endpoint.config.name
            )),
        }
    }

    /// A talker declaration came or went: inputs settled on its stream
    /// declare themselves and listen, or stop.
    fn talker_heard(&mut self, stream_id: u64) {
        for index in 0..self.endpoints.len() {
            let settled = self.endpoints[index]
                .entity
                .input_stream(0)
                .is_some_and(|stream| stream.stream_id.0 == stream_id);
            if settled {
                self.update_listener(index);
            }
        }
    }

    /// Declares a listener endpoint's input as its talker's declaration
    /// allows, and listens when it is advertised. A talker of ours needs
    /// no reservation, as the bridge never sends our own declarations back
    /// to us; its frames reach the listener as they leave.
    fn update_listener(&mut self, index: usize) {
        let since = self.elapsed();
        let Some(stream) = self.endpoints[index].entity.input_stream(0) else {
            return;
        };
        let own = self.own_talker(stream.stream_id.0);
        let declaration = match own {
            Some(talker) => self.declaration(talker),
            None => self.talkers.get(&stream.stream_id.0).copied(),
        };
        let key = stream.stream_id.0.to_be_bytes().to_vec();
        let (state, reservation) = match declaration {
            Some(declaration) => (
                Some(if declaration.failure.is_some() {
                    ListenerState::AskingFailed
                } else {
                    ListenerState::Ready
                }),
                InputReservation {
                    talker_registered: declaration.failure.is_none(),
                    failure: declaration
                        .failure
                        .map(|failure| (failure.code, failure.bridge_id)),
                    accumulated_latency: declaration.accumulated_latency,
                    registering: true,
                },
            ),
            None => (None, InputReservation::default()),
        };
        match state {
            Some(_) if own.is_some() => {
                if let Some(key) = self.endpoints[index].declared.take() {
                    self.msrp_participant
                        .withdraw(since, msrp::attribute::LISTENER, &key);
                }
            }
            Some(state) => {
                self.msrp_participant.declare(
                    since,
                    msrp::attribute::LISTENER,
                    key.clone(),
                    Some(state.number()),
                );
                self.endpoints[index].declared = Some(key);
            }
            None => {
                self.msrp_participant
                    .withdraw(since, msrp::attribute::LISTENER, &key);
                self.endpoints[index].declared = None;
            }
        }
        self.endpoints[index]
            .entity
            .set_input_reservation(0, reservation);
        let advertised = state == Some(ListenerState::Ready);
        let endpoint = &mut self.endpoints[index];
        if advertised && endpoint.listener.is_none() {
            let Some(model) = endpoint.entity.model().inputs.first() else {
                return;
            };
            let config = ListenerConfig {
                interface: self.interface.clone(),
                stream_id: stream.stream_id.0,
                destination: stream.destination,
                format: model.current_format,
                sink: endpoint.config.sink(),
            };
            let started = Listener::start(config.clone(), self.clock.clone()).or_else(|error| {
                if config.sink == Sink::Discard {
                    return Err(error);
                }
                log(format!(
                    "{}: {error}, playing nowhere",
                    endpoint.config.name
                ));
                let discarding = ListenerConfig {
                    sink: Sink::Discard,
                    ..config
                };
                Listener::start(discarding, self.clock.clone())
            });
            match started {
                Ok(listener) => {
                    log(format!(
                        "{}: listening to {}",
                        endpoint.config.name, stream.stream_id
                    ));
                    endpoint.listener = Some(listener);
                }
                Err(error) => log(format!(
                    "{}: could not listen: {error}",
                    endpoint.config.name
                )),
            }
        } else if !advertised && endpoint.listener.is_some() {
            log(format!(
                "{}: the talker is gone, not listening",
                endpoint.config.name
            ));
            endpoint.listener = None;
        }
        self.update_own_talkers();
    }

    /// Our talkers, again, after one of our listeners started or stopped.
    fn update_own_talkers(&mut self) {
        for index in 0..self.endpoints.len() {
            if self.endpoints[index].config.kind == Kind::Talker {
                self.update_talker(index);
            }
        }
    }

    fn stop_listener(&mut self, index: usize) {
        let since = self.elapsed();
        let endpoint = &mut self.endpoints[index];
        if endpoint.listener.take().is_some() {
            log(format!("{}: not listening", endpoint.config.name));
        }
        endpoint
            .entity
            .set_input_reservation(0, InputReservation::default());
        if let Some(key) = endpoint.declared.take() {
            self.msrp_participant
                .withdraw(since, msrp::attribute::LISTENER, &key);
        }
        self.update_own_talkers();
    }

    fn entity_event(&mut self, index: usize, event: EntityEvent) {
        let name = self.endpoints[index].config.name.clone();
        match event {
            EntityEvent::Identify(on) => log(format!(
                "{name}: identify {}",
                if on { "on" } else { "off" }
            )),
            EntityEvent::InputBound { binding, .. } => {
                log(format!(
                    "{name}: bound to {} output {}",
                    binding.talker, binding.talker_unique_id
                ));
            }
            EntityEvent::InputSettled { stream, .. } => {
                log(format!(
                    "{name}: probed stream {} to {}",
                    stream.stream_id, stream.destination
                ));
                self.update_listener(index);
            }
            EntityEvent::InputUnsettled { .. } => self.stop_listener(index),
            EntityEvent::InputUnbound { .. } => {
                log(format!("{name}: unbound"));
                self.stop_listener(index);
            }
            EntityEvent::StreamFormatChanged { format, .. } => {
                log(format!("{name}: format {format}"));
                let endpoint = &mut self.endpoints[index];
                if endpoint.talker.take().is_some() {
                    self.declare_talker(index);
                    self.start_talker(index);
                } else if endpoint.config.kind == Kind::Talker {
                    self.declare_talker(index);
                }
                if self.endpoints[index].listener.take().is_some() {
                    self.update_listener(index);
                }
            }
            EntityEvent::MaxTransitTimeChanged { nanoseconds, .. } => {
                log(format!("{name}: max transit time {nanoseconds} ns"));
                if self.endpoints[index].talker.take().is_some() {
                    self.start_talker(index);
                }
            }
            EntityEvent::SamplingRateChanged(rate) => log(format!("{name}: sampling rate {rate}")),
            EntityEvent::ClockSourceChanged(source) => {
                log(format!("{name}: clock source {source}"))
            }
            EntityEvent::NameChanged => {
                log(format!(
                    "{name}: renamed {}",
                    self.endpoints[index].entity.model().entity_name
                ));
                self.keep_name(index);
            }
        }
    }

    /// Hands each running stream's counters to its entity, in Milan's
    /// places for them.
    fn update_counters(&mut self) {
        let low = |value: u64| value as u32;
        for endpoint in &mut self.endpoints {
            if let Some(talker) = &endpoint.talker {
                let stats = talker.stats();
                let mut counters = [0; 32];
                counters[0] = 1; // stream_start
                counters[3] = low(stats.media_resets);
                counters[5] = low(stats.frames_sent);
                counters[7] = low(stats.frames_sent);
                endpoint.entity.set_stream_counters(
                    atdecc::DescriptorType::STREAM_OUTPUT,
                    0,
                    counters,
                );
            }
            if let Some(listener) = &endpoint.listener {
                let stats = listener.stats();
                let mut counters = [0; 32];
                counters[0] = low(stats.media_locked);
                counters[1] = low(stats.media_unlocked);
                counters[2] = low(stats.interrupted);
                counters[3] = low(stats.sequence_mismatches);
                counters[6] = low(stats.frames_received);
                counters[8] = low(stats.unsupported_formats);
                counters[9] = low(stats.late);
                counters[10] = low(stats.early);
                counters[11] = low(stats.frames_received);
                endpoint.entity.set_stream_counters(
                    atdecc::DescriptorType::STREAM_INPUT,
                    0,
                    counters,
                );
            }
        }
    }

    fn report(&self) {
        for endpoint in &self.endpoints {
            if let Some(talker) = &endpoint.talker {
                let stats = talker.stats();
                log(format!(
                    "{}: sent {} frames, {} media resets, {} send errors",
                    endpoint.config.name, stats.frames_sent, stats.media_resets, stats.send_errors
                ));
            }
            if let Some(listener) = &endpoint.listener {
                let stats = listener.stats();
                log(format!(
                    "{}: received {} frames, {} out of sequence, {} late, {} early, peak {:.3}",
                    endpoint.config.name,
                    stats.frames_received,
                    stats.sequence_mismatches,
                    stats.late,
                    stats.early,
                    stats.peak
                ));
            }
        }
    }

    /// Stops the streams, withdraws the declarations and says the entities
    /// are leaving.
    fn shut_down(&mut self) {
        let since = self.elapsed();
        for endpoint in &mut self.endpoints {
            endpoint.talker = None;
            endpoint.listener = None;
            endpoint.entity.depart();
        }
        let declared: Vec<(u8, Vec<u8>)> = self
            .endpoints
            .iter()
            .filter_map(|endpoint| endpoint.entity.output_stream(0))
            .map(|(stream_id, _)| {
                (
                    msrp::attribute::TALKER_ADVERTISE,
                    stream_id.0.to_be_bytes().to_vec(),
                )
            })
            .collect();
        for (kind, key) in declared {
            self.msrp_participant.withdraw(since, kind, &key);
        }
        // Long enough for the withdrawals to go out.
        let until = since + Duration::from_millis(400);
        while self.elapsed() < until {
            let now = self.elapsed();
            self.msrp_participant.handle_timeout(now);
            self.mvrp_participant.handle_timeout(now);
            let _ = self.flush();
            std::thread::sleep(Duration::from_millis(20));
        }
        log("stopped");
    }
}

fn is_talker(attribute_type: u8) -> bool {
    matches!(
        attribute_type,
        msrp::attribute::TALKER_ADVERTISE | msrp::attribute::TALKER_FAILED
    )
}

/// The stream ID a talker or listener value starts with.
fn stream_of(value: &[u8]) -> u64 {
    let mut octets = [0; 8];
    let length = value.len().min(8);
    octets[..length].copy_from_slice(&value[..length]);
    u64::from_be_bytes(octets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_formats_are_the_bench_ones() {
        assert_eq!(aaf_48k(8, 32), StreamFormat(0x0205_0220_0200_6000));
        assert_eq!(aaf_48k(8, 24), StreamFormat(0x0205_0218_0200_6000));
    }

    #[test]
    fn ids_stay_apart_from_the_apps() {
        let mac = MacAddress([0xf0, 0xa7, 0x31, 0xf4, 0x0f, 0x14]);
        assert_eq!(entity_id(mac, 0), EntityId(0xf0a7_31ff_00f4_0f14));
        let (stream, destination) = output_stream(mac, 2, 0);
        assert_eq!(stream, StreamId(0xf0a7_31f4_0f14_0200));
        assert_eq!(
            destination,
            MacAddress([0x91, 0xe0, 0xf0, 0x00, 0x14, 0x02])
        );
    }
}
