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

    /// Waits up to `timeout` (forever when `None`) for a frame and writes
    /// its payload to `buffer`, cut short if the buffer is. `None` when the
    /// time ran out or a signal interrupted the wait. Frames this computer
    /// sent are left out.
    pub fn receive(
        &self,
        buffer: &mut [u8],
        timeout: Option<Duration>,
    ) -> io::Result<Option<Received>> {
        self.inner.receive(buffer, timeout)
    }
}

/// The error for a raw socket the process may not open, with what to do.
fn permission_error() -> io::Error {
    let program = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "the program".to_owned());
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "raw Ethernet needs CAP_NET_RAW; grant it with: sudo setcap cap_net_raw+ep {program}"
        ),
    )
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod platform {
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::time::Duration;

    use super::Received;
    use crate::MacAddress;

    pub struct Socket {
        fd: OwnedFd,
        index: i32,
        ethertype: u16,
    }

    impl Socket {
        pub fn open(interface: &str, ethertype: u16) -> io::Result<Self> {
            let index = crate::interfaces::index_of(interface).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no interface named {interface}"),
                )
            })?;
            // SAFETY: plain system call with integer arguments.
            let raw = unsafe {
                libc::socket(
                    libc::AF_PACKET,
                    libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                    i32::from(ethertype.to_be()),
                )
            };
            if raw < 0 {
                let error = io::Error::last_os_error();
                return Err(match error.raw_os_error() {
                    Some(libc::EPERM | libc::EACCES) => super::permission_error(),
                    _ => error,
                });
            }
            // SAFETY: `raw` is a socket this function just opened and owns.
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            let socket = Self {
                fd,
                index,
                ethertype,
            };
            let address = socket.address(None);
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

        pub fn send(&self, destination: MacAddress, payload: &[u8]) -> io::Result<()> {
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
                if address.sll_pkttype == libc::PACKET_OUTGOING {
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

#[cfg(not(target_os = "linux"))]
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

        pub fn receive(
            &self,
            _buffer: &mut [u8],
            _timeout: Option<Duration>,
        ) -> io::Result<Option<Received>> {
            Err(unsupported())
        }
    }
}
