//! Real-time scheduling for the threads that pace and take frames, asked
//! of RealtimeKit as PipeWire asks it, so a busy computer does not hold
//! them up: no privileges needed in a desktop session. Without it they
//! run as any thread does.

/// Asks for the calling thread to run real time, saying whether it does.
pub fn raise() -> bool {
    platform::raise()
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod platform {
    use std::process::{Command, Stdio};

    /// The priority asked for, below PipeWire's own threads (20) so audio
    /// never waits on a stream.
    const PRIORITY: u32 = 15;
    /// The most CPU time a real-time thread may take without sleeping, in
    /// microseconds, which RealtimeKit wants a process to promise first.
    const MOST_TIME: u64 = 200_000;

    pub fn raise() -> bool {
        let limit = libc::rlimit {
            rlim_cur: MOST_TIME,
            rlim_max: MOST_TIME,
        };
        // SAFETY: `limit` is a valid rlimit that lives through the call.
        if unsafe { libc::setrlimit(libc::RLIMIT_RTTIME, &raw const limit) } < 0 {
            return false;
        }
        // SAFETY: gettid has no arguments and cannot fail.
        let thread = unsafe { libc::gettid() };
        Command::new("busctl")
            .args([
                "call",
                "--system",
                "org.freedesktop.RealtimeKit1",
                "/org/freedesktop/RealtimeKit1",
                "org.freedesktop.RealtimeKit1",
                "MakeThreadRealtimeWithPID",
                "ttu",
            ])
            .arg(std::process::id().to_string())
            .arg(thread.to_string())
            .arg(PRIORITY.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    pub fn raise() -> bool {
        false
    }
}
