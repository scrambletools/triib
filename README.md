# triib

A lightweight ATDECC (IEEE 1722.1) controller with Milan and AVB Lite
support, for Linux, Windows and macOS. On a computer with a hardware
timestamping Ethernet interface it will also run its own talkers and
listeners, routed to the computer's audio. Built in Rust with
[iced](https://iced.rs), sharing its look with
[prev](https://github.com/scrambletools/prev) through
[scramble-ui](https://github.com/scrambletools/scramble-ui).

> **Status:** early. On Linux, macOS and Windows, triib discovers ATDECC entities, reads their
> entity models (names, streams, clocks, Milan support) and keeps them for
> next time, shows and changes stream connections in a matrix, maps the
> network from each entity's gPTP path, identifies entities, renames
> them, changes their stream formats, sampling rates and clock sources
> (each entity's media clock right in the entity list), shows and
> changes how their channels map to streams, shows and sets their
> controls, such as gain and mute, saves and recalls presets of all of
> that and the connections, logs every ATDECC frame sent and heard,
> marking those that break the rules, and shows how entities run AVB
> Lite, what their streams take of each link and the alarms the AVB Lite
> profile calls for. On Linux it runs talkers and listeners of its own,
> Milan entities streaming 48 kHz AAF or AM824 to and from the
> computer's audio devices. It speaks 38 languages, following the system's or the
> one picked in Settings. See the [plan](docs/PLAN.md).

## Installing

Releases have packages for each system, each setting up what triib needs
to send and receive raw Ethernet:

| System | Package | What it sets up |
|---|---|---|
| Linux | .deb, .rpm, AUR | `CAP_NET_RAW` for `triib`, `triib-cli` and `triib-endpointd` |
| Linux | .tar.gz | nothing: run `sudo setcap cap_net_raw+ep` on the three programs |
| macOS (Apple Silicon) | .pkg | access to `/dev/bpf*` for the user installing it, at every start, as Wireshark's ChmodBPF does; `triib-cli` in /usr/local/bin |
| Windows | .msi, .zip | nothing itself: install [Npcap](https://npcap.com) first, which the installer and triib point to when it is missing |

On macOS the Mac's own AVB entity cannot be read from the same Mac, as
the system never hands it the commands triib writes; triib says so.
[docs/RELEASING.md](docs/RELEASING.md) has the details.

## This computer's talkers and listeners

On Linux, the Entities view adds talkers and listeners of the computer's
own, which `triib-endpointd` runs, on a wired interface with a PTP
hardware clock (`ethtool -T <interface>` shows one). They need
[linuxptp](https://linuxptp.nwtime.org)'s `ptp4l` running gPTP on that
interface, as with its `configs/gPTP.cfg`, so the clock keeps the
network's time; triib reads ptp4l's state through its read-only socket,
`/var/run/ptp4lro`, and says when ptp4l isn't there. Each endpoint is a
Milan entity with one stream of 48 kHz AAF or AM824, 8 channels unless
the inspector picks another count, which controllers, triib among them,
bind like any other; the inspector picks the audio device a talker
sends from or a listener plays to. Talkers take their stream addresses
with MAAP, and the stream threads ask RealtimeKit for real-time
scheduling, as PipeWire does.

The endpoints are listed in `endpoints.toml` in triib's data folder
(`~/.local/share/triib` on Linux), which the app writes and the daemon
reads again whenever it changes; the names, formats and bindings
controllers give are kept in it, so a listener binds again after a
restart, and presets keep it too:

```toml
interface = "enp2s0"
ptp4l_socket = "/var/run/ptp4lro"

[[endpoint]]
kind = "talker"
instance = 0
name = "Host talker 1"
source = "default"   # or "silence", "tone", or an input's name

[[endpoint]]
kind = "listener"
instance = 1
name = "Host listener 1"
sink = "default"     # or "discard", or an output's name
```

## Building

```
git clone https://github.com/scrambletools/triib
cd triib
cargo run -p triib          # the app
cargo run -p triib-cli -- interfaces
cargo run -p triib-cli -- discover <interface>
cargo run -p triib-cli -- describe <interface> [entity-id]
cargo run -p triib-cli -- network <interface>
cargo run -p triib-cli -- name <interface> <entity-id> <entity|group|type:index> <name>
cargo run -p triib-cli -- format <interface> <entity-id> <stream-input:N|stream-output:N> <hex>
cargo run -p triib-cli -- rate <interface> <entity-id> <audio-unit> <hertz>
cargo run -p triib-cli -- clock <interface> <entity-id> <clock-domain> <clock-source>
cargo run -p triib-cli -- clocks <interface>
cargo run -p triib-cli -- maps <interface> [entity-id]
cargo run -p triib-cli -- map <interface> <entity-id> <add|remove> <stream-port-input:N|stream-port-output:N> <stream:channel=cluster:channel>...
cargo run -p triib-cli -- controls <interface> [entity-id]
cargo run -p triib-cli -- control <interface> <entity-id> <control> <value>...
cargo run -p triib-cli -- connect <interface> <talker-id>:<output> <listener-id>:<input>
cargo run -p triib-cli -- disconnect <interface> <listener-id>:<input>
cargo run -p triib-cli -- disconnect-talker <interface> <talker-id>:<output> <listener-id>:<input>
cargo run -p triib-cli -- identify <interface> <entity-id> [seconds]
cargo run -p triib-cli -- streams <interface> <entity-id>
cargo run -p triib-cli -- transit <interface> <entity-id> <output> [nanoseconds]
cargo run -p triib-cli -- descriptor <interface> <entity-id> <type:index>
cargo run -p triib-cli -- harvest <interface> <entity-id> [repeat]
cargo run -p triib-endpointd -- [--config <endpoints.toml>] [--interface <interface>]
```

triib uses [scramble-ui](https://github.com/scrambletools/scramble-ui)
at the revision `Cargo.toml` pins, as prev does. To change both together,
clone scramble-ui beside triib and build with
`cargo --config .cargo/scramble-ui-local.toml build`, a git-ignored file
pointing at that checkout (for clippy and test, put `--config` after the
subcommand):

```toml
[patch."https://github.com/scrambletools/scramble-ui"]
scramble-ui = { path = "../scramble-ui" }

[patch.crates-io]
iced_graphics = { path = "../scramble-ui/vendor/iced_graphics" }
iced_widget = { path = "../scramble-ui/vendor/iced_widget" }
```

Once the scramble-ui change is pushed, move the pin to its revision and
build once without the file, so `Cargo.lock` names the revision again.

On Linux, sending and receiving ATDECC frames needs `CAP_NET_RAW`:

```
sudo setcap cap_net_raw+ep target/debug/triib
sudo setcap cap_net_raw+ep target/debug/triib-endpointd
```

Building on Linux needs ALSA's headers for audio (`libasound2-dev` on
Debian and Ubuntu, `alsa-lib-devel` on Fedora, `alsa-lib` on Arch).

On macOS it needs access to `/dev/bpf*`, as Wireshark's ChmodBPF or
triib's package gives (or, until the next restart,
`sudo chown $USER /dev/bpf*`), and on Windows it needs
[Npcap](https://npcap.com). `cargo run -p triib-cli -- interfaces` says
whether raw Ethernet is ready.

## Workspace

| Crate | What it does |
|---|---|
| `triib` | The app |
| `triib-cli` | Headless controller |
| `triib-endpointd` | Runs this computer's talkers and listeners |
| `triib-stream` | AAF streams, the media clock and audio devices |
| `triib-store` | Settings and cache files |
| `atdecc` | IEEE 1722.1 ATDECC with Milan: frames and state machines, no I/O |
| `avb-mrp` | IEEE 802.1Q MRP, MSRP and MVRP: frames and state machines, no I/O |
| `avb-net` | Raw Ethernet, interfaces and hardware timestamps per operating system |

`atdecc`, `avb-mrp` and `avb-net` are meant for other applications too:
they depend on nothing but `avb-net`, do no I/O of their own outside its
optional platform support, and build without std for microcontrollers.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in triib
by you, as defined in the Apache-2.0 license, shall be dual licensed as
above, without any additional terms or conditions.

Milan and AVB are trademarks of Avnu Alliance. triib is not certified by
Avnu Alliance.
