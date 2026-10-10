# Changelog

All notable changes to triib. Versions follow
[Semantic Versioning](https://semver.org).

## [0.9.0] - 2026-10-10

The first release: an ATDECC (IEEE 1722.1) controller with Milan and AVB
Lite support for Linux, Windows and macOS, and on Linux talkers and
listeners of the computer's own.

### Controller

- Discovers the ATDECC entities on the interface you pick, reads each
  entity model, its names, streams, clocks and Milan support, and keeps
  it, so a device seen before shows in full at once.
- A connection matrix, talker outputs across and listener inputs down,
  each cell showing whether the stream flows, waits or has failed, and
  whether the two formats can meet.
- A map of the network from each entity's gPTP path: bridges, devices,
  every stream as its own wire, failed reservations where they stop,
  and the clock tree from the grandmaster with each device in sync or
  not.
- Renames entities, changes stream formats, sampling rates and clock
  sources, maps channels to streams, sets controls such as gain and
  mute, and makes a device identify itself.
- Presets of each entity's clock sources, sampling rates, stream
  formats, controls and connections, recalled by changing only what
  differs.
- A log of every ATDECC frame sent and heard, decoded in plain words,
  with the frames that break the rules marked and the reason given.
- AVB Lite: each device's mode and why, its PTP offset, what its streams
  take of each link, and the alarms the profile calls for.
- `triib-cli` does the same from scripts.

### This computer's talkers and listeners (Linux)

- `triib-endpointd` runs Milan talkers and listeners streaming AAF or
  AM824 at 48, 96 or 192 kHz to and from the computer's audio devices,
  on a wired interface with a PTP hardware clock and linuxptp's ptp4l.
- Where no AVB bridge answers, they fall back to AVB Lite over switches
  that are not AVB bridges, and the daemon moves ptp4l between gPTP and
  the AVB Lite PTP profile through the `triib-ptp4l-gptp@` and
  `triib-ptp4l-lite@` units.
- `triib-link@` keeps Energy-Efficient Ethernet and PAUSE off the link,
  as AVB Lite asks, and `triib-link-reset@` restarts an interface whose
  driver keeps its time stamps on received PTP frames.

### Everywhere

- 38 languages, following the system's or the one picked in Settings,
  right-to-left languages mirrored.
- Material 3 design, light and dark, shared with prev.
- Packages for Linux on x86_64, ARM64 and RISC-V (.deb, .rpm, tarball,
  and a PKGBUILD for Arch), Windows on x64 and ARM64 (.msi and .zip,
  signed by Scramble Tools LLC), and macOS on Apple Silicon (.pkg), each
  setting up what triib needs to send and receive raw Ethernet.
- The protocols are Rust crates of their own, `atdecc`, `avb-mrp` and
  `avb-net`, which build without std for microcontrollers.

### Known limits

- The macOS package is not signed with a Developer ID or notarized, so
  macOS blocks it until you choose Open Anyway in System Settings,
  Privacy & Security.
- Talkers and listeners of the computer's own run on Linux only.
- The translations await review by native speakers.
- The AVB Lite Endpoint Declaration TLV goes out only while ptp4l does
  not run gPTP on the interface, until linuxptp can carry it.
- On Aquantia cards (Linux `atlantic`), the driver cuts unicast PTP
  frames short, so AVB Lite's delay requests stay multicast, and keeps
  time stamps on received PTP frames after the link renegotiates, which
  the daemon works around by restarting the interface.

[0.9.0]: https://github.com/scrambletoolsllc/triib/releases/tag/v0.9.0
