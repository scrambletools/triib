//! ptp4l's profile on the interface, where triib's own systemd units run
//! it: `triib-ptp4l-gptp@<interface>` for gPTP and
//! `triib-ptp4l-lite@<interface>` for the AVB Lite PTP profile, each
//! stopping the other as it starts. A polkit rule lets the people at the
//! computer start them, so the daemon moves ptp4l to the AVB Lite profile
//! when its endpoints fall back, and to gPTP again when the link comes
//! back up (AVB Lite profile, 2.2). ptp4l run any other way stays as it
//! is.

use std::io;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

/// How often the link and the units are looked at.
const POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Gptp,
    Lite,
}

impl Profile {
    pub fn unit(self, interface: &str) -> String {
        let name = match self {
            Profile::Gptp => "gptp",
            Profile::Lite => "lite",
        };
        format!("triib-ptp4l-{name}@{interface}.service")
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Profile::Gptp => "gPTP",
            Profile::Lite => "the AVB Lite PTP profile",
        })
    }
}

/// What the interface's link and triib's units are doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    /// The link is up; `None` when the system does not say.
    pub carrier: Option<bool>,
    /// Which of triib's units runs ptp4l on the interface, if one does.
    pub unit: Option<Profile>,
}

/// Whether `interface` names a unit as it is: what Linux allows in an
/// interface name, less what a unit name would need escaped.
fn nameable(interface: &str) -> bool {
    !interface.is_empty()
        && interface
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Which of triib's units runs ptp4l on `interface`, if one does.
pub fn running(interface: &str) -> Option<Profile> {
    if !nameable(interface) {
        return None;
    }
    [Profile::Gptp, Profile::Lite]
        .into_iter()
        .find(|profile| active(&profile.unit(interface)))
}

/// Starts the unit running `profile` on `interface`, which stops the
/// other; ptp4l then starts again in that profile, its clock locking in
/// a few seconds.
pub fn start(interface: &str, profile: Profile) -> io::Result<()> {
    if !nameable(interface) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{interface} cannot name a unit"),
        ));
    }
    let output = Command::new("systemctl")
        .args(["start", "--no-block", "--no-ask-password"])
        .arg(profile.unit(interface))
        .stdin(Stdio::null())
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

/// Whether `interface`'s link is up, where the system says.
pub fn carrier(interface: &str) -> Option<bool> {
    if !nameable(interface) {
        return None;
    }
    let text = std::fs::read_to_string(format!("/sys/class/net/{interface}/carrier")).ok()?;
    match text.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

/// Looks at the link and the units every few seconds and sends what
/// changed, until `stop`.
pub fn watch<T: Send + 'static>(
    interface: String,
    stop: Arc<AtomicBool>,
    sender: Sender<T>,
    wrap: fn(Link) -> T,
) {
    let _ = std::thread::Builder::new()
        .name("link".into())
        .spawn(move || {
            let mut last = None;
            while !stop.load(Ordering::Relaxed) {
                let link = Link {
                    carrier: carrier(&interface),
                    unit: running(&interface),
                };
                if last != Some(link) {
                    if sender.send(wrap(link)).is_err() {
                        return;
                    }
                    last = Some(link);
                }
                std::thread::sleep(POLL);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_are_named_for_the_interface() {
        assert_eq!(
            Profile::Lite.unit("enp2s0"),
            "triib-ptp4l-lite@enp2s0.service"
        );
        assert!(nameable("enp2s0.2"));
        assert!(!nameable("enp2s0 x"));
        assert!(!nameable("../x"));
        assert!(!nameable(""));
        assert_eq!(running("no/such"), None);
    }
}
