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
| Virtual endpoints | One ATDECC entity per spawned talker or listener |
| Controller entity | triib advertises itself as a controller (valid time 62 s, entity model ID `0x8c1f6436c0000001` under the Scramble Tools MA-S `8C-1F-64-36-C`), answers CONTROLLER_AVAILABLE, and registers for unsolicited notifications from each entity it reads |
| ATDECC | Our own Rust stack, controller and entity roles, in the reusable `atdecc` crate, see below |
| PTP | Decided by the PTP analysis that opens milestone 7 (see PTP analysis): our own Rust gPTP (802.1AS-2020) and AVB Lite PTP engine as the reusable `avb-ptp` crate, following linuxptp where it runs, or both, and in what order |
| SRP | Our own MRP, MSRP and MVRP in the reusable `avb-mrp` crate; MAAP with AVTP streaming; AVB Lite CVU SRP framing in `atdecc`, its attribute lists in `avb-mrp` |
| Concurrency | No tokio. Network thread per interface with a poll loop and timer wheel; real time threads for streaming and audio; `std::sync::mpsc` into an iced subscription (prev's `External` + `post()` pattern) |
| Processes | GUI process (controller) and an optional `triib-endpointd` process for host talkers and listeners, so audio survives GUI restarts and only the daemon holds the extra privileges |
| Platform layer | `avb-net`: raw Ethernet frames, interfaces, hardware timestamps and launch time per operating system, the only platform code the protocol crates use. Audio (cpal) and clock discipline live in triib's streaming crates. Everything else is shared |
| Settings, cache, presets | TOML settings via a `prev-store` style crate; the entity model cache keeps raw descriptor bytes, decoded by `atdecc` on load, so the protocol crates need no serde; TOML presets beside the settings |
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
answer: virtual endpoints are enabled per interface when it reports hardware
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
4. Host audio over AVB: if (3) fails, macOS virtual endpoints are the OS's
   own AVB audio device, controlled by triib like any entity, with its
   limits documented; otherwise our own `triib-endpointd` as on Linux.

The first two questions come before the macOS controller backend (P2),
the last two before virtual endpoints on macOS (P4).

Answered for P2 (2026-10-07, AVB on): BPF receives and sends `0x22F0`
and `0x88F7` frames, and responses to triib's controller arrive, so the
controller uses BPF and the framework was not needed. The system's own
entity is heard but cannot be commanded from the same Mac, as nothing
written to BPF reaches it; the framework may be the way to read it
there, later. triib's entity ID and the system's differ on the same MAC. The result decides
only the macOS backends in `avb-net` and the clock source for virtual
endpoints; nothing above them changes.

## PTP analysis

Virtual endpoints need PTP: answering peer delay so the bridge treats the
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
    Windows virtual endpoints wait.
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

Testing virtual endpoints also needs a card with a hardware clock on the AVB
network. This computer's is a TP-Link TX401 (Marvell AQtion AQC107, Linux
`atlantic`, firmware 3.1.100): hardware transmit and receive timestamps,
the PTP v2 layer 2 event filter, two-step only, PHC `ptp0`. Its PTP
depends on the card's firmware enabling it, which retail AQC107 cards do
not all do; whether it can pace transmission (launch time, CBS) is not
known yet. Reading its PHC now and then gives a time 2^32 ns off, a
torn read of its two halves (6 strays in 324,000 reads over 3 minutes),
or about 167 us off for a tenth of a second; the media clock takes the
median of a burst of readings and follows a move of more than 50 us
only once three measurements in a row show it. Its Realtek RTL8125 under r8169 exposes no PHC. Intel i210,
i225 and i226 are the known choices with launch time.

## Virtual endpoints: host talkers and listeners

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
- Local frames between the controller and local entities go over the
  wire, which Linux sockets hear when bound to every protocol, so
  behaviour matches remote entities exactly.

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
- **Mode reporting:** section 2.2 said a device must advertise its mode
  to the controller but defined no mechanism; section 2.4 now does: an
  AECP vendor unique query under the AVB Lite MA-S OUI (sub-protocol
  `0x003`, next after CVU SRP's `0x002`), GET_LITE_STATUS, returning
  capable, active, fallback reason, PTP profile and domain, media VLAN,
  unicast fan-out, link speed, committed egress, grandmaster and offset,
  with an unsolicited response on change. triib asks it; esp_avb and
  triib's own endpoints still have to answer it.
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
   Lite capable and active, clock domain, BTC, online state. Each column
   after the name is picked from its heading's menu, which also removes
   it or moves it left or right, and a last heading adds one. Columns
   resize by dragging a heading's right edge, a double click there fitting
   the text again. Fields, order and widths are kept in triib.toml. The
   heading row is shaded, and the Media clock cell picks the clock source.
3. Stream matrix: connect and disconnect, status per cell.
4. Identify.
5. Inspector: AEM tree and dynamic values, in tabs under its title:
   the entity (names, product, media clock, advertisement), its streams
   and channel mappings, its controls, its interfaces and counters, and
   its descriptors.
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

- Media clock in the entity list: each entity's Media clock cell picks
  its clock domain's clock source, in place of a view of its own (which
  showed each media clock reference with the domains following it, and
  went as the list does the job). atdecc still works out each domain's
  chain (media_clock), with GET_MEDIA_CLOCK_REFERENCE_INFO's priority and
  domain name, for `triib-cli clocks`. Electing a reference by priority
  and connecting the CRF streams to it can follow.
- Channel routing via audio maps (in the inspector,
  read with GET_AUDIO_MAP, changed with ADD_AUDIO_MAPPINGS and
  REMOVE_AUDIO_MAPPINGS, kept current from their notifications).
- Controls, in the inspector: each CONTROL descriptor with what sets it,
  a slider for a linear value (sent once let go, snapped to its step), a
  switch for one that only goes from off to on, a picker for a selector,
  and the value alone for read only, array, text and other controls;
  values in their units. SET_CONTROL responses and notifications from
  other controllers update the model in place without reading the
  descriptor again. The identify control is left to the Identify button.
  `triib-cli controls` and `control` show and set them. Editing arrays,
  text and Bode plots, and meters drawn as bars, can follow.
- Diagnostics, in the inspector: each clock domain's, stream input's and
  stream output's counters in words, what went wrong marked (lost locks,
  interruptions, late or early frames, media resets), read once an entity
  is read and kept current from the GET_COUNTERS notifications entities
  send to registered controllers as counters change; stream output
  counters numbered by the entity's Milan version, as Milan 1.3 renumbered
  them. Each bound input's accumulated latency, or why and where its
  talker's reservation failed, by MSRP failure code and bridge. gPTP state
  and interface counters show with each AVB interface.
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
- Log, in a panel under whichever view shows, beside the inspector,
  opened from the toolbar beside the inspector's button and resized from
  its top edge (its height kept in the settings, the view keeping room
  above it): the ATDECC frames the controller sent and heard, newest
  first, the newest 5000 kept, each with its time, direction, entity and
  what it says, refusals marked; filtered by protocol or to warnings,
  paused and cleared, a line opening to its octets. Warnings mark frames
  that do not decode, a control_data_length claiming octets past the end
  of the frame (the decoder accepts those, and `pdu::missing_octets`
  counts them), and the long ACMP form from a Milan entity. The blocking
  driver keeps the frames when asked. Saving the log, and warnings for
  timing rules such as an ENTITY_DISCOVER answered late, can follow.
- AVB Lite: each entity is asked GET_LITE_STATUS once read, and every
  5 seconds while it answers, to follow its offset; one that says it
  does not implement it is not asked again, and unsolicited responses
  update it in place. CVU SRP talker declarations are heard from any
  entity and kept 30 seconds past the last, so a talker that declares
  shows as running AVB Lite without the query. The entity list has AVB
  Lite and Egress columns, the inspector's Entity tab how each interface
  runs (mode, why it fell back, PTP profile and domain, offset from the
  grandmaster, media VLAN, unicast fan-out, link), and its Diagnostics
  tab what the talker streams take of the link: as the entity reports
  it, else worked out from the formats of its connected outputs.
  Alarms for an offset past 50 us and egress past 75% show in
  Diagnostics and on the status bar, which opens the entity. The log
  describes the query and CVU SRP. The matrix showing Lite transport
  (unicast, fan-out count, escalated to multicast) can follow, with
  endpoints that answer the query to try it on.
- Presets, from a toolbar button: each a TOML file in the data folder
  (copyable between computers) keeping each entity's clock sources,
  sampling rates, stream formats, settable level and selector controls
  and stream input bindings, entity IDs and formats as hex. Recalling
  one sends what differs, formats, rates and clock sources first, then
  controls, then unbinds and binds, and names the entities it holds that
  are not here. Virtual endpoints join them with P3.
- `triib-cli` at parity with `atdecc_controller.py`: besides reading,
  naming, formats, rates, clock sources, mappings and controls, it
  connects and disconnects, tells a talker to stop sending (DISCONNECT_TX),
  identifies, prints each stream's state with what the talker sends
  (GET_TX_STATE) and its max transit time (GET_MAX_TRANSIT_TIME, read
  from Milan talkers once read and settable), reads a descriptor again
  and prints its octets, and times a read of each descriptor type. The
  inspector shows each stream output's max transit time.

### P1.1: every language prev speaks

- triib's text in all 38 languages prev has, kept as prev keeps them:
  Project Fluent files under `i18n/<language>/`, loaded with i18n-embed,
  the `fl!` macro checking each key against English when triib compiles,
  and English standing in for keys a language lacks. Adding a language
  is adding its folder.
- The interface language follows the system or is picked in Settings, as
  now; right to left languages mirror the layout through scramble-ui.
- Text fields take the system's input language. Unlike prev, triib has
  no setting to force one.
- Names the standards give, such as descriptor types and the commands
  and statuses in the log, stay as the standards write them, and so do
  the log's frame summaries; `triib-cli` stays English.
- Done: the text of every view in `i18n/en/triib.ftl` and its 37
  translations, their terms following `docs/GLOSSARY.md`, each
  language's choices in `docs/glossary/` with those a native speaker
  should check. Each language sets its list, thousands and decimal
  marks and how it writes a percentage; capitals follow the language
  (Turkish İ, Greek without accents). Tests check every translation
  loads, keeps English's keys and variables, keeps the standards' names
  and units, and picks its plural forms.
- Right to left languages mirror the insides of the views and panels:
  the entity list runs from the right with its dividers dragged from
  their left, the inspector's labels sit on the right, the log's lines
  start with their time on the right, arrows and chevrons point the way
  the line reads, and the matrix's and the network's controls and
  details mirror. The window's bars and the inspector's place stay as
  in left to right languages, and so do the matrix grid and the network
  map, which are drawings, and sliders, which iced draws left to right.
  Translated text reads from the right even when it opens with a Latin
  word; data such as entity names keeps its own direction, and text cut
  short ends in an ellipsis on the side it reads towards.
- Text fields start on the side the keyboard layout in use types from,
  read through scramble-ui (the active XKB layout on Linux, the input
  language on Windows, the input source on macOS), and once they hold
  text follow its direction; placeholders sit on the interface's side.
- To do: review by native speakers.

### P2: macOS and Windows, everything but virtual endpoints

- Controller frames on macOS (BPF, or the AudioVideoBridging framework,
  as the investigation's first two questions decide) and on Windows
  (Npcap), with the interface list and capability probe of each.
- The app and `triib-cli` doing all of P0 and P1 on both: discovery,
  enumeration, connections, the network view, media clock, mappings,
  controls, diagnostics and the log; where a platform cannot hear the
  bridge's gPTP messages, the network view says so instead of guessing.
- Checked on the bench from the Windows PC and the Mac mini, against the
  same entities as Linux.
- Packages for both with what they need set up (Npcap on Windows), and
  the release pipeline from prev.
- Done (checked 2026-10-07 from the Windows PC's I226-V and the Mac
  mini's en0, against the same entities as Linux):
  - macOS through BPF: the investigation's first question answered yes,
    so the AudioVideoBridging framework is not needed for the
    controller. Each socket is its own `/dev/bpf` with an ethertype
    filter, seeing what the computer sends so the system's own AVB
    entity is heard; group addresses are added to the interface when
    the process may, else the device listens promiscuously. Interfaces
    come from getifaddrs and SIOCGIFMEDIA.
  - Windows through Npcap, loaded from its own folder: a promiscuous
    capture to read, where AVB's group addresses and gPTP arrive, and
    another to send; interfaces from GetAdaptersAddresses by their
    friendly names, physical when a connector is present.
  - On both, a socket leaves out only the frames it sent itself.
    `avb_net::check_access` is the capability probe, `triib-cli
    interfaces` prints it, and each system's reason (no `CAP_NET_RAW`,
    no access to `/dev/bpf*`, Npcap missing or for administrators only)
    reaches the app with its fix: a command to copy, or a link to Npcap.
  - Both hear the bridge's gPTP: the network view and `triib-cli
    network` place this computer at its bridge port (synced on the Mac,
    whose system runs gPTP). Where the listener cannot open, the view
    says this computer cannot listen for gPTP instead of reporting no
    bridge.
  - The Mac's own AVB entity cannot be read from the same Mac: macOS
    never hands it what is written to BPF, and there is no loopback. The
    controller does not try, and the views say it runs on this computer
    and to read it from another one. No clash between triib's entity ID
    and the system's on the same MAC.
  - Discovery, reading, connections (a CRF binding made and undone from
    each), media clocks, mappings, controls, AVB Lite status, the log
    and the network view all work on both; idle CPU on Windows about 1%.
  - Packages: Debian and RPM (and AUR) set `CAP_NET_RAW`; the macOS
    installer package opens the BPF devices to `access_bpf` at every
    start, as Wireshark's ChmodBPF does; the per-user Windows MSI looks
    for Npcap and offers its download page when it is missing, as
    Npcap's license keeps it out of other installers. The release
    workflow builds them for a tag (docs/RELEASING.md).
- To do: try the macOS package's install on a Mac (it needs an
  administrator's password), run the release workflow once by hand, and
  the app icon for the packages.

### P3: virtual endpoints on Linux

- `triib-endpointd` with gPTP and AVB Lite PTP, MSRP, MVRP, CVU SRP.
- Spawn talkers and listeners, AAF and AM824, audio routing via cpal.
- Show them in the matrix and inspector; save them in presets.
- Done (checked 2026-10-07 from this computer's TX401 through the
  bridge, against a Milan endpoint both ways):
  - gPTP from linuxptp: ptp4l disciplines the PHC and the daemon asks
    its read-only socket through `pmc` every 2 s for each entity's
    GET_AVB_INFO, GET_AS_PATH and counters. No PTP engine of our own
    yet. The PHC runs on the BTC's timescale, not TAI.
  - `avb-mrp`: an MRP participant (applicant, registrar, join, leave,
    LeaveAll and periodic timers) with MSRP's and MVRP's encodings; the
    daemon declares class A's domain, VLAN 2 and each talker's stream,
    and registers listeners and talkers. The bridge reserves and
    forwards.
  - `atdecc`'s entity side: ADP, an AEM responder for what Milan
    controllers read and set (descriptors, names, formats, sampling
    rate, clock source, identify, AVB info, AS path, counters, max
    transit time, Milan info), Milan's ACMP with the listener probing
    its talker, and unsolicited notifications.
  - No in-process router: Linux sockets now hear what other programs on
    the computer send, so the app and the daemon's entities talk over
    the wire as remote ones do. Within the daemon, its entities' frames
    for each other go to them directly too, as a socket never hears its
    own.
  - `triib-stream`: AAF (32-bit, or 24 in 32) and IEC 61883-6 AM824 at
    48, 96 or 192 kHz, one class A frame of samples every 125 us, paced in
    user space on gPTP time read from the PHC, presentation time the max
    transit time ahead; AM824's timestamp falls on each frame's
    SYT_INTERVAL boundary, as strict listeners such as macOS want. The
    listener counts what Milan counts. Audio from and to any device
    through cpal, a test tone, or nothing; the device's drift is
    followed by cubic resampling at a ratio steered by the fill of the
    buffer between them, within 20 ppm of the drift after a few seconds
    in simulation. The pacing and receiving
    threads ask RealtimeKit for real-time scheduling, as PipeWire does,
    so a build on every core leaves the streams on time.
  - MAAP (`atdecc::maap`): the daemon claims one destination address for
    each talker, declares the streams once it has them, defends them,
    and moves when a lower address holds them; the first block tried
    comes from the interface's address, the same each start.
  - `triib-endpointd`: endpoints from `endpoints.toml`, read again when
    it changes; the names, formats and bindings controllers give kept
    in it, so a listener binds again after a restart; what each endpoint
    is doing written to `endpointd.toml` in the runtime folder. A
    talker streams while a listener is ready for it, on the network or
    in the same daemon. A talker whose format changes withdraws its
    declaration and declares again 2 s later, as the bridge kept the old
    frame size for a declaration changed in place. A missing audio
    device leaves the stream running silent. Release build 1.3 MB,
    about 10 MB resident; an 8 channel stream takes 4% of a core to send
    and 3% to receive.
  - The app adds and removes them in the Entities view on interfaces
    with a PTP hardware clock and a wired link, starts the daemon when
    it isn't running, and says when ptp4l is missing; they show as this
    computer's in the list, the matrix and the network view, and the
    inspector picks their audio device and channels; the list can show
    only them. Other controllers see and bind them like any entity.
  - Presets keep this computer's endpoints; recalling one starts them
    again and waits for them before binding, and on another computer
    their entity IDs become its own.
  - AVB Lite (checked 2026-10-07 on this computer's LAN interface,
    where no AVB bridge answers): the daemon tells AVB from AVB Lite as
    the profile's 2.2 says, sending its own Pdelay_Req with the Endpoint
    Declaration TLV where ptp4l does not run on the interface (ptp4l's
    clock identity says which), taking ptp4l's asCapable where it does,
    and listening for other endpoints' TLV; after falling back it sends
    the beacon every 3 s. Its endpoints then declare with CVU SRP, as
    esp_avb does: talkers broadcast, listeners unicast to their talker,
    VLAN 2 priority 5, refreshed every second and aged out after 30 s.
    A talker sends a unicast copy to each ready listener, two at most,
    else its MAAP address, within 75% of the link, and listeners take
    either. GET_LITE_STATUS answers from every entity, unsolicited
    to registered controllers on change. `avb_lite = "off"` or `"on"` in
    endpoints.toml keeps AVB or forces AVB Lite. On the LAN a talker
    and listener streamed 79,012 frames unicast with none lost.
  - AVB Lite against an ESP (checked 2026-10-08 through a plain switch
    on the LAN interface, which has no PTP clock): the daemon fell back
    on the ESP's Endpoint Declaration TLV within seconds, CVU SRP bound
    both ways, and each side sent its stream unicast to the other, over
    700,000 frames each way with no sequence mismatches. The ESP sends
    its declarations to the ATDECC multicast address and its streams
    and declarations at priority 3, where the profile says broadcast
    and priority 5; triib takes either.
  - Sampling rates (checked 2026-10-08 in the same daemon): 48, 96 and
    192 kHz, each where every packing fits one Ethernet frame, so up to
    60 channels at 48 kHz, 30 at 96 and 15 at 192. Setting the rate moves
    the entity's streams to the same packing at it, and a format at
    another rate moves the rate. A device opens at the stream's rate
    when it takes it, else at its own, and the resampler converts: a
    48 kHz-only USB microphone fed a 96 and a 192 kHz stream, nothing
    lost, its drift read as before.
  - Playing out at presentation time (checked 2026-10-08 into a
    PipeWire sink): the output notes when each chunk the device takes
    plays, from the latency cpal reports, and each frame held is known
    by its presentation time, so the resampler steers by how late frames
    play rather than by the fill. Where the device's latency is longer
    than the stream allows, it watches for 2 s, then plays every sample
    the same whole milliseconds after its presentation time, a step
    more each time the device then finds nothing to play; the daemon
    reports the delay. Through PipeWire: 40 ms after, held within
    0.01 ms. In simulation, frames far enough ahead play at their
    presentation time.
  - The Linux packages carry `triib-endpointd` with `CAP_NET_RAW`.
  - Checked with AM824 both ways against a Milan endpoint and into
    macOS's AVB listener, which counted no sequence mismatches.
  - AVB Lite's revised profile (avbcommunity/profiles ea25611, checked
    2026-10-08 on the LAN interface against this computer's own
    endpoints and an ESP): the Endpoint Declaration TLV as type 0x8000;
    CVU SRP on MRP's timers, unanswered; the declared destination
    showing unicast or multicast; escalation to multicast only where
    allowed (endpoints.toml or SET_LITE_CONFIG, which triib-cli
    lite-config sends), with the frames moving 200 ms after the
    declaration; a listener refused alone with a unicast Talker Failed;
    listeners admitting against their link; and a configurable fan-out
    and media VLAN.
  - ptp4l switched between gPTP and the AVB Lite PTP profile through
    triib's systemd units and polkit rule, back to gPTP when the link
    comes up again, run on the bench.
  - EEE and PAUSE kept off the endpoint's link by a boot unit,
    `triib-link@`, which the daemon runs again should they come back.
  - Against an ESP grandmaster through a non-AVB switch (checked
    2026-10-09, this computer's TX401 at 1 Gb/s, the ESP at 100 Mb/s):
    the daemon fell back on the ESP's beacon and moved ptp4l to the
    AVB Lite PTP profile, which followed the ESP within 40 to 65 ns
    RMS, and corrected the media clock 2304 ns for the ESP's link
    speed from its Grandmaster Link TLV. A ptp4l switch the 30 s wait
    put off is now made once the wait is over. The ESP answers unicast
    delay requests (hybrid_e2e), but the TX401's atlantic driver trims
    12 octets from received unicast PTP frames, so ptp4l drops every
    answer; triib's AVB Lite configuration keeps delay requests
    multicast. The ESP's tap showed each Delay_Resp leaving whole, 72
    octets with its VLAN 0 priority tag, which the TX401 strips before
    trimming the 12. The ESP since asks for a delay request every
    second and claims the PTP timescale (esp_ptp 1.4.4, not yet
    published).
  - The TX401's atlantic driver leaves its time stamp on received PTP
    frames after the link renegotiates, from a cable unplugged as well
    as a PAUSE change, until the interface goes down and up, and ptp4l
    drops them all: the daemon sees 16 such frames in a row and
    restarts the interface through `triib-link-reset@`, at most every
    ten minutes (checked 2026-10-09 with the cable unplugged and back).
  - The fallback weighs nothing while the link is down, and starts
    afresh as it comes up (profile 2.2). From the AVB Lite profile,
    where nobody here asks for peer delay, the fallback listens: one
    gPTP peer asking without the TLV, as a bridge does every second,
    moves ptp4l to gPTP at once while it keeps listening, and a
    beacon then brings it back without the 30 s wait; two such peers
    are a flooding switch, and none for 10 s is condition 2's silence.
    On the bridge, from the AVB Lite unit: gPTP after 0.5 s and
    ptp4l in sync after 6.8 s. Through the non-AVB switch with the
    ESP: from the AVB Lite unit ptp4l stays and the correction is in
    after 0.8 s; from the gPTP unit it moves to AVB Lite after 1 s;
    and with a plain gPTP device beside the ESP it goes to gPTP and is
    back on AVB Lite 3 s after starting, on the ESP's beacon.
- To do:
  - The atlantic driver's time stamps, reported to its maintainer and
    netdev on 2026-10-09: answer the thread, and drop the workaround
    once a fix is in the kernels triib's users run.
  - The linuxptp organization TLV tables (v2 of the series, a table
    or a built-in option as Erez prefers; replies sent 2026-10-09),
    then the Endpoint Declaration TLV from ptp4l in gPTP mode.
  - Sinc resampling (`rubato`) if cubic ever falls short.

### P4: investigating virtual endpoints on Windows, then macOS

- Windows first: whether a hardware timestamp path exists, starting from
  the Intel I210 card and its driver's time sync; then gPTP, SRP and
  paced transmit on it. The result decides whether Windows gets
  `triib-endpointd`.
- Then macOS: the investigation's last two questions, gPTP time and
  hardware timestamps from user space, or the OS's own AVB audio device
  controlled like any entity.

### Later

- Firmware update over MVU, several interfaces at once, saving the log,
  MCP server, CRF talker, PipeWire native nodes per endpoint.
- Pacing virtual endpoints' streams in hardware: launch time (SO_TXTIME
  with the etf qdisc) or CBS where the card has them.

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
5. **Hive parity and AVB Lite controller features** (P1), with the
   status query in esp_avb, then **every language prev speaks** (P1.1).
6. **macOS and Windows** (P2): the macOS investigation's frame access
   questions, then the controller backends of both, checked on the
   bench; packages with capabilities set, release pipeline from prev.
7. **Virtual endpoints on Linux** (P3): the PTP analysis first, then PTP,
   SRP, local entity engine, streaming, audio, daemon.
8. **Virtual endpoint investigation** (P4): Windows, then macOS.

## Footprint targets

- GUI binary under 20 MB; daemon under 10 MB.
- Cold start to window under 300 ms.
- Idle CPU under 1% with 200 entities; memory under 100 MB.
- Virtual endpoint: under 5% of one core per 8 channel 48 kHz stream.
- Measured in CI with `triib-sim`.

## Open decisions

- No Flatpak: raw sockets and PHC access cannot be granted in a sandbox.
- Milan only, or also plain 1722.1 devices with reduced features.
