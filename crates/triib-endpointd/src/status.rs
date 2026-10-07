//! What the daemon is doing, for the app: `endpointd.toml` in the runtime
//! folder, written every two seconds while it runs and removed when it
//! stops.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::Kind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub pid: u32,
    /// When it started on the endpoints it runs, in milliseconds since the
    /// Unix epoch.
    #[serde(default)]
    pub started: u64,
    pub interface: String,
    pub gptp: String,
    #[serde(rename = "endpoint", default)]
    pub endpoints: Vec<EndpointStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointStatus {
    /// As text, `0x` and sixteen hex digits.
    pub entity_id: String,
    pub name: String,
    pub kind: Kind,
    /// What it is doing: streaming, waiting, listening, bound or unbound.
    pub state: String,
    /// Where its audio comes from or goes, as endpoints.toml says it.
    #[serde(default)]
    pub audio: String,
    /// Its stream's channels.
    #[serde(default = "eight")]
    pub channels: u16,
}

fn eight() -> u16 {
    8
}

/// Where the daemon says what it is doing.
pub fn status_path() -> Option<PathBuf> {
    triib_store::paths::runtime_dir().map(|dir| dir.join("endpointd.toml"))
}

/// What the daemon says, while it runs: its file is there and its
/// process too.
pub fn read() -> Option<DaemonStatus> {
    let status: DaemonStatus = triib_store::load(&status_path()?).ok()?;
    running(status.pid).then_some(status)
}

#[cfg(target_os = "linux")]
fn running(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(target_os = "linux"))]
fn running(_pid: u32) -> bool {
    true
}
