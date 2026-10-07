//! Raw Ethernet frames of one ethertype on one interface.

use std::io;
use std::time::Duration;

use crate::MacAddress;

/// A frame received on a [`Socket`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Received {
    pub source: MacAddress,
    /// Octets of payload written to the buffer, after the Ethernet header.
    pub length: usize,
    /// Sent to a multicast or broadcast address rather than to us.
    pub group: bool,
}

/// Sends and receives the payloads of Ethernet frames of one ethertype on
/// one interface. The kernel adds and removes the Ethernet header.
pub struct Socket {
    inner: platform::Socket,
    mac: MacAddress,
}

impl Socket {
    /// Opens `interface` for frames of `ethertype`. On Linux this needs
    /// `CAP_NET_RAW`.
    pub fn open(interface: &str, ethertype: u16) -> io::Result<Self> {
        let mac = crate::interfaces::mac_of(interface).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no Ethernet interface named {interface}"),
            )
        })?;
        Ok(Self {
            inner: platform::Socket::open(interface, ethertype)?,
            mac,
        })
    }

    /// The interface's MAC address, the source of every frame sent.
    pub fn mac(&self) -> MacAddress {
        self.mac
    }

    /// Receives frames sent to `group` as well.
    pub fn join_multicast(&self, group: MacAddress) -> io::Result<()> {
        self.inner.join_multicast(group)
    }

    pub fn send(&self, destination: MacAddress, payload: &[u8]) -> io::Result<()> {
        self.inner.send(destination, payload)
    }

    /// Keeps only frames whose payload starts with an octet in `first`,
    /// such as the AVTP subtypes of ATDECC, dropped before they reach the
    /// program.
    pub fn keep_payloads_starting(&self, first: std::ops::RangeInclusive<u8>) -> io::Result<()> {
        self.inner
            .keep_payloads_starting(*first.start(), *first.end())
    }

    /// Waits up to `timeout` (forever when `None`) for a frame and writes
    /// its payload to `buffer`, cut short if the buffer is. `None` when the
    /// time ran out or a signal interrupted the wait. Frames this socket
    /// sent are left out; those other programs on this computer send are
    /// not, so entities and controllers on the same computer hear each
    /// other.
    pub fn receive(
        &self,
        buffer: &mut [u8],
        timeout: Option<Duration>,
    ) -> io::Result<Option<Received>> {
        self.inner.receive(buffer, timeout)
    }
}

/// Why raw Ethernet is out of this program's reach. [`Socket::open`] and
/// [`check_access`] return it inside their error; [`Blocked::of`] finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Blocked {
    /// Linux: the program lacks `CAP_NET_RAW`.
    RawCapability,
    /// macOS: the capture devices, `/dev/bpf*`, are not open to the user.
    CaptureDevices,
    /// Windows: Npcap is not installed.
    Npcap,
    /// Windows: Npcap lets only administrators capture.
    NpcapAdministrators,
}

impl Blocked {
    /// The reason inside `error`, when it is one.
    pub fn of(error: &io::Error) -> Option<Self> {
        error.get_ref()?.downcast_ref::<Self>().copied()
    }

    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos", windows)),
        allow(dead_code)
    )]
    fn error(self) -> io::Error {
        let kind = match self {
            Self::Npcap => io::ErrorKind::NotFound,
            _ => io::ErrorKind::PermissionDenied,
        };
        io::Error::new(kind, self)
    }
}

impl std::fmt::Display for Blocked {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawCapability => {
                let program = std::env::current_exe()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|_| "the program".to_owned());
                write!(
                    formatter,
                    "raw Ethernet needs CAP_NET_RAW; grant it with: sudo setcap cap_net_raw+ep \
                     {program}"
                )
            }
            Self::CaptureDevices => formatter.write_str(
                "raw Ethernet needs access to /dev/bpf*: install triib's capture helper, or \
                 Wireshark's ChmodBPF, and log in again",
            ),
            Self::Npcap => formatter.write_str(
                "raw Ethernet needs Npcap: install it from https://npcap.com, then open triib again",
            ),
            Self::NpcapAdministrators => formatter.write_str(
                "Npcap lets only administrators capture: run triib as administrator, or \
                 reinstall Npcap without \"Restrict Npcap driver's access to Administrators \
                 only\"",
            ),
        }
    }
}

impl std::error::Error for Blocked {}

/// The error for a raw socket Linux will not open without `CAP_NET_RAW`.
#[cfg(target_os = "linux")]
pub(crate) fn raw_capability() -> io::Error {
    Blocked::RawCapability.error()
}

/// Whether this program may send and receive raw Ethernet at all, before
/// any interface is picked: an error carrying [`Blocked`] when it may not.
pub fn check_access() -> io::Result<()> {
    platform::check_access()
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod platform {
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::time::Duration;

    use super::{Received, Sent};
    use crate::MacAddress;

    pub struct Socket {
        fd: OwnedFd,
        index: i32,
        ethertype: u16,
        sent: Sent,
    }

    /// Opens a packet socket that hears nothing, which takes the same
    /// permission as one that does.
    pub fn check_access() -> io::Result<()> {
        // SAFETY: plain system call with integer arguments.
        let raw =
            unsafe { libc::socket(libc::AF_PACKET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
        if raw < 0 {
            let error = io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::EPERM | libc::EACCES) => super::Blocked::RawCapability.error(),
                _ => error,
            });
        }
        // SAFETY: `raw` is a socket this function just opened and owns.
        drop(unsafe { OwnedFd::from_raw_fd(raw) });
        Ok(())
    }

    impl Socket {
        pub fn open(interface: &str, ethertype: u16) -> io::Result<Self> {
            let index = crate::interfaces::index_of(interface).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no interface named {interface}"),
                )
            })?;
            // Opened for no protocol, so nothing arrives before the filter
            // is on: bound to one ethertype the socket would never hear what
            // other programs here send, which Linux hands only to sockets
            // bound to every protocol.
            // SAFETY: plain system call with integer arguments.
            let raw =
                unsafe { libc::socket(libc::AF_PACKET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
            if raw < 0 {
                let error = io::Error::last_os_error();
                return Err(match error.raw_os_error() {
                    Some(libc::EPERM | libc::EACCES) => super::Blocked::RawCapability.error(),
                    _ => error,
                });
            }
            // SAFETY: `raw` is a socket this function just opened and owns.
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            let socket = Self {
                fd,
                index,
                ethertype,
                sent: Sent::default(),
            };
            socket.attach(&socket.program(None))?;
            let mut address = socket.address(None);
            address.sll_protocol = (libc::ETH_P_ALL as u16).to_be();
            // SAFETY: `address` is a valid sockaddr_ll that lives through the
            // call, and the length passed is its size.
            let bound = unsafe {
                libc::bind(
                    socket.fd.as_raw_fd(),
                    (&raw const address).cast(),
                    size_of::<libc::sockaddr_ll>() as libc::socklen_t,
                )
            };
            if bound < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(socket)
        }

        /// A link-layer address on this socket's interface and ethertype,
        /// to `destination` if given.
        fn address(&self, destination: Option<MacAddress>) -> libc::sockaddr_ll {
            // SAFETY: sockaddr_ll is plain data, valid when zeroed.
            let mut address: libc::sockaddr_ll = unsafe { zeroed() };
            address.sll_family = libc::AF_PACKET as u16;
            address.sll_protocol = self.ethertype.to_be();
            address.sll_ifindex = self.index;
            if let Some(destination) = destination {
                address.sll_halen = 6;
                address.sll_addr[..6].copy_from_slice(&destination.0);
            }
            address
        }

        pub fn join_multicast(&self, group: MacAddress) -> io::Result<()> {
            // SAFETY: packet_mreq is plain data, valid when zeroed.
            let mut request: libc::packet_mreq = unsafe { zeroed() };
            request.mr_ifindex = self.index;
            request.mr_type = libc::PACKET_MR_MULTICAST as u16;
            request.mr_alen = 6;
            request.mr_address[..6].copy_from_slice(&group.0);
            // SAFETY: `request` is a valid packet_mreq that lives through
            // the call, and the length passed is its size.
            let joined = unsafe {
                libc::setsockopt(
                    self.fd.as_raw_fd(),
                    libc::SOL_PACKET,
                    libc::PACKET_ADD_MEMBERSHIP,
                    (&raw const request).cast(),
                    size_of::<libc::packet_mreq>() as libc::socklen_t,
                )
            };
            if joined < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        /// A classic BPF program keeping this socket's ethertype, and of it
        /// only payloads whose first octet is in `first`, when given. A
        /// packet socket's program sees the payload from its start, and the
        /// ethertype as the kernel's protocol (`SKF_AD_PROTOCOL`).
        fn program(&self, first: Option<(u8, u8)>) -> Vec<libc::sock_filter> {
            let step = |code: u16, jt: u8, jf: u8, k: u32| libc::sock_filter { code, jt, jf, k };
            let protocol = 0xffff_f000; // SKF_AD_OFF + SKF_AD_PROTOCOL
            match first {
                None => vec![
                    step(0x20, 0, 0, protocol),                  // ld protocol
                    step(0x15, 0, 1, u32::from(self.ethertype)), // jeq #ethertype
                    step(0x06, 0, 0, u32::MAX),                  // ret #-1
                    step(0x06, 0, 0, 0),                         // ret #0
                ],
                Some((first, last)) => vec![
                    step(0x20, 0, 0, protocol),                  // ld protocol
                    step(0x15, 0, 4, u32::from(self.ethertype)), // jeq #ethertype
                    step(0x30, 0, 0, 0),                         // ldb [0]
                    step(0x35, 0, 2, u32::from(first)),          // jge #first
                    step(0x25, 1, 0, u32::from(last)),           // jgt #last
                    step(0x06, 0, 0, u32::MAX),                  // ret #-1
                    step(0x06, 0, 0, 0),                         // ret #0
                ],
            }
        }

        fn attach(&self, program: &[libc::sock_filter]) -> io::Result<()> {
            let filter = libc::sock_fprog {
                len: program.len() as u16,
                filter: program.as_ptr().cast_mut(),
            };
            // SAFETY: `filter` points at `program`, which both live through
            // the call; the kernel copies the program and does not write it.
            let attached = unsafe {
                libc::setsockopt(
                    self.fd.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_ATTACH_FILTER,
                    (&raw const filter).cast(),
                    size_of::<libc::sock_fprog>() as libc::socklen_t,
                )
            };
            if attached < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn keep_payloads_starting(&self, first: u8, last: u8) -> io::Result<()> {
            self.attach(&self.program(Some((first, last))))
        }

        pub fn send(&self, destination: MacAddress, payload: &[u8]) -> io::Result<()> {
            self.sent.note(payload);
            let address = self.address(Some(destination));
            // SAFETY: `payload` and `address` are valid for the lengths
            // passed and live through the call.
            let sent = unsafe {
                libc::sendto(
                    self.fd.as_raw_fd(),
                    payload.as_ptr().cast(),
                    payload.len(),
                    0,
                    (&raw const address).cast(),
                    size_of::<libc::sockaddr_ll>() as libc::socklen_t,
                )
            };
            if sent < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn receive(
            &self,
            buffer: &mut [u8],
            timeout: Option<Duration>,
        ) -> io::Result<Option<Received>> {
            let milliseconds = match timeout {
                // Round up, so a short wait does not become a busy loop.
                Some(timeout) => {
                    timeout.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as i32
                }
                None => -1,
            };
            let mut poll = libc::pollfd {
                fd: self.fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one valid pollfd, which lives through the call.
            let ready = unsafe { libc::poll(&raw mut poll, 1, milliseconds) };
            if ready < 0 {
                let error = io::Error::last_os_error();
                return match error.kind() {
                    io::ErrorKind::Interrupted => Ok(None),
                    _ => Err(error),
                };
            }
            if ready == 0 {
                return Ok(None);
            }
            loop {
                // SAFETY: sockaddr_ll is plain data, valid when zeroed.
                let mut address: libc::sockaddr_ll = unsafe { zeroed() };
                let mut length = size_of::<libc::sockaddr_ll>() as libc::socklen_t;
                // SAFETY: `buffer` and `address` are valid for the lengths
                // passed and live through the call.
                let received = unsafe {
                    libc::recvfrom(
                        self.fd.as_raw_fd(),
                        buffer.as_mut_ptr().cast(),
                        buffer.len(),
                        libc::MSG_DONTWAIT,
                        (&raw mut address).cast(),
                        &raw mut length,
                    )
                };
                if received < 0 {
                    let error = io::Error::last_os_error();
                    return match error.kind() {
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => Ok(None),
                        _ => Err(error),
                    };
                }
                // What this computer sends comes back as outgoing: this
                // socket's own frames are left out, other programs' kept.
                let outgoing = address.sll_pkttype == libc::PACKET_OUTGOING;
                if outgoing && self.sent.heard_back(&buffer[..received as usize]) {
                    continue;
                }
                let mut source = [0; 6];
                source.copy_from_slice(&address.sll_addr[..6]);
                return Ok(Some(Received {
                    source: MacAddress(source),
                    length: received as usize,
                    group: matches!(
                        address.sll_pkttype,
                        libc::PACKET_MULTICAST | libc::PACKET_BROADCAST
                    ),
                }));
            }
        }
    }
}

/// The frames a socket sent lately, to tell them from the same frames
/// heard back: every system hands a socket the frames the computer sends,
/// the socket's own and those of other programs alike, and only the
/// socket's own are left out. Linux hands back the payload, the others
/// the whole frame.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[derive(Default)]
struct Sent {
    recent: std::sync::Mutex<std::collections::VecDeque<(u64, std::time::Instant)>>,
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
impl Sent {
    /// How long a sent frame is looked for, and how many are kept.
    const KEPT_FOR: Duration = Duration::from_secs(2);
    const MOST: usize = 256;

    fn key(frame: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        frame.hash(&mut hasher);
        hasher.finish()
    }

    /// Notes a whole frame as sent.
    fn note(&self, frame: &[u8]) {
        let now = std::time::Instant::now();
        let mut recent = self
            .recent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while recent
            .front()
            .is_some_and(|&(_, at)| now.duration_since(at) > Self::KEPT_FOR)
            || recent.len() >= Self::MOST
        {
            recent.pop_front();
        }
        recent.push_back((Self::key(frame), now));
    }

    /// Whether a whole frame heard is one this socket sent, which is then
    /// forgotten.
    fn heard_back(&self, frame: &[u8]) -> bool {
        let key = Self::key(frame);
        let mut recent = self
            .recent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match recent.iter().position(|&(sent, _)| sent == key) {
            Some(place) => {
                recent.remove(place);
                true
            }
            None => false,
        }
    }
}

/// A whole Ethernet frame from `source` to `destination` of `ethertype`
/// carrying `payload`, padded to Ethernet's least length.
#[cfg(any(target_os = "macos", windows))]
fn frame(source: MacAddress, destination: MacAddress, ethertype: u16, payload: &[u8]) -> Vec<u8> {
    const LEAST: usize = 60;
    let mut frame = Vec::with_capacity((14 + payload.len()).max(LEAST));
    frame.extend_from_slice(&destination.0);
    frame.extend_from_slice(&source.0);
    frame.extend_from_slice(&ethertype.to_be_bytes());
    frame.extend_from_slice(payload);
    frame.resize(frame.len().max(LEAST), 0);
    frame
}

/// What a captured frame says, when it is one of `ethertype` this socket
/// did not send: its source and payload, written to `buffer`.
#[cfg(any(target_os = "macos", windows))]
fn take(frame: &[u8], ethertype: u16, sent: &Sent, buffer: &mut [u8]) -> Option<Received> {
    if frame.len() < 14 || frame[12..14] != ethertype.to_be_bytes() || sent.heard_back(frame) {
        return None;
    }
    let payload = &frame[14..];
    let length = payload.len().min(buffer.len());
    buffer[..length].copy_from_slice(&payload[..length]);
    let mut source = [0; 6];
    source.copy_from_slice(&frame[6..12]);
    Some(Received {
        source: MacAddress(source),
        length,
        group: frame[0] & 1 == 1,
    })
}

/// A classic BPF instruction (`struct bpf_insn`).
#[cfg(any(target_os = "macos", windows))]
#[repr(C)]
struct Instruction {
    code: u16,
    jump_true: u8,
    jump_false: u8,
    value: u32,
}

/// A filter program (`struct bpf_program`).
#[cfg(any(target_os = "macos", windows))]
#[repr(C)]
struct Program {
    length: u32,
    instructions: *mut Instruction,
}

/// A filter that passes whole frames of `ethertype`, or none at all.
#[cfg(any(target_os = "macos", windows))]
fn filter(ethertype: Option<u16>) -> Vec<Instruction> {
    let reject = Instruction {
        code: 0x06, // ret #0
        jump_true: 0,
        jump_false: 0,
        value: 0,
    };
    let Some(ethertype) = ethertype else {
        return vec![reject];
    };
    // Load the ethertype, and keep the whole frame when it matches.
    vec![
        Instruction {
            code: 0x28, // ldh [12]
            jump_true: 0,
            jump_false: 0,
            value: 12,
        },
        Instruction {
            code: 0x15, // jeq #ethertype
            jump_true: 0,
            jump_false: 1,
            value: u32::from(ethertype),
        },
        Instruction {
            code: 0x06, // ret #-1
            jump_true: 0,
            jump_false: 0,
            value: u32::MAX,
        },
        reject,
    ]
}

/// A filter that passes whole frames of `ethertype` whose payload starts
/// with an octet from `first` to `last`.
#[cfg(any(target_os = "macos", windows))]
fn payload_filter(ethertype: u16, first: u8, last: u8) -> Vec<Instruction> {
    let step = |code: u16, jump_true: u8, jump_false: u8, value: u32| Instruction {
        code,
        jump_true,
        jump_false,
        value,
    };
    vec![
        step(0x28, 0, 0, 12),                   // ldh [12]
        step(0x15, 0, 4, u32::from(ethertype)), // jeq #ethertype
        step(0x30, 0, 0, 14),                   // ldb [14]
        step(0x35, 0, 2, u32::from(first)),     // jge #first
        step(0x25, 1, 0, u32::from(last)),      // jgt #last
        step(0x06, 0, 0, u32::MAX),             // ret #-1
        step(0x06, 0, 0, 0),                    // ret #0
    ]
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod platform {
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::{Program, Received, Sent};
    use crate::MacAddress;

    /// `BIOCSSEESENT`: whether the device also sees frames this computer
    /// sends, which it must for the system's own AVB entity.
    const SEE_SENT: libc::c_ulong = 0x8004_4277;
    /// `SIOCADDMULTI` and `SIOCDELMULTI`.
    const ADD_MULTICAST: libc::c_ulong = 0x8020_6931;
    const DELETE_MULTICAST: libc::c_ulong = 0x8020_6932;
    /// The capture buffer asked for.
    const BUFFER: u32 = 1 << 16;

    /// Frames read from the device and not yet handed out.
    struct Pending {
        data: Vec<u8>,
        start: usize,
        end: usize,
    }

    pub struct Socket {
        fd: OwnedFd,
        interface: String,
        ethertype: u16,
        mac: MacAddress,
        pending: Mutex<Pending>,
        sent: Sent,
        /// Multicast groups added to the interface, to take off again.
        joined: Mutex<Vec<MacAddress>>,
    }

    /// An interface request naming `interface`.
    fn request(interface: &str) -> io::Result<libc::ifreq> {
        // SAFETY: ifreq is plain data, valid when zeroed.
        let mut request: libc::ifreq = unsafe { zeroed() };
        let bytes = interface.as_bytes();
        if bytes.len() >= request.ifr_name.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("no interface named {interface}"),
            ));
        }
        for (place, &byte) in request.ifr_name.iter_mut().zip(bytes) {
            *place = byte as libc::c_char;
        }
        Ok(request)
    }

    fn control(fd: &OwnedFd, code: libc::c_ulong, value: &mut u32) -> io::Result<()> {
        // SAFETY: each request used here takes a pointer to a u_int, which
        // lives through the call.
        if unsafe { libc::ioctl(fd.as_raw_fd(), code, std::ptr::from_mut(value)) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// A capture device opens.
    pub fn check_access() -> io::Result<()> {
        open_device().map(drop)
    }

    /// The first capture device free.
    fn open_device() -> io::Result<OwnedFd> {
        let mut denied = false;
        for index in 0..256 {
            let path = format!("/dev/bpf{index}\0");
            // SAFETY: `path` is NUL-terminated and lives through the call.
            let raw = unsafe { libc::open(path.as_ptr().cast(), libc::O_RDWR | libc::O_CLOEXEC) };
            if raw >= 0 {
                // SAFETY: `raw` is a descriptor this function just opened.
                return Ok(unsafe { OwnedFd::from_raw_fd(raw) });
            }
            match io::Error::last_os_error().raw_os_error() {
                Some(libc::EBUSY) => {}
                Some(libc::EACCES | libc::EPERM) => denied = true,
                Some(libc::ENOENT) => break,
                _ => {}
            }
        }
        Err(if denied {
            super::Blocked::CaptureDevices.error()
        } else {
            io::Error::other("no free /dev/bpf device")
        })
    }

    impl Socket {
        pub fn open(interface: &str, ethertype: u16) -> io::Result<Self> {
            let mac = crate::interfaces::mac_of(interface).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no interface named {interface}"),
                )
            })?;
            let fd = open_device()?;
            // The buffer is sized before the device is bound.
            let mut size = BUFFER;
            control(&fd, libc::BIOCSBLEN, &mut size)?;
            let mut bound = request(interface)?;
            // SAFETY: `bound` is a valid ifreq that lives through the call.
            if unsafe { libc::ioctl(fd.as_raw_fd(), libc::BIOCSETIF, &raw mut bound) } < 0 {
                return Err(io::Error::last_os_error());
            }
            control(&fd, libc::BIOCIMMEDIATE, &mut 1)?;
            // The frames written carry their own source address.
            control(&fd, libc::BIOCSHDRCMPLT, &mut 1)?;
            control(&fd, SEE_SENT, &mut 1)?;
            let mut instructions = super::filter(Some(ethertype));
            let mut program = Program {
                length: instructions.len() as u32,
                instructions: instructions.as_mut_ptr(),
            };
            // SAFETY: `program` points at `instructions`, which both live
            // through the call.
            if unsafe { libc::ioctl(fd.as_raw_fd(), libc::BIOCSETF, &raw mut program) } < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut length = 0;
            control(&fd, libc::BIOCGBLEN, &mut length)?;
            Ok(Self {
                fd,
                interface: interface.to_owned(),
                ethertype,
                mac,
                pending: Mutex::new(Pending {
                    data: vec![0; length.max(BUFFER) as usize],
                    start: 0,
                    end: 0,
                }),
                sent: Sent::default(),
                joined: Mutex::new(Vec::new()),
            })
        }

        /// Adds or takes off a multicast group on the interface.
        fn membership(&self, group: MacAddress, code: libc::c_ulong) -> io::Result<()> {
            let mut changed = request(&self.interface)?;
            // SAFETY: the union's address member is plain data; writing it
            // whole leaves the request valid.
            unsafe {
                let mut address: libc::sockaddr = zeroed();
                address.sa_len = size_of::<libc::sockaddr>() as u8;
                address.sa_family = libc::AF_UNSPEC as u8;
                for (place, &octet) in address.sa_data.iter_mut().zip(&group.0) {
                    *place = octet as libc::c_char;
                }
                changed.ifr_ifru.ifru_addr = address;
            }
            // SAFETY: plain system call with integer arguments.
            let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if raw < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `raw` is a socket this function just opened.
            let query = unsafe { OwnedFd::from_raw_fd(raw) };
            // SAFETY: `changed` is a valid ifreq that lives through the call.
            if unsafe { libc::ioctl(query.as_raw_fd(), code, &raw mut changed) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn keep_payloads_starting(&self, first: u8, last: u8) -> io::Result<()> {
            let mut instructions = super::payload_filter(self.ethertype, first, last);
            let mut program = Program {
                length: instructions.len() as u32,
                instructions: instructions.as_mut_ptr(),
            };
            // SAFETY: `program` points at `instructions`, which both live
            // through the call.
            if unsafe { libc::ioctl(self.fd.as_raw_fd(), libc::BIOCSETF, &raw mut program) } < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn join_multicast(&self, group: MacAddress) -> io::Result<()> {
            // Adding a group to the interface takes root; without it the
            // device listens to everything, which its filter still sorts.
            match self.membership(group, ADD_MULTICAST) {
                Ok(()) => {
                    self.joined
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(group);
                    Ok(())
                }
                Err(_) => {
                    // SAFETY: BIOCPROMISC takes no argument.
                    if unsafe { libc::ioctl(self.fd.as_raw_fd(), libc::BIOCPROMISC as _) } < 0 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                }
            }
        }

        pub fn send(&self, destination: MacAddress, payload: &[u8]) -> io::Result<()> {
            let frame = super::frame(self.mac, destination, self.ethertype, payload);
            self.sent.note(&frame);
            // SAFETY: `frame` is valid for its length through the call.
            let written =
                unsafe { libc::write(self.fd.as_raw_fd(), frame.as_ptr().cast(), frame.len()) };
            if written < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn receive(
            &self,
            buffer: &mut [u8],
            timeout: Option<Duration>,
        ) -> io::Result<Option<Received>> {
            let deadline = timeout.map(|timeout| Instant::now() + timeout);
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                // The frames already read, each after a header and aligned
                // to four octets.
                while pending.start + size_of::<libc::bpf_hdr>() <= pending.end {
                    let at = pending.start;
                    // SAFETY: a whole header lies within the data read, and
                    // is read unaligned.
                    let header: libc::bpf_hdr =
                        unsafe { std::ptr::read_unaligned(pending.data.as_ptr().add(at).cast()) };
                    let start = at + usize::from(header.bh_hdrlen);
                    let end = start + header.bh_caplen as usize;
                    pending.start = (end + 3) & !3;
                    if end > pending.end {
                        pending.start = pending.end;
                        break;
                    }
                    if let Some(received) = super::take(
                        &pending.data[start..end],
                        self.ethertype,
                        &self.sent,
                        buffer,
                    ) {
                        return Ok(Some(received));
                    }
                }
                // Out of time, the device is still looked at once without
                // waiting.
                let wait =
                    deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
                if !self.readable(wait)? {
                    return Ok(None);
                }
                let capacity = pending.data.len();
                // SAFETY: the buffer is valid for its whole length through
                // the call.
                let read = unsafe {
                    libc::read(
                        self.fd.as_raw_fd(),
                        pending.data.as_mut_ptr().cast(),
                        capacity,
                    )
                };
                if read < 0 {
                    let error = io::Error::last_os_error();
                    return match error.kind() {
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => Ok(None),
                        _ => Err(error),
                    };
                }
                pending.start = 0;
                pending.end = read as usize;
            }
        }

        /// Waits up to `wait` (forever when `None`) for frames to read.
        fn readable(&self, wait: Option<Duration>) -> io::Result<bool> {
            let fd = self.fd.as_raw_fd();
            // SAFETY: fd_set is plain data, valid when zeroed, and the
            // descriptor is below FD_SETSIZE for a process this small.
            let mut set: libc::fd_set = unsafe { zeroed() };
            unsafe { libc::FD_SET(fd, &raw mut set) };
            let mut time = wait.map(|wait| libc::timeval {
                tv_sec: wait.as_secs() as libc::time_t,
                tv_usec: wait.subsec_micros() as libc::suseconds_t,
            });
            let timeout = time
                .as_mut()
                .map_or(std::ptr::null_mut(), std::ptr::from_mut);
            // SAFETY: `set` and `timeout` are valid through the call.
            let ready = unsafe {
                libc::select(
                    fd + 1,
                    &raw mut set,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    timeout,
                )
            };
            if ready < 0 {
                let error = io::Error::last_os_error();
                return match error.kind() {
                    io::ErrorKind::Interrupted => Ok(false),
                    _ => Err(error),
                };
            }
            Ok(ready > 0)
        }
    }

    impl Drop for Socket {
        fn drop(&mut self) {
            let joined = std::mem::take(
                &mut *self
                    .joined
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            for group in joined {
                let _ = self.membership(group, DELETE_MULTICAST);
            }
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};
    use std::io;
    use std::sync::{Mutex, OnceLock, PoisonError};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{HANDLE, WAIT_FAILED};
    use windows_sys::Win32::System::LibraryLoader::{
        GetProcAddress, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW,
    };
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};

    use super::{Program, Received, Sent};
    use crate::MacAddress;

    /// Room libpcap writes an error message to (`PCAP_ERRBUF_SIZE`).
    const ERROR_ROOM: usize = 256;
    /// The longest frame kept, and the driver buffer asked for.
    const SNAPSHOT: c_int = 65_535;
    const BUFFER: c_int = 1 << 20;
    /// What `pcap_activate` returns when the process may not capture
    /// (`PCAP_ERROR_PERM_DENIED`) or the device is gone
    /// (`PCAP_ERROR_NO_SUCH_DEVICE`).
    const PERMISSION_DENIED: c_int = -8;
    const NO_SUCH_DEVICE: c_int = -5;

    /// A `pcap_t`.
    type Capture = *mut c_void;

    /// `struct pcap_pkthdr`, whose `timeval` holds two 32-bit longs on
    /// Windows.
    #[repr(C)]
    struct PacketHeader {
        seconds: i32,
        microseconds: i32,
        captured: u32,
        length: u32,
    }

    /// The parts of Npcap's wpcap.dll used here.
    struct Npcap {
        create: unsafe extern "C" fn(*const c_char, *mut c_char) -> Capture,
        set_snaplen: unsafe extern "C" fn(Capture, c_int) -> c_int,
        set_promisc: unsafe extern "C" fn(Capture, c_int) -> c_int,
        set_immediate_mode: unsafe extern "C" fn(Capture, c_int) -> c_int,
        set_buffer_size: unsafe extern "C" fn(Capture, c_int) -> c_int,
        activate: unsafe extern "C" fn(Capture) -> c_int,
        setfilter: unsafe extern "C" fn(Capture, *mut Program) -> c_int,
        setnonblock: unsafe extern "C" fn(Capture, c_int, *mut c_char) -> c_int,
        getevent: unsafe extern "C" fn(Capture) -> HANDLE,
        next_ex: unsafe extern "C" fn(Capture, *mut *mut PacketHeader, *mut *const u8) -> c_int,
        sendpacket: unsafe extern "C" fn(Capture, *const u8, c_int) -> c_int,
        geterr: unsafe extern "C" fn(Capture) -> *const c_char,
        close: unsafe extern "C" fn(Capture),
    }

    /// Npcap, loaded the first time it is needed.
    fn npcap() -> io::Result<&'static Npcap> {
        static LOADED: OnceLock<Option<Npcap>> = OnceLock::new();
        LOADED
            .get_or_init(load)
            .as_ref()
            .ok_or_else(|| super::Blocked::Npcap.error())
    }

    /// Npcap is installed and loads.
    pub fn check_access() -> io::Result<()> {
        npcap().map(|_| ())
    }

    // Each field's type gives its function's signature.
    #[allow(clippy::missing_transmute_annotations)]
    fn load() -> Option<Npcap> {
        // Npcap keeps its libraries in System32\Npcap, which programs
        // search only when it was installed in WinPcap's compatible mode.
        let mut system = [0u16; 260];
        // SAFETY: the buffer and its length are passed together.
        let length =
            unsafe { GetSystemDirectoryW(system.as_mut_ptr(), system.len() as u32) } as usize;
        if length == 0 || length >= system.len() {
            return None;
        }
        let mut path = system[..length].to_vec();
        path.extend("\\Npcap\\wpcap.dll".encode_utf16());
        path.push(0);
        // SAFETY: `path` is a NUL-terminated wide string; the altered
        // search path finds Packet.dll beside wpcap.dll.
        let library = unsafe {
            LoadLibraryExW(
                path.as_ptr(),
                std::ptr::null_mut(),
                LOAD_WITH_ALTERED_SEARCH_PATH,
            )
        };
        if library.is_null() {
            return None;
        }
        // Each function by name, as the type of the field it fills; the
        // library is never unloaded.
        macro_rules! function {
            ($name:literal) => {{
                // SAFETY: the name is NUL-terminated.
                let address = unsafe { GetProcAddress(library, concat!($name, "\0").as_ptr()) }?;
                // SAFETY: libpcap declares the function with this
                // signature.
                unsafe { std::mem::transmute(address) }
            }};
        }
        Some(Npcap {
            create: function!("pcap_create"),
            set_snaplen: function!("pcap_set_snaplen"),
            set_promisc: function!("pcap_set_promisc"),
            set_immediate_mode: function!("pcap_set_immediate_mode"),
            set_buffer_size: function!("pcap_set_buffer_size"),
            activate: function!("pcap_activate"),
            setfilter: function!("pcap_setfilter"),
            setnonblock: function!("pcap_setnonblock"),
            getevent: function!("pcap_getevent"),
            next_ex: function!("pcap_next_ex"),
            sendpacket: function!("pcap_sendpacket"),
            geterr: function!("pcap_geterr"),
            close: function!("pcap_close"),
        })
    }

    /// An open capture, closed when dropped.
    struct Handle(Capture);

    // SAFETY: a pcap_t may move between threads; each is used by one
    // thread at a time, under its lock.
    unsafe impl Send for Handle {}

    impl Drop for Handle {
        fn drop(&mut self) {
            if let Ok(npcap) = npcap() {
                // SAFETY: the capture is open and closed once.
                unsafe { (npcap.close)(self.0) };
            }
        }
    }

    /// The message libpcap wrote to `error`.
    fn message(error: &[c_char]) -> String {
        // SAFETY: libpcap writes a NUL-terminated message within the room.
        unsafe { CStr::from_ptr(error.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    /// The last error on a capture.
    fn last_error(npcap: &Npcap, handle: &Handle) -> io::Error {
        // SAFETY: the capture is open, and its message lives until its
        // next call.
        let text = unsafe { CStr::from_ptr((npcap.geterr)(handle.0)) }
            .to_string_lossy()
            .into_owned();
        io::Error::other(text)
    }

    /// An activated capture on `device` that keeps frames of `ethertype`,
    /// or none when `None`, without waiting when there are none.
    fn capture(
        npcap: &Npcap,
        device: &CStr,
        interface: &str,
        promiscuous: bool,
        ethertype: Option<u16>,
    ) -> io::Result<Handle> {
        let mut error = [0 as c_char; ERROR_ROOM];
        // SAFETY: `device` is NUL-terminated and `error` has the room
        // libpcap needs.
        let opened = unsafe { (npcap.create)(device.as_ptr(), error.as_mut_ptr()) };
        if opened.is_null() {
            return Err(io::Error::other(message(&error)));
        }
        let handle = Handle(opened);
        // SAFETY: settings on a capture not yet active.
        let activated = unsafe {
            (npcap.set_snaplen)(opened, SNAPSHOT);
            (npcap.set_promisc)(opened, c_int::from(promiscuous));
            (npcap.set_immediate_mode)(opened, 1);
            (npcap.set_buffer_size)(opened, BUFFER);
            (npcap.activate)(opened)
        };
        match activated {
            PERMISSION_DENIED => return Err(super::Blocked::NpcapAdministrators.error()),
            NO_SUCH_DEVICE => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Npcap does not capture on {interface}"),
                ));
            }
            failed if failed < 0 => return Err(last_error(npcap, &handle)),
            _ => {}
        }
        let mut instructions = super::filter(ethertype);
        let mut program = Program {
            length: instructions.len() as u32,
            instructions: instructions.as_mut_ptr(),
        };
        // SAFETY: `program` points at `instructions`, which both live
        // through the call; libpcap copies the program.
        if unsafe { (npcap.setfilter)(opened, &raw mut program) } != 0 {
            return Err(last_error(npcap, &handle));
        }
        // SAFETY: `error` has the room libpcap needs.
        if unsafe { (npcap.setnonblock)(opened, 1, error.as_mut_ptr()) } != 0 {
            return Err(io::Error::other(message(&error)));
        }
        Ok(handle)
    }

    pub struct Socket {
        /// Frames are read on one capture and sent on another, so a send
        /// need not wait for a read.
        receiver: Mutex<Handle>,
        sender: Mutex<Handle>,
        /// The event the receiving capture signals when frames wait, as an
        /// address so the socket can be shared between threads.
        event: usize,
        ethertype: u16,
        mac: MacAddress,
        sent: Sent,
    }

    impl Socket {
        pub fn open(interface: &str, ethertype: u16) -> io::Result<Self> {
            let npcap = npcap()?;
            let not_found = || {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no interface named {interface}"),
                )
            };
            let mac = crate::interfaces::mac_of(interface).ok_or_else(not_found)?;
            let device = crate::interfaces::device_of(interface)
                .and_then(|device| CString::new(device).ok())
                .ok_or_else(not_found)?;
            // Promiscuous, so the group addresses AVB uses reach the
            // capture; adapters that cannot be are opened as they are.
            let receiver = match capture(npcap, &device, interface, true, Some(ethertype)) {
                Ok(receiver) => receiver,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
                    ) =>
                {
                    return Err(error);
                }
                Err(_) => capture(npcap, &device, interface, false, Some(ethertype))?,
            };
            let sender = capture(npcap, &device, interface, false, None)?;
            // SAFETY: the capture is open; the event belongs to it.
            let event = unsafe { (npcap.getevent)(receiver.0) };
            if event.is_null() {
                return Err(last_error(npcap, &receiver));
            }
            Ok(Self {
                receiver: Mutex::new(receiver),
                sender: Mutex::new(sender),
                event: event as usize,
                ethertype,
                mac,
                sent: Sent::default(),
            })
        }

        pub fn join_multicast(&self, _group: MacAddress) -> io::Result<()> {
            // The receiving capture is promiscuous, or, on an adapter that
            // cannot be, hears what the adapter does.
            Ok(())
        }

        pub fn keep_payloads_starting(&self, first: u8, last: u8) -> io::Result<()> {
            let npcap = npcap()?;
            let receiver = self.receiver.lock().unwrap_or_else(PoisonError::into_inner);
            let mut instructions = super::payload_filter(self.ethertype, first, last);
            let mut program = Program {
                length: instructions.len() as u32,
                instructions: instructions.as_mut_ptr(),
            };
            // SAFETY: `program` points at `instructions`, which both live
            // through the call; libpcap copies the program.
            if unsafe { (npcap.setfilter)(receiver.0, &raw mut program) } != 0 {
                return Err(last_error(npcap, &receiver));
            }
            Ok(())
        }

        pub fn send(&self, destination: MacAddress, payload: &[u8]) -> io::Result<()> {
            let npcap = npcap()?;
            let frame = super::frame(self.mac, destination, self.ethertype, payload);
            self.sent.note(&frame);
            let sender = self.sender.lock().unwrap_or_else(PoisonError::into_inner);
            // SAFETY: `frame` is valid for its length through the call.
            if unsafe { (npcap.sendpacket)(sender.0, frame.as_ptr(), frame.len() as c_int) } != 0 {
                return Err(last_error(npcap, &sender));
            }
            Ok(())
        }

        pub fn receive(
            &self,
            buffer: &mut [u8],
            timeout: Option<Duration>,
        ) -> io::Result<Option<Received>> {
            let npcap = npcap()?;
            let deadline = timeout.map(|timeout| Instant::now() + timeout);
            let receiver = self.receiver.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                // Every frame waiting, until one is for this socket.
                loop {
                    let mut header: *mut PacketHeader = std::ptr::null_mut();
                    let mut data: *const u8 = std::ptr::null();
                    // SAFETY: the capture is open, and libpcap fills in the
                    // two pointers.
                    let read =
                        unsafe { (npcap.next_ex)(receiver.0, &raw mut header, &raw mut data) };
                    match read {
                        1 => {
                            // SAFETY: the header and the octets it counts
                            // stay valid until the next read.
                            let frame = unsafe {
                                std::slice::from_raw_parts(data, (*header).captured as usize)
                            };
                            if let Some(received) =
                                super::take(frame, self.ethertype, &self.sent, buffer)
                            {
                                return Ok(Some(received));
                            }
                        }
                        0 => break,
                        _ => return Err(last_error(npcap, &receiver)),
                    }
                }
                let milliseconds = match deadline {
                    Some(deadline) => {
                        let left = deadline.saturating_duration_since(Instant::now());
                        if left.is_zero() {
                            return Ok(None);
                        }
                        // Round up, so a short wait does not become a busy
                        // loop.
                        left.as_nanos()
                            .div_ceil(1_000_000)
                            .min(u128::from(INFINITE - 1)) as u32
                    }
                    None => INFINITE,
                };
                // Ready or out of time, the frames are looked at again and
                // the deadline decides.
                // SAFETY: the event belongs to the capture, which is open
                // through the wait.
                if unsafe { WaitForSingleObject(self.event as HANDLE, milliseconds) } == WAIT_FAILED
                {
                    return Err(io::Error::last_os_error());
                }
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod platform {
    use std::io;
    use std::time::Duration;

    use super::Received;
    use crate::MacAddress;

    /// Not available on this system yet.
    pub struct Socket;

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "raw Ethernet is not supported on this system yet",
        )
    }

    pub fn check_access() -> io::Result<()> {
        Err(unsupported())
    }

    impl Socket {
        pub fn open(_interface: &str, _ethertype: u16) -> io::Result<Self> {
            Err(unsupported())
        }

        pub fn join_multicast(&self, _group: MacAddress) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn send(&self, _destination: MacAddress, _payload: &[u8]) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn keep_payloads_starting(&self, _first: u8, _last: u8) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn receive(
            &self,
            _buffer: &mut [u8],
            _timeout: Option<Duration>,
        ) -> io::Result<Option<Received>> {
            Err(unsupported())
        }
    }
}
