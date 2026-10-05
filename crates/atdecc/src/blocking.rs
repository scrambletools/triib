//! A blocking driver for apps without a loop of their own: one raw socket
//! on one interface, feeding a [`Controller`] on the calling thread.

use std::io;
use std::time::Duration;

use avb_net::Socket;

use crate::controller::{Config, Controller};
use crate::id::EntityId;
use crate::time::Instant;

/// Room for any Ethernet payload.
const FRAME: usize = 1536;

pub struct Driver {
    socket: Socket,
    controller: Controller,
    epoch: std::time::Instant,
    received: [u8; FRAME],
    sending: [u8; FRAME],
}

impl Driver {
    /// Opens `interface` for ATDECC, with a controller whose entity ID is
    /// made from the interface's MAC address and the default
    /// [`Config`].
    pub fn open(interface: &str) -> io::Result<Self> {
        Self::open_with(interface, |_| {})
    }

    /// Opens `interface`, letting `configure` adjust the controller's
    /// configuration first.
    pub fn open_with(interface: &str, configure: impl FnOnce(&mut Config)) -> io::Result<Self> {
        let socket = Socket::open(interface, crate::ETHERTYPE_AVTP)?;
        socket.join_multicast(crate::ADP_ACMP_MULTICAST)?;
        let mut config = Config::new(EntityId::from_mac(socket.mac()));
        // A new start for sequence IDs each run, so a restarted controller
        // is not taken for a retry of its earlier commands.
        let clock = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        config.first_sequence_id = clock as u16;
        config.random_seed = config.entity_id.0 ^ clock;
        configure(&mut config);
        let controller = Controller::new(config);
        Ok(Self {
            socket,
            controller,
            epoch: std::time::Instant::now(),
            received: [0; FRAME],
            sending: [0; FRAME],
        })
    }

    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut Controller {
        &mut self.controller
    }

    /// The controller's clock: the time since the driver opened.
    pub fn now(&self) -> Instant {
        Instant::from_nanos(self.epoch.elapsed().as_nanos() as u64)
    }

    /// Sends what the controller queued, waits up to `max_wait` (less if a
    /// deadline comes first) for a frame, hands it over, runs due
    /// timeouts, and sends again. Events are then ready on the controller.
    pub fn turn(&mut self, max_wait: Duration) -> io::Result<()> {
        self.flush()?;
        let now = self.now();
        let wait = match self.controller.poll_timeout() {
            Some(deadline) => deadline.saturating_duration_since(now).min(max_wait),
            None => max_wait,
        };
        if let Some(received) = self.socket.receive(&mut self.received, Some(wait))? {
            let now = self.now();
            // A frame that does not decode is counted by the controller.
            let _ = self.controller.handle_frame(
                now,
                received.source,
                &self.received[..received.length],
            );
        }
        let now = self.now();
        self.controller.handle_timeout(now);
        self.flush()
    }

    /// Advertises that the controller is leaving, if it advertised, and
    /// sends what is still queued.
    pub fn close(mut self) -> io::Result<()> {
        self.controller.depart();
        self.flush()
    }

    fn flush(&mut self) -> io::Result<()> {
        while let Some(transmit) = self
            .controller
            .poll_transmit(&mut self.sending)
            .map_err(io::Error::other)?
        {
            self.socket
                .send(transmit.destination, &self.sending[..transmit.length])?;
        }
        Ok(())
    }
}
