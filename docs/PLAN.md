# triib: design and plan

triib is a lightweight ATDECC (IEEE 1722.1-2021) controller with first class
Milan 1.3 and AVB Lite support, for Linux, Windows and macOS, in the spirit
of Hive. It discovers entities, shows and edits their entity model, connects
streams, manages media clocks and diagnoses problems. On a host with a
hardware timestamping Ethernet interface it can also spawn its own talkers
and listeners, routed to the host's audio inputs and outputs, which appear in
the connection matrix like any other entity. Written in Rust with iced,
sharing prev's look, widgets and window layout.

## Guiding priorities

1. Correct on the wire: every frame we send is checked against 1722.1-2021,
   Milan 1.3 and the AVB Lite profile, and every frame we parse is tested
   against captures from real devices.
2. Small footprint: few dependencies, no async runtime, idle CPU near zero.
3. One portable core: protocol logic is plain Rust shared by every platform;
   platform code is confined to a thin layer.
4. Features follow capabilities, not operating systems: a feature is
   available wherever a backend can provide what it needs, and the
   architecture must never rule a platform out of one.

Hive is the reference for controller features; prev for look, layout and
code style; L-Acoustics avdecc is reference and test oracle, not a
dependency.

## Decisions

| Area | Decision |
|---|---|
| Language | Rust stable, edition 2024, same toolchain floor as prev (1.95) |
| GUI | iced 0.14, same features and vendor patches as prev |
| Design | prev's Material 3 Expressive layer, Omarchy and system accent, from `scramble-ui`, a crate shared with prev |
| License | MIT OR Apache-2.0 (see Licensing) |
| Host endpoints | One ATDECC entity per spawned talker or listener |
| Controller entity | triib advertises itself as a controller (valid time 62 s, entity model ID `0x8c1f6436c0000001` under the Scramble Tools MA-S `8C-1F-64-36-C`), answers CONTROLLER_AVAILABLE, and registers for unsolicited notifications from each entity it reads |
| ATDECC | Our own Rust stack, controller and entity roles, in the reusable `atdecc` crate, see below |
| PTP | Decided by the PTP analysis that opens milestone 7 (see PTP analysis): our own Rust gPTP (802.1AS-2020) and AVB Lite PTP engine as the reusable `avb-ptp` crate, following linuxptp where it runs, or both, and in what order |
| SRP | Our own MRP, MSRP and MVRP in the reusable `avb-mrp` crate; MAAP with AVTP streaming; AVB Lite CVU SRP framing in `atdecc`, its attribute lists in `avb-mrp` |
| Concurrency | No tokio. Network thread per interface with a poll loop and timer wheel; real time threads for streaming and audio; `std::sync::mpsc` into an iced subscription (prev's `External` + `post()` pattern) |
| Processes | GUI process (controller) and an optional `triib-endpointd` process for host talkers and listeners, so audio survives GUI restarts and only the daemon holds the extra privileges |
| Platform layer | `avb-net`: raw Ethernet frames, interfaces, hardware timestamps and launch time per operating system, the only platform code the protocol crates use. Audio (cpal) and clock discipline live in triib's streaming crates. Everything else is shared |
| Settings, cache, presets | TOML settings via a `prev-store` style crate; the entity model cache keeps raw descriptor bytes, decoded by `atdecc` on load, so the protocol crates need no serde; JSON presets |
| Interface text | Fluent `i18n/` like prev, English only at first |

## ATDECC: our own stack, not la_avdecc

Using L-Acoustics avdecc (what Hive uses) would get years of field quirks
for free, but:

- It gates our core features. Host talker and listener entities, AVB Lite
  CVU SRP, the AVB Lite status query and any vendor extensions would mean
  patching a C++ library we do not control, or keeping a fork.
- Its focus is the controller. Being a full Milan talker or listener entity
  (AEM responder, Milan listener binding state machine, MVU) would be our
  code either way, so we would carry two ATDECC implementations.
- C++17, CMake and its own threads and pcap backend behind FFI, on six
  targets including RISC-V, works against the small, cargo-only build.
- LGPL-3.0 means dynamic linking or shipping relinkable objects with a
  static Rust binary.
- We already have both halves in hand: the entity side in esp_avb, the
  controller side in `tools/atdecc_controller.py`, and the standards on disk.

Mitigating the quirks risk: golden captures from every device we can reach,
Hive running beside triib on the bench as an oracle (diff its entity model
JSON dumps against ours), fuzzing the codec, and a review point after the
engine milestone. No abstraction layer to swap la_avdecc in later; it would
bend our model toward theirs.

## Reusable protocol crates

The protocol implementations are crates of their own, meant for other
applications with very different constraints (a blocking CLI, an async
daemon, a GUI, firmware on a microcontroller), not only for triib:

- **`atdecc`**: IEEE 1722.1-2021 with Milan 1.3 and the AVB Lite vendor
  unique messages: frame codecs, controller state machines (discovery,
  AECP command tracking, enumeration, ACMP, unsolicited notifications) and
  entity state machines (AEM responder, Milan listener binding, ACMP
  talker), and the entity model as data.
- **`avb-mrp`**: IEEE 802.1Q MRP (applicant, registrar, LeaveAll, periodic
  timers), MSRP and MVRP. MSRP attribute lists are also usable outside
  MRPDUs, for AVB Lite's CVU SRP.
- **`avb-net`**: raw Ethernet per operating system: interfaces, frames,
  multicast membership, hardware timestamps, launch time. The only crate
  with unsafe code, and the only dependency of the two above.
- Later on the same rules: `avb-ptp` (gPTP, AVB Lite PTP profile and its
  fallback detection), and perhaps an AVTP streaming crate with MAAP.

Rules for these crates:

1. **Sans-I/O core.** No sockets, threads, clock or runtime in the protocol
   code. The caller feeds received frames and the current time
   (`handle_frame(now, bytes)`, `handle_timeout(now)`) and drains frames
   to send, events and the next deadline (`poll_transmit`, `poll_event`,
   `poll_timeout`). Commands return an ID and complete as an event. This
   fits a blocking loop, tokio, embassy or a firmware main loop alike.
2. **`no_std`.** Codecs need neither std nor alloc: fixed-size PDUs decode
   into plain values, variable ones as views over the received bytes, and
   both encode into the caller's buffer. Reserved values are kept, so a
   frame decodes and encodes back unchanged. State machines need `alloc`
   only. The `std` feature (default) adds the
   `avb-net` transports and a small blocking driver for simple apps.
3. **No dependencies beyond `avb-net`**, and `avb-net` only depends on
   what its platform needs (for example `libc` on Linux and macOS, `pcap`
   on Windows). No serde, no logging framework, no async runtime.
4. **Never panic on input.** Malformed frames are errors, fuzzed in CI.
5. **No knowledge of each other.** Glue between protocols, such as CVU SRP
   (`atdecc` framing around `avb-mrp` attribute lists) or MSRP state in a
   Milan stream input's flags, lives in the application.
6. **Enforced in CI**: each crate tested on its own, and built with
   `--no-default-features` for `riscv32imac-unknown-none-elf` (the ESP32-C6).
7. **Published** to crates.io under these names once the API settles, with
   their own semver; they stay in this workspace until outside users need
   them elsewhere.

## Workspace layout

```
crates/
  atdecc          reusable: ATDECC frames, controller and entity machines
  avb-mrp         reusable: MRP, MSRP, MVRP
  avb-net         reusable: raw Ethernet, interfaces, timestamps per OS
  avb-ptp         reusable, later: gPTP and AVB Lite PTP
  triib-store     settings and cache files
  triib-stream    AVTP AAF and AM824 talker and listener, CRF listener,
                  MAAP, presentation time, media clock recovery, drift and
                  ASRC, audio through cpal
  triib-endpointd daemon hosting spawned talkers and listeners
  triib-sim       virtual entities and networks for tests and GUI work
  triib-cli       headless controller, parity with atdecc_controller.py
  triib           the iced app, including the entity store the GUI draws
```

## Platform layer

| Capability | Linux | macOS | Windows |
|---|---|---|---|
| Controller frames (`0x22F0`) | `AF_PACKET` | BPF, or the AudioVideoBridging framework (investigation) | Npcap via `pcap` |
| Hardware timestamps, PTP frames | `SO_TIMESTAMPING` on `AF_PACKET`, PHC via `/dev/ptpN` | OS owns gPTP (investigation) | Not available to us yet |
| Paced transmit | `SO_TXTIME` with ETF/taprio launch time, CBS qdisc, or user space pacing | Investigation | Not yet |
| Audio | cpal (PipeWire, ALSA, JACK) | cpal (Core Audio) | cpal (WASAPI) |
| Capability probe | `ETHTOOL_GET_TS_INFO`, PHC index | Investigation | None |

The app asks `avb-net` what an interface can do and shows the
answer: host endpoints are enabled per interface when it reports hardware
timestamps and a usable time source, with the reason shown when not.

Linux privileges: the GUI needs `CAP_NET_RAW`; `triib-endpointd` needs
`CAP_NET_RAW`, `CAP_NET_ADMIN` (hardware timestamping config, qdiscs) and
write access to the PHC (`CAP_SYS_TIME` or a udev rule for `/dev/ptp*`). If
ptp4l already disciplines the PHC, the daemon follows it instead of running
its own servo.

## macOS investigation

macOS runs its own AVB stack (its own entity, gPTP and MSRP) on interfaces
where AVB is enabled, so our access to PTP and AVTP traffic and to hardware
timestamps is uncertain. Before writing the macOS backend, on the Mac mini:

1. With AVB on and off in Audio MIDI Setup: can BPF receive and send
   `0x22F0` and `0x88F7` frames? Do responses to our controller entity
   arrive? Any clash between our entity and the OS's on the same MAC?
2. AudioVideoBridging framework (`AVB17221EntityDiscovery`, AECP and ACMP
   interfaces, `AVBInterface`): complete enough for the controller, as
   la_avdecc's macOS native interface suggests? Through objc2, which prev
   already uses.
3. Time: can user space read the OS's gPTP time and the PHC relation, or
   get hardware timestamps at all?
4. Host audio over AVB: if (3) fails, macOS host endpoints are the OS's
   own AVB audio device, controlled by triib like any entity, with its
   limits documented; otherwise our own `triib-endpointd` as on Linux.

The result decides only the macOS backends in `avb-net` and the clock
source for host endpoints; nothing above them changes.

## PTP analysis

Host endpoints need PTP: answering peer delay so the bridge treats the
link as asCapable (without it reservations stop there, as the network
view shows for this computer today), a disciplined hardware clock for
presentation times, and gPTP state for each host entity's GET_AVB_INFO,
GET_AS_PATH and GET_COUNTERS. The controller needs none of this; it only
listens to the bridge. Before shaping `avb-ptp`, milestone 7 answers:

- **linuxptp (ptp4l, phc2sys, pmc):**
  - gPTP as an end station: its 802.1AS settings, asCapable and
    neighborPropDelayThresh handling, the follow-up information TLV,
    path trace, and behaving as timereceiver only and as BTC.
  - What its management socket gives, and whether it is enough for host
    entities: time status and offset, BTC identity, port state, peer
    delay, asCapable, the path trace list, and event subscription for
    changes.
  - The AVB Lite profile: layer 2 end to end with Sync at log -3 fits;
    VLAN 0 priority 7 tagging, the endpoint declaration TLV and beacons,
    and switching profiles on fallback likely do not. Can those live
    beside ptp4l, or does fallback need our own engine?
  - Sharing the PHC: ptp4l disciplines it, our daemon reads it; versions
    in the distributions triib targets; GPL-2.0 as a separate process.
  - Tried on the bench (2026-10-04, linuxptp 4.4, the TX401 at 1 Gb/s to
    the MOTU switch, gPTP settings with gmCapable 0 and priority 255):
    locked in about 10 s, offset 7 to 14 ns RMS, peer delay about 302 ns,
    asCapable. Its read-only management socket (`/var/run/ptp4lro`, open
    to every user) answers without root when asked with transportSpecific
    1 (`pmc -t 1`): TIME_STATUS_NP (offset, BTC present and identity),
    PORT_DATA_SET (port state, peer delay), PORT_DATA_SET_NP (asCapable),
    PARENT_DATA_SET (the bridge port, BTC priorities and class) and
    CURRENT_DATA_SET (steps removed). pmc does not decode
    PATH_TRACE_LIST; whether ptp4l answers it is still to check. Despite
    gmCapable 0 it took the timetransmitter role for a few seconds at
    start, before hearing the BTC; whether it sent Announce or Sync then
    is still to check.
- **Windows:**
  - The Windows Time Service PTP client (UDP, end to end): useful at all
    for gPTP or AVB Lite at layer 2?
  - Hardware timestamps: the Winsock and NDIS timestamping support in
    recent Windows (UDP only?), Npcap's timestamp types for layer 2, and
    Intel driver interfaces as OpenAvnu's Windows gPTP used.
  - Whether any path gives layer 2 PTP with hardware timestamps, or
    Windows host endpoints wait.
  - Tried on the bench (2026-10-05, Windows 11 26H2, an Acemagic AM06
    Pro): its Intel I226-V (Intel driver e2fn 2.1.5.7, NDIS 6.89) and
    Realtek RTL8111 (Microsoft's driver 1.0.0.14) both answer
    GetInterfaceSupportedTimestampCapabilities with ERROR_NOT_SUPPORTED,
    so Windows sees neither as timestamping, not even in software. The
    I226 driver has no PTP Hardware Timestamp setting; Intel's forum names
    the I210, I211, I350 and E810 drivers as the ones with it. The API's
    hardware flags name PTP v2 over UDP only, so layer 2 gPTP would rely
    on its all-receive and tagged-transmit flags; trying that needs one
    of those cards.
  - Earlier gPTP on Windows: Avnu's gptp daemon (from Intel, last
    changed 2018) sent and received with WinPcap and read hardware
    timestamps through private Intel driver OIDs (OID_INTEL_GET_TXSTAMP,
    GET_RXSTAMP, GET_SYSTIM), turned on by a hidden `TimeSync` registry
    value; its clock table lists the I217-LM and I219-V, and it also ran
    over Intel 8260 Wi-Fi. Its notes say some Windows 10 versions stopped
    WinPcap from sending. The I226's driver (e2fn 2.1.5.7) has no trace
    of that path: no `TimeSync` or PTP strings, though setting names
    such as `*JumboPacket` show, and none of the four OID values.
    Intel's current I210 driver (e1r 14.1.24.0, in package 31.2.2) keeps
    it: `TimeSync` sits in its registry table beside
    `*PtpHardwareTimestamp` and the hidden `AllTransmitHw` and
    `TaggedTransmitHw`, all four OID values appear, and it binds any
    I210 by device ID, with flash (1533) or without (157B), though not
    one with a blank NVM (1531). Npcap does not pass on NDIS hardware
    timestamps yet (its issue 581), so the standard path at layer 2
    would need our own NDIS driver. The I219 driver is still to check.
    Meinberg's commercial PTP Client has the 802.1AS profile on Windows,
    with hardware timestamps only on Oregano syn1588 cards.
- **macOS** (with the macOS investigation): the OS's own gPTP when AVB is
  enabled, what the AudioVideoBridging framework and IOKit expose of its
  time, BTC and path, whether BPF can send and receive `0x88F7` beside
  it, and whether any hardware timestamp reaches user space.

The answers decide whether `avb-ptp` is a full stack on every platform, a
layer over the platform's PTP where one exists, or both, and which comes
first. esp_ptp (C, both profiles) is the reference for our own engine.

Testing host endpoints also needs a card with a hardware clock on the AVB
network. This computer's is a TP-Link TX401 (Marvell AQtion AQC107, Linux
`atlantic`, firmware 3.1.100): hardware transmit and receive timestamps,
the PTP v2 layer 2 event filter, two-step only, PHC `ptp0`. Its PTP
depends on the card's firmware enabling it, which retail AQC107 cards do
not all do; whether it can pace transmission (launch time, CBS) is not
known yet. Its Realtek RTL8125 under r8169 exposes no PHC. Intel i210,
i225 and i226 are the known choices with launch time.

## Host talkers and listeners

- Shown in the Entities view like any entity, with a filter for host
  entities and buttons there to add and remove them; adding is offered on
  interfaces with a hardware clock and a wired link. The inspector edits a
  host entity's configuration (channels, format, audio routing).
- Spawned from the app (or saved in a preset), each one its own ATDECC
  entity with a stable entity ID derived from the interface MAC and an
  instance number, advertised on the wire, so Hive and other controllers
  see and connect them too.
- Each has a Milan entity model: one stream (talker) or stream sink
  (listener), configurable channel count, sample rate and format (AAF
  PCM, AM824 for gear like the MOTU 8D), audio cluster and map, identify
  control, counters, names.
- Each is routed to a host audio device and channel range through cpal.
  The audio device's clock is not gPTP locked, so each endpoint estimates
  drift against the media clock and resamples (`rubato`) at the boundary.
- Transmission is paced per stream: hardware launch time where the NIC
  supports it (i210, i225, i226), else CBS, else user space pacing.
- Reservations through MSRP and MVRP on an AVB network; AVB Lite on a plain
  network (standard PTP, CVU SRP, unicast by default, MAAP on escalation).
- Local frames between the controller and local entities go through an
  in-process router as well as the wire, so behaviour matches remote
  entities exactly.

## AVB Lite

Per `profiles/avb_lite.md`:

- **Capable vs active:** show both, per entity, in the list and inspector,
  plus the reason fallback fired when known, PTP profile, domain and media
  VLAN.
- **Detection today:** CVU SRP talker declarations are broadcast in the
  media VLAN, so active Lite talkers are visible anywhere on the segment
  (the receive path must keep VLAN tags, `PACKET_AUXDATA` on Linux).
  Endpoint Declaration TLVs and beacons only reach us when no AVB bridge
  is between us and the device. Lite listeners and Lite-capable devices
  still in AVB mode are invisible.
- **Gap in the profile:** section 2.2 says a device must advertise its mode
  to the controller but defines no mechanism. Proposal: an AECP vendor
  unique query under the AVB Lite MA-S OUI (sub-protocol `0x003`, next
  after CVU SRP's `0x002`) returning capable, active, fallback reason, PTP
  profile and domain, media VLAN and unicast fan-out, with unsolicited
  notification on change. Implement in esp_avb, triib's own endpoints and
  the profile document together.
- **Profile inconsistency to settle:** section 4 and section 9 put admission
  control in the controller ("bandwidth ledger", refuse past 75%), while
  section 6 puts it in the endpoints (CVU SRP, talker 75% rule). triib can
  be the AVB Lite controller either way: a per-link bandwidth view from
  CVU declarations, warnings near 75%, PTP offset alarms above 50 us.
- **Matrix:** Lite connections show transport (unicast, fan-out count,
  escalated to multicast) and mismatches such as AVB talker to Lite
  listener without a gateway.

## Reuse from prev

`scramble-ui`, in its own repository, MIT OR Apache-2.0, used by both
prev and triib:

- Moves out of prev: `ui/` (except `export.rs`, which is prev's),
  `omarchy.rs`, the settings side of `portal.rs`, the right to left aware
  `row!`/`column!` macros, the fonts, and the vendored iced, winit and
  smithay-clipboard patches.
- Decoupled from prev: user-facing strings (toolbar "more", "close", keep
  shown and auto hide) become parameters or a small Fluent bundle of the
  crate's own; the seed color becomes an argument.
- Cargo applies `[patch.crates-io]` only in the top level workspace, so
  prev and triib each carry the same patch section pointing at the vendored
  crates in the shared repository (git dependency, pinned by revision).
- Copied, not shared: `i18n.rs`, `instance.rs`, the `External`/`post()`
  pattern, the settings view and shortcuts patterns, `prev-store`, release
  scripts, packaging, CI, `deny.toml`, lints. Promote any of them to the
  shared crate if they stop diverging.

Left behind: PDF, image, Markdown, drag and drop, clipboard, printing and
the assistant.

## Licensing

triib is MIT OR Apache-2.0. Things to keep that true:

- **The shared UI crate** is AGPL today as part of prev. Every commit to
  the shared code is by Scramble Tools, so it can be relicensed MIT OR
  Apache-2.0; prev stays AGPL and may use it. Any outside contribution to those
  files before the split would need its author's consent.
- **Dependencies**: iced, wgpu, cosmic-text, material-colors, i18n-embed,
  rust-embed, ashpd, smithay-clipboard (MIT or MIT/Apache), winit (Apache),
  interprocess (0BSD/Apache), rustix (Apache/MIT), cpal (Apache), rubato
  (MIT), pcap (MIT/Apache) are all compatible. MPL-2.0 (for example
  `audio_thread_priority`) is fine: file level copyleft, unmodified use.
- **System libraries** loaded dynamically: libasound and JACK (LGPL) through
  cpal are fine as long as they stay dynamically linked; libpcap is BSD.
- **Npcap** (Windows) is proprietary and may not be redistributed without an
  OEM license: users install it themselves, as with Hive and Wireshark.
- **Avoid**: GPL or AGPL crates, and statically linked LGPL. triib's
  `deny.toml` allows only permissive licenses plus MPL-2.0, unlike prev's.
- **Reference code**: la_avdecc (LGPL-3.0) and Hive (GPL) are for
  behaviour, never copied; the IEEE and Avnu specifications are the source
  for implementation. Do not paste standard text into code or docs beyond
  field names and short citations.
- **Fonts**: Roboto Flex (OFL-1.1) and Material Symbols (Apache-2.0) ship
  with their license files; the vendored iced crates carry iced's MIT
  notice (missing from prev's copies).
- **Trademarks**: "Milan" and "AVB" belong to Avnu Alliance. Say "Milan
  compatible", never "certified", unless certified.
- **Why dual**: the Rust convention, and Apache adds an explicit patent
  grant from contributors, which matters for a protocol implementation.

## Window layout

- **Toolbar**: interface picker with link and timestamping capability,
  discovery indicator, search, view switcher (Matrix, Entities, Network,
  Media clock).
  Tools that do not fit move into a "More" menu, as in prev.
- **Content**: the active view, Matrix by default. Each view selects
  entities its own way (the matrix's headings, the entity table, the
  network's cards); there is no entity sidebar.
- **Right inspector** (toggle, like prev's Ctrl+I): the entity selected in
  any view: identity, AEM tree,
  editable properties, channel mappings, controls, counters; for host
  endpoints also audio routing, drift, buffer and PTP state.
- **Channel mappings**, in the inspector: for each stream port with
  dynamic mappings, a picker for each channel, on an input the stream
  channel each cluster channel takes and on an output the cluster channel
  each stream channel sends; re-mapping a channel removes its mapping
  before adding the new one, as an entity may refuse a second. Fixed
  mappings show as text, a run of channels to a line. A grid editor for
  entities with many channels can follow.
- **Status bar**: interface, entity count, BTC, PTP offset when we run PTP,
  pending commands, last error.
- **Narrow windows**, down to a phone's width: the toolbar keeps the
  interface picker and the inspector button with the rest under "More",
  the inspector takes the view's place until closed, the matrix narrows
  its row headings, tables scroll sideways, and the status bar keeps the
  entity count and state.

## Use cases, prioritized

### P0: controller daily driver

1. Choose an interface; entities appear and leave live.
2. Entity list with name, ID, model, firmware, group, Milan version, AVB
   Lite capable and active, clock domain, BTC, online state.
3. Stream matrix: connect and disconnect, status per cell.
4. Identify.
5. Inspector: AEM tree and dynamic values.
6. Edit names, stream format, sampling rate, clock source.
7. Unsolicited notifications; changes from Hive or the Mac mini show live.
8. Entity model cache: models kept by entity model ID, firmware and
   configuration. When an entity of a kept model comes online, triib reads
   its ENTITY, CONFIGURATION and the descriptors that differ between
   entities (AVB interfaces, clock sources), and fetches names, stream
   formats, sampling rates and clock sources with GET_DYNAMIC_INFO, or
   reads those descriptors when the entity does not answer it. Strings,
   locales, static maps and stream ports are never read again.

### P1: Hive parity and AVB Lite control

- Media clock view: each media clock reference with the clock domains
  following it, as a tree through the streams carrying its clock, each
  with where its clock comes from, whether that stream flows, its rate
  against the reference's and its Milan media clock reference priority
  and domain name (GET_MEDIA_CLOCK_REFERENCE_INFO); then the domains whose
  chain breaks (an unbound input, an unknown talker, a loop) and why. The
  clock source of each is picked where it shows. Electing a reference by
  priority and connecting the CRF streams to it can follow.
- Channel routing via audio maps (in the inspector,
  read with GET_AUDIO_MAP, changed with ADD_AUDIO_MAPPINGS and
  REMOVE_AUDIO_MAPPINGS, kept current from their notifications), generic
  CONTROL editors, diagnostics (Milan counters, gPTP info, MSRP failure
  codes).
- Network view: the gPTP tree built from each entity's GET_AS_PATH, with
  bridges as nodes and each link's peer delay, asCapable and link
  counters (`triib-cli network` prints it as text). The grandmaster sits
  on top with its own devices in a middle column; the bridges under it
  form a row, each heading a column of its devices. It shows the gPTP
  clock, the audio streams or the media clock (CRF) streams, each stream
  on a wire of its own coloured by its talker, entering and leaving every
  bridge it crosses; failed and advertised streams stop at the bridge
  where they stop, with a mark. Clicking a wire or a device brings it
  forward, and the panel beside the map explains it. Narrow windows show
  the map or the panel, and stack the map when its columns would be too
  small. Switch ports need LLDP tables over SNMP, a later addition.
- Log view: what the controller did and heard, with warnings for
  entities that break the rules, such as a control_data_length longer
  than the frame. The decoder accepts those frames, so it needs to report
  how many octets were claimed but missing.
- AVB Lite status query, bandwidth view and alarms.
- Presets, `triib-cli` at parity with `atdecc_controller.py`.

### P1: host endpoints on Linux

- `triib-endpointd` with gPTP and AVB Lite PTP, MSRP, MVRP, CVU SRP.
- Spawn talkers and listeners, AAF and AM824, audio routing via cpal.
- Show them in the matrix and inspector; save them in presets.

### P2: later

- Host endpoints on macOS (per the investigation) and Windows (when a
  hardware timestamp path exists).
- Firmware update over MVU, several interfaces at once, decoded packet log,
  MCP server, CRF talker, PipeWire native nodes per endpoint.

### Not in scope

Dante, AES67, video streams, acting as an AVB bridge or AVB/Lite gateway.

## Milestones

1. **Scaffold**: workspace, lints, CI, prev's UI layer, empty window.
2. **Wire**: codecs with round trip property tests and golden tshark
   captures (MOTU 8D, ESP endpoints in AVB and AVB Lite mode, Mac mini,
   Hive).
3. **Engine + CLI** on Linux: discovery, AECP, enumeration, ACMP,
   unsolicited; checked on the bench against Hive. Review the own-stack
   decision here.
4. **GUI P0**.
5. **macOS investigation** (can run alongside 3 and 4).
6. **AVB Lite controller features**, with the status query in esp_avb.
7. **Host endpoints on Linux**: the PTP analysis first, then PTP, SRP,
   local entity engine, streaming, audio, daemon.
8. **Ports and packaging**: macOS and Windows backends, packages with
   capabilities set, release pipeline from prev.

## Footprint targets

- GUI binary under 20 MB; daemon under 10 MB.
- Cold start to window under 300 ms.
- Idle CPU under 1% with 200 entities; memory under 100 MB.
- Host endpoint: under 5% of one core per 8 channel 48 kHz stream.
- Measured in CI with `triib-sim`.

## Open decisions

- No Flatpak: raw sockets and PHC access cannot be granted in a sandbox.
- Milan only, or also plain 1722.1 devices with reduced features.
