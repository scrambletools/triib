//! Reads a PTP hardware clock beside the monotonic clock for a while and
//! prints any reading that strays from the others.
//!
//!     cargo run -p avb-net --example phc_watch -- <index> <seconds>

use std::time::{Duration, Instant};

use avb_net::clock::{HardwareClock, monotonic_now};

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let index = args.next().and_then(|text| text.parse().ok()).unwrap_or(0);
    let seconds = args.next().and_then(|text| text.parse().ok()).unwrap_or(60);
    let clock = HardwareClock::open(index)?;
    let until = Instant::now() + Duration::from_secs(seconds);
    let mut last: Option<i128> = None;
    let mut reads = 0u64;
    let mut strays = 0u64;
    while Instant::now() < until {
        let before = monotonic_now().as_nanos() as i128;
        let phc = clock.now()?.as_nanos() as i128;
        let after = monotonic_now().as_nanos() as i128;
        let offset = phc - (before + after) / 2;
        reads += 1;
        if let Some(previous) = last
            && (offset - previous).abs() > 100_000
        {
            strays += 1;
            println!(
                "read {reads}: offset moved {} ns (read took {} ns)",
                offset - previous,
                after - before
            );
        }
        last = Some(offset);
        std::thread::sleep(Duration::from_micros(500));
    }
    println!("{reads} reads, {strays} strays");
    Ok(())
}
